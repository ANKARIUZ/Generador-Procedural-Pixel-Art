use glam::{Vec3, Vec4};
use procedural_city_engine::render::{
    Camera, CameraController, CameraInput, FacePalette, PixelScale, RenderMode, RenderSettingsError,
};
use winit::dpi::PhysicalSize;

#[test]
fn mode_and_scale_are_explicit_and_validated() {
    assert_eq!(RenderMode::parse(None, None), Ok(RenderMode::Classic));
    assert_eq!(
        RenderMode::parse(Some("classic"), None),
        Ok(RenderMode::Classic)
    );
    assert_eq!(
        RenderMode::parse(Some("pixel"), None),
        Ok(RenderMode::Pixel(PixelScale::DEFAULT))
    );
    assert_eq!(
        RenderMode::parse(Some("pixels"), None),
        Err(RenderSettingsError::InvalidMode)
    );
    for value in ["0", "9", "-1", "2.5", ""] {
        assert_eq!(
            RenderMode::parse(Some("pixel"), Some(value)),
            Err(RenderSettingsError::InvalidScale)
        );
    }
    for scale in 1..=8 {
        assert!(PixelScale::new(scale).is_some());
    }
}

#[test]
fn layout_preserves_square_texels_with_centered_integer_viewport() {
    let scale = PixelScale::DEFAULT;
    assert_eq!(scale.layout(PhysicalSize::new(0, 10)), None);
    assert_eq!(scale.layout(PhysicalSize::new(10, 0)), None);
    let layout = scale.layout(PhysicalSize::new(1, 2)).unwrap();
    assert_eq!(layout.internal, PhysicalSize::new(1, 2));
    assert_eq!(layout.size, layout.internal);
    let layout = scale.layout(PhysicalSize::new(1025, 770)).unwrap();
    assert_eq!(layout.internal, PhysicalSize::new(341, 256));
    assert_eq!(layout.size, PhysicalSize::new(1023, 768));
    assert_eq!(layout.origin, [1, 1]);
    assert_eq!(
        layout.size.width * layout.internal.height,
        layout.size.height * layout.internal.width
    );
}

#[test]
fn house_view_is_isometric_framed_and_restored_after_navigation() {
    for size in [PhysicalSize::new(341, 256), PhysicalSize::new(180, 320)] {
        let mut camera = Camera::isometric(size);
        let initial = camera.view_projection();
        let direction = camera.position().normalize();
        assert!(direction.abs_diff_eq(Vec3::splat(1.0 / 3.0_f32.sqrt()), 1e-6));
        assert_eq!(camera.projection_matrix().w_axis.w, 1.0);
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let boundary = initial * (axis * 2.5).extend(1.0);
            assert!(boundary.x.abs() < 1.0 && boundary.y.abs() < 1.0);
            assert!((0.0..=1.0).contains(&boundary.z));
        }
        assert!(camera.orbit(0.3, -0.1));
        assert!(camera.pan_world(Vec3::X));
        assert!(camera.zoom_by_factor(0.8));
        assert_ne!(camera.view_projection() * Vec4::W, initial * Vec4::W);
        let mut controller = CameraController::default();
        assert!(controller.process_input(&mut camera, CameraInput::Reset, size));
        assert_eq!(camera.view_projection(), initial);
    }
}

#[test]
fn custom_palette_keeps_material_ids() {
    let entries = std::array::from_fn(|i| (100 + i as u32, [i as f32 / 5.0, 0.0, 0.0]));
    let palette = FacePalette::from_entries(entries);
    for (id, color) in entries {
        assert_eq!(palette.color_for(id), Ok(color));
        assert_eq!(
            palette.color_for(id),
            FacePalette::from_entries(entries).color_for(id)
        );
    }
    assert!(palette.color_for(0).is_err());
}
