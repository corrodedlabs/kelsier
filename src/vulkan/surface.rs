use ash::vk;

use super::instance::VulkanInstance;

use crate::platforms;
use anyhow::{Context, Result};

pub struct SurfaceInfo {
    pub loader: ash::extensions::khr::Surface,
    pub surface: vk::SurfaceKHR,

    screen_width: u32,
    screen_height: u32,
}

impl SurfaceInfo {
    pub fn new(
        instance: &VulkanInstance,
        window: &winit::window::Window,
        screen_width: u32,
        screen_height: u32,
    ) -> Result<SurfaceInfo> {
        let loader = ash::extensions::khr::Surface::new(&instance.entry, &instance.instance);
        unsafe {
            platforms::create_surface(&instance.entry, &instance.instance, window)
                .context("Failed to create surface.")
        }
        .map(|surface| SurfaceInfo {
            loader,
            surface,
            screen_width,
            screen_height,
        })
    }
}
