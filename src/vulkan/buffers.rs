use std::ffi::CString;

use ash::version::DeviceV1_0;
use ash::vk;

use anyhow::anyhow;
use anyhow::{Context, Result};

use super::device;
use super::pipeline;
use super::queue;
use super::swapchain;

pub struct BufferDetails {
    pub framebuffers: Vec<vk::Framebuffer>,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
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
        render_pass: vk::RenderPass,
        surface_extent: vk::Extent2D,
    ) -> Result<Vec<vk::CommandBuffer>> {
        let command_buffer_info = vk::CommandBufferAllocateInfo {
            command_buffer_count: framebuffers.len() as u32,
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            ..Default::default()
        };

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_info)
                .context("failed to allocate command buffers")
        }?;

        // recording command buffers

        command_buffers
            .iter()
            .zip(framebuffers.iter())
            .map(|(&command_buffer, &framebuffer)| {
                let begin_info = vk::CommandBufferBeginInfo {
                    flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
                    ..Default::default()
                };

                unsafe {
                    device
                        .begin_command_buffer(command_buffer, &begin_info)
                        .context("failed to start command buffer recording")
                }?;

                let clear_values = [vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                }];

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

                    device.cmd_draw(command_buffer, 3, 1, 0, 0);

                    device.cmd_end_render_pass(command_buffer);

                    device
                        .end_command_buffer(command_buffer)
                        .context("failed to end command buffer recording")
                }
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(command_buffers)
    }

    pub fn new(
        device: device::Device,
        pipeline: pipeline::PipelineDetail,
        render_pass: vk::RenderPass,
        swapchain_details: swapchain::SwapchainDetails,
    ) -> Result<BufferDetails> {
        let logical_device = &device.logical_device;

        let framebuffers = BufferDetails::create_framebuffers(
            logical_device,
            render_pass,
            &swapchain_details.image_views,
            swapchain_details.extent,
        )?;

        let command_pool =
            BufferDetails::create_command_pool(logical_device, &device.family_indices)?;

        let command_buffers = BufferDetails::create_command_buffers(
            logical_device,
            command_pool,
            pipeline,
            &framebuffers,
            render_pass,
            swapchain_details.extent,
        )?;

        Ok(BufferDetails {
            framebuffers,
            command_pool,
            command_buffers,
        })
    }
}
