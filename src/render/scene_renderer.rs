//! Preparación de comandos de dibujo de escena sin acceso a wgpu.

use glam::Mat4;

use crate::scene::{MeshId, ObjectId, Scene};

/// Datos inmutables necesarios para dibujar una instancia visible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneDraw {
    pub object_id: ObjectId,
    pub mesh_id: MeshId,
    pub model_matrix: Mat4,
}

/// Prepara un comando por objeto visible sin alterar geometría ni transformaciones.
pub fn prepare_scene_draws(scene: &Scene) -> Vec<SceneDraw> {
    scene
        .visible_objects()
        .filter_map(|object| {
            object
                .transform()
                .model_matrix()
                .ok()
                .map(|model_matrix| SceneDraw {
                    object_id: object.id(),
                    mesh_id: object.mesh_id(),
                    model_matrix,
                })
        })
        .collect()
}
