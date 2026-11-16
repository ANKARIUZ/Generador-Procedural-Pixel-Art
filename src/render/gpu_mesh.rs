use std::error::Error;
use std::fmt::{Display, Formatter};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::geometry::{GeometryError, Mesh};

/// Color RGB temporal asociado a un identificador de material geométrico.
pub type FaceColor = [f32; 3];

/// Paleta de presentación para hasta seis identificadores de material.
#[derive(Clone, Debug)]
pub struct FacePalette {
    colors: [(u32, FaceColor); 6],
}

impl FacePalette {
    pub(crate) const fn entries(&self) -> &[(u32, FaceColor); 6] {
        &self.colors
    }

    /// Define los colores RGB lineales de hasta seis materiales de presentación.
    /// Cada entrada puede utilizar el identificador original de la malla.
    pub const fn from_entries(colors: [(u32, FaceColor); 6]) -> Self {
        Self { colors }
    }

    /// Paleta para frente, atrás, derecha, izquierda, arriba y abajo.
    pub const fn cube() -> Self {
        Self {
            colors: [
                (0, [0.85, 0.12, 0.12]),
                (1, [0.12, 0.26, 0.85]),
                (2, [0.12, 0.72, 0.24]),
                (3, [0.95, 0.78, 0.10]),
                (4, [0.95, 0.38, 0.08]),
                (5, [0.48, 0.18, 0.78]),
            ],
        }
    }

    /// Colores de demostración para los seis materiales de `HouseConfig::default`.
    pub const fn house() -> Self {
        Self {
            colors: [
                (0, [0.19, 0.21, 0.24]),
                (1, [0.56, 0.49, 0.38]),
                (2, [0.79, 0.66, 0.46]),
                (3, [0.23, 0.09, 0.04]),
                (4, [0.09, 0.39, 0.57]),
                (5, [0.42, 0.11, 0.08]),
            ],
        }
    }

    /// Resuelve un material o devuelve un error explícito si no pertenece a la paleta.
    pub fn color_for(&self, material_id: u32) -> Result<FaceColor, RenderDataError> {
        self.colors
            .iter()
            .find_map(|(id, color)| (*id == material_id).then_some(*color))
            .ok_or(RenderDataError::UnknownMaterial { material_id })
    }
}

/// Vértice específico del renderizador: posición local y color por cara.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GpuVertex {
    /// Posición local de la geometría.
    pub position: [f32; 3],
    /// Color RGB plano procedente del material de la cara.
    pub color: FaceColor,
}

impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    /// Describe el formato en memoria que consume la pipeline de wgpu.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Datos de CPU preparados para transferirse a buffers de GPU.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuMeshData {
    /// Vértices expandidos por triángulo para conservar atributos por cara.
    pub vertices: Vec<GpuVertex>,
    /// Índices que referencian los vértices expandidos.
    pub indices: Vec<u32>,
}

impl GpuMeshData {
    /// Convierte una malla validada en datos renderizables, sin inicializar wgpu.
    pub fn from_mesh(mesh: &Mesh, palette: &FacePalette) -> Result<Self, RenderDataError> {
        mesh.validate().map_err(RenderDataError::InvalidGeometry)?;

        let mut vertices = Vec::with_capacity(mesh.faces.len() * 3);
        let mut indices = Vec::with_capacity(mesh.faces.len() * 3);
        for face in &mesh.faces {
            let color = palette.color_for(face.material_id)?;
            for vertex_index in face.indices {
                let position = mesh.vertices[vertex_index as usize].position;
                let index = u32::try_from(vertices.len()).map_err(|_| {
                    RenderDataError::VertexCountOverflow {
                        vertex_count: vertices.len(),
                    }
                })?;
                vertices.push(GpuVertex {
                    position: position.to_array(),
                    color,
                });
                indices.push(index);
            }
        }
        Ok(Self { vertices, indices })
    }
}

/// Errores de la conversión CPU a datos de renderizado.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderDataError {
    /// La malla fue modificada y ya no cumple sus invariantes geométricos.
    InvalidGeometry(GeometryError),
    /// La paleta temporal no define el material indicado.
    UnknownMaterial { material_id: u32 },
    /// La cantidad de vértices expandidos no cabe en índices `u32`.
    VertexCountOverflow { vertex_count: usize },
}

impl Display for RenderDataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGeometry(error) => write!(f, "malla geométrica inválida: {error}"),
            Self::UnknownMaterial { material_id } => {
                write!(f, "la paleta temporal no conoce el material {material_id}")
            }
            Self::VertexCountOverflow { vertex_count } => write!(
                f,
                "no se pueden indexar {vertex_count} vértices de renderizado con u32"
            ),
        }
    }
}

impl Error for RenderDataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidGeometry(error) => Some(error),
            Self::UnknownMaterial { .. } | Self::VertexCountOverflow { .. } => None,
        }
    }
}

/// Buffers permanentes de una malla estática en GPU.
pub struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl GpuMesh {
    /// Crea buffers una vez durante la inicialización de una malla estática.
    pub fn new(device: &wgpu::Device, data: &GpuMeshData) -> Result<Self, RenderDataError> {
        let index_count = u32::try_from(data.indices.len()).map_err(|_| {
            RenderDataError::VertexCountOverflow {
                vertex_count: data.indices.len(),
            }
        })?;
        Ok(Self {
            vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&data.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(&data.indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            index_count,
        })
    }

    pub(super) fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub(super) fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub(super) fn index_count(&self) -> u32 {
        self.index_count
    }
}
