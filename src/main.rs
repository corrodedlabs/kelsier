#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

mod errors {
    error_chain! {}
}

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use ash::{
    version::{EntryV1_0, InstanceV1_0},
    vk, vk_make_version,
};

use std::{
    ffi::{CStr, CString},
    os::raw::c_void,
    ptr,
};

use kelsier::foreign;
use kelsier::platforms;

use errors::*;

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };

    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };

    let message = CStr::from_ptr((*p_callback_data).p_message);
    println!("[Debug]{}{}{:?}", severity, types, message);

    vk::FALSE
}

// Constants

const WINDOW_TITLE: &'static str = "Kelsier";
const ENABLE_VALIDATION: bool = true;

const VALIDATION_LAYER: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub const APPLICATION_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const ENGINE_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const API_VERSION: u32 = vk_make_version!(1, 0, 92);

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;

// Vulkan Instance
struct VulkanInstance {
    _entry: ash::Entry,
    instance: ash::Instance,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanInstance {
    // // Checking for validation
    fn check_validation_layer_support(entry: &ash::Entry) -> bool {
        // if support validation layer, then return true

        let layer_properties = entry
            .enumerate_instance_layer_properties()
            .expect("Failed to enumerate Instance Layers Properties!");

        if layer_properties.len() <= 0 {
            eprintln!("No available layers.");
            return false;
        } else {
            println!("Instance Available Layers: ");
            for layer in layer_properties.iter() {
                let layer_name = foreign::vk_to_string(&layer.layer_name);
                println!("\t{}", layer_name);
            }
        }

        // layer_properties
        //     .iter()
        //     .find(|layer_property| {
        //         VALIDATION_LAYER
        //             .first()
        //             .iter()
        //             .filter(|layer_name| {
        //                 foreign::vk_to_string(&layer_property.layer_name) == *layer_name
        //             })
        //             .is_some()
        //     })
        //     .is_some()
        true
    }

    fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT {
            s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: ptr::null(),
            flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
            // vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
            // vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            pfn_user_callback: Some(vulkan_debug_utils_callback),
            p_user_data: ptr::null_mut(),
        }
    }

    fn create_instance(entry: &ash::Entry) -> Result<ash::Instance> {
        if ENABLE_VALIDATION && VulkanInstance::check_validation_layer_support(entry) == false {
            panic!("Validation layers requested, but not available");
        }

        let app_name = CString::new(WINDOW_TITLE).chain_err(|| "window title is null")?;
        let engine_name = CString::new("Kelsier").chain_err(|| "invalid engine name")?;

        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: ptr::null(),
            application_version: APPLICATION_VERSION,
            p_engine_name: engine_name.as_ptr(),
            engine_version: ENGINE_VERSION,
            api_version: API_VERSION,
        };

        let debug_utils_create_info = VulkanInstance::populate_debug_messenger_create_info();

        // Debug utils extension also requested here
        let extension_names = platforms::required_extension_names();

        // CString vec should be in scope for the ptr conversion
        let raw_enabled_layer_names: Vec<CString> = VALIDATION_LAYER
            .iter()
            .map(|layer_name| CString::new(*layer_name).unwrap())
            .collect();

        let enabled_layer_names: Vec<*const i8> = raw_enabled_layer_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();

        println!("enabled layer {:?}", VALIDATION_LAYER);

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: if ENABLE_VALIDATION {
                &debug_utils_create_info as *const vk::DebugUtilsMessengerCreateInfoEXT
                    as *const c_void
            } else {
                ptr::null()
            },
            flags: vk::InstanceCreateFlags::empty(),
            p_application_info: &app_info,
            pp_enabled_layer_names: if ENABLE_VALIDATION {
                enabled_layer_names.as_ptr()
            } else {
                ptr::null()
            },
            enabled_layer_count: if ENABLE_VALIDATION {
                enabled_layer_names.len()
            } else {
                0
            } as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
        };

        unsafe {
            entry
                .create_instance(&create_info, None)
                .chain_err(|| "failed to create instance")
        }
    }

    fn setup_debug_utils(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> (ash::extensions::ext::DebugUtils, vk::DebugUtilsMessengerEXT) {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

        if ENABLE_VALIDATION {
            (debug_utils_loader, ash::vk::DebugUtilsMessengerEXT::null())
        } else {
            let messenger_info = VulkanInstance::populate_debug_messenger_create_info();

            let utils_messenger = unsafe {
                debug_utils_loader
                    .create_debug_utils_messenger(&messenger_info, None)
                    .expect("Debug utils callback")
            };

            (debug_utils_loader, utils_messenger)
        }
    }

    pub fn new() -> Result<VulkanInstance> {
        let entry = ash::Entry::new().chain_err(|| "cannot load ash entry")?;
        let instance = VulkanInstance::create_instance(&entry)?;

        let (debug_utils_loader, debug_messenger) =
            VulkanInstance::setup_debug_utils(&entry, &instance);

        Ok(VulkanInstance {
            _entry: entry,
            instance,
            debug_utils_loader,
            debug_messenger,
        })
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            if ENABLE_VALIDATION {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

struct VulkanApp {
    instance: VulkanInstance,
}

impl VulkanApp {
    fn init_window(event_loop: &EventLoop<()>) -> Result<Window> {
        WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .chain_err(|| "failed to create window")
    }

    pub fn run_game_loop(self, event_loop: EventLoop<()>, window: Window) {
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            virtual_keycode,
                            state,
                            ..
                        } => match (virtual_keycode, state) {
                            (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                *control_flow = ControlFlow::Exit
                            }

                            _ => (),
                        },
                    },

                    _ => (),
                },

                Event::MainEventsCleared => window.request_redraw(),

                // todo draw frame on this
                Event::RedrawRequested(_window_id) => (),

                _ => (),
            }
        });
    }

    pub fn new() -> Result<VulkanApp> {
        VulkanInstance::new().map(|instance| VulkanApp { instance })
    }
}

fn run() -> Result<VulkanApp> {
    VulkanApp::new()
}

fn main() {
    match run() {
        Ok(app) => {
            let event_loop = EventLoop::new();
            let window = VulkanApp::init_window(&event_loop).expect("cannot create window");

            app.run_game_loop(event_loop, window)
        }
        Err(ref e) => {
            use std::io::Write;

            let stderr = &mut ::std::io::stderr();
            let errmsg = "Error writing to stderr";

            writeln!(stderr, "error: {}", e).expect(errmsg);

            for e in e.iter().skip(1) {
                writeln!(stderr, "caused by: {}", e).expect(errmsg);
            }

            if let Some(backtrace) = e.backtrace() {
                writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
            }

            ::std::process::exit(1);
        }
    }
}
