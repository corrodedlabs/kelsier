use std::ffi::CString;

use ash::version::DeviceV1_0;
use ash::vk;

use anyhow::anyhow;
use anyhow::{Context, Result};

use crate::shaderc;

use super::swapchain;

pub struct PipelineDetail {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
}

pub trait VertexData<T = Self> {
    fn get_input_binding_description(&self) -> Vec<vk::VertexInputBindingDescription>;
    fn get_attribute_description(&self) -> Vec<vk::VertexInputAttributeDescription>;
}

impl PipelineDetail {
    fn create_shader_module(device: &ash::Device, code: Vec<u8>) -> Result<vk::ShaderModule> {
        let shader_module_info = vk::ShaderModuleCreateInfo {
            code_size: code.len(),
            p_code: code.as_ptr() as *const u32,
            ..Default::default()
        };

        unsafe {
            device
                .create_shader_module(&shader_module_info, None)
                .context("failed to create shader module")
        }
    }

    fn create_render_pass(
        device: &ash::Device,
        surface_format: vk::Format,
    ) -> Result<vk::RenderPass> {
        let color_attachment = vk::AttachmentDescription {
            format: surface_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::CLEAR,
            stencil_store_op: vk::AttachmentStoreOp::STORE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        };

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpasses = [vk::SubpassDescription {
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            ..Default::default()
        }];

        let render_pass_attachments = [color_attachment];

        let subpass_dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        }];

        let renderpass_create_info = vk::RenderPassCreateInfo {
            attachment_count: render_pass_attachments.len() as u32,
            p_attachments: render_pass_attachments.as_ptr(),
            subpass_count: subpasses.len() as u32,
            p_subpasses: subpasses.as_ptr(),
            dependency_count: subpass_dependencies.len() as u32,
            p_dependencies: subpass_dependencies.as_ptr(),
            ..Default::default()
        };

        unsafe {
            device
                .create_render_pass(&renderpass_create_info, None)
                .context("failed to create render pass!")
        }
    }

    pub fn create_graphics_pipeline(
        device: &ash::Device,
        swapchain: &swapchain::SwapchainDetails,
        shaders: shaderc::ShaderSource,
        vertex_data: impl VertexData,
    ) -> Result<PipelineDetail> {
        let extent = swapchain.extent;
        let surface_format = swapchain.format.format;

        println!("compiling shaders..");
        let compiled_shaders = shaders.compile()?;
        println!("shaders compiled");

        let vert_shader_module =
            PipelineDetail::create_shader_module(device, compiled_shaders.vertex)?;
        let frag_shader_module =
            PipelineDetail::create_shader_module(device, compiled_shaders.fragment)?;

        let main_function_name = CString::new("main").context("invalid fn name")?;

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo {
                module: vert_shader_module,
                p_name: main_function_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: frag_shader_module,
                p_name: main_function_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];

        // ..enter
        let binding_description = vertex_data.get_input_binding_description();
        let attribute_description = vertex_data.get_attribute_description();
        println!(
            "descriptions {:?} {:?}",
            binding_description, attribute_description
        );

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: binding_description.len() as u32,
            p_vertex_binding_descriptions: binding_description.as_ptr(),
            vertex_attribute_description_count: attribute_description.len() as u32,
            p_vertex_attribute_descriptions: attribute_description.as_ptr(),
            ..Default::default()
        };

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            primitive_restart_enable: vk::FALSE,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };

        let viewport = vk::Viewport {
            width: extent.width as f32,
            height: extent.height as f32,
            max_depth: 1.0,
            ..Default::default()
        };

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: extent,
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            line_width: 1.0,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::all(),
            blend_enable: vk::FALSE,
            ..Default::default()
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            logic_op: vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: [color_blend_attachment].as_ptr(),
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        };

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            ..Default::default()
        };

        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .context("failed to create pipeline layout")
        }?;

        let render_pass = PipelineDetail::create_render_pass(device, surface_format)?;

        let pipeline_info = vk::GraphicsPipelineCreateInfo {
            stage_count: shader_stages.len() as u32,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &vertex_input_assembly_state_info,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_color_blend_state: &color_blending,
            layout: pipeline_layout,
            base_pipeline_index: -1,
            render_pass,
            ..Default::default()
        };

        println!("going to create pipelines");
        let pipelines = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                //todo handle this with anyhow! somehow
                .expect("failed to create pipelines")
        };

        unsafe {
            device.destroy_shader_module(vert_shader_module, None);
            device.destroy_shader_module(frag_shader_module, None);
        }

        Ok(PipelineDetail {
            pipeline: pipelines[0],
            layout: pipeline_layout,
            render_pass,
        })
    }
}
