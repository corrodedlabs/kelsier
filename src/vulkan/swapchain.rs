use ash::extensions::khr::Swapchain;
use ash::vk;

use super::device;
use super::surface;

use anyhow::anyhow;
use anyhow::{Context, Result};

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
        //  todo this should ideally be:
        //  max(capabilities.minImageExtent.width,
        //  min(capabilities.maxImageExtent.width, actualExtent.width));
        //  actualExtent comes from window dimension
        support_detail.capabilities.current_extent
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

        let image_count = support.capabilities.min_image_count;

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

        unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .context("failed to get swapchain images")
        }
        .map(|images| SwapchainDetails {
            loader: swapchain_loader,
            swapchain,
            images,
            format: surface_format,
            extent,
        })
    }
}
