use glam::Vec3;

use crate::geometry::{GeometryError, Mesh, Vertex};

/// Generador de un cubo centrado en el origen.
pub struct Cube;

impl Cube {
    /// Construye un cubo de arista `size` con el mismo material en sus seis lados.
    pub fn create(size: f32, material_id: u32) -> Result<Mesh, GeometryError> {
        Self::with_materials(size, [material_id; 6])
    }

    /// Construye un cubo con materiales en el orden: frente, atrás, derecha,
    /// izquierda, arriba y abajo.
    pub fn with_materials(size: f32, materials: [u32; 6]) -> Result<Mesh, GeometryError> {
        super::prism::build_prism(size, size, size, materials)
    }

    /// Crea los ocho vértices de un cubo centrado en el origen.
    pub fn vertices(size: f32) -> Result<[Vertex; 8], GeometryError> {
        validate_size(size, "size")?;
        let half = size / 2.0;
        Ok([
            Vertex::new(Vec3::new(-half, -half, -half)),
            Vertex::new(Vec3::new(half, -half, -half)),
            Vertex::new(Vec3::new(half, half, -half)),
            Vertex::new(Vec3::new(-half, half, -half)),
            Vertex::new(Vec3::new(-half, -half, half)),
            Vertex::new(Vec3::new(half, -half, half)),
            Vertex::new(Vec3::new(half, half, half)),
            Vertex::new(Vec3::new(-half, half, half)),
        ])
    }
}

pub(super) fn validate_size(value: f32, name: &'static str) -> Result<(), GeometryError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(GeometryError::InvalidDimension { name, value })
    }
}
