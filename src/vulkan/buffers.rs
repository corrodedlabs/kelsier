use std::ffi::CString;

use ash::version::DeviceV1_0;
use ash::vk;

use anyhow::anyhow;
use anyhow::{Context, Result};

use super::device;
use super::pipeline;
use super::queue;
use super::swapchain;

struct CommandBuffer {}

impl CommandBuffer {
    pub fn record_and_submit_single_command<F>(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        f: F,
    ) -> Result<()>
    where
        F: Fn(vk::CommandBuffer),
    {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo {
            command_buffer_count: 1,
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            ..Default::default()
        };

        let command_buffer = unsafe {
            device
                .allocate_command_buffers(&command_buffer_alloc_info)
                .context("failed to allocate command buffers")
        }?[0];

        let command_buffer_begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .context("failed to begin command buffer recording")
        }?;

        // Call hof to record command
        f(command_buffer);

        unsafe {
            device
                .end_command_buffer(command_buffer)
                .context("failed to end command buffer recording")
        }?;

        let buffers = [command_buffer];

        let submit_infos = [vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: buffers.as_ptr(),
            ..Default::default()
        }];

        unsafe {
            device
                .queue_submit(graphics_queue, &submit_infos, vk::Fence::null())
                .and_then(|_| device.queue_wait_idle(graphics_queue))
                .context("failed to submit command buffer to graphics queue")
                .map(|_| device.free_command_buffers(command_pool, &buffers))
        }
    }

    pub fn record_command_to_buffers<F>(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        num_buffers: u32,
        f: F,
    ) -> Result<Vec<vk::CommandBuffer>>
    where
        F: Fn(usize, vk::CommandBuffer),
    {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo {
            command_buffer_count: num_buffers,
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            ..Default::default()
        };

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_alloc_info)
                .context("failed to allocate command buffers")
        }?;

        command_buffers
            .iter()
            .enumerate()
            .map(|(i, command_buffer)| {
                let begin_info = vk::CommandBufferBeginInfo {
                    ..Default::default()
                };

                unsafe {
                    device
                        .begin_command_buffer(*command_buffer, &begin_info)
                        .context("failed to begin recording command buffer")
                }?;

                f(i, *command_buffer);

                unsafe {
                    device
                        .end_command_buffer(*command_buffer)
                        .context("failed to end command buffer recording")
                }?;

                Ok(())
            })
            .collect::<Result<Vec<()>>>()
            .map(|_| command_buffers)
    }
}

pub struct BufferInfo {
    buffer: vk::Buffer,
    device_memory: vk::DeviceMemory,
    size: vk::DeviceSize,
}

type VertexBuffer = BufferInfo;
type IndexBuffer = BufferInfo;

impl BufferInfo {
    fn find_memory_type(
        type_filter: u32,
        required_properties: vk::MemoryPropertyFlags,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> Result<u32> {
        mem_properties
            .memory_types
            .iter()
            .enumerate()
            .find(|(i, memory_type)| {
                (type_filter & (1u32 << i)) > 0
                    && memory_type.property_flags.contains(required_properties)
            })
            .map(|(i, _)| i as u32)
            .ok_or(anyhow!("failed to find suitable memory type"))
    }

    fn create(
        device: &ash::Device,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        required_memory_properties: vk::MemoryPropertyFlags,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> Result<BufferInfo> {
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            device
                .create_buffer(&buffer_info, None)
                .context("failed to create buffer")
        }?;

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type = BufferInfo::find_memory_type(
            mem_requirements.memory_type_bits,
            required_memory_properties,
            device_memory_properties,
        )?;

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index: memory_type,
            ..Default::default()
        };

        let buffer_memory = unsafe {
            device
                .allocate_memory(&allocate_info, None)
                .context("Failed to allocate vertex buffer memory!")
        }?;

        unsafe {
            device
                .bind_buffer_memory(buffer, buffer_memory, 0)
                .context("Failed to bind buffer")
        }
        .map(|_| BufferInfo {
            buffer,
            device_memory: buffer_memory,
            size: size,
        })
    }

    fn copy_to_gpu(
        &self,
        device: &ash::Device,
        graphics_queue: vk::Queue,
        command_pool: vk::CommandPool,
        dest: &BufferInfo,
    ) -> Result<()> {
        let copy_regions = [vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: self.size,
        }];

        CommandBuffer::record_and_submit_single_command(
            device,
            command_pool,
            graphics_queue,
            |command_buffer| unsafe {
                device.cmd_copy_buffer(command_buffer, self.buffer, dest.buffer, &copy_regions)
            },
        )
    }

    fn create_gpu_local_buffer<T>(
        device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        usage_flag: vk::BufferUsageFlags,
        data: &[T],
    ) -> Result<BufferInfo> {
        let buffer_size = ::std::mem::size_of_val(data) as vk::DeviceSize;

        let staging_buffer = BufferInfo::create(
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &device_memory_properties,
        )?;

        // copy data from cpu to gpu staging
        unsafe {
            let data_ptr = device
                .map_memory(
                    staging_buffer.device_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .context("failed to map memory")? as *mut T;

            data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());

            device.unmap_memory(staging_buffer.device_memory);
        }

        let gpu_buffer = BufferInfo::create(
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | usage_flag,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &device_memory_properties,
        )?;

        staging_buffer.copy_to_gpu(device, graphics_queue, command_pool, &gpu_buffer)?;

        // todo free staging buffer

        Ok(gpu_buffer)
    }

    pub fn create_vertex_buffer<T>(
        device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        data: &[T],
    ) -> Result<VertexBuffer> {
        BufferInfo::create_gpu_local_buffer(
            device,
            device_memory_properties,
            command_pool,
            graphics_queue,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            data,
        )
    }

    pub fn create_index_buffer(
        device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        data: &[u32],
    ) -> Result<IndexBuffer> {
        BufferInfo::create_gpu_local_buffer(
            device,
            device_memory_properties,
            command_pool,
            graphics_queue,
            vk::BufferUsageFlags::INDEX_BUFFER,
            data,
        )
    }
}

pub struct BufferDetails {
    pub framebuffers: Vec<vk::Framebuffer>,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
}

impl BufferDetails {
    // todo should this fn be in swapchain module?
    fn create_framebuffers(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        image_views: &Vec<vk::ImageView>,
        swapchain_extent: vk::Extent2D,
    ) -> Result<Vec<vk::Framebuffer>> {
        image_views
            .iter()
            .map(|&image_view| {
                let attachments = [image_view];

                let framebuffer_info = vk::FramebufferCreateInfo {
                    render_pass,
                    attachment_count: attachments.len() as u32,
                    p_attachments: attachments.as_ptr(),
                    width: swapchain_extent.width,
                    height: swapchain_extent.height,
                    layers: 1,
                    ..Default::default()
                };

                unsafe {
                    device
                        .create_framebuffer(&framebuffer_info, None)
                        .context("failed to create framebuffer")
                }
            })
            .collect()
    }

    fn create_command_pool(
        device: &ash::Device,
        queue_families: &queue::FamilyIndices,
    ) -> Result<vk::CommandPool> {
        let queue_index = queue_families
            .graphics
            .ok_or_else(|| anyhow!("graphics family index not present"))?;

        let command_pool_info = vk::CommandPoolCreateInfo {
            queue_family_index: queue_index,
            ..Default::default()
        };

        unsafe {
            device
                .create_command_pool(&command_pool_info, None)
                .context("Failed to create command pool!")
        }
    }

    fn create_command_buffers(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        pipeline: pipeline::PipelineDetail,
        framebuffers: &Vec<vk::Framebuffer>,
        vertex_buffer: &VertexBuffer,
        index_buffer: &IndexBuffer,
        render_pass: vk::RenderPass,
        surface_extent: vk::Extent2D,
    ) -> Result<Vec<vk::CommandBuffer>> {
        // recording command buffers
        CommandBuffer::record_command_to_buffers(
            device,
            command_pool,
            framebuffers.len() as u32,
            |i, command_buffer| {
                let clear_values = [vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                }];

                let framebuffer = framebuffers[i];

                let render_pass_begin_info = vk::RenderPassBeginInfo {
                    render_pass,
                    framebuffer: framebuffer,
                    render_area: vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: surface_extent,
                    },
                    clear_value_count: clear_values.len() as u32,
                    p_clear_values: clear_values.as_ptr(),
                    ..Default::default()
                };

                let vertex_buffers = [vertex_buffer.buffer];
                let offsets = [0 as vk::DeviceSize];

                // render pass
                unsafe {
                    device.cmd_begin_render_pass(
                        command_buffer,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    );

                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.pipeline,
                    );

                    device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
                    device.cmd_bind_index_buffer(
                        command_buffer,
                        index_buffer.buffer,
                        0 as vk::DeviceSize,
                        vk::IndexType::UINT16,
                    );

                    // todo replace hard coded 6 with with index_buffer data size
                    device.cmd_draw_indexed(command_buffer, 6, 1, 0, 0, 0);

                    device.cmd_end_render_pass(command_buffer);
                }
            },
        )
    }

    pub fn new(
        device: &device::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        graphics_queue: vk::Queue,
        pipeline: pipeline::PipelineDetail,
        swapchain_details: &swapchain::SwapchainDetails,
        vertex_data: Vec<impl pipeline::VertexData>,
        index_data: Vec<u32>,
    ) -> Result<BufferDetails> {
        let logical_device = &device.logical_device;
        let render_pass = pipeline.render_pass;

        let framebuffers = BufferDetails::create_framebuffers(
            logical_device,
            render_pass,
            &swapchain_details.image_views,
            swapchain_details.extent,
        )?;

        let command_pool =
            BufferDetails::create_command_pool(logical_device, &device.family_indices)?;

        let vertex_buffer = BufferInfo::create_vertex_buffer(
            &device.logical_device,
            device_memory_properties,
            command_pool,
            graphics_queue,
            &vertex_data,
        )?;

        let index_buffer = BufferInfo::create_index_buffer(
            &device.logical_device,
            device_memory_properties,
            command_pool,
            graphics_queue,
            index_data.as_slice(),
        )?;

        let command_buffers = BufferDetails::create_command_buffers(
            logical_device,
            command_pool,
            pipeline,
            &framebuffers,
            &vertex_buffer,
            &index_buffer,
            render_pass,
            swapchain_details.extent,
        )?;

        Ok(BufferDetails {
            framebuffers,
            command_pool,
            command_buffers,
            vertex_buffer,
            index_buffer,
        })
    }
}
