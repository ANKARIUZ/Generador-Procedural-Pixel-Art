use glam::{Quat, Vec3};
use procedural_city_engine::geometry::primitives::Cube;
use procedural_city_engine::render::prepare_scene_draws;
use procedural_city_engine::scene::{Scene, SceneError, Transform, TransformError};

fn cube() -> procedural_city_engine::geometry::Mesh {
    Cube::create(1.0, 0).expect("el cubo de prueba es válido")
}

fn transform(translation: Vec3) -> Transform {
    Transform::new(translation, Quat::IDENTITY, Vec3::ONE)
        .expect("la transformación de prueba es válida")
}

#[test]
fn transform_identity_translation_rotation_scale_and_order_are_correct() {
    let identity = Transform::identity();
    assert_eq!(
        identity.model_matrix().expect("identidad válida"),
        glam::Mat4::IDENTITY
    );

    let transform = Transform::new(
        Vec3::new(3.0, 4.0, 5.0),
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        Vec3::new(2.0, 3.0, 4.0),
    )
    .expect("transformación válida");
    let transformed = transform.model_matrix().expect("matriz finita") * Vec3::X.extend(1.0);

    assert!(transformed
        .truncate()
        .abs_diff_eq(Vec3::new(3.0, 6.0, 5.0), 1e-5));
    assert!(transform.rotation().is_normalized());
}

#[test]
fn transform_rejects_non_finite_zero_negative_and_invalid_rotation() {
    assert_eq!(
        Transform::new(Vec3::new(f32::NAN, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
        Err(TransformError::NonFiniteTranslation)
    );
    assert_eq!(
        Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO),
        Err(TransformError::InvalidScale)
    );
    assert_eq!(
        Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.0, 1.0, 1.0)),
        Err(TransformError::InvalidScale)
    );
    assert_eq!(
        Transform::new(Vec3::ZERO, Quat::from_xyzw(0.0, 0.0, 0.0, 0.0), Vec3::ONE),
        Err(TransformError::InvalidRotation)
    );
}

#[test]
fn scene_ids_are_stable_and_meshes_are_reused() {
    let mut scene = Scene::new();
    let mesh_id = scene.register_mesh(cube()).expect("registro válido");
    let first = scene
        .create_object(mesh_id, transform(Vec3::ZERO))
        .expect("objeto válido");
    let second = scene
        .create_object(mesh_id, transform(Vec3::X))
        .expect("objeto válido");
    scene.remove_object(first).expect("eliminación válida");
    let third = scene
        .create_object(mesh_id, transform(Vec3::Y))
        .expect("objeto válido");

    assert_ne!(first, second);
    assert_ne!(first, third);
    assert_eq!(
        scene.object(second).expect("objeto estable").mesh_id(),
        mesh_id
    );
    assert_eq!(
        scene.object(third).expect("objeto estable").mesh_id(),
        mesh_id
    );
    assert_eq!(scene.mesh_count(), 1);
    assert_eq!(scene.object_count(), 2);
}

#[test]
fn scene_rejects_missing_mesh_and_preserves_state_after_errors() {
    let mut scene = Scene::new();
    let mesh_id = scene.register_mesh(cube()).expect("registro válido");
    scene.remove_mesh(mesh_id).expect("malla sin uso");

    assert!(matches!(
        scene.create_object(mesh_id, transform(Vec3::ZERO)),
        Err(SceneError::MeshNotFound(id)) if id == mesh_id
    ));
    assert_eq!(scene.object_count(), 0);
    assert_eq!(scene.mesh_count(), 0);
}

#[test]
fn scene_visibility_transform_and_mesh_removal_follow_invariants() {
    let mut scene = Scene::new();
    let mesh_id = scene.register_mesh(cube()).expect("registro válido");
    let object_id = scene
        .create_object(mesh_id, transform(Vec3::ZERO))
        .expect("objeto válido");
    let updated = transform(Vec3::new(2.0, 0.0, 0.0));
    scene
        .set_transform(object_id, updated)
        .expect("transformación válida");
    scene
        .set_visibility(object_id, false)
        .expect("objeto existente");

    assert_eq!(scene.visible_objects().count(), 0);
    assert_eq!(
        scene
            .object(object_id)
            .expect("objeto existente")
            .transform(),
        updated
    );
    assert!(matches!(scene.remove_mesh(mesh_id), Err(SceneError::MeshInUse(id)) if id == mesh_id));
    scene.remove_object(object_id).expect("eliminación válida");
    assert_eq!(scene.remove_mesh(mesh_id).expect("malla liberada"), cube());
}

#[test]
fn scene_draw_preparation_shares_meshes_filters_visibility_and_keeps_geometry_local() {
    let mut scene = Scene::new();
    let mesh = cube();
    let original = mesh.clone();
    let mesh_id = scene.register_mesh(mesh).expect("registro válido");
    let first = scene
        .create_object(mesh_id, transform(Vec3::new(-1.0, 0.0, 0.0)))
        .expect("objeto válido");
    let second = scene
        .create_object(mesh_id, transform(Vec3::new(1.0, 0.0, 0.0)))
        .expect("objeto válido");
    let hidden = scene
        .create_object(mesh_id, transform(Vec3::new(0.0, 0.0, 2.0)))
        .expect("objeto válido");
    scene
        .set_visibility(hidden, false)
        .expect("objeto existente");

    let draws = prepare_scene_draws(&scene);
    assert_eq!(draws.len(), 2);
    assert_eq!(draws[0].object_id, first);
    assert_eq!(draws[1].object_id, second);
    assert!(draws.iter().all(|draw| draw.mesh_id == mesh_id));
    assert_ne!(draws[0].model_matrix, draws[1].model_matrix);
    assert_eq!(scene.mesh(mesh_id).expect("malla existente"), &original);
}
