//! Escena CPU independiente de winit y wgpu.
//!
//! `Scene` posee las mallas y crea objetos que las referencian mediante
//! identificadores estables. La geometría permanece local; cada objeto aplica
//! su propia [`Transform`] al ser consumido por el renderizador.

mod object;
#[path = "scene.rs"]
mod registry;
mod transform;

pub use object::{MeshId, ObjectId, SceneObject};
pub use registry::{Scene, SceneError};
pub use transform::{Transform, TransformError};
