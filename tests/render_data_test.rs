use glam::Vec3;
use procedural_city_engine::geometry::{primitives::Cube, Face, GeometryError, Mesh, Vertex};
use procedural_city_engine::render::{
    FacePalette, FixedCamera, GpuMeshData, GpuVertex, RenderDataError,
};
use winit::dpi::PhysicalSize;

#[test]
fn mesh_conversion_expands_triangles_and_preserves_positions() {
    let mesh = Cube::with_materials(2.0, [0, 1, 2, 3, 4, 5]).unwrap();
    let data = GpuMeshData::from_mesh(&mesh, &FacePalette::cube()).unwrap();

    assert_eq!(data.vertices.len(), 36);
    assert_eq!(data.indices.len(), 36);
    assert_eq!(data.indices, (0_u32..36).collect::<Vec<_>>());

    for (face_index, face) in mesh.faces.iter().enumerate() {
        for (corner, vertex_index) in face.indices.iter().enumerate() {
            assert_eq!(
                data.vertices[face_index * 3 + corner].position,
                mesh.vertices[*vertex_index as usize].position.to_array()
            );
        }
    }
}

#[test]
fn mesh_conversion_uses_the_material_id_for_every_face_color() {
    let mesh = Cube::with_materials(2.0, [0, 1, 2, 3, 4, 5]).unwrap();
    let palette = FacePalette::cube();
    let data = GpuMeshData::from_mesh(&mesh, &palette).unwrap();

    for (face_index, face) in mesh.faces.iter().enumerate() {
        let expected = palette.color_for(face.material_id).unwrap();
        assert!(data.vertices[face_index * 3..face_index * 3 + 3]
            .iter()
            .all(|vertex| vertex.color == expected));
    }
}

#[test]
fn mesh_conversion_rejects_unknown_materials() {
    let mesh = Cube::create(2.0, 99).unwrap();
    assert!(matches!(
        GpuMeshData::from_mesh(&mesh, &FacePalette::cube()),
        Err(RenderDataError::UnknownMaterial { material_id: 99 })
    ));
}

#[test]
fn mesh_conversion_revalidates_meshes_modified_after_construction() {
    let mut mesh = Cube::create(2.0, 0).unwrap();
    mesh.faces[0] = Face::new([0, 1, 99], 0);

    assert!(matches!(
        GpuMeshData::from_mesh(&mesh, &FacePalette::cube()),
        Err(RenderDataError::InvalidGeometry(
            GeometryError::IndexOutOfBounds { .. }
        ))
    ));
}

#[test]
fn gpu_vertex_layout_matches_the_cpu_representation() {
    let layout = GpuVertex::layout();
    assert_eq!(layout.array_stride, std::mem::size_of::<GpuVertex>() as u64);
    assert_eq!(std::mem::size_of::<GpuVertex>(), 24);
    assert_eq!(layout.attributes.len(), 2);
    assert_eq!(layout.attributes[0].format, wgpu::VertexFormat::Float32x3);
    assert_eq!(layout.attributes[0].offset, 0);
    assert_eq!(layout.attributes[1].format, wgpu::VertexFormat::Float32x3);
    assert_eq!(layout.attributes[1].offset, 12);
}

#[test]
fn fixed_camera_updates_aspect_ratio_and_keeps_origin_in_depth_range() {
    let mut camera = FixedCamera::new(PhysicalSize::new(800, 400));
    assert_eq!(camera.aspect_ratio(), 2.0);
    let clip_position = camera.view_projection() * Vec3::ZERO.extend(1.0);
    assert!(clip_position.z >= 0.0 && clip_position.z <= clip_position.w);

    camera.resize(PhysicalSize::new(400, 800));
    assert_eq!(camera.aspect_ratio(), 0.5);
}

#[test]
fn conversion_handles_a_valid_manual_triangle() {
    let mesh = Mesh::new(
        vec![
            Vertex::new(Vec3::ZERO),
            Vertex::new(Vec3::X),
            Vertex::new(Vec3::Y),
        ],
        vec![Face::new([0, 1, 2], 0)],
    )
    .unwrap();
    let data = GpuMeshData::from_mesh(&mesh, &FacePalette::cube()).unwrap();
    assert_eq!(data.vertices.len(), 3);
    assert_eq!(data.indices, vec![0, 1, 2]);
}
