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
use kelsier::vulkan::constants::*;
use kelsier::vulkan::instance;
use kelsier::vulkan::surface;

use anyhow::{Context, Result};

// Constants

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;

struct VulkanApp {
    instance: instance::VulkanInstance,
}

impl VulkanApp {
    fn init_window(event_loop: &EventLoop<()>) -> Result<Window> {
        WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .context("failed to create window")
    }

    pub fn run_game_loop(self, event_loop: EventLoop<()>, window: Window) -> Result<()> {
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
                                *control_flow = ControlFlow::Exit;
                                ()
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

    pub fn setup(&self, window: &winit::window::Window) -> Result<()> {
        let surface_info =
            surface::SurfaceInfo::new(&self.instance, window, WINDOW_WIDTH, WINDOW_HEIGHT)?;

        Ok(())
    }

    pub fn new() -> Result<VulkanApp> {
        instance::VulkanInstance::new().map(|instance| VulkanApp { instance })
    }
}

fn main() -> Result<()> {
    let app = VulkanApp::new()?;
    let event_loop = EventLoop::new();
    let window = VulkanApp::init_window(&event_loop).expect("cannot create window");

    app.setup(&window)?;

    app.run_game_loop(event_loop, window)
}
