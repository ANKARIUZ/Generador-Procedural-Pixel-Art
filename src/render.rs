//! Conversión de geometría y presentación clásica o pixel art mediante wgpu.

mod camera;
mod gpu_mesh;
mod offscreen;
mod pixel_art;
mod scene_renderer;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use gpu_mesh::GpuMesh;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::scene::{MeshId, Scene};

pub use camera::{
    Camera, CameraController, CameraInput, CameraMouseButton, CameraMove, FixedCamera,
};
pub use gpu_mesh::{FaceColor, FacePalette, GpuMeshData, GpuVertex, RenderDataError};
pub use offscreen::{render_sprite, OffscreenError, SpriteSize};
pub use pixel_art::{PixelLayout, PixelScale, RenderMode, RenderSettingsError};
pub use scene_renderer::{prepare_scene_draws, SceneDraw};

/// Error durante la creación del contexto de renderizado.
#[derive(Debug)]
pub enum RenderInitError {
    /// wgpu no pudo crear una superficie asociada a la ventana.
    SurfaceCreation(wgpu::CreateSurfaceError),
    /// No se encontró un adaptador compatible con la superficie.
    AdapterNotFound,
    /// wgpu no pudo crear el dispositivo lógico.
    DeviceRequest(wgpu::RequestDeviceError),
    /// El adaptador no expuso configuraciones de superficie utilizables.
    MissingSurfaceCapability(&'static str),
    /// La conversión de una malla registrada falló.
    MeshPreparation {
        mesh_id: MeshId,
        source: RenderDataError,
    },
    /// La cantidad de objetos no cabe en el buffer de transformaciones.
    ModelBufferSizeOverflow,
}

impl Display for RenderInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SurfaceCreation(error) => write!(f, "no se pudo crear la superficie: {error}"),
            Self::AdapterNotFound => write!(f, "no se encontró un adaptador GPU compatible"),
            Self::DeviceRequest(error) => write!(f, "no se pudo crear el dispositivo GPU: {error}"),
            Self::MissingSurfaceCapability(capability) => {
                write!(
                    f,
                    "la superficie no ofrece la capacidad requerida: {capability}"
                )
            }
            Self::MeshPreparation { mesh_id, source } => {
                write!(f, "no se pudo preparar la malla {:?}: {source}", mesh_id)
            }
            Self::ModelBufferSizeOverflow => {
                write!(
                    f,
                    "el buffer de transformaciones excede los límites representables"
                )
            }
        }
    }
}

impl Error for RenderInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SurfaceCreation(error) => Some(error),
            Self::DeviceRequest(error) => Some(error),
            Self::MeshPreparation { source, .. } => Some(source),
            Self::AdapterNotFound
            | Self::MissingSurfaceCapability(_)
            | Self::ModelBufferSizeOverflow => None,
        }
    }
}

/// Error recuperable al dibujar una escena ya preparada.
#[derive(Debug)]
pub enum RenderError {
    Surface(wgpu::SurfaceError),
    SceneOutOfSync { mesh_id: MeshId },
    DrawCountOutOfSync,
}

/// Resultado de un intento de dibujo que no produjo un error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOutcome {
    /// El renderizador entregó el fotograma a la superficie mediante `present`.
    Presented,
    /// La superficie tenía una dimensión cero y no se presentó un fotograma.
    Skipped,
}

impl Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Surface(error) => write!(f, "error de superficie: {error}"),
            Self::SceneOutOfSync { mesh_id } => write!(
                f,
                "la escena contiene la malla {:?}, que no fue preparada en GPU",
                mesh_id
            ),
            Self::DrawCountOutOfSync => write!(
                f,
                "la escena tiene más objetos visibles que el buffer de transformaciones preparado"
            ),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Surface(error) => Some(error),
            Self::SceneOutOfSync { .. } | Self::DrawCountOutOfSync => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    fn from_camera(camera: &FixedCamera) -> Self {
        Self {
            view_projection: camera.view_projection().to_cols_array_2d(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
}

impl ModelUniform {
    fn from_matrix(matrix: glam::Mat4) -> Self {
        Self {
            model: matrix.to_cols_array_2d(),
        }
    }
}

struct DepthTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: PhysicalSize<u32>,
}

impl DepthTexture {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

    fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
            size,
        }
    }
}

fn create_scene_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    color_format: wgpu::TextureFormat,
    pixel_art: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("render/basic.wgsl"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Basic Render Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Basic Cube Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[gpu_mesh::GpuVertex::layout()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(if pixel_art { "fs_pixel" } else { "fs_main" }),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DepthTexture::FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

/// Administra la superficie, los recursos GPU compartidos y la pipeline de dibujo.
pub struct RenderContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    gpu_meshes: BTreeMap<MeshId, GpuMesh>,
    camera: Camera,
    camera_controller: CameraController,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    model_buffer: wgpu::Buffer,
    model_stride: u64,
    model_capacity: usize,
    model_upload: Vec<u8>,
    depth_texture: DepthTexture,
    pixel: Option<pixel_art::PixelArt>,
}

impl RenderContext {
    /// Inicializa wgpu y prepara una vez los recursos GPU de las mallas de escena.
    pub async fn new(window: Arc<Window>, scene: &Scene) -> Result<Self, RenderInitError> {
        Self::with_palette(window, scene, &FacePalette::cube()).await
    }

    /// Prepara una escena con la paleta proporcionada por la aplicación.
    pub async fn with_palette(
        window: Arc<Window>,
        scene: &Scene,
        palette: &FacePalette,
    ) -> Result<Self, RenderInitError> {
        Self::with_presentation(window, scene, palette, RenderMode::Classic, false).await
    }

    /// Configura el modo gráfico y la vista inicial de la escena.
    pub async fn with_presentation(
        window: Arc<Window>,
        scene: &Scene,
        palette: &FacePalette,
        mode: RenderMode,
        isometric: bool,
    ) -> Result<Self, RenderInitError> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window)
            .map_err(RenderInitError::SurfaceCreation)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderInitError::AdapterNotFound)?;

        let adapter_info = adapter.get_info();
        log::info!(
            "Adaptador GPU seleccionado: {} ({:?}, backend: {:?})",
            adapter_info.name,
            adapter_info.device_type,
            adapter_info.backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Primary GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(RenderInitError::DeviceRequest)?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| surface_caps.formats.first().copied())
            .ok_or(RenderInitError::MissingSurfaceCapability("formato"))?;
        let present_mode = *surface_caps.present_modes.first().ok_or(
            RenderInitError::MissingSurfaceCapability("modo de presentación"),
        )?;
        let alpha_mode = *surface_caps
            .alpha_modes
            .first()
            .ok_or(RenderInitError::MissingSurfaceCapability("modo alfa"))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let initial_size = PhysicalSize::new(width, height);
        let pixel = match mode {
            RenderMode::Classic => None,
            RenderMode::Pixel(scale) => {
                let pixel = pixel_art::PixelArt::new(&device, config.format, scale, initial_size);
                let internal = pixel.internal_size();
                log::info!(
                    "Modo pixel art: {} × {} píxeles internos",
                    internal.width,
                    internal.height
                );
                Some(pixel)
            }
        };
        let scene_size = pixel
            .as_ref()
            .map_or(initial_size, pixel_art::PixelArt::internal_size);
        let camera = if isometric {
            Camera::isometric(scene_size)
        } else {
            Camera::new(scene_size)
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::bytes_of(&CameraUniform::from_camera(&camera)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_capacity = scene.object_count().max(1);
        let uniform_alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let model_size = std::mem::size_of::<ModelUniform>() as u64;
        let model_stride = model_size
            .checked_add(uniform_alignment.saturating_sub(1))
            .and_then(|size| size.checked_div(uniform_alignment))
            .and_then(|count| count.checked_mul(uniform_alignment))
            .ok_or(RenderInitError::ModelBufferSizeOverflow)?;
        let model_buffer_size = model_stride
            .checked_mul(
                u64::try_from(model_capacity)
                    .map_err(|_| RenderInitError::ModelBufferSizeOverflow)?,
            )
            .ok_or(RenderInitError::ModelBufferSizeOverflow)?;
        let model_upload_size = usize::try_from(model_buffer_size)
            .map_err(|_| RenderInitError::ModelBufferSizeOverflow)?;
        let model_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Model Uniform Buffer"),
            size: model_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: wgpu::BufferSize::new(model_size),
                        },
                        count: None,
                    },
                ],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &model_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(model_size),
                    }),
                },
            ],
        });

        let pipeline = create_scene_pipeline(
            &device,
            &camera_bind_group_layout,
            pixel
                .as_ref()
                .map_or(config.format, |_| pixel_art::COLOR_FORMAT),
            pixel.is_some(),
        );

        let mut gpu_meshes = BTreeMap::new();
        for (mesh_id, mesh) in scene.meshes() {
            let mesh_data = GpuMeshData::from_mesh(mesh, palette)
                .map_err(|source| RenderInitError::MeshPreparation { mesh_id, source })?;
            let gpu_mesh = GpuMesh::new(&device, &mesh_data)
                .map_err(|source| RenderInitError::MeshPreparation { mesh_id, source })?;
            gpu_meshes.insert(mesh_id, gpu_mesh);
        }
        let depth_texture = DepthTexture::new(&device, scene_size);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            gpu_meshes,
            camera,
            camera_controller: CameraController::default(),
            camera_buffer,
            camera_bind_group,
            model_buffer,
            model_stride,
            model_capacity,
            model_upload: vec![0; model_upload_size],
            depth_texture,
            pixel,
        })
    }

    /// Redimensiona los recursos de superficie y profundidad, sin recrear buffers estáticos.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        if !self.has_drawable_surface() {
            return;
        }
        if self.config.width == new_size.width && self.config.height == new_size.height {
            return;
        }
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        let scene_size = self
            .pixel
            .as_mut()
            .map_or(new_size, |pixel| pixel.resize(&self.device, new_size));
        self.camera.resize(scene_size);
        self.upload_camera();
        if self.depth_texture.size != scene_size {
            self.depth_texture = DepthTexture::new(&self.device, scene_size);
        }
    }

    /// Procesa una entrada de cámara y actualiza el uniform solo si la cámara cambió.
    pub fn handle_camera_input(&mut self, input: CameraInput) -> bool {
        let viewport = if matches!(input, CameraInput::Reset) {
            self.depth_texture.size
        } else {
            self.pixel
                .as_ref()
                .map_or(self.size, pixel_art::PixelArt::presentation_size)
        };
        if self
            .camera_controller
            .process_input(&mut self.camera, input, viewport)
        {
            self.upload_camera();
            return true;
        }
        false
    }

    /// Actualiza el movimiento continuo de teclado sin recrear recursos estáticos.
    pub fn update_camera(&mut self, delta: Duration) -> bool {
        if self
            .camera_controller
            .update(&mut self.camera, delta, self.size)
        {
            self.upload_camera();
            return true;
        }
        false
    }

    /// Indica si hay teclas de cámara que requieren temporización limitada.
    pub fn has_continuous_camera_motion(&self) -> bool {
        self.camera_controller.has_continuous_motion()
    }

    /// Reconfigura la superficie tras una pérdida sin modificar los recursos estáticos.
    pub fn reconfigure(&mut self) {
        if self.has_drawable_surface() {
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Indica si el tamaño actual permite dibujar en la superficie.
    pub fn has_drawable_surface(&self) -> bool {
        self.size.width > 0 && self.size.height > 0
    }

    fn upload_camera(&self) {
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&CameraUniform::from_camera(&self.camera)),
        );
    }

    /// Dibuja cada objeto visible con su matriz de modelo y presenta el resultado.
    pub fn render(&mut self, scene: &Scene) -> Result<FrameOutcome, RenderError> {
        if !self.has_drawable_surface() {
            return Ok(FrameOutcome::Skipped);
        }
        let draws = prepare_scene_draws(scene);
        if draws.len() > self.model_capacity {
            return Err(RenderError::DrawCountOutOfSync);
        }
        self.upload_models(&draws);

        let output = self
            .surface
            .get_current_texture()
            .map_err(RenderError::Surface)?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Basic Cube Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.pixel.as_ref().map_or(&view, pixel_art::PixelArt::view),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.10,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            for (index, draw) in draws.iter().enumerate() {
                let gpu_mesh =
                    self.gpu_meshes
                        .get(&draw.mesh_id)
                        .ok_or(RenderError::SceneOutOfSync {
                            mesh_id: draw.mesh_id,
                        })?;
                let offset = u32::try_from(
                    self.model_stride
                        .checked_mul(
                            u64::try_from(index).map_err(|_| RenderError::DrawCountOutOfSync)?,
                        )
                        .ok_or(RenderError::DrawCountOutOfSync)?,
                )
                .map_err(|_| RenderError::DrawCountOutOfSync)?;
                render_pass.set_bind_group(0, &self.camera_bind_group, &[offset]);
                render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer().slice(..));
                render_pass
                    .set_index_buffer(gpu_mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..gpu_mesh.index_count(), 0, 0..1);
            }
        }

        if let Some(pixel) = &self.pixel {
            pixel.present(&mut encoder, &view);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(FrameOutcome::Presented)
    }

    fn upload_models(&mut self, draws: &[SceneDraw]) {
        self.model_upload.fill(0);
        let stride = self.model_stride as usize;
        for (index, draw) in draws.iter().enumerate() {
            let start = index * stride;
            let end = start + std::mem::size_of::<ModelUniform>();
            self.model_upload[start..end].copy_from_slice(bytemuck::bytes_of(
                &ModelUniform::from_matrix(draw.model_matrix),
            ));
        }
        self.queue
            .write_buffer(&self.model_buffer, 0, &self.model_upload);
    }
}
