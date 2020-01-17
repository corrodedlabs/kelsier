use ash::vk;

use std::os::raw::c_char;

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

impl DeviceExtension {
    pub fn get_raw_names(&self) -> [*const c_char; 1] {
        [ash::extensions::khr::Swapchain::name().as_ptr()]
    }
}

impl QueueFamilyIndices {
    pub fn new() -> QueueFamilyIndices {
        QueueFamilyIndices {
            graphics: None,
            present: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }
}

impl Device {
    // fn is_physical_device_suitable(
    //     instance: &ash::Instance,
    //     physical_device: vk::PhysicalDevice,
    //     surface_stuff: &SurfaceStuff,
    //     required_device_extensions: &DeviceExtension,
    // ) -> bool {
    //     let device_features = unsafe { instance.get_physical_device_features(physical_device) };

    //     let indices = find_queue_family(instance, physical_device, surface_stuff);

    //     let is_queue_family_supported = indices.is_complete();
    //     let is_device_extension_supported =
    //         check_device_extension_support(instance, physical_device, required_device_extensions);
    //     let is_swapchain_supported = if is_device_extension_supported {
    //         let swapchain_support = query_swapchain_support(physical_device, surface_stuff);
    //         !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
    //     } else {
    //         false
    //     };
    //     let is_support_sampler_anisotropy = device_features.sampler_anisotropy == 1;

    //     return is_queue_family_supported
    //         && is_device_extension_supported
    //         && is_swapchain_supported
    //         && is_support_sampler_anisotropy;
    // }

    // fn pick_physical_device(
    //     instance: &ash::Instance,
    //     surface_stuff: &SurfaceStuff,
    //     required_device_extensions: &DeviceExtension,
    // ) -> Result<vk::PhysicalDevice> {
    //     let physical_devices = unsafe { instance.enumerate_physical_devices() }?;

    //     physical_devices
    //         .iter()
    //         .find(|physical_device| {
    //             is_physical_device_suitable(
    //                 instance,
    //                 **physical_device,
    //                 surface_stuff,
    //                 required_device_extensions,
    //             );
    //         })
    //         .map(|p_physical_device| *p_physical_device)
    //         .chain_err("Failed to find a suitable GPU!")
    // }

    // fn create_logical_device(
    // 	instance: &ash::Instance,
    // 	physical_device: vk::PhysicalDevice,
    // 	validation:
    // )
}
