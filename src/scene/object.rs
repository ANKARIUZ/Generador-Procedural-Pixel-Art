use super::Transform;

/// Identificador estable de una malla registrada en una escena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeshId(pub(crate) u64);

/// Identificador estable de un objeto dentro de una escena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub(crate) u64);

/// Instancia lógica de una malla con transformación y visibilidad propias.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneObject {
    id: ObjectId,
    mesh_id: MeshId,
    transform: Transform,
    visible: bool,
}

impl SceneObject {
    pub(crate) const fn new(id: ObjectId, mesh_id: MeshId, transform: Transform) -> Self {
        Self {
            id,
            mesh_id,
            transform,
            visible: true,
        }
    }

    /// Identificador estable del objeto.
    pub const fn id(&self) -> ObjectId {
        self.id
    }

    /// Malla local reutilizada por este objeto.
    pub const fn mesh_id(&self) -> MeshId {
        self.mesh_id
    }

    /// Transformación local validada.
    pub const fn transform(&self) -> Transform {
        self.transform
    }

    /// Indica si la instancia participa en el dibujo.
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn set_transform(&mut self, transform: Transform) {
        self.transform = transform;
    }

    pub(crate) fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}
