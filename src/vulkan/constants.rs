use ash::vk_make_version;

use std::os::raw::c_char;

// Validation Layers

pub const ENABLE_VALIDATION: bool = true;

pub const VALIDATION_LAYER: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub struct EnabledLayers {
    pub count: u32,
    pub names: *const *const c_char,
}

// pub struct EnabledLayers<'a> {
//     pub count: u32,
//     pub names: &'a *const *const c_char,
// }

// impl<'a> EnabledLayers<'a> {
//     pub fn query() -> EnabledLayers<'static> {
//         let raw_enabled_layer_names: Vec<CString> = VALIDATION_LAYER
//             .iter()
//             .map(|layer_name| CString::new(*layer_name).unwrap())
//             .collect();

//         let enabled_layer_names: Vec<*const i8> = raw_enabled_layer_names
//             .iter()
//             .map(|layer_name| layer_name.as_ptr())
//             .collect();

//         EnabledLayers {
//             count: if ENABLE_VALIDATION {
//                 enabled_layer_names.len()
//             } else {
//                 0
//             } as u32,
//             names: if ENABLE_VALIDATION {
//                 &enabled_layer_names.as_ptr()
//             } else {
//                 &std::ptr::null()
//             },
//         }
//     }
// }

// Versions

pub const APPLICATION_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const ENGINE_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const API_VERSION: u32 = vk_make_version!(1, 0, 92);

pub const WINDOW_TITLE: &'static str = "Kelsier";

// Device extensions

pub struct DeviceExtension {
    pub names: [&'static str; 1],
}

impl DeviceExtension {
    pub fn get_raw_names(&self) -> [*const c_char; 1] {
        [ash::extensions::khr::Swapchain::name().as_ptr()]
    }
}
