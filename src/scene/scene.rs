use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::geometry::{GeometryError, Mesh};

use super::{MeshId, ObjectId, SceneObject, Transform, TransformError};

/// Error de operaciones que pueden romper las invariantes de una escena.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneError {
    InvalidMesh(GeometryError),
    InvalidTransform(TransformError),
    MeshNotFound(MeshId),
    ObjectNotFound(ObjectId),
    MeshInUse(MeshId),
    MeshIdExhausted,
    ObjectIdExhausted,
}

impl Display for SceneError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMesh(error) => write!(f, "malla inválida: {error}"),
            Self::InvalidTransform(error) => write!(f, "transformación inválida: {error}"),
            Self::MeshNotFound(id) => write!(f, "la malla {:?} no existe", id),
            Self::ObjectNotFound(id) => write!(f, "el objeto {:?} no existe", id),
            Self::MeshInUse(id) => write!(f, "la malla {:?} sigue referenciada por objetos", id),
            Self::MeshIdExhausted => write!(f, "se agotaron los identificadores de malla"),
            Self::ObjectIdExhausted => write!(f, "se agotaron los identificadores de objeto"),
        }
    }
}

impl Error for SceneError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidMesh(error) => Some(error),
            Self::InvalidTransform(error) => Some(error),
            Self::MeshNotFound(_)
            | Self::ObjectNotFound(_)
            | Self::MeshInUse(_)
            | Self::MeshIdExhausted
            | Self::ObjectIdExhausted => None,
        }
    }
}

/// Propietaria de mallas CPU e instancias lógicas de una escena.
#[derive(Default)]
pub struct Scene {
    meshes: BTreeMap<MeshId, Mesh>,
    objects: BTreeMap<ObjectId, SceneObject>,
    next_mesh_id: u64,
    next_object_id: u64,
}

impl Scene {
    /// Crea una escena vacía.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra una malla validada y transfiere su propiedad a la escena.
    pub fn register_mesh(&mut self, mesh: Mesh) -> Result<MeshId, SceneError> {
        mesh.validate().map_err(SceneError::InvalidMesh)?;
        let id = MeshId(self.next_mesh_id);
        self.next_mesh_id = self
            .next_mesh_id
            .checked_add(1)
            .ok_or(SceneError::MeshIdExhausted)?;
        self.meshes.insert(id, mesh);
        Ok(id)
    }

    /// Retira una malla que no está referenciada por ningún objeto.
    pub fn remove_mesh(&mut self, id: MeshId) -> Result<Mesh, SceneError> {
        if !self.meshes.contains_key(&id) {
            return Err(SceneError::MeshNotFound(id));
        }
        if self.objects.values().any(|object| object.mesh_id() == id) {
            return Err(SceneError::MeshInUse(id));
        }
        self.meshes.remove(&id).ok_or(SceneError::MeshNotFound(id))
    }

    /// Crea una instancia visible que refiere una malla existente.
    pub fn create_object(
        &mut self,
        mesh_id: MeshId,
        transform: Transform,
    ) -> Result<ObjectId, SceneError> {
        if !self.meshes.contains_key(&mesh_id) {
            return Err(SceneError::MeshNotFound(mesh_id));
        }
        transform
            .model_matrix()
            .map_err(SceneError::InvalidTransform)?;
        let id = ObjectId(self.next_object_id);
        self.next_object_id = self
            .next_object_id
            .checked_add(1)
            .ok_or(SceneError::ObjectIdExhausted)?;
        self.objects
            .insert(id, SceneObject::new(id, mesh_id, transform));
        Ok(id)
    }

    /// Consulta un objeto por identidad estable.
    pub fn object(&self, id: ObjectId) -> Result<&SceneObject, SceneError> {
        self.objects.get(&id).ok_or(SceneError::ObjectNotFound(id))
    }

    /// Sustituye la transformación de un objeto tras validarla.
    pub fn set_transform(&mut self, id: ObjectId, transform: Transform) -> Result<(), SceneError> {
        transform
            .model_matrix()
            .map_err(SceneError::InvalidTransform)?;
        self.objects
            .get_mut(&id)
            .ok_or(SceneError::ObjectNotFound(id))?
            .set_transform(transform);
        Ok(())
    }

    /// Activa o desactiva la visibilidad sin eliminar el objeto.
    pub fn set_visibility(&mut self, id: ObjectId, visible: bool) -> Result<(), SceneError> {
        self.objects
            .get_mut(&id)
            .ok_or(SceneError::ObjectNotFound(id))?
            .set_visible(visible);
        Ok(())
    }

    /// Elimina una instancia, sin alterar otras identidades ni reutilizar su id.
    pub fn remove_object(&mut self, id: ObjectId) -> Result<SceneObject, SceneError> {
        self.objects
            .remove(&id)
            .ok_or(SceneError::ObjectNotFound(id))
    }

    /// Número de objetos, visibles o no.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Número de mallas propietarias de la escena.
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// Recorre los objetos visibles por orden estable de identificador.
    pub fn visible_objects(&self) -> impl Iterator<Item = &SceneObject> {
        self.objects.values().filter(|object| object.is_visible())
    }

    /// Recorre todos los objetos por orden estable de identificador.
    pub fn objects(&self) -> impl Iterator<Item = &SceneObject> {
        self.objects.values()
    }

    /// Consulta una malla registrada.
    pub fn mesh(&self, id: MeshId) -> Result<&Mesh, SceneError> {
        self.meshes.get(&id).ok_or(SceneError::MeshNotFound(id))
    }

    /// Recorre mallas registradas por orden estable de identificador.
    pub fn meshes(&self) -> impl Iterator<Item = (MeshId, &Mesh)> {
        self.meshes.iter().map(|(id, mesh)| (*id, mesh))
    }
}
