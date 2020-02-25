use ash::version::DeviceV1_0;
use ash::vk;

use image;
use image::GenericImageView;

use std::path::Path;

use anyhow::anyhow;
use anyhow::{Context, Result};

use super::{buffers, device, image as img};

// Represents data obtained for raw image file
pub struct RawImage {
    pub object: image::DynamicImage,
    pub data: Vec<u8>,
    pub size: vk::DeviceSize,
}

impl RawImage {
    pub fn new(path: &Path) -> Result<RawImage> {
        let object = image::open(path).map(|i| i.flipv())?;

        let data = match &object {
            image::DynamicImage::ImageBgr8(_)
            | image::DynamicImage::ImageLuma8(_)
            | image::DynamicImage::ImageRgb8(_) => Ok(object.to_rgba().into_raw()),
            image::DynamicImage::ImageBgra8(_)
            | image::DynamicImage::ImageLumaA8(_)
            | image::DynamicImage::ImageRgba8(_) => Ok(object.to_bytes()),
            _ => Err(anyhow!("image cannot be converted to bytes")),
        }?;
        let size = (::std::mem::size_of::<u8>() as u32 * object.width() * object.height() * 4)
            as vk::DeviceSize;

        if size <= 0 {
            Err(anyhow!(format!("failed to load image: {:?}", path)))
        } else {
            Ok(RawImage { object, data, size })
        }
    }
}

pub struct Texture {
    pub image_data: img::ImageData,
    pub sampler: vk::Sampler,
}

impl Texture {
    pub fn create_texture_image(
        device: &device::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
        image_path: &Path,
    ) -> Result<img::ImageData> {
        let image = RawImage::new(image_path)?;

        let texture_property =
            img::ImagePropertyType::texture_property(device, command_pool, submit_queue, image)?;

        img::ImageData::new(device, command_pool, submit_queue, texture_property)
    }

    pub fn create_texture_sampler(device: &ash::Device) -> Result<vk::Sampler> {
        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            max_anisotropy: 16.0,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            anisotropy_enable: vk::TRUE,
            unnormalized_coordinates: vk::FALSE,
            ..Default::default()
        };

        unsafe {
            device
                .create_sampler(&sampler_info, None)
                .context("failed to create sampler!")
        }
    }

    pub fn new(
        device: &device::Device,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
        image_path: &Path,
    ) -> Result<Texture> {
        let image_data =
            Texture::create_texture_image(device, command_pool, submit_queue, image_path)?;

        let sampler = Texture::create_texture_sampler(&device.logical_device)?;

        Ok(Texture {
            image_data,
            sampler,
        })
    }
}
