use ash::extensions::khr::Swapchain;
use ash::version::DeviceV1_0;
use ash::vk;

use super::constants::*;
use super::device;
use super::surface;
use std::cmp;

use anyhow::anyhow;
use anyhow::{Context, Result};
use ash::vk::Extent2D;

pub struct SupportDetail {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SupportDetail {
    pub fn query(
        physical_device: vk::PhysicalDevice,
        surface_info: &surface::SurfaceInfo,
    ) -> Result<SupportDetail> {
        unsafe {
            let capabilities = surface_info
                .loader
                .get_physical_device_surface_capabilities(physical_device, surface_info.surface)
                .context("failed to query for surface capabilities")?;

            let formats = surface_info
                .loader
                .get_physical_device_surface_formats(physical_device, surface_info.surface)
                .context("failed to query for surface formats")?;

            surface_info
                .loader
                .get_physical_device_surface_present_modes(physical_device, surface_info.surface)
                .context("failed to query for surface present modes")
                .map(|present_modes| SupportDetail {
                    capabilities,
                    formats,
                    present_modes,
                })
        }
    }
}

pub struct SwapchainDetails {
    pub loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
    pub image_views: Vec<vk::ImageView>,
}

impl SwapchainDetails {
    fn choose_format(support_detail: &SupportDetail) -> Result<vk::SurfaceFormatKHR> {
        support_detail
            .formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_UNORM
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .or(support_detail.formats.first())
            .cloned()
            .ok_or(anyhow!("cannot find suitable swapchain format"))
    }

    fn choose_present_mode(support_detail: &SupportDetail) -> Result<vk::PresentModeKHR> {
        support_detail
            .present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .or(support_detail.present_modes.first())
            .cloned()
            .ok_or(anyhow!("cannot find suitable present mode"))
    }

    fn choose_swap_extent(support_detail: &SupportDetail) -> vk::Extent2D {
        /*
        Vulkan tells us to match the resolution of the window by setting the width and height in the currentExtent member.
        However, some window managers do allow us to differ here and this is indicated by setting the width
        and height in currentExtent to a special value: the maximum value of uint32_t.
        In that case we'll pick the resolution that best matches the window within the minImageExtent and maxImageExtent bounds.
        But somehow in either cases same resolution is being picked up {1600, 1200}...strange
        */
        if support_detail.capabilities.current_extent.width != std::u32::MAX {
            println!("Current extent {:?}",support_detail.capabilities.current_extent);
            support_detail.capabilities.current_extent
        } else {
            let mut actual_extent: vk::Extent2D = Extent2D { width: WINDOW_WIDTH, height: WINDOW_HEIGHT };
            actual_extent.width = cmp::max(
                support_detail.capabilities.min_image_extent.width,
                cmp::min(support_detail.capabilities.min_image_extent.width, actual_extent.width));
            actual_extent.height = cmp::max(
                support_detail.capabilities.min_image_extent.height,
                cmp::min(support_detail.capabilities.min_image_extent.height, actual_extent.height));

            actual_extent
        }
    }

    fn create_image_view(
        device: &ash::Device,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
        mip_levels: u32,
    ) -> Result<vk::ImageView> {
        let imageview_info = vk::ImageViewCreateInfo {
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect_flags,
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
                .create_image_view(&imageview_info, None)
                .context("Failed to create image view")
        }
    }

    pub fn new(
        instance: &ash::Instance,
        device: &device::Device,
        window: &winit::window::Window,
        family_indices: &super::queue::FamilyIndices,
        surface_info: &surface::SurfaceInfo,
    ) -> Result<SwapchainDetails> {
        let support = &SupportDetail::query(device.physical_device, surface_info)?;

        let surface_format = SwapchainDetails::choose_format(support)?;
        let present_mode = SwapchainDetails::choose_present_mode(support)?;
        let extent = SwapchainDetails::choose_swap_extent(support);

        let image_count = support.capabilities.max_image_count;
        println!("swapchain image count: {}", image_count);

        let (image_sharing_mode, queue_family_index_count, queue_family_indices) =
            if family_indices.graphics != family_indices.present {
                (
                    vk::SharingMode::CONCURRENT,
                    2,
                    vec![
                        family_indices.graphics.unwrap(),
                        family_indices.present.unwrap(),
                    ],
                )
            } else {
                (vk::SharingMode::EXCLUSIVE, 0, vec![])
            };

        let swapchain_info = vk::SwapchainCreateInfoKHR {
            surface: surface_info.surface,
            min_image_count: image_count,
            image_color_space: surface_format.color_space,
            image_format: surface_format.format,
            image_extent: extent,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: image_sharing_mode,
            p_queue_family_indices: queue_family_indices.as_ptr(),
            queue_family_index_count: queue_family_index_count,
            pre_transform: support.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: present_mode,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            image_array_layers: 1,
            ..Default::default()
        };

        let swapchain_loader = Swapchain::new(instance, &device.logical_device);
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_info, None)
                .context("failed to create swapchain")
        }?;

        let images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .context("failed to get swapchain images")
        }?;

        let image_views = images
            .iter()
            .flat_map(|&image| {
                SwapchainDetails::create_image_view(
                    &device.logical_device,
                    image,
                    surface_format.format,
                    vk::ImageAspectFlags::COLOR,
                    1,
                )
            })
            .collect::<Vec<vk::ImageView>>();

        Ok(SwapchainDetails {
            loader: swapchain_loader,
            swapchain,
            images,
            format: surface_format,
            extent,
            image_views,
        })
    }
}
