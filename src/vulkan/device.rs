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

use std::ffi::{CStr, CString};
use std::collections::HashSet;

pub struct Device {
    pub physical_device: vk::PhysicalDevice,
    pub logical_device: ash::Device,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub family_indices: queue::FamilyIndices,
}

pub struct DeviceExtension {
    pub names: [&'static str; 1],
}

pub const DEVICE_EXTENSIONS: DeviceExtension = DeviceExtension {
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

        let mut available_extension_names = HashSet::new();

        for extension in available_extensions.iter() {
            let extension_name = foreign::vk_to_string(&extension.extension_name);

            available_extension_names.insert(extension_name);
        }

        let mut required_extensions = HashSet::new();
        // can directly convert device_extensions to set and check for subset, but for now it's fine
        for extension in device_extensions.names.iter() {
            required_extensions.insert(extension.to_string());
        }

        return Ok(available_extension_names.is_superset(&required_extensions));
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

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surface_info: &surface::SurfaceInfo,
    ) -> Result<(ash::Device, queue::FamilyIndices)> {
        let indices = queue::FamilyIndices::new(instance, physical_device, surface_info);
        let unique_families = indices.get_unique();

        let queue_priorities = [1.0_f32];

        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_families
            .iter()
            .map(|queue_family| vk::DeviceQueueCreateInfo {
                s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DeviceQueueCreateFlags::empty(),
                queue_family_index: *queue_family,
                p_queue_priorities: queue_priorities.as_ptr(),
                queue_count: queue_priorities.len() as u32,
            })
            .collect();

        let physical_device_features = vk::PhysicalDeviceFeatures {
            sampler_anisotropy: vk::TRUE,
            ..Default::default()
        };

        let extension_names = &DEVICE_EXTENSIONS.get_raw_names();

        // let enabled_layers = EnabledLayers::query();

        let raw_enabled_layer_names: Vec<CString> = VALIDATION_LAYER
            .iter()
            .map(|layer_name| CString::new(*layer_name).unwrap())
            .collect();

        let enabled_layer_names: Vec<*const i8> = raw_enabled_layer_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();

        let layers = EnabledLayers {
            count: if ENABLE_VALIDATION {
                enabled_layer_names.len()
            } else {
                0
            } as u32,
            names: if ENABLE_VALIDATION {
                enabled_layer_names.as_ptr()
            } else {
                &std::ptr::null()
            },
        };

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: queue_create_infos.len() as u32,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_layer_count: layers.count,
            pp_enabled_layer_names: layers.names,
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            p_enabled_features: &physical_device_features,
        };

        unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .context("failed to create logical device")
        }
        .map(|device| (device, indices))
    }

    pub fn are_properties_supported(
        &self,
        type_filter: u32,
        required_properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        self.memory_properties
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

    pub fn new(instance: &ash::Instance, surface_info: &surface::SurfaceInfo) -> Result<Device> {
        let physical_device = Device::pick_physical_device(instance, surface_info)?;

        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        let (logical_device, family_indices) =
            Device::create_logical_device(instance, physical_device, surface_info)?;

        Ok(Device {
            physical_device,
            logical_device,
            memory_properties,
            family_indices,
        })
    }
}
