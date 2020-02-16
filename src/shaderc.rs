use shaderc;

use std::fs::File;
use std::io::prelude::*;

use anyhow::anyhow;
use anyhow::{Context, Result};

pub struct ShaderSource {
    pub vertex_shader_file: String,
    pub fragment_shader_file: String,
}

pub struct CompiledShader {
    pub vertex: Vec<u32>,
    pub fragment: Vec<u32>,
}

impl ShaderSource {
    fn read_file(filename: &String) -> Result<String> {
        let mut file = File::open(filename).context(format!("cannot open file {}", filename))?;
        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .map(|_| contents)
            .context(format!("error reading file to string: {}", filename))
    }

    pub fn compile(&self) -> Result<CompiledShader> {
        let vertex_shader = ShaderSource::read_file(&self.vertex_shader_file)?;
        let fragment_shader = ShaderSource::read_file(&self.fragment_shader_file)?;

        let mut compiler = shaderc::Compiler::new().context("cannot init shaderc compiler")?;

        let options =
            shaderc::CompileOptions::new().context("cannot init shaderc compiler options")?;

        let vertex_shader_result = compiler
            .compile_into_spirv(
                &vertex_shader,
                shaderc::ShaderKind::Vertex,
                &self.vertex_shader_file,
                "main",
                Some(&options),
            )
            .context("failed to compile vertex shader")?;

        let fragment_shader_result = compiler
            .compile_into_spirv(
                &fragment_shader,
                shaderc::ShaderKind::Fragment,
                &self.fragment_shader_file,
                "main",
                Some(&options),
            )
            .context("failed to compile fragment shader")?;

        Ok(CompiledShader {
            vertex: vertex_shader_result.as_binary().to_vec(),
            fragment: fragment_shader_result.as_binary().to_vec(),
        })
    }
}
