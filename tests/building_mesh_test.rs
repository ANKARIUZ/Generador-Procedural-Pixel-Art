use glam::Vec3;
use procedural_city_engine::{
    building::{Building, ComponentKind},
    meshing::{mesh_building, validate_closed_mesh},
    procedural::{derive_house_roof, generate_house, HouseConfig},
    render::{FacePalette, GpuMeshData},
};

#[test]
fn closed_envelope_and_inserts_work_at_different_scales() {
    let base = HouseConfig::default();
    for config in [
        base,
        HouseConfig {
            window_count: 0,
            ..base
        },
        HouseConfig {
            window_count: 11,
            roof_rise: 3.5,
            ..base
        },
        HouseConfig {
            width: 0.5011,
            depth: 0.4001,
            door_width: 0.001,
            door_height: 0.001,
            wall_height: 0.2011,
            window_count: 0,
            roof_rise: 0.001,
            ..base
        },
        HouseConfig {
            width: 1024.0,
            depth: 1024.0,
            wall_height: 1024.0,
            window_width: 0.001,
            roof_rise: 0.001,
            ..base
        },
    ] {
        let house = generate_house(&config).unwrap();
        let result = mesh_building(&house).unwrap_or_else(|error| panic!("{config:?}: {error}"));
        validate_closed_mesh(&result.mesh).unwrap();
        let mut envelope = result.mesh.clone();
        envelope.faces.truncate(result.envelope_faces.end);
        validate_closed_mesh(&envelope).unwrap();
        assert_eq!(result.sources.len(), result.mesh.faces.len());
        assert_eq!(
            result.mesh.faces.len() - envelope.faces.len(),
            12 * (1 + config.window_count)
        );
        let bounds = result.mesh.bounds().unwrap();
        assert_eq!(bounds.min.x, -config.width / 2.0);
        assert_eq!(bounds.max.x, config.width / 2.0);
        assert_eq!(bounds.min.z, -config.depth / 2.0);
        assert_eq!(bounds.max.z, config.depth / 2.0);
        assert_eq!(
            bounds.max.y,
            derive_house_roof(&house).unwrap().ridge()[1][0].y as f32
        );
        for (source, face) in result.sources.iter().zip(&result.mesh.faces) {
            let part = house.component(*source).unwrap();
            assert_eq!(face.material_id, part.material().0);
        }
        for part in house.components() {
            assert!(result.sources.contains(&part.reference()));
        }
        assert!(GpuMeshData::from_mesh(&result.mesh, &FacePalette::house()).is_ok());
    }
}

fn snapshot(house: &Building) -> Vec<impl PartialEq + std::fmt::Debug> {
    house
        .components()
        .map(|part| (part.clone(), house.parent_of(part.reference()).unwrap()))
        .collect()
}

#[test]
fn conversion_is_deterministic_and_preserves_the_building() {
    let config = HouseConfig {
        seed: 42,
        ..HouseConfig::default()
    };
    let house = generate_house(&config).unwrap();
    let before = snapshot(&house);
    let a = mesh_building(&house).unwrap();
    let b = mesh_building(&generate_house(&config).unwrap()).unwrap();
    assert_eq!(before, snapshot(&house));
    assert_eq!(a.mesh, b.mesh);
    assert_eq!(a.envelope_faces, b.envelope_faces);
    assert!(a
        .sources
        .iter()
        .zip(b.sources)
        .all(|(a, b)| a.building_id() != b.building_id() && a.component_id() == b.component_id()));
}

fn segment_hits_triangle(origin: Vec3, segment: Vec3, [a, b, c]: [Vec3; 3]) -> bool {
    let [a, b, c] = [a, b, c].map(|point| point.as_dvec3());
    let origin = origin.as_dvec3();
    let segment = segment.as_dvec3();
    let p = segment.cross(c - a);
    let det = (b - a).dot(p);
    if det.abs() < 1e-12 {
        return false;
    }
    let u = (origin - a).dot(p) / det;
    let q = (origin - a).cross(b - a);
    let v = segment.dot(q) / det;
    let t = (c - a).dot(q) / det;
    u >= 0.0 && v >= 0.0 && u + v <= 1.0 && (0.0..=1.0).contains(&t)
}

#[test]
fn opening_centers_cross_the_wall_and_jambs_face_the_void() {
    let house = generate_house(&HouseConfig::default()).unwrap();
    let result = mesh_building(&house).unwrap();
    for hole in house
        .components()
        .filter(|part| matches!(part.kind(), ComponentKind::Door | ComponentKind::Window))
    {
        let pose = hole.transform();
        let wall = house.parent_of(hole.reference()).unwrap().unwrap();
        let through = pose.rotation() * Vec3::Z * hole.dimensions().depth() * 1.2;
        let center = pose.translation();
        let hit = |position: Vec3, faces: std::ops::Range<usize>| {
            result.mesh.faces[faces].iter().any(|face| {
                segment_hits_triangle(
                    position + through * 0.5,
                    -through,
                    face.indices
                        .map(|index| result.mesh.vertices[index as usize].position),
                )
            })
        };
        for offset in [-0.3, 0.0, 0.3] {
            let point = center + pose.rotation() * Vec3::X * hole.dimensions().width() * offset;
            assert!(!hit(point, result.envelope_faces.clone()));
            assert!(hit(
                point,
                result.envelope_faces.end..result.mesh.faces.len()
            ));
        }
        let beside = center + pose.rotation() * Vec3::X * (hole.dimensions().width() / 2.0 + 0.1);
        assert!(hit(beside, result.envelope_faces.clone()));
        let wall_pose = house.component(wall).unwrap().transform();
        let mut jambs = [false; 2];
        for i in result
            .envelope_faces
            .clone()
            .filter(|&i| result.sources[i] == wall)
        {
            let face = result.mesh.faces[i];
            let points = face.indices.map(|index| {
                wall_pose.rotation().inverse()
                    * (result.mesh.vertices[index as usize].position - center)
            });
            for (i_side, sign) in [-1.0, 1.0].into_iter().enumerate() {
                if points
                    .iter()
                    .all(|p| (p.x - sign * hole.dimensions().width() / 2.0).abs() < 1e-5)
                    && points.iter().any(|p| p.z > 0.09)
                    && points.iter().any(|p| p.z < -0.09)
                {
                    let normal = result.mesh.face_normal(i).unwrap().normalize();
                    assert!((wall_pose.rotation().inverse() * normal).x * sign < -0.99);
                    jambs[i_side] = true;
                }
            }
        }
        assert_eq!(jambs, [true, true]);
    }
}

#[test]
fn roof_boundary_has_two_outer_slopes_and_no_internal_ridge_faces() {
    let house = generate_house(&HouseConfig::default()).unwrap();
    let result = mesh_building(&house).unwrap();
    let profile = derive_house_roof(&house).unwrap();
    let [lower, upper] = profile.ridge();
    let mut slopes = [[false; 2]; 2];
    let mut gables = [false; 2];
    let mut signed_volume = 0.0;
    for index in result.envelope_faces.clone() {
        let [a, b, c] = result.mesh.faces[index]
            .indices
            .map(|id| result.mesh.vertices[id as usize].position.as_dvec3());
        signed_volume += a.dot(b.cross(c)) / 6.0;
        let normal = (b - a).cross(c - a).normalize();
        let part = house.component(result.sources[index]).unwrap();
        assert!(![a, b, c].iter().all(|point| point.x == 0.0));

        if part.kind() == ComponentKind::Roof && normal.y.abs() > 0.1 {
            let side = usize::from(part.transform().translation().x > 0.0);
            let ridge = if normal.y > 0.0 { upper } else { lower };
            for point in [a, b, c] {
                let expected = ridge[0].y - profile.slope_radians().tan() * point.x.abs();
                assert!((point.y - expected).abs() < 1e-5);
            }
            slopes[side][usize::from(normal.y > 0.0)] = true;
        }
        if part.kind() == ComponentKind::Wall
            && [a, b, c].iter().all(|p| p.z.abs() == 3.0)
            && [a, b, c].iter().any(|p| p.y > 4.0)
        {
            assert!(normal.z * a.z.signum() > 0.99);
            gables[usize::from(a.z > 0.0)] = true;
        }
    }
    assert_eq!(slopes, [[true; 2]; 2]);
    assert_eq!(gables, [true; 2]);
    assert!(signed_volume > 0.0);
}

#[test]
fn closed_validation_rejects_duplicate_faces_and_missing_or_reversed_triangles() {
    let house = generate_house(&HouseConfig::default()).unwrap();
    let original = mesh_building(&house).unwrap().mesh;
    for action in 0..3 {
        let mut mesh = original.clone();
        match action {
            0 => {
                mesh.faces.pop();
            }
            1 => mesh.faces.push(mesh.faces[0]),
            _ => mesh.faces[0].indices.swap(0, 1),
        }
        assert!(matches!(
            validate_closed_mesh(&mesh),
            Err(procedural_city_engine::meshing::MeshingError::InvalidTopology)
        ));
    }
}
