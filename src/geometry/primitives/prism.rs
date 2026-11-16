use crate::geometry::{GeometryError, Mesh};

use super::cube::validate_size;

/// Generador de un prisma rectangular centrado en el origen.
pub struct RectangularPrism;

impl RectangularPrism {
    /// Construye un prisma con un material compartido por sus seis lados.
    pub fn create(
        width: f32,
        height: f32,
        depth: f32,
        material_id: u32,
    ) -> Result<Mesh, GeometryError> {
        Self::with_materials(width, height, depth, [material_id; 6])
    }

    /// Construye un prisma con materiales por lado: frente, atrás, derecha,
    /// izquierda, arriba y abajo.
    pub fn with_materials(
        width: f32,
        height: f32,
        depth: f32,
        materials: [u32; 6],
    ) -> Result<Mesh, GeometryError> {
        build_prism(width, height, depth, materials)
    }
}

pub(super) fn build_prism(
    width: f32,
    height: f32,
    depth: f32,
    materials: [u32; 6],
) -> Result<Mesh, GeometryError> {
    validate_size(width, "width")?;
    validate_size(height, "height")?;
    validate_size(depth, "depth")?;

    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let half_depth = depth / 2.0;
    let vertices = vec![
        (-half_width, -half_height, -half_depth),
        (half_width, -half_height, -half_depth),
        (half_width, half_height, -half_depth),
        (-half_width, half_height, -half_depth),
        (-half_width, -half_height, half_depth),
        (half_width, -half_height, half_depth),
        (half_width, half_height, half_depth),
        (-half_width, half_height, half_depth),
    ]
    .into_iter()
    .map(|(x, y, z)| crate::geometry::Vertex::new(glam::Vec3::new(x, y, z)))
    .collect();

    let mut mesh = Mesh::new(vertices, Vec::new())?;
    for (indices, material_id) in [
        ([4, 5, 6, 7], materials[0]),
        ([1, 0, 3, 2], materials[1]),
        ([5, 1, 2, 6], materials[2]),
        ([0, 4, 7, 3], materials[3]),
        ([3, 7, 6, 2], materials[4]),
        ([0, 1, 5, 4], materials[5]),
    ] {
        mesh.add_quad(indices, material_id)?;
    }
    Ok(mesh)
}
