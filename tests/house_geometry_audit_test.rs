use glam::{DVec2, DVec3};
use procedural_city_engine::{
    building::{Building, BuildingComponent, ComponentKind, ComponentRef},
    procedural::{derive_house_roof, generate_house, HouseConfig},
};

// La transformación se emite en f32. Ocho épsilon a la escala de la casa
// cubren composición y proyección. Las mediciones posteriores usan f64.
fn tolerance(config: &HouseConfig) -> f64 {
    8.0 * f64::from(f32::EPSILON)
        * f64::from(
            config
                .width
                .max(config.depth)
                .max(config.wall_height + config.roof_rise)
                .max(1.0),
        )
}

fn near(actual: f64, expected: f64, epsilon: f64) {
    assert!(
        (actual - expected).abs() <= epsilon,
        "{actual} ≉ {expected} (tol. {epsilon})"
    );
}

fn model(part: &BuildingComponent) -> glam::DMat4 {
    part.transform().model_matrix().unwrap().as_dmat4()
}

fn parts(house: &Building, kind: ComponentKind) -> impl Iterator<Item = &BuildingComponent> {
    house.components().filter(move |part| part.kind() == kind)
}

#[test]
fn roof_midplanes_meet_at_both_ends_with_symmetric_slopes() {
    let base = HouseConfig::default();
    for config in [
        base,
        HouseConfig {
            width: 0.5011,
            depth: 0.4001,
            roof_rise: 1024.0,
            ..base
        },
        HouseConfig {
            width: 1024.0,
            depth: 1024.0,
            wall_height: 1024.0,
            roof_rise: 0.001,
            ..base
        },
        HouseConfig {
            width: 1024.0,
            depth: 1024.0,
            wall_height: 1024.0,
            roof_rise: 1024.0,
            ..base
        },
    ] {
        let config = HouseConfig {
            door_width: 0.001,
            window_count: 0,
            ..config
        };
        let house = generate_house(&config).unwrap();
        let profile = derive_house_roof(&house).unwrap();
        near(
            profile.slope_radians(),
            f64::from(config.roof_rise).atan2(f64::from(config.width) / 2.0),
            8.0 * f64::from(f32::EPSILON),
        );
        let roofs: Vec<_> = parts(&house, ComponentKind::Roof).collect();
        assert_eq!(roofs.len(), 2);
        let epsilon = tolerance(&config);
        let run = f64::from(config.width) / 2.0;
        let rise = f64::from(config.roof_rise);
        let half_depth = f64::from(config.depth) / 2.0;
        let mut ridge_edges = [[DVec3::ZERO; 2]; 2];
        for (index, roof) in roofs.into_iter().enumerate() {
            let sign = if index == 0 { -1.0 } else { 1.0 };
            let parent = house
                .component(house.parent_of(roof.reference()).unwrap().unwrap())
                .unwrap();
            let wall_top = f64::from(parent.transform().translation().y)
                + f64::from(parent.dimensions().height()) / 2.0;
            assert_eq!(parent.kind(), ComponentKind::Wall);
            assert_eq!(f64::from(parent.transform().translation().x).signum(), sign);
            let half_length = f64::from(roof.dimensions().width()) / 2.0;
            near(half_length * 2.0, run.hypot(rise), epsilon);
            near(
                f64::from(roof.dimensions().depth()),
                half_depth * 2.0,
                epsilon,
            );
            for (end, z) in [-half_depth, half_depth].into_iter().enumerate() {
                let ridge = model(roof).transform_point3(DVec3::new(-sign * half_length, 0.0, z));
                let eave = model(roof).transform_point3(DVec3::new(sign * half_length, 0.0, z));
                ridge_edges[index][end] = ridge;
                near(ridge.x, 0.0, epsilon);
                near(ridge.y, wall_top + rise, epsilon);
                near(eave.x, sign * run, epsilon);
                near(eave.y, wall_top, epsilon);
                near(ridge.z, z, epsilon);
                near(eave.z, z, epsilon);
                let delta = ridge - eave;
                near(
                    delta.y.atan2(delta.x.abs()),
                    rise.atan2(run),
                    8.0 * f64::from(f32::EPSILON),
                );
            }
        }
        for (left, right) in ridge_edges[0].into_iter().zip(ridge_edges[1]) {
            near(left.distance(right), 0.0, epsilon);
        }
    }
}

#[test]
fn derived_roof_resolves_the_original_ridge_and_wall_join_conflicts() {
    let config = HouseConfig::default();
    let house = generate_house(&config).unwrap();
    let roofs: Vec<_> = parts(&house, ComponentKind::Roof).collect();
    let epsilon = tolerance(&config);
    let left = roofs[0];
    let right = roofs[1];
    let half_thickness = f64::from(left.dimensions().height()) / 2.0;
    let half_length = f64::from(left.dimensions().width()) / 2.0;
    let theta = f64::from(config.roof_rise).atan2(f64::from(config.width) / 2.0);
    let ridge_section = |normal_offset| {
        let a = model(left).transform_point3(DVec3::new(half_length, normal_offset, 0.0));
        let b = model(right).transform_point3(DVec3::new(-half_length, normal_offset, 0.0));
        near(a.y, b.y, epsilon);
        b.x - a.x
    };
    near(
        ridge_section(half_thickness),
        2.0 * half_thickness * theta.sin(),
        epsilon,
    );
    near(
        ridge_section(-half_thickness),
        -2.0 * half_thickness * theta.sin(),
        epsilon,
    );

    let support = house
        .component(house.parent_of(right.reference()).unwrap().unwrap())
        .unwrap();
    let inside_both = DVec3::new(3.9, 3.24, 0.0);
    for part in [right, support] {
        let point = model(part).inverse().transform_point3(inside_both);
        let half = part.dimensions().as_vec3().as_dvec3() / 2.0;
        assert!(
            point.abs().cmplt(half - DVec3::splat(epsilon)).all(),
            "{point:?} fuera de {half:?}"
        );
    }

    let corrected = derive_house_roof(&house).unwrap();
    for panel in corrected.panels() {
        assert!(panel.profile.iter().all(|vertex| vertex.y > inside_both.y));
        for (end, section) in panel.ends().iter().enumerate() {
            near(section[1].distance(corrected.ridge()[0][end]), 0.0, epsilon);
            near(section[2].distance(corrected.ridge()[1][end]), 0.0, epsilon);
        }
    }
}

#[derive(Clone, Copy)]
struct Opening {
    min: DVec2,
    max: DVec2,
}

fn opening_in_wall(wall: &BuildingComponent, part: &BuildingComponent, epsilon: f64) -> Opening {
    let relative = model(wall).inverse() * model(part);
    let half = part.dimensions().as_vec3().as_dvec3() / 2.0;
    let wall_half = wall.dimensions().as_vec3().as_dvec3() / 2.0;
    let mut bounds = Opening {
        min: DVec2::splat(f64::INFINITY),
        max: DVec2::splat(f64::NEG_INFINITY),
    };
    for x in [-half.x, half.x] {
        for y in [-half.y, half.y] {
            for z in [-half.z, half.z] {
                let point = relative.transform_point3(DVec3::new(x, y, z));
                near(point.z, z, epsilon);
                assert!(point.z.abs() <= wall_half.z + epsilon);
                let projected = point.truncate();
                bounds.min = bounds.min.min(projected);
                bounds.max = bounds.max.max(projected);
            }
        }
    }
    bounds
}

fn canonical(house: &Building) -> Vec<impl PartialEq + std::fmt::Debug> {
    house
        .components()
        .map(|part| {
            (
                part.id(),
                part.kind(),
                part.dimensions(),
                part.transform(),
                part.material(),
                house
                    .parent_of(part.reference())
                    .unwrap()
                    .map(ComponentRef::component_id),
            )
        })
        .collect()
}

#[test]
fn openings_respect_wall_local_rectangles_and_repeat_across_seed_sweep() {
    use ComponentKind::{Door, Foundation, Wall, Window};
    let base = HouseConfig::default();
    let configs = [
        HouseConfig {
            window_count: 0,
            ..base
        },
        HouseConfig {
            window_count: 1,
            ..base
        },
        HouseConfig {
            window_count: 11,
            ..base
        },
        HouseConfig {
            width: 2.5,
            depth: 0.4001,
            window_width: 2.0,
            window_count: 1,
            ..base
        },
        HouseConfig {
            width: 0.5011,
            depth: 0.4001,
            wall_height: 0.2011,
            roof_rise: 0.001,
            door_width: 0.001,
            door_height: 0.001,
            window_count: 0,
            ..base
        },
        HouseConfig {
            width: 1024.0,
            depth: 1024.0,
            wall_height: 1024.0,
            window_count: 25,
            window_width: 0.001,
            ..base
        },
    ];
    for template in configs {
        let mut prior = None;
        let mut varied = false;
        for seed in 0..32 {
            let config = HouseConfig { seed, ..template };
            let epsilon = tolerance(&config);
            let house = generate_house(&config).unwrap();
            let copy = generate_house(&config).unwrap();
            assert_ne!(house.id(), copy.id());
            let snapshot = canonical(&house);
            assert_eq!(snapshot, canonical(&copy));
            varied |= prior.as_ref().is_some_and(|previous| previous != &snapshot);
            prior = Some(snapshot);
            let mut found = [0, 0];
            for part in house.components() {
                assert_eq!(part.reference().building_id(), house.id());
                assert!(part.dimensions().as_vec3().is_finite());
                assert!(part.dimensions().as_vec3().min_element() > 0.0);
                assert!(model(part).is_finite());
                let parent = house.parent_of(part.reference()).unwrap();
                assert_eq!(parent.is_none(), part.kind() == Foundation);
                if let Some(parent) = parent {
                    assert_eq!(parent.building_id(), house.id());
                    assert!(house.component(parent).is_ok());
                }
            }
            for wall in parts(&house, Wall) {
                let half = wall.dimensions().as_vec3().as_dvec3() / 2.0;
                let front = model(wall).z_axis.z > 0.9;
                let mut openings: Vec<(ComponentKind, Opening)> = Vec::new();
                for child in house.children_of(wall.reference()).unwrap() {
                    let part = house.component(child).unwrap();
                    let bottom = match part.kind() {
                        Door => {
                            found[0] += 1;
                            assert!(front);
                            0.0
                        }
                        Window => {
                            found[1] += 1;
                            assert!(!front);
                            0.65
                        }
                        _ => continue,
                    };
                    let rect = opening_in_wall(wall, part, epsilon);
                    near(rect.min.y + half.y, bottom, epsilon);
                    assert!(rect.max.y <= half.y - 0.2 + epsilon);
                    assert!(rect.min.x >= -half.x + 0.25 - epsilon);
                    assert!(rect.max.x <= half.x - 0.25 + epsilon);
                    for (other_kind, other) in &openings {
                        let overlap = rect.max.min(other.max) - rect.min.max(other.min);
                        assert!(overlap.min_element() <= epsilon);
                        if *other_kind == Window && part.kind() == Window {
                            assert!(overlap.x <= -0.3 + epsilon);
                        }
                    }
                    openings.push((part.kind(), rect));
                }
            }
            assert_eq!(found, [1, config.window_count]);
        }
        if template.window_count == 0 || template.depth == 0.4001 {
            assert!(!varied);
        } else {
            assert!(varied);
        }
    }
}
