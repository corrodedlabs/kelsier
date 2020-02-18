use ash::{version::DeviceV1_0, vk};

use super::vulkan::pipeline;

use memoffset::offset_of;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VertexData {
    pub pos: [f32; 2],
    pub color: [f32; 3],
}

pub const VERTICES: [VertexData; 4] = [
    VertexData {
        pos: [-0.5, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    VertexData {
        pos: [0.5, -0.5],
        color: [0.0, 1.0, 0.0],
    },
    VertexData {
        pos: [0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
    VertexData {
        pos: [-0.5, 0.5],
        color: [1.0, 1.0, 1.0],
    },
];

pub const INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

impl pipeline::VertexData for VertexData {
    fn get_input_binding_description(&self) -> Vec<vk::VertexInputBindingDescription> {
        // All of our per-vertex data is packed together in one array,
        // so we're only going to have one binding.
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: ::std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
        .to_vec()
    }

    fn get_attribute_description(&self) -> Vec<vk::VertexInputAttributeDescription> {
        [vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Self, pos) as u32,
        }]
        .to_vec()
    }
}
