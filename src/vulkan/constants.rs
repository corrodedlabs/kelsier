use ash::vk_make_version;

pub const ENABLE_VALIDATION: bool = true;

pub const VALIDATION_LAYER: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub const APPLICATION_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const ENGINE_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const API_VERSION: u32 = vk_make_version!(1, 0, 92);

pub const WINDOW_TITLE: &'static str = "Kelsier";
