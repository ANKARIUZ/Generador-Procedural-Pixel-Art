//! Renderizado y lectura de una malla fuera de pantalla.

use glam::{Mat4, Vec3};
use thiserror::Error;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use super::gpu_mesh::GpuMesh;
use super::{create_scene_pipeline, pixel_art, Camera, DepthTexture, FacePalette, GpuMeshData};
use crate::geometry::Mesh;

/// Dimensiones lógicas, en téxeles, del destino de renderizado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteSize {
    width: u32,
    height: u32,
}

impl SpriteSize {
    pub const MAX_EDGE: u32 = 2048;

    /// Valida cada eje antes de reservar memoria.
    pub fn new(width: u32, height: u32) -> Result<Self, OffscreenError> {
        if !(1..=Self::MAX_EDGE).contains(&width) || !(1..=Self::MAX_EDGE).contains(&height) {
            return Err(OffscreenError::Size);
        }
        Ok(Self { width, height })
    }

    /// Dimensiones en orden ancho, alto.
    pub const fn dimensions(self) -> [u32; 2] {
        [self.width, self.height]
    }
}

/// Fallos de preparación o lectura del sprite.
#[derive(Debug, Error)]
pub enum OffscreenError {
    #[error("las dimensiones deben estar entre 1 y 2048")]
    Size,
    #[error("el margen debe dejar al menos un téxel útil por eje")]
    Margin,
    #[error("la geometría no admite un encuadre ortográfico finito")]
    Bounds,
    #[error(transparent)]
    Mesh(#[from] super::RenderDataError),
    #[error("no hay un adaptador GPU para renderizado offscreen")]
    Adapter,
    #[error("no se pudo abrir el dispositivo GPU: {0}")]
    Device(#[source] wgpu::RequestDeviceError),
    #[error("el sprite o sus buffers superan los límites del dispositivo")]
    GpuLimits,
    #[error("falló el trabajo GPU: {0}")]
    Gpu(#[source] wgpu::Error),
    #[error("se perdió el dispositivo GPU: {0}")]
    DeviceLost(String),
    #[error("no se pudo mapear el buffer de lectura: {0}")]
    Map(#[source] wgpu::BufferAsyncError),
    #[error("la GPU no notificó el resultado del mapeo")]
    MapCallback,
}

fn frame(mesh: &Mesh, size: SpriteSize, margin: u32) -> Result<Mat4, OffscreenError> {
    let [width, height] = size.dimensions();
    let padding = margin.checked_mul(2).ok_or(OffscreenError::Margin)?;
    if padding >= width || padding >= height {
        return Err(OffscreenError::Margin);
    }
    let camera = Camera::isometric(PhysicalSize::new(width, height));
    let basis = Mat4::look_at_rh(camera.position(), Vec3::ZERO, Vec3::Y);
    let basis = glam::Mat3::from_mat4(basis);
    let (min, max) = mesh.vertices.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(min, max), vertex| {
            let point = basis * vertex.position;
            (min.min(point), max.max(point))
        },
    );
    let span = max - min;
    if !span.is_finite() || span.min_element() <= 0.0 {
        return Err(OffscreenError::Bounds);
    }
    let units_per_texel =
        (span.x / (width - padding) as f32).max(span.y / (height - padding) as f32);
    let half_width = units_per_texel * width as f32 / 2.0;
    let half_height = units_per_texel * height as f32 / 2.0;
    let center = min + span / 2.0;
    let depth = span.z.max(units_per_texel);
    let view = Mat4::from_translation(-center) * Mat4::from_mat3(basis);
    let projection = Mat4::orthographic_rh(
        -half_width,
        half_width,
        -half_height,
        half_height,
        -depth,
        depth,
    );
    let view_projection = projection * view;
    view_projection
        .is_finite()
        .then_some(view_projection)
        .ok_or(OffscreenError::Bounds)
}

/// Renderiza la malla en vista isométrica. Devuelve RGBA8 sRGB con alfa recto,
/// filas compactas y origen superior izquierdo. El margen se mide en téxeles.
pub fn render_sprite(
    mesh: &Mesh,
    palette: &FacePalette,
    size: SpriteSize,
    margin: u32,
) -> Result<Vec<u8>, OffscreenError> {
    let data = GpuMeshData::from_mesh(mesh, palette)?;
    let projection = frame(mesh, size, margin)?;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .ok_or(OffscreenError::Adapter)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default(), None))
        .map_err(OffscreenError::Device)?;
    let limits = device.limits();
    let row_bytes = size.width * 4;
    let stride =
        row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let read_size = u64::from(stride) * u64::from(size.height);
    if size.width.max(size.height) > limits.max_texture_dimension_2d
        || read_size > limits.max_buffer_size
        || std::mem::size_of_val(data.vertices.as_slice()) as u64 > limits.max_buffer_size
        || std::mem::size_of_val(data.indices.as_slice()) as u64 > limits.max_buffer_size
    {
        return Err(OffscreenError::GpuLimits);
    }
    let (lost_tx, lost_rx) = std::sync::mpsc::channel();
    device.set_device_lost_callback(move |_, message| {
        let _ = lost_tx.send(message);
    });
    for filter in [
        wgpu::ErrorFilter::OutOfMemory,
        wgpu::ErrorFilter::Internal,
        wgpu::ErrorFilter::Validation,
    ] {
        device.push_error_scope(filter);
    }
    let readback = draw_and_copy(&device, &queue, &data, projection, size, stride, read_size);
    let mut gpu_error = None;
    for _ in 0..3 {
        gpu_error = pollster::block_on(device.pop_error_scope()).or(gpu_error);
    }
    if let Some(error) = gpu_error {
        return Err(OffscreenError::Gpu(error));
    }
    let readback = readback?;
    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    if let Ok(message) = lost_rx.try_recv() {
        return Err(OffscreenError::DeviceLost(message));
    }
    rx.try_recv()
        .map_err(|_| OffscreenError::MapCallback)?
        .map_err(OffscreenError::Map)?;
    let mapped = slice.get_mapped_range();
    let mut rgba = Vec::with_capacity((row_bytes * size.height) as usize);
    for row in mapped.chunks_exact(stride as usize) {
        rgba.extend_from_slice(&row[..row_bytes as usize]);
    }
    drop(mapped);
    readback.unmap();
    Ok(rgba)
}

fn draw_and_copy(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &GpuMeshData,
    projection: Mat4,
    size: SpriteSize,
    stride: u32,
    read_size: u64,
) -> Result<wgpu::Buffer, OffscreenError> {
    let mesh = GpuMesh::new(device, data)?;
    let uniforms = [projection.to_cols_array(), Mat4::IDENTITY.to_cols_array()].map(|matrix| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Matriz de sprite"),
            contents: bytemuck::bytes_of(&matrix),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Matrices de sprite"),
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
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Matrices de sprite"),
        layout: &layout,
        entries: &std::array::from_fn::<_, 2, _>(|i| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: uniforms[i].as_entire_binding(),
        }),
    });
    let pipeline = create_scene_pipeline(device, &layout, pixel_art::COLOR_FORMAT, true);
    let depth = DepthTexture::new(device, PhysicalSize::new(size.width, size.height));
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Color de sprite"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: pixel_art::COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = color.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Lectura de sprite"),
        size: read_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pase offscreen de sprite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
        pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
        pass.set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.index_count(), 0, 0..1);
    }
    encoder.copy_texture_to_buffer(
        color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: None,
            },
        },
        color.size(),
    );
    queue.submit([encoder.finish()]);
    Ok(readback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vertex;

    #[test]
    fn framing_contains_and_centers_vertices_of_translated_rectangular_buildings() {
        for [width, height] in [[71, 199], [183, 58]] {
            let vertices = [
                Vec3::new(-57.0, 1.0, 2.0),
                Vec3::new(51.0, 8.0, -24.0),
                Vec3::new(49.0, 46.0, -8.0),
                Vec3::new(12.0, 7.0, 29.0),
            ]
            .map(|position| Vertex::new(position + Vec3::new(533.0, -248.0, 303.0)));
            let mesh = Mesh::new(vertices.to_vec(), Vec::new()).unwrap();
            let margin = 5;
            let projection = frame(&mesh, SpriteSize::new(width, height).unwrap(), margin).unwrap();
            let points = vertices.map(|vertex| projection.project_point3(vertex.position));
            let minimum = points.into_iter().reduce(Vec3::min).unwrap();
            let maximum = points.into_iter().reduce(Vec3::max).unwrap();
            assert!((minimum.x + maximum.x).abs() < 1e-5);
            assert!((minimum.y + maximum.y).abs() < 1e-5);
            assert!(minimum.z > 0.0 && maximum.z < 1.0);
            let expected = [width, height].map(|edge| 1.0 - (2 * margin) as f32 / edge as f32);
            assert!(maximum.x <= expected[0] + 1e-5 && maximum.y <= expected[1] + 1e-5);
            assert!(
                (maximum.x - expected[0]).abs() < 1e-5 || (maximum.y - expected[1]).abs() < 1e-5
            );
        }
    }
}
