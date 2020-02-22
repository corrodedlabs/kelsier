use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use ash::{
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
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
        mut frame: sync::Objects<app::UniformBuffer>,
    ) -> Result<()> {
        event_loop.run(move |event, _, control_flow| {
            // *control_flow = ControlFlow::Wait;

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
                        Ok(_) => (),
                        Err(e) => {
                            println!("Error occurred: {}", e);
                            panic!(e)
                        }
                    };
                }

                Event::LoopDestroyed => unsafe {
                    frame
                        .device
                        .device_wait_idle()
                        .expect("failed to wait evice idele!")
                },

                _ => (),
            }
        });
    }

    pub fn setup(
        &self,
        window: &winit::window::Window,
    ) -> Result<sync::Objects<app::UniformBuffer>> {
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
        println!("swapchain created");

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
        println!("pipeline created");

        let uniform_buffer_data = app::UniformBuffer::new(swapchain.extent);

        let buffer_details = buffers::BufferDetails::new(
            &device,
            queue.graphics,
            pipeline_detail,
            &swapchain,
            app::VERTICES.to_vec(),
            app::INDICES.to_vec(),
            uniform_buffer_data,
            std::path::Path::new("textures/winter.jpeg"),
        )?;
        println!("buffers created");

        // For some reason frames in flight needs to be set to 3 as only 3 uniform buffers are being created in macOS.
        //TODO: Need to fix this
        sync::Objects::new(device.logical_device, queue, swapchain, buffer_details, 8)
    }

    pub fn new() -> Result<VulkanApp> {
        instance::VulkanInstance::new().map(|instance| VulkanApp { instance })
    }
}

fn main() -> Result<()> {
    let app = VulkanApp::new()?;
    let event_loop = EventLoop::new();
    let window = VulkanApp::init_window(&event_loop).expect("cannot create window");

    let frame = match app.setup(&window) {
        Ok(obj) => obj,
        Err(e) => {
            println!("Setup failed {:?}", e);
            panic!(e);
        }
    };

    app.run_game_loop(event_loop, window, frame)
}
