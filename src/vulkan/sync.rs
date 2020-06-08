use ash::version::DeviceV1_0;
use ash::vk;

use anyhow::anyhow;
use anyhow::{Context, Result};

use super::buffers;
use super::queue;
use super::swapchain;

use std::time::Instant;

pub struct FrameState {
    swapchain_image_index: u32,
    current_frame: usize,
    images_in_flight: Vec<Option<vk::Fence>>,
}

impl FrameState {
    pub fn default(num_swapchain_images: u32) -> FrameState {
        let images_in_flight = (0..num_swapchain_images)
            .into_iter()
            .map(|_| None)
            .collect();

        FrameState {
            swapchain_image_index: 0,
            current_frame: 0,
            images_in_flight,
        }
    }
}

pub struct Objects<T: buffers::UniformBuffers> {
    pub device: ash::Device,
    pub swapchain_details: swapchain::SwapchainDetails,
    pub queue: queue::Queue,
    pub buffers: buffers::BufferDetails<T>,

    pub frames_in_flight: u32,

    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,

    pub in_flight_fences: Vec<vk::Fence>,
    pub start_time: Instant,

    pub frame_state: FrameState,
}

impl<T: buffers::UniformBuffers> Objects<T> {
    pub fn new(
        device: ash::Device,
        queue: queue::Queue,
        swapchain_details: swapchain::SwapchainDetails,
        buffers: buffers::BufferDetails<T>,
        frames_in_flight: u32,
    ) -> Result<Objects<T>> {
        let (image_available_semaphores, render_finished_semaphores) = (0..frames_in_flight)
            .into_iter()
            .map(|_| {
                let semaphore_info = vk::SemaphoreCreateInfo {
                    ..Default::default()
                };

                let available_semaphore = unsafe {
                    device
                        .create_semaphore(&semaphore_info, None)
                        .context("failed to create render available semaphore")
                }?;

                let finished_semaphore = unsafe {
                    device
                        .create_semaphore(&semaphore_info, None)
                        .context("failed to create render finished semaphore")
                }?;

                Ok((available_semaphore, finished_semaphore))
            })
            .collect::<Result<Vec<(_, _)>>>()
            .map(|semaphore_tuple_vec| semaphore_tuple_vec.into_iter().unzip())?;

        let in_flight_fences = (0..frames_in_flight)
            .into_iter()
            .map(|_| {
                let fence_info = vk::FenceCreateInfo {
                    flags: vk::FenceCreateFlags::SIGNALED,
                    ..Default::default()
                };

                unsafe {
                    device
                        .create_fence(&fence_info, None)
                        .context("failed to created in flight fences")
                }
            })
            .collect::<Result<Vec<vk::Fence>>>()?;

        let start_time = Instant::now();

        let frame_state = FrameState::default(swapchain_details.images.len() as u32);

        Ok(Objects {
            device: device,
            queue,
            swapchain_details,
            buffers,
            frames_in_flight,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            start_time,
            frame_state: frame_state,
        })
    }

    fn submit_buffers_to_queue(sync_objects: &Objects<T>, acquired_image_index: u32) -> Result<()> {
        let current_frame = sync_objects.frame_state.current_frame as usize;
        // println!("submitting buffer for frame: {}", current_frame);

        let command_buffer = sync_objects
            .buffers
            .command_buffers
            .get(acquired_image_index as usize)
            .ok_or(anyhow!("could not find buffer for current frame"))?;

        let in_flight_fence = sync_objects
            .in_flight_fences
            .get(current_frame)
            .ok_or(anyhow!(
                "could not find find flight fence for current frame"
            ))?;

        let img_semaphore = sync_objects
            .image_available_semaphores
            .get(current_frame)
            .ok_or(anyhow!("count not find image available semaphore"))?;
        let wait_semaphores = [*img_semaphore];

        let render_semaphore = sync_objects
            .render_finished_semaphores
            .get(current_frame)
            .ok_or(anyhow!(
                "coult not find render finished semaphore for current frame"
            ))?;
        let signal_semaphores = [*render_semaphore];

        let submit_info = vk::SubmitInfo {
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT].as_ptr(),

            command_buffer_count: 1u32,
            p_command_buffers: command_buffer,

            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };

        // Submit to graphics queue
        unsafe {
            sync_objects.device.reset_fences(&[*in_flight_fence])?;
            sync_objects
                .device
                .queue_submit(
                    sync_objects.queue.graphics,
                    &[submit_info],
                    *in_flight_fence,
                )
                .context("failed to submit to graphics queue")
        }?;
        // println!("buffer submitted to graphics queue");

        let swapchains = [sync_objects.swapchain_details.swapchain];

        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: signal_semaphores.len() as u32,
            p_wait_semaphores: signal_semaphores.as_ptr(),
            swapchain_count: 1u32,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &acquired_image_index,
            ..Default::default()
        };

        // Submit to presentation queue
        unsafe {
            sync_objects
                .swapchain_details
                .loader
                .queue_present(sync_objects.queue.present, &present_info)
                .context("could not present to queue")
        }
        .and_then(|is_swapchain_suboptimal| {
            if is_swapchain_suboptimal {
                // recreate swapchain
                Err(anyhow!("swapchain is invalid"))
            } else {
                Ok(())
            }
        })
    }

    pub fn draw_next_frame(&mut self) -> Result<()> {
        // println!("drawing frame");

        let in_flight_fence = self
            .in_flight_fences
            .get(self.frame_state.current_frame)
            .ok_or(anyhow!("could not find fence for current frame"))?;

        unsafe {
            self.device
                .wait_for_fences(&[*in_flight_fence], true, std::u64::MAX)?;
        }

        let image_available_semaphore = self
            .image_available_semaphores
            .get(self.frame_state.current_frame)
            .ok_or(anyhow!("could not find semaphore for current frame"))?;

        let (acquired_image_index, _) = unsafe {
            self.swapchain_details.loader.acquire_next_image(
                self.swapchain_details.swapchain,
                std::u64::MAX,
                *image_available_semaphore,
                vk::Fence::null(),
            )
        }
        .map_err(|err| {
            match err {
                vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    // recreate swapchain
                    anyhow!("swapchain is out of date")
                }
                _ => anyhow!(format!("failed to acquire swapchain images: {}", err)),
            }
        })?;
        // println!(
        //     "acquired image index is {}, current_frame is {}",
        //     acquired_image_index, self.frame_state.current_frame,
        // );

        // println!("images in flight: {:?}", self.frame_state.images_in_flight);

        // updating uniform buffers
        let delta_time = self.start_time.elapsed();
        self.start_time = Instant::now();

        let uniform_buffer = self
            .buffers
            .uniform_buffers
            .get(acquired_image_index as usize)
            .ok_or(anyhow!("could not find uniform buffer for the image"))?;

        self.buffers.uniform_buffer_data.update_buffer(
            &self.device,
            uniform_buffer,
            delta_time.subsec_micros() as f32 / 1000_000.0_f32,
        )?;

        let image_in_flight = self
            .frame_state
            .images_in_flight
            .get(acquired_image_index as usize)
            .ok_or(anyhow!("in flight image fence not found"))?;

        image_in_flight
            .map(|image_in_flight| unsafe {
                // println!(
                //     "waiting for fence; acquired image index is {} ",
                //     acquired_image_index
                // );
                self.device
                    .wait_for_fences(&[image_in_flight], true, std::u64::MAX)
                    .context("failed to wait for in flight fence")
            })
            .transpose()?;
        self.frame_state.images_in_flight[acquired_image_index as usize] = Some(*in_flight_fence);

        Objects::submit_buffers_to_queue(self, acquired_image_index)?;

        self.frame_state.current_frame =
            ((self.frame_state.current_frame + 1) % self.frames_in_flight as usize) as usize;

        Ok(())
    }
}

impl<T: buffers::UniformBuffers> Iterator for Objects<T> {
    type Item = Result<()>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.draw_next_frame())
    }
}
