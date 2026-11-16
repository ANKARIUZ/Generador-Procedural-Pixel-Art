use std::{error::Error, fmt};

use winit::dpi::PhysicalSize;

/// Factor entero solicitado para la presentación pixel art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelScale(u32);

impl PixelScale {
    pub const DEFAULT: Self = Self(3);

    /// Acepta factores de 1 a 8.
    pub const fn new(value: u32) -> Option<Self> {
        if value >= 1 && value <= 8 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Calcula la textura y el viewport. Cada téxel ocupa un cuadrado entero.
    pub fn layout(self, window: PhysicalSize<u32>) -> Option<PixelLayout> {
        let scale = self.0.min(window.width).min(window.height);
        if scale == 0 {
            return None;
        }
        let internal = PhysicalSize::new(window.width / scale, window.height / scale);
        let size = PhysicalSize::new(internal.width * scale, internal.height * scale);
        Some(PixelLayout {
            internal,
            origin: [
                (window.width - size.width) / 2,
                (window.height - size.height) / 2,
            ],
            size,
        })
    }
}

/// Dimensiones en píxeles físicos y origen superior izquierdo del viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelLayout {
    pub internal: PhysicalSize<u32>,
    pub origin: [u32; 2],
    pub size: PhysicalSize<u32>,
}

/// Modo de presentación de la escena.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderMode {
    #[default]
    Classic,
    Pixel(PixelScale),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderSettingsError {
    InvalidMode,
    InvalidScale,
}

impl fmt::Display for RenderSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidMode => "PCE_RENDER_MODE debe ser classic o pixel",
            Self::InvalidScale => "PCE_PIXEL_SCALE debe ser un entero entre 1 y 8",
        })
    }
}

impl Error for RenderSettingsError {}

impl RenderMode {
    /// Interpreta la configuración de arranque. La escala se aplica en modo pixel.
    pub fn parse(mode: Option<&str>, scale: Option<&str>) -> Result<Self, RenderSettingsError> {
        match mode {
            None | Some("classic") => Ok(Self::Classic),
            Some("pixel") => {
                let scale = match scale {
                    None => PixelScale::DEFAULT,
                    Some(value) => value
                        .parse()
                        .ok()
                        .and_then(PixelScale::new)
                        .ok_or(RenderSettingsError::InvalidScale)?,
                };
                Ok(Self::Pixel(scale))
            }
            Some(_) => Err(RenderSettingsError::InvalidMode),
        }
    }
}

pub(super) const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

struct ColorTarget {
    #[cfg(test)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    binding: wgpu::BindGroup,
}

pub(super) struct PixelArt {
    scale: PixelScale,
    layout: PixelLayout,
    target: ColorTarget,
    binding_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
}

impl PixelArt {
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        scale: PixelScale,
        size: PhysicalSize<u32>,
    ) -> Self {
        let layout = scale
            .layout(size)
            .expect("superficie inicial con tamaño positivo");
        let binding_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Pixel present bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Pixel nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let target = Self::target(device, &binding_layout, &sampler, layout.internal);
        let shader = device.create_shader_module(wgpu::include_wgsl!("upscale.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pixel present layout"),
            bind_group_layouts: &[&binding_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pixel present pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            scale,
            layout,
            target,
            binding_layout,
            sampler,
            pipeline,
        }
    }

    fn target(
        device: &wgpu::Device,
        binding_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        size: PhysicalSize<u32>,
    ) -> ColorTarget {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pixel color"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | if cfg!(test) {
                    wgpu::TextureUsages::COPY_DST
                } else {
                    wgpu::TextureUsages::empty()
                },
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pixel color and sampler"),
            layout: binding_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        ColorTarget {
            #[cfg(test)]
            texture,
            view,
            binding,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) -> PhysicalSize<u32> {
        if let Some(layout) = self.scale.layout(size) {
            if layout.internal != self.layout.internal {
                self.target =
                    Self::target(device, &self.binding_layout, &self.sampler, layout.internal);
                log::debug!(
                    "Textura pixel art: {}x{}",
                    layout.internal.width,
                    layout.internal.height
                );
            }
            self.layout = layout;
        }
        self.internal_size()
    }

    pub fn internal_size(&self) -> PhysicalSize<u32> {
        self.layout.internal
    }

    pub fn presentation_size(&self) -> PhysicalSize<u32> {
        self.layout.size
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.target.view
    }

    pub fn present(&self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pixel nearest pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        let [x, y] = self.layout.origin;
        let size = self.layout.size;
        pass.set_viewport(
            x as f32,
            y as f32,
            size.width as f32,
            size.height as f32,
            0.0,
            1.0,
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.target.binding, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{create_scene_pipeline, DepthTexture, GpuVertex};
    use wgpu::util::DeviceExt;

    fn gpu_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        pollster::block_on(adapter.request_device(&Default::default(), None)).unwrap()
    }

    #[test]
    #[ignore = "requiere un adaptador GPU real"]
    fn gpu_nearest_preserves_texels_and_reuses_target_until_internal_resize() {
        let (device, queue) = gpu_device();
        let mut pixel = PixelArt::new(
            &device,
            COLOR_FORMAT,
            PixelScale::DEFAULT,
            PhysicalSize::new(6, 3),
        );
        let original = pixel.target.texture.clone();
        assert_eq!(
            pixel.resize(&device, PhysicalSize::new(7, 4)),
            PhysicalSize::new(2, 1)
        );
        assert_eq!(pixel.target.texture, original);
        let extent = |width, height| wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        queue.write_texture(
            pixel.target.texture.as_image_copy(),
            &[255, 0, 0, 255, 0, 255, 0, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: None,
            },
            extent(2, 1),
        );
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Prueba de presentación"),
            size: extent(7, 4),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        pixel.present(&mut encoder, &output.create_view(&Default::default()));
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: None,
                },
            },
            extent(7, 4),
        );
        queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        let _ = device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let contents = slice.get_mapped_range();
        for y in 0..4 {
            for x in 0..7 {
                let expected = if x == 6 || y == 3 {
                    [0, 0, 0, 255]
                } else if x < 3 {
                    [255, 0, 0, 255]
                } else {
                    [0, 255, 0, 255]
                };
                let offset = y * 256 + x * 4;
                assert_eq!(contents[offset..offset + 4], expected);
            }
        }
        assert_eq!(
            pixel.resize(&device, PhysicalSize::new(9, 4)),
            PhysicalSize::new(3, 1)
        );
        assert_ne!(pixel.target.texture, original);
    }

    #[test]
    #[ignore = "requiere un adaptador GPU real"]
    fn gpu_scene_depth_and_lighting_preserve_three_discrete_tones() {
        let (device, queue) = gpu_device();
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[0, 1].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }),
        });
        let identity = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&glam::Mat4::IDENTITY.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &group_layout,
            entries: &[0, 1].map(|binding| wgpu::BindGroupEntry {
                binding,
                resource: identity.as_entire_binding(),
            }),
        });
        let pipeline = create_scene_pipeline(&device, &group_layout, COLOR_FORMAT, true);

        // Dibujar primero las caras cercanas y después las caras rojas ocluidas.
        let triangles = |depth, color| {
            [(-0.65, 1.0), (0.0, 0.0), (0.65, -1.0)]
                .into_iter()
                .flat_map(move |(center, slope)| {
                    [(-0.30, -0.2), (0.30, -0.2), (0.0, 0.2)].map(|(x, y)| GpuVertex {
                        position: [center + x, y, depth + slope * y],
                        color,
                    })
                })
        };
        let vertices: Vec<_> = triangles(0.25, [1.0; 3])
            .chain(triangles(0.75, [1.0, 0.0, 0.0]))
            .collect();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let size = PhysicalSize::new(48, 16);
        let depth = DepthTexture::new(&device, size);
        let pixel = PixelArt::new(&device, COLOR_FORMAT, PixelScale::new(1).unwrap(), size);
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * u64::from(size.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: pixel.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.draw(0..18, 0..1);
        }
        pixel.present(&mut encoder, &output.create_view(&Default::default()));
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: None,
                },
            },
            output.size(),
        );
        queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        let _ = device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let contents = slice.get_mapped_range();
        // Factores lineales 0,58 / 0,78 / 1,00 codificados en sRGB de 8 bits.
        for (x, expected) in [(8, 200u8), (24, 229), (39, 255)] {
            for y in 7..=8 {
                let offset = y * 256 + x * 4;
                let [r, g, b, a]: [u8; 4] = contents[offset..offset + 4].try_into().unwrap();
                assert!(r.abs_diff(expected) <= 1, "tono inesperado: {r}");
                assert_eq!([g, b, a], [r, r, 255]);
            }
        }
    }
}
