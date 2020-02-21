use std::ffi::CString;

use ash::version::DeviceV1_0;
use ash::vk;

use anyhow::anyhow;
use anyhow::{Context, Result};

use super::device;
use super::pipeline;
use super::queue;
use super::swapchain;

pub struct CommandBuffer {}

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

#[derive(Debug, Copy, Clone)]
pub struct BufferInfo {
    pub buffer: vk::Buffer,
    device_memory: vk::DeviceMemory,
    size: vk::DeviceSize,
}

type VertexBuffer = BufferInfo;
type IndexBuffer = BufferInfo;

impl BufferInfo {
    fn create(
        device: &device::Device,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> Result<BufferInfo> {
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            device
                .logical_device
                .create_buffer(&buffer_info, None)
                .context("failed to create buffer")
        }?;

        let mem_requirements =
            unsafe { device.logical_device.get_buffer_memory_requirements(buffer) };
        let memory_type = device.are_properties_supported(
            mem_requirements.memory_type_bits,
            required_memory_properties,
        )?;

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index: memory_type,
            ..Default::default()
        };

        let buffer_memory = unsafe {
            device
                .logical_device
                .allocate_memory(&allocate_info, None)
                .context("Failed to allocate vertex buffer memory!")
        }?;

        unsafe {
            device
                .logical_device
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

    pub fn create_gpu_local_buffer<T>(
        device: &device::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        usage_flag: vk::BufferUsageFlags,
        data: &[T],
        buffer_size: Option<vk::DeviceSize>,
    ) -> Result<BufferInfo> {
        let default_buffer_size = ::std::mem::size_of_val(data) as vk::DeviceSize;
        let buffer_size = buffer_size.unwrap_or(default_buffer_size);

        let staging_buffer = BufferInfo::create(
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // copy data from cpu to gpu staging
        unsafe {
            let data_ptr = device
                .logical_device
                .map_memory(
                    staging_buffer.device_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .context("failed to map memory")? as *mut T;

            data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());

            device
                .logical_device
                .unmap_memory(staging_buffer.device_memory);
        }

        let gpu_buffer = BufferInfo::create(
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | usage_flag,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        staging_buffer.copy_to_gpu(
            &device.logical_device,
            graphics_queue,
            command_pool,
            &gpu_buffer,
        )?;

        // todo free staging buffer

        Ok(gpu_buffer)
    }

    pub fn create_vertex_buffer<T>(
        device: &device::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        data: &[T],
    ) -> Result<VertexBuffer> {
        BufferInfo::create_gpu_local_buffer(
            device,
            command_pool,
            graphics_queue,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            data,
            None,
        )
    }

    pub fn create_index_buffer(
        device: &device::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        data: &[u32],
    ) -> Result<IndexBuffer> {
        BufferInfo::create_gpu_local_buffer(
            device,
            command_pool,
            graphics_queue,
            vk::BufferUsageFlags::INDEX_BUFFER,
            data,
            None,
        )
    }
}

pub trait UniformBuffers: Copy {
    type Data;

    fn create(&self, device: &device::Device) -> Result<BufferInfo> {
        let buffer_size = ::std::mem::size_of::<Self::Data>() as vk::DeviceSize;
        BufferInfo::create(
            device,
            buffer_size as vk::DeviceSize,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
    }

    fn update(&mut self, delta_time: f32) -> ();

    fn get_data(self) -> Self::Data;

    fn update_buffer(
        &mut self,
        device: &ash::Device,
        uniform_buffer: &BufferInfo,
        delta_time: f32,
    ) -> Result<()> {
        let buffer_size = ::std::mem::size_of::<Self::Data>() as u64;

        self.update(delta_time);
        let data = [self.get_data()];

        unsafe {
            let data_ptr = device
                .map_memory(
                    uniform_buffer.device_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .context("failed to map memory")? as *mut Self::Data;

            data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());

            device.unmap_memory(uniform_buffer.device_memory);
        }

        Ok(())
    }

    fn create_descriptor_pool(
        &self,
        device: &ash::Device,
        pool_size_count: u32,
    ) -> Result<vk::DescriptorPool> {
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: pool_size_count,
        };

        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: 1,
            p_pool_sizes: &pool_size,
            max_sets: pool_size_count,
            ..Default::default()
        };

        unsafe {
            device
                .create_descriptor_pool(&pool_info, None)
                .context("failed to create descriptor pool!")
        }
    }

    fn create_descriptor_sets(
        &self,
        device: &ash::Device,
        descriptor_layout: vk::DescriptorSetLayout,
        uniform_buffers: &Vec<BufferInfo>,
    ) -> Result<Vec<vk::DescriptorSet>> {
        let num_sets = uniform_buffers.len();

        let pool = self.create_descriptor_pool(device, num_sets as u32)?;
        let layouts = vec![descriptor_layout; num_sets];

        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: pool,
            descriptor_set_count: num_sets as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };

        let descriptor_sets = unsafe {
            device
                .allocate_descriptor_sets(&alloc_info)
                .context("failed to allocate descriptor sets")
        }?;

        uniform_buffers
            .iter()
            .zip(descriptor_sets)
            .map(|(buffer, descriptor_set)| {
                let buffer_info = [vk::DescriptorBufferInfo {
                    buffer: buffer.buffer,
                    offset: 0,
                    range: ::std::mem::size_of::<Self::Data>() as u64,
                }];

                let descriptor_write = vk::WriteDescriptorSet {
                    dst_set: descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: buffer_info.as_ptr(),
                    ..Default::default()
                };

                unsafe { device.update_descriptor_sets(&[descriptor_write], &[]) };

                Ok(descriptor_set)
            })
            .collect()
    }
}

pub struct BufferDetails<T: UniformBuffers> {
    pub framebuffers: Vec<vk::Framebuffer>,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub uniform_buffers: Vec<BufferInfo>,
    pub uniform_buffer_data: T,
}

impl<T: UniformBuffers> BufferDetails<T> {
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
        descriptor_sets: Vec<vk::DescriptorSet>,
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
                let offsets = [0_u64];
                let descriptor_sets = [descriptor_sets[i]];

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
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.layout,
                        0,
                        &descriptor_sets,
                        &[],
                    );

                    // todo replace hard coded 6 with with index_buffer data size
                    device.cmd_draw_indexed(command_buffer, 6u32, 1, 0, 0, 0);

                    device.cmd_end_render_pass(command_buffer);
                }
            },
        )
    }

    pub fn new(
        device: &device::Device,
        graphics_queue: vk::Queue,
        pipeline: pipeline::PipelineDetail,
        swapchain_details: &swapchain::SwapchainDetails,
        vertex_data: Vec<impl pipeline::VertexData>,
        index_data: Vec<u32>,
        uniform_buffer_data: T,
    ) -> Result<BufferDetails<T>> {
        let logical_device = &device.logical_device;
        let render_pass = pipeline.render_pass;

        println!(
            "num of swapchain images are: {}",
            swapchain_details.image_views.len()
        );

        let framebuffers = BufferDetails::<T>::create_framebuffers(
            logical_device,
            render_pass,
            &swapchain_details.image_views,
            swapchain_details.extent,
        )?;

        let command_pool =
            BufferDetails::<T>::create_command_pool(logical_device, &device.family_indices)?;

        let vertex_buffer =
            BufferInfo::create_vertex_buffer(device, command_pool, graphics_queue, &vertex_data)?;

        let index_buffer = BufferInfo::create_index_buffer(
            device,
            command_pool,
            graphics_queue,
            index_data.as_slice(),
        )?;

        let uniform_buffers = (0..framebuffers.len())
            .map(|_| uniform_buffer_data.create(&device))
            .collect::<Result<Vec<BufferInfo>>>()?;

        let descriptor_sets = uniform_buffer_data.create_descriptor_sets(
            logical_device,
            pipeline.descriptor_set_layout,
            &uniform_buffers,
        )?;

        let command_buffers = BufferDetails::<T>::create_command_buffers(
            logical_device,
            command_pool,
            pipeline,
            &framebuffers,
            &vertex_buffer,
            &index_buffer,
            descriptor_sets,
            render_pass,
            swapchain_details.extent,
        )?;

        Ok(BufferDetails {
            framebuffers,
            command_pool,
            command_buffers,
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            uniform_buffer_data,
        })
    }
}
