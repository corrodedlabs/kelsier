use ash::vk;

use super::surface;

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
