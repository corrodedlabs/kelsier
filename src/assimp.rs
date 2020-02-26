use assimp::Importer;

use anyhow::anyhow;
use anyhow::{Context, Error, Result};

struct ModelData {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
}

impl ModelData {
    fn create_buffers(mesh: assimp::Mesh) -> Vec<ModelData> {
        let vertex_iter = mesh.vertex_iter();
        let normal_iter = mesh.normal_iter();

        let mut texture_coord_iter = mesh.texture_coords_iter(0);
        // let num_channels = texture_coord_iter.cloned().count();
        // println!("texture coord iter count {}", num_channels);

        let mut vertex_color_iter = mesh.vertex_color_iter(0);

        vertex_iter
            .zip(normal_iter)
            .map(|(vertex, normal)| {
                let position = vertex.into();
                let normal = normal.into();

                let texture = texture_coord_iter.next().unwrap();
                let vertex_color = vertex_color_iter.next().unwrap();

                let uv = [texture.x, texture.x];
                let color = if mesh.has_vertex_colors(0) {
                    [vertex_color.r, vertex_color.g, vertex_color.b]
                } else {
                    [1.0f32, 1.0f32, 1.0f32]
                };

                ModelData {
                    position,
                    normal,
                    uv,
                    color,
                }
            })
            .collect()
    }

    pub fn load_model(file: &std::path::Path) -> Result<Vec<ModelData>> {
        let file_name = file
            .to_str()
            .context("cannot convert file path to string")?;

        let scale = 1.0;

        {
            let mut importer = Importer::new();
            importer.flip_winding_order(true);
            importer.generate_normals(|x| x.enable = true);
            importer.triangulate(true);
            importer.pre_transform_vertices(|x| {
                x.enable = true;
                x.normalize = true;
            });

            importer
                .read_file(file_name)
                .map_err(|s| anyhow!(s))
                .map(|scene| {
                    scene
                        .mesh_iter()
                        .by_ref()
                        .map(ModelData::create_buffers)
                        .flatten()
                        .collect()
                })
        }
    }
}
