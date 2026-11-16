use std::time::Duration;

use glam::{Vec2, Vec3};
use procedural_city_engine::render::{
    Camera, CameraController, CameraInput, CameraMouseButton, CameraMove,
};
use winit::dpi::PhysicalSize;

const VIEWPORT: PhysicalSize<u32> = PhysicalSize::new(800, 600);

#[test]
fn camera_starts_at_the_previous_diagonal_position() {
    let camera = Camera::new(VIEWPORT);
    assert!(camera
        .position()
        .abs_diff_eq(Vec3::new(4.0, 3.0, 5.0), 1.0e-5));
    assert_eq!(camera.target(), Vec3::ZERO);
}

#[test]
fn view_matrix_points_toward_the_target_and_all_matrices_are_finite() {
    let camera = Camera::new(VIEWPORT);
    let target_in_view = camera.view_matrix() * camera.target().extend(1.0);
    assert!(target_in_view.x.abs() < 1.0e-5);
    assert!(target_in_view.y.abs() < 1.0e-5);
    assert!(target_in_view.z < 0.0);
    assert!(camera.view_matrix().is_finite());
    assert!(camera.projection_matrix().is_finite());
}

#[test]
fn orbit_wraps_yaw_and_clamps_pitch() {
    let mut camera = Camera::new(VIEWPORT);
    assert!(camera.orbit(std::f32::consts::TAU * 3.0, 100.0));
    assert!((0.0..std::f32::consts::TAU).contains(&camera.yaw()));
    assert_eq!(camera.pitch(), Camera::MAX_PITCH);
    camera.orbit(0.0, -100.0);
    assert_eq!(camera.pitch(), -Camera::MAX_PITCH);
}

#[test]
fn orbit_rejects_non_finite_deltas_without_changing_camera() {
    let mut camera = Camera::new(VIEWPORT);
    let position = camera.position();
    assert!(!camera.orbit(f32::NAN, 0.0));
    assert_eq!(camera.position(), position);
}

#[test]
fn orthographic_zoom_respects_limits_and_rejects_invalid_factors() {
    let mut camera = Camera::new(VIEWPORT);
    assert!(camera.zoom_by_factor(0.0001));
    assert_eq!(camera.ortho_scale(), Camera::MIN_ORTHO_SCALE);
    assert!(camera.zoom_by_factor(10_000.0));
    assert_eq!(camera.ortho_scale(), Camera::MAX_ORTHO_SCALE);
    let scale = camera.ortho_scale();
    assert!(!camera.zoom_by_factor(0.0));
    assert!(!camera.zoom_by_factor(f32::NAN));
    assert_eq!(camera.ortho_scale(), scale);
}

#[test]
fn reset_restores_the_initial_camera_state() {
    let mut camera = Camera::new(VIEWPORT);
    camera.orbit(1.0, -0.5);
    camera.zoom_by_factor(2.0);
    camera.pan_world(Vec3::new(3.0, 0.0, -4.0));
    camera.reset(VIEWPORT);
    assert!(camera
        .position()
        .abs_diff_eq(Vec3::new(4.0, 3.0, 5.0), 1.0e-5));
    assert_eq!(camera.target(), Vec3::ZERO);
    assert_eq!(camera.ortho_scale(), 7.0);
}

#[test]
fn pan_remains_horizontal_and_tracks_camera_orientation() {
    let mut camera = Camera::new(VIEWPORT);
    let right_before = camera.horizontal_right();
    assert!(camera.pan_world(right_before));
    assert_eq!(camera.target().y, 0.0);
    let target_after_first_pan = camera.target();
    camera.orbit(std::f32::consts::FRAC_PI_2, 0.0);
    let right_after = camera.horizontal_right();
    assert!(!right_before.abs_diff_eq(right_after, 1.0e-5));
    camera.pan_world(right_after);
    assert_ne!(camera.target(), target_after_first_pan);
}

#[test]
fn controller_rotates_only_after_a_drag_delta() {
    let mut camera = Camera::new(VIEWPORT);
    let mut controller = CameraController::default();
    controller.process_input(
        &mut camera,
        CameraInput::MouseButton {
            button: CameraMouseButton::Left,
            pressed: true,
        },
        VIEWPORT,
    );
    assert!(!controller.process_input(
        &mut camera,
        CameraInput::CursorMoved(Vec2::new(100.0, 100.0)),
        VIEWPORT,
    ));
    let yaw = camera.yaw();
    assert!(controller.process_input(
        &mut camera,
        CameraInput::CursorMoved(Vec2::new(140.0, 120.0)),
        VIEWPORT,
    ));
    assert_ne!(camera.yaw(), yaw);
}

#[test]
fn controller_scroll_and_pan_follow_input_rules() {
    let mut camera = Camera::new(VIEWPORT);
    let mut controller = CameraController::default();
    let initial_scale = camera.ortho_scale();
    assert!(controller.process_input(&mut camera, CameraInput::ScrollLines(1.0), VIEWPORT));
    assert!(camera.ortho_scale() < initial_scale);
    assert!(!controller.process_input(
        &mut camera,
        CameraInput::MouseButton {
            button: CameraMouseButton::Middle,
            pressed: true,
        },
        VIEWPORT,
    ));
    controller.process_input(
        &mut camera,
        CameraInput::CursorMoved(Vec2::new(10.0, 10.0)),
        VIEWPORT,
    );
    assert!(controller.process_input(
        &mut camera,
        CameraInput::CursorMoved(Vec2::new(50.0, 30.0)),
        VIEWPORT,
    ));
    assert_ne!(camera.target(), Vec3::ZERO);
}

#[test]
fn controller_keyboard_motion_is_temporized_and_stops_after_focus_loss() {
    let mut camera = Camera::new(VIEWPORT);
    let mut controller = CameraController::default();
    controller.process_input(
        &mut camera,
        CameraInput::Move {
            direction: CameraMove::Forward,
            pressed: true,
        },
        VIEWPORT,
    );
    assert!(controller.has_continuous_motion());
    let target = camera.target();
    assert!(controller.update(&mut camera, Duration::from_millis(16), VIEWPORT));
    assert_ne!(camera.target(), target);
    controller.process_input(&mut camera, CameraInput::FocusLost, VIEWPORT);
    assert!(!controller.has_continuous_motion());
    assert!(!controller.update(&mut camera, Duration::from_millis(16), VIEWPORT));
}

#[test]
fn resize_preserves_orthographic_projection_and_depth_conventions() {
    let mut camera = Camera::new(PhysicalSize::new(800, 400));
    assert_eq!(camera.aspect_ratio(), 2.0);
    assert_eq!(camera.projection_matrix().w_axis.w, 1.0);
    let clip_position = camera.view_projection() * Vec3::ZERO.extend(1.0);
    assert!(clip_position.z >= 0.0 && clip_position.z <= clip_position.w);
    camera.resize(PhysicalSize::new(400, 800));
    assert_eq!(camera.aspect_ratio(), 0.5);
    assert!(camera.view_projection().is_finite());
}

#[test]
fn pointer_left_clears_drag_state() {
    let mut camera = Camera::new(VIEWPORT);
    let mut controller = CameraController::default();
    controller.process_input(
        &mut camera,
        CameraInput::MouseButton {
            button: CameraMouseButton::Left,
            pressed: true,
        },
        VIEWPORT,
    );
    controller.process_input(
        &mut camera,
        CameraInput::CursorMoved(Vec2::new(0.0, 0.0)),
        VIEWPORT,
    );
    controller.process_input(&mut camera, CameraInput::PointerLeft, VIEWPORT);
    assert!(!controller.process_input(
        &mut camera,
        CameraInput::CursorMoved(Vec2::new(50.0, 0.0)),
        VIEWPORT,
    ));
}
