use glam::Vec3;

/// Vértice geométrico expresado en coordenadas locales de una malla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    /// Posición tridimensional en unidades de mundo.
    pub position: Vec3,
}

impl Vertex {
    /// Crea un vértice a partir de una posición.
    pub const fn new(position: Vec3) -> Self {
        Self { position }
    }
}
