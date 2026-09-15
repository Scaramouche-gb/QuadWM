use glam::{Mat4, Quat, Vec3};
use smithay::{
    backend::renderer::utils::with_renderer_surface_state,
    wayland::{shell::xdg::ToplevelSurface, shm::with_buffer_contents},
};
use wgpu::util::DeviceExt;

use crate::render::state::ModelUniform;

pub struct WindowQuad {
    pub toplevel: ToplevelSurface,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    // GPU resources
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub model_buffer: wgpu::Buffer,
    pub bind_group: Option<wgpu::BindGroup>,
    pub dimensions: (u32, u32),
}

impl WindowQuad {
    pub fn new(
        toplevel: ToplevelSurface,
        position: Vec3,
        device: &wgpu::Device,
    ) -> Self {
        let scale = Vec3::new(2.0, 1.5, 1.0);
        let rotation = Quat::IDENTITY;
        let model_mat = Mat4::from_scale_rotation_translation(scale, rotation, position);

        let model_uniform = ModelUniform {
            model: model_mat.to_cols_array_2d(),
        };

        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Model Uniform Buffer"),
            contents: bytemuck::cast_slice(&[model_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            toplevel,
            position,
            rotation,
            scale,
            texture: None,
            texture_view: None,
            model_buffer,
            bind_group: None,
            dimensions: (0, 0),
        }
    }

    pub fn ensure_initialized(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sampler: &wgpu::Sampler,
        layout: &wgpu::BindGroupLayout,
    ) {
        if self.bind_group.is_none() {
            // Create a default 256x256 test pattern texture
            let width = 256u32;
            let height = 256u32;
            let mut pattern = Vec::with_capacity((width * height * 4) as usize);

            for y in 0..height {
                for x in 0..width {
                    let is_border = x < 4 || x >= width - 4 || y < 4 || y >= height - 4;
                    let is_check = ((x / 32) + (y / 32)) % 2 == 0;
                    if is_border {
                        pattern.extend_from_slice(&[0, 200, 255, 255]); // Cyan border
                    } else if is_check {
                        pattern.extend_from_slice(&[40, 44, 52, 255]); // Dark grey
                    } else {
                        pattern.extend_from_slice(&[60, 66, 78, 255]); // Lighter grey
                    }
                }
            }

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Placeholder Quad Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &pattern,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Window Quad Bind Group"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.model_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });

            self.texture = Some(texture);
            self.texture_view = Some(texture_view);
            self.bind_group = Some(bind_group);
            self.dimensions = (width, height);
            self.update_transform(queue);
        }
    }

    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    pub fn update_transform(&mut self, queue: &wgpu::Queue) {
        let model_mat = self.model_matrix();
        let model_uniform = ModelUniform {
            model: model_mat.to_cols_array_2d(),
        };
        queue.write_buffer(&self.model_buffer, 0, bytemuck::cast_slice(&[model_uniform]));
    }

    /// Read SHM buffer attached to WlSurface and upload to WGPU texture
    pub fn sync_surface_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sampler: &wgpu::Sampler,
        layout: &wgpu::BindGroupLayout,
    ) {
        let toplevel = self.toplevel.clone();
        let surface = toplevel.wl_surface();

        with_renderer_surface_state(surface, |surface_data| {
            if let Some(buffer) = surface_data.buffer() {
                let _ = with_buffer_contents(buffer, |ptr, len, data| {
                    let width = data.width as u32;
                    let height = data.height as u32;

                    if width == 0 || height == 0 {
                        return;
                    }

                    // Recreate texture if size changed or first time
                    if self.dimensions != (width, height) || self.texture.is_none() {
                        let texture = device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("Wayland Window Texture"),
                            size: wgpu::Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });

                        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("Window Quad Bind Group"),
                            layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: self.model_buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::TextureView(&texture_view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: wgpu::BindingResource::Sampler(sampler),
                                },
                            ],
                        });

                        self.texture = Some(texture);
                        self.texture_view = Some(texture_view);
                        self.bind_group = Some(bind_group);
                        self.dimensions = (width, height);

                        // Maintain aspect ratio in 3D: width = 2.0, height = 2.0 * (height / width)
                        let aspect = height as f32 / width as f32;
                        self.scale = Vec3::new(2.0, 2.0 * aspect, 1.0);
                        self.update_transform(queue);
                    }

                    // Upload raw pixel data
                    if let Some(texture) = &self.texture {
                        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };

                        queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            slice,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(data.stride as u32),
                                rows_per_image: Some(height),
                            },
                            wgpu::Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                        );
                    }
                });
            }
        });
    }
}
