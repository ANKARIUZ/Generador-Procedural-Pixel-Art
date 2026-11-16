/// Triángulo indexado de una malla y su identificador de material.
///
/// Los índices apuntan a [`crate::geometry::Mesh::vertices`]. Un cuadrilátero se
/// representa mediante dos instancias contiguas creadas por `Mesh::add_quad`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face {
    /// Índices de los tres vértices en orden antihorario desde el exterior.
    pub indices: [u32; 3],
    /// Identificador opaco que el futuro sistema de materiales resolverá.
    pub material_id: u32,
}

impl Face {
    /// Crea una cara triangular indexada.
    pub const fn new(indices: [u32; 3], material_id: u32) -> Self {
        Self {
            indices,
            material_id,
        }
    }
}
