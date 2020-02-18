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

use kelsier::{
    app, foreign, platforms, shaderc,
    vulkan::constants::*,
    vulkan::{buffers, device, instance, pipeline, queue, surface, swapchain, sync},
};

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

    pub fn run_game_loop(
        self,
        event_loop: EventLoop<()>,
        window: Window,
        mut frame: sync::Objects,
    ) -> Result<()> {
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
                Event::RedrawRequested(_window_id) => {
                    match frame.next().transpose() {
                        _ => (),
                    };
                }

                _ => (),
            }
        });
    }

    pub fn setup(&self, window: &winit::window::Window) -> Result<sync::Objects> {
        let surface_info =
            surface::SurfaceInfo::new(&self.instance, window, WINDOW_WIDTH, WINDOW_HEIGHT)?;

        let device = device::Device::new(&self.instance.instance, &surface_info)?;

        let queue = queue::Queue::new(&device);

        let swapchain = swapchain::SwapchainDetails::new(
            &self.instance.instance,
            &device,
            window,
            &device.family_indices,
            &surface_info,
        )?;

        let shaders = shaderc::ShaderSource {
            vertex_shader_file: "shaders/shader.vert".to_string(),
            fragment_shader_file: "shaders/shader.frag".to_string(),
        };

        let pipeline_detail = pipeline::PipelineDetail::create_graphics_pipeline(
            &device.logical_device,
            &swapchain,
            shaders,
            app::VERTICES[0],
        )?;

        let device_memory_properties = unsafe {
            self.instance
                .instance
                .get_physical_device_memory_properties(device.physical_device)
        };
        let buffer_details = buffers::BufferDetails::new(
            &device,
            &device_memory_properties,
            queue.graphics,
            pipeline_detail,
            &swapchain,
            app::VERTICES.to_vec(),
            app::INDICES.to_vec(),
        )?;

        sync::Objects::new(device.logical_device, queue, swapchain, buffer_details, 4)
    }

    pub fn new() -> Result<VulkanApp> {
        instance::VulkanInstance::new().map(|instance| VulkanApp { instance })
    }
}

fn main() -> Result<()> {
    let app = VulkanApp::new()?;
    let event_loop = EventLoop::new();
    let window = VulkanApp::init_window(&event_loop).expect("cannot create window");

    let frame = app.setup(&window)?;

    app.run_game_loop(event_loop, window, frame)
}
