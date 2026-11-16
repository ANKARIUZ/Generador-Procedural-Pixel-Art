#[test]
fn test_glam_math_dependency() {
    let vec = glam::Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(vec.x, 1.0);
    assert_eq!(vec.y, 2.0);
    assert_eq!(vec.z, 3.0);
}
