use ash::{version::DeviceV1_0, vk};

use super::vulkan::{buffers, pipeline};

use memoffset::offset_of;

use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector3};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VertexData {
    pub pos: [f32; 2],
    pub color: [f32; 3],
    pub tex_coord: [f32; 2],
}

pub const VERTICES: [VertexData; 4] = [
    VertexData {
        pos: [-0.75, -0.75],
        color: [1.0, 0.0, 0.0],
        tex_coord: [1.0, 0.0],
    },
    VertexData {
        pos: [0.75, -0.75],
        color: [0.0, 1.0, 0.0],
        tex_coord: [0.0, 0.0],
    },
    VertexData {
        pos: [0.75, 0.75],
        color: [0.0, 0.0, 1.0],
        tex_coord: [0.0, 1.0],
    },
    VertexData {
        pos: [-0.75, 0.75],
        color: [1.0, 1.0, 1.0],
        tex_coord: [1.0, 1.0],
    },
];

pub const INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

impl pipeline::VertexData for VertexData {
    fn get_input_binding_description(&self) -> Vec<vk::VertexInputBindingDescription> {
        // All of our per-vertex data is packed together in one array,
        // so we're only going to have one binding.
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: ::std::mem::size_of::<VertexData>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
        .to_vec()
    }

    fn get_attribute_description(&self) -> Vec<vk::VertexInputAttributeDescription> {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(VertexData, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(VertexData, color) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(VertexData, tex_coord) as u32,
            },
        ]
        .to_vec()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UniformBuffer {
    pub model: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
}

impl UniformBuffer {
    pub fn new(extent: vk::Extent2D) -> UniformBuffer {
        let mut projection = cgmath::perspective(
            Deg(45.0),
            extent.width as f32 / extent.height as f32,
            0.1,
            10.0,
        );

        projection[1][1] = projection[1][1] * -1.0;

        UniformBuffer {
            model: Matrix4::from_angle_z(Deg(90.0)),
            view: Matrix4::look_at(
                Point3::new(2.0, 2.0, 2.0),
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
            ),
            proj: projection,
        }
    }
}

impl buffers::UniformBuffers for UniformBuffer {
    type Data = UniformBuffer;

    fn update(&mut self, delta_time: f32) -> () {
        self.model = Matrix4::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), Deg(90.0) * delta_time)
            * self.model;
    }

    fn get_data(self) -> Self::Data {
        UniformBuffer {
            model: self.model,
            view: self.view,
            proj: self.proj,
        }
    }
}
