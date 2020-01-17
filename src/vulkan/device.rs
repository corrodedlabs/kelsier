use ash::version::InstanceV1_0;
use ash::vk;
use std::os::raw::c_char;

use crate::foreign;

use super::constants::*;
use super::instance;
use super::queue;
use super::surface;
use super::swapchain;

use anyhow::anyhow;
use anyhow::{Context, Result};

struct Device {
    physical_device: vk::PhysicalDevice,
    logical_device: vk::Device,
}

struct QueueFamilyIndices {
    graphics: Option<u32>,
    present: Option<u32>,
}

struct DeviceExtension {
    pub names: [&'static str; 1],
}

const DEVICE_EXTENSIONS: DeviceExtension = DeviceExtension {
    names: ["VK_KHR_swapchain"],
};

impl DeviceExtension {
    pub fn get_raw_names(&self) -> [*const c_char; 1] {
        [ash::extensions::khr::Swapchain::name().as_ptr()]
    }
}

impl Device {
    pub fn check_device_extension_support(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device_extensions: &DeviceExtension,
    ) -> Result<bool> {
        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(physical_device)
                .context("Failed to get device extension properties.")
        }?;

        let mut available_extension_names = vec![];

        for extension in available_extensions.iter() {
            let extension_name = foreign::vk_to_string(&extension.extension_name);

            available_extension_names.push(extension_name);
        }

        use std::collections::HashSet;
        let mut required_extensions = HashSet::new();
        for extension in device_extensions.names.iter() {
            required_extensions.insert(extension.to_string());
        }

        for extension_name in available_extension_names.iter() {
            required_extensions.remove(extension_name);
        }

        return Ok(required_extensions.is_empty());
    }

    fn is_physical_device_suitable(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surface_info: &surface::SurfaceInfo,
    ) -> Result<bool> {
        let device_features = unsafe { instance.get_physical_device_features(physical_device) };

        let indices = queue::FamilyIndices::new(instance, physical_device, surface_info);

        let is_queue_family_supported = indices.is_available();

        let is_device_extension_supported =
            Device::check_device_extension_support(instance, physical_device, &DEVICE_EXTENSIONS)?;

        let is_swapchain_supported = if is_device_extension_supported {
            let swapchain_support = swapchain::SupportDetail::query(physical_device, surface_info)?;
            !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
        } else {
            false
        };

        let is_support_sampler_anisotropy = device_features.sampler_anisotropy == 1;

        return Ok(is_queue_family_supported
            && is_device_extension_supported
            && is_swapchain_supported
            && is_support_sampler_anisotropy);
    }

    fn pick_physical_device(
        instance: &ash::Instance,
        surface_info: &surface::SurfaceInfo,
    ) -> Result<vk::PhysicalDevice> {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }?;

        physical_devices
            .iter()
            .flat_map(|physical_device| {
                Device::is_physical_device_suitable(instance, *physical_device, surface_info)
                    .and_then(|is_suitable| {
                        if is_suitable {
                            Ok(physical_device)
                        } else {
                            Err(anyhow!("device not suitable"))
                        }
                    })
            })
            .collect::<Vec<&vk::PhysicalDevice>>()
            .first()
            .map(|physical_device| **physical_device)
            .ok_or(anyhow!("failed to find a gpu"))
    }

    // fn create_logical_device(
    // 	instance: &ash::Instance,
    // 	physical_device: vk::PhysicalDevice,
    // 	validation:
    // )
}
