use ash::version::DeviceV1_0;
use ash::vk;

use anyhow::anyhow;
use anyhow::{Context, Result};

use super::{buffers, device};

pub struct TransitionBarrier {
    src_access_mask: vk::AccessFlags,
    dst_access_mask: vk::AccessFlags,
    source_stage: vk::PipelineStageFlags,
    destination_stage: vk::PipelineStageFlags,
}

impl TransitionBarrier {
    pub fn from_layout(
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<TransitionBarrier> {
        match old_layout {
            vk::ImageLayout::UNDEFINED => match new_layout {
                vk::ImageLayout::TRANSFER_DST_OPTIMAL => Ok(TransitionBarrier {
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                    destination_stage: vk::PipelineStageFlags::TRANSFER,
                }),

                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => Ok(TransitionBarrier {
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                        | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                    destination_stage: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                }),

                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => Ok(TransitionBarrier {
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                    destination_stage: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                }),

                _ => Err(anyhow!("unsupported new_layout for transition")),
            },

            vk::ImageLayout::TRANSFER_DST_OPTIMAL
                if new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL =>
            {
                Ok(TransitionBarrier {
                    src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                    source_stage: vk::PipelineStageFlags::TRANSFER,
                    destination_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
                })
            }

            _ => Err(anyhow!("unsupported old_layout for transition")),
        }
    }
}

pub struct ImageData {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub memory: vk::DeviceMemory,
}

impl ImageData {
    fn create_image(
        device: &device::Device,
        width: u32,
        height: u32,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, vk::DeviceMemory)> {
        let image_create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_SRGB,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            ..Default::default()
        };

        let image = unsafe {
            device
                .logical_device
                .create_image(&image_create_info, None)
                .context("Failed to create texture image!")
        }?;

        let image_memory_requirement =
            unsafe { device.logical_device.get_image_memory_requirements(image) };
        let memory_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: image_memory_requirement.size,
            memory_type_index: device.are_properties_supported(
                image_memory_requirement.memory_type_bits,
                required_memory_properties,
            )?,
            ..Default::default()
        };

        let image_memory = unsafe {
            device
                .logical_device
                .allocate_memory(&memory_allocate_info, None)
                .context("failed to allocate texture image memory!")
        }?;

        unsafe {
            device
                .logical_device
                .bind_image_memory(image, image_memory, 0)
                .context("Failed to bind image memory!")
        }?;

        Ok((image, image_memory))
    }

    pub fn has_stencil_component(format: vk::Format) -> bool {
        format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D24_UNORM_S8_UINT
    }

    pub fn transition_image_layout(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        mip_levels: u32,
    ) -> Result<()> {
        let transition_barrier_info = TransitionBarrier::from_layout(old_layout, new_layout)?;

        let aspect_mask = match new_layout {
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                if ImageData::has_stencil_component(format) {
                    vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
                } else {
                    vk::ImageAspectFlags::DEPTH
                }
            }

            _ => vk::ImageAspectFlags::COLOR,
        };

        let image_barriers = [vk::ImageMemoryBarrier {
            src_access_mask: transition_barrier_info.src_access_mask,
            dst_access_mask: transition_barrier_info.dst_access_mask,
            old_layout: old_layout,
            new_layout: new_layout,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        }];

        buffers::CommandBuffer::record_and_submit_single_command(
            device,
            command_pool,
            submit_queue,
            |command_buffer| unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    transition_barrier_info.source_stage,
                    transition_barrier_info.destination_stage,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &image_barriers,
                )
            },
        )
    }

    pub fn copy_buffer_to_image(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) -> Result<()> {
        ImageData::transition_image_layout(
            device,
            command_pool,
            submit_queue,
            image,
            vk::Format::R8G8B8A8_SNORM,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            1,
        )?;

        let buffer_image_regions = [vk::BufferImageCopy {
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            ..Default::default()
        }];

        buffers::CommandBuffer::record_and_submit_single_command(
            device,
            command_pool,
            submit_queue,
            |command_buffer| unsafe {
                device.cmd_copy_buffer_to_image(
                    command_buffer,
                    buffer,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &buffer_image_regions,
                )
            },
        )?;

        ImageData::transition_image_layout(
            device,
            command_pool,
            submit_queue,
            image,
            vk::Format::R8G8B8A8_SNORM,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            1,
        )
    }

    pub fn create_image_view(
        device: &ash::Device,
        image: vk::Image,
        format: vk::Format,
        mip_levels: u32,
    ) -> Result<vk::ImageView> {
        let imageview_create_info = vk::ImageViewCreateInfo {
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            },
            image,
            ..Default::default()
        };

        unsafe {
            device
                .create_image_view(&imageview_create_info, None)
                .context("Failed to create image view!")
        }
    }

    pub fn new(
        device: &device::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        data_buffer: buffers::BufferInfo,
        width: u32,
        height: u32,
    ) -> Result<ImageData> {
        let (image, memory) =
            ImageData::create_image(device, width, height, vk::MemoryPropertyFlags::DEVICE_LOCAL)?;

        ImageData::copy_buffer_to_image(
            &device.logical_device,
            command_pool,
            graphics_queue,
            data_buffer.buffer,
            image,
            width,
            height,
        )?;

        let image_view = ImageData::create_image_view(
            &device.logical_device,
            image,
            vk::Format::R8G8B8A8_UNORM,
            0,
        )?;

        Ok(ImageData {
            image,
            image_view,
            memory,
        })
    }
}
