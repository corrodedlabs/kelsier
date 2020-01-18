use super::device;
use super::surface;

use ash::vk;

use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;

pub struct FamilyIndices {
    pub graphics: Option<u32>,
    pub present: Option<u32>,
}

impl FamilyIndices {
    pub fn new(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surface_info: &surface::SurfaceInfo,
    ) -> FamilyIndices {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut indices = FamilyIndices {
            graphics: None,
            present: None,
        };

        let mut i = 0;
        for family in queue_families.iter() {
            if family.queue_count > 0 && family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics = Some(i);
            }

            let is_present_support = unsafe {
                surface_info.loader.get_physical_device_surface_support(
                    physical_device,
                    i as u32,
                    surface_info.surface,
                )
            };
            if family.queue_count > 0 && is_present_support {
                indices.present = Some(i);
            }

            if indices.is_available() {
                break;
            }

            i += 1;
        }

        indices
    }

    pub fn is_available(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }

    pub fn get_unique(&self) -> std::collections::HashSet<u32> {
        if self.is_available() {
            vec![self.graphics.unwrap(), self.present.unwrap()]
                .into_iter()
                .collect()
        } else {
            std::collections::HashSet::new()
        }
    }
}

pub struct Queue {
    pub graphics: vk::Queue,
    pub present: vk::Queue,
}

impl Queue {
    pub fn new(device: device::Device) -> Queue {
        let graphics = unsafe {
            device
                .logical_device
                .get_device_queue(device.family_indices.graphics.unwrap(), 0)
        };

        let present = unsafe {
            device
                .logical_device
                .get_device_queue(device.family_indices.present.unwrap(), 0)
        };

        Queue { graphics, present }
    }
}
