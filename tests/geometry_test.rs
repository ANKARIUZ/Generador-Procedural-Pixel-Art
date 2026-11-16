use glam::Vec3;
use procedural_city_engine::geometry::{
    primitives::{Cube, RectangularPrism},
    Face, GeometryError, Mesh, Vertex,
};

#[test]
fn cube_has_expected_dimensions_and_topology() {
    let mesh = Cube::create(4.0, 7).expect("un cubo válido debe construirse");
    let bounds = mesh.bounds().expect("el cubo tiene vértices");

    assert_eq!(mesh.vertices.len(), 8);
    assert_eq!(
        mesh.faces.len(),
        12,
        "seis cuadriláteros son doce triángulos"
    );
    assert_eq!(bounds.min, Vec3::splat(-2.0));
    assert_eq!(bounds.max, Vec3::splat(2.0));
}

#[test]
fn cube_indices_are_valid_and_normals_point_outward() {
    let mesh = Cube::create(2.0, 0).unwrap();
    mesh.validate().unwrap();

    for (index, face) in mesh.faces.iter().enumerate() {
        assert!(face
            .indices
            .iter()
            .all(|&vertex| vertex < mesh.vertices.len() as u32));
        let center = face
            .indices
            .iter()
            .map(|&vertex| mesh.vertices[vertex as usize].position)
            .sum::<Vec3>()
            / 3.0;
        assert!(
            mesh.face_normal(index).unwrap().dot(center) > 0.0,
            "la cara {index} no apunta hacia afuera"
        );
    }
}

#[test]
fn prism_respects_each_dimension() {
    let mesh = RectangularPrism::create(2.0, 4.0, 6.0, 3).unwrap();
    let bounds = mesh.bounds().unwrap();
    assert_eq!(bounds.max - bounds.min, Vec3::new(2.0, 4.0, 6.0));
    assert_normals_point_outward(&mesh);
}

#[test]
fn invalid_primitive_dimensions_are_rejected() {
    for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(matches!(
            Cube::create(value, 0),
            Err(GeometryError::InvalidDimension { .. })
        ));
    }
    assert!(matches!(
        RectangularPrism::create(1.0, 0.0, 1.0, 0),
        Err(GeometryError::InvalidDimension { .. })
    ));
}

#[test]
fn materials_are_preserved_per_quad_side() {
    let mesh = Cube::with_materials(1.0, [10, 11, 12, 13, 14, 15]).unwrap();
    let materials: Vec<u32> = mesh.faces.iter().map(|face| face.material_id).collect();
    assert_eq!(
        materials,
        vec![10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15]
    );
}

#[test]
fn primitive_generation_is_deterministic_and_gpu_independent() {
    assert_eq!(
        Cube::with_materials(3.0, [1, 2, 3, 4, 5, 6]),
        Cube::with_materials(3.0, [1, 2, 3, 4, 5, 6])
    );
}

#[test]
fn invalid_mesh_data_is_rejected() {
    let vertices = vec![
        Vertex::new(Vec3::ZERO),
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
    ];
    assert!(matches!(
        Mesh::new(vertices.clone(), vec![Face::new([0, 1, 3], 0)]),
        Err(GeometryError::IndexOutOfBounds { .. })
    ));
    assert!(matches!(
        Mesh::new(vertices, vec![Face::new([0, 0, 1], 0)]),
        Err(GeometryError::RepeatedTriangleIndex { .. })
    ));
    assert!(matches!(
        Mesh::new(vec![Vertex::new(Vec3::new(f32::NAN, 0.0, 0.0))], vec![]),
        Err(GeometryError::NonFiniteVertex { .. })
    ));
}

#[test]
fn repeated_triangle_indices_are_rejected_explicitly() {
    let vertices = triangle_vertices();
    assert!(matches!(
        Mesh::new(vertices, vec![Face::new([0, 1, 1], 0)]),
        Err(GeometryError::RepeatedTriangleIndex { .. })
    ));
}

#[test]
fn collinear_triangles_are_rejected_as_degenerate() {
    assert!(matches!(
        Mesh::new(
            vec![
                Vertex::new(Vec3::ZERO),
                Vertex::new(Vec3::X),
                Vertex::new(Vec3::X * 2.0),
            ],
            vec![Face::new([0, 1, 2], 0)],
        ),
        Err(GeometryError::DegenerateTriangle { .. })
    ));
}

#[test]
fn a_small_but_valid_triangle_is_not_rejected_by_an_absolute_area_threshold() {
    let mesh = Mesh::new(
        vec![
            Vertex::new(Vec3::ZERO),
            Vertex::new(Vec3::new(1.0e-10, 0.0, 0.0)),
            Vertex::new(Vec3::new(0.0, 1.0e-10, 0.0)),
        ],
        vec![Face::new([0, 1, 2], 0)],
    )
    .expect("un triángulo pequeño no degenerado debe ser válido");

    assert!(mesh.face_normal(0).unwrap().z > 0.0);
}

#[test]
fn face_normal_reports_corrupted_mesh_data_without_panicking() {
    let mut mesh = Mesh::new(triangle_vertices(), vec![Face::new([0, 1, 2], 0)]).unwrap();
    assert_eq!(mesh.face_normal(0).unwrap(), Vec3::Z);

    mesh.faces[0] = Face::new([0, 1, 99], 0);
    assert!(matches!(
        mesh.face_normal(0),
        Err(GeometryError::IndexOutOfBounds { .. })
    ));
    assert!(matches!(
        mesh.face_normal(1),
        Err(GeometryError::FaceIndexOutOfBounds { .. })
    ));
}

#[test]
fn face_normal_reports_values_outside_vec3_range() {
    let mesh = Mesh::new(
        vec![
            Vertex::new(Vec3::ZERO),
            Vertex::new(Vec3::new(f32::MAX, 0.0, 0.0)),
            Vertex::new(Vec3::new(0.0, f32::MAX, 0.0)),
        ],
        vec![Face::new([0, 1, 2], 0)],
    )
    .unwrap();
    assert!(matches!(
        mesh.face_normal(0),
        Err(GeometryError::NormalOutOfRange { .. })
    ));
}

#[test]
fn add_quad_accepts_a_convex_coplanar_quad() {
    let mut mesh = quad_mesh([
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]);

    mesh.add_quad([0, 1, 2, 3], 9).unwrap();
    assert_eq!(mesh.faces.len(), 2);
    assert!(mesh.face_normal(0).unwrap().z > 0.0);
    assert!(mesh.face_normal(1).unwrap().z > 0.0);
}

#[test]
fn add_quad_rejects_concave_non_coplanar_and_self_intersecting_input_atomically() {
    let mut concave = quad_mesh([
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(1.0, 0.5, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]);
    assert!(matches!(
        concave.add_quad([0, 1, 2, 3], 0),
        Err(GeometryError::NonConvexQuad)
    ));
    assert!(concave.faces.is_empty());

    let mut non_coplanar = quad_mesh([
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.25),
    ]);
    assert!(matches!(
        non_coplanar.add_quad([0, 1, 2, 3], 0),
        Err(GeometryError::NonCoplanarQuad)
    ));
    assert!(non_coplanar.faces.is_empty());

    let mut self_intersecting = quad_mesh([
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
    ]);
    assert!(matches!(
        self_intersecting.add_quad([0, 1, 2, 3], 0),
        Err(GeometryError::SelfIntersectingQuad)
    ));
    assert!(self_intersecting.faces.is_empty());
}

#[test]
fn add_quad_rejects_invalid_indices_without_mutating_the_mesh() {
    let mut mesh = quad_mesh([
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]);

    assert!(matches!(
        mesh.add_quad([0, 1, 2, 8], 0),
        Err(GeometryError::IndexOutOfBounds { .. })
    ));
    assert!(mesh.faces.is_empty());
    assert!(matches!(
        mesh.add_quad([0, 1, 1, 3], 0),
        Err(GeometryError::InvalidQuad)
    ));
    assert!(mesh.faces.is_empty());
}

#[test]
fn add_quad_rejects_non_finite_vertices_without_mutating_the_mesh() {
    let mut mesh = quad_mesh([
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]);
    mesh.vertices[3] = Vertex::new(Vec3::new(f32::NAN, 0.0, 0.0));

    assert!(matches!(
        mesh.add_quad([0, 1, 2, 3], 0),
        Err(GeometryError::NonFiniteVertex { .. })
    ));
    assert!(mesh.faces.is_empty());
}

fn triangle_vertices() -> Vec<Vertex> {
    vec![
        Vertex::new(Vec3::ZERO),
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
    ]
}

fn quad_mesh(points: [Vec3; 4]) -> Mesh {
    Mesh::new(points.into_iter().map(Vertex::new).collect(), Vec::new()).unwrap()
}

fn assert_normals_point_outward(mesh: &Mesh) {
    for (index, face) in mesh.faces.iter().enumerate() {
        let center = face
            .indices
            .iter()
            .map(|&vertex| mesh.vertices[vertex as usize].position)
            .sum::<Vec3>()
            / 3.0;
        assert!(mesh.face_normal(index).unwrap().dot(center) > 0.0);
    }
}
