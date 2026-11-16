use glam::DVec3;
use procedural_city_engine::{
    building::Building,
    geometry::roof::{RoofDimensions, RoofProfile, RoofProfileError},
    procedural::{derive_house_roof, generate_house, HouseConfig, HouseRoofError},
};

fn dimensions() -> RoofDimensions {
    RoofDimensions {
        width: 8.0,
        depth: 6.0,
        wall_top: 3.25,
        rise: 1.5,
        normal_thickness: 0.12,
        wall_thickness: 0.2,
    }
}

fn near(a: f64, b: f64, epsilon: f64) {
    assert!((a - b).abs() <= epsilon, "{a} ≉ {b}, tolerancia {epsilon}");
}

#[test]
fn panels_join_at_both_ridge_edges_and_keep_normal_thickness_and_flush_cuts() {
    let base = dimensions();
    for input in [
        base,
        RoofDimensions {
            width: 0.5011,
            depth: 0.4001,
            rise: 0.001,
            ..base
        },
        RoofDimensions {
            width: 1024.0,
            depth: 1024.0,
            wall_top: 1024.45,
            rise: 1024.0,
            normal_thickness: 0.4,
            ..base
        },
        RoofDimensions {
            width: 0.5011,
            depth: 0.4001,
            rise: 1024.0,
            ..base
        },
    ] {
        let roof = RoofProfile::new(input).unwrap();
        let [left, right] = roof.panels();
        let scale = input.width.max(input.depth).max(roof.ridge()[1][0].y);
        let epsilon = 32.0 * f64::EPSILON * scale;
        let z_limit = input.depth / 2.0;
        for (panel, side) in [(left, -1.0), (right, 1.0)] {
            for (end, points) in panel.ends().into_iter().enumerate() {
                let [lower_eave, lower_ridge, upper_ridge, upper_eave] = points;
                let z = [-z_limit, z_limit][end];
                for point in &points {
                    assert!(point.is_finite());
                    near(point.z, z, epsilon);
                    assert!(point.x * side >= 0.0 && point.x.abs() <= input.width / 2.0);
                }
                near(lower_ridge.distance(roof.ridge()[0][end]), 0.0, epsilon);
                near(upper_ridge.distance(roof.ridge()[1][end]), 0.0, epsilon);
                near(lower_ridge.y, input.wall_top + input.rise, epsilon);
                near(lower_eave.y, input.wall_top, epsilon);
                near(lower_eave.x, side * input.width / 2.0, epsilon);
                near(upper_eave.x, lower_eave.x, epsilon);
                let run = lower_ridge - lower_eave;
                let normal = DVec3::new(side * run.y, run.x.abs(), 0.0).normalize();
                near(run.y.atan2(run.x.abs()), roof.slope_radians(), epsilon);
                near(
                    (upper_eave - lower_eave).dot(normal),
                    input.normal_thickness,
                    epsilon,
                );
                near(
                    (upper_ridge - lower_ridge).dot(normal),
                    input.normal_thickness,
                    epsilon,
                );
            }
        }
    }
}

#[test]
fn wall_extensions_fill_to_the_underside_and_partition_the_corner_bands() {
    let input = dimensions();
    let roof = RoofProfile::new(input).unwrap();
    let panels = roof.panels();
    let gables = roof.gables();
    let caps = roof.side_caps();
    let epsilon = 32.0 * f64::EPSILON * input.width;
    let roof_bottom = |x: f64| {
        let panel = &panels[usize::from(x > 0.0)];
        let [eave, ridge, ..] = panel.profile;
        let fraction = (x - eave.x) / (ridge.x - eave.x);
        eave.y + fraction * (ridge.y - eave.y)
    };
    for gable in &gables {
        let [left, right, peak] = gable.profile;
        near(left.x, -input.width / 2.0, epsilon);
        near(right.x, input.width / 2.0, epsilon);
        for point in [left, right, peak] {
            near(point.y, roof_bottom(point.x), epsilon);
        }
        near(
            *gable.z.end() - *gable.z.start(),
            input.wall_thickness,
            epsilon,
        );
    }
    near(*gables[0].z.start(), *panels[0].z.start(), epsilon);
    near(*gables[1].z.end(), *panels[0].z.end(), epsilon);
    for cap in &caps {
        near(*cap.z.start(), *gables[0].z.end(), epsilon);
        near(*cap.z.end(), *gables[1].z.start(), epsilon);
        let [outer, inner_base, inner_top] = cap.profile;
        near(
            (outer.x - inner_base.x).abs(),
            input.wall_thickness,
            epsilon,
        );
        near(inner_top.x, inner_base.x, epsilon);
        near(outer.y, input.wall_top, epsilon);
        near(inner_base.y, input.wall_top, epsilon);
        near(inner_top.y, roof_bottom(inner_top.x), epsilon);
        // Al ser lineales, los extremos iguales garantizan contacto en la arista completa.
        for (end, gable) in gables.iter().enumerate() {
            let section = cap.ends()[end];
            let gable_end = gable.ends()[1 - end];
            for point in section {
                near(point.z, gable_end[0].z, epsilon);
                assert!(point.y >= input.wall_top && point.y <= roof_bottom(point.x) + epsilon);
            }
        }
    }
}

#[test]
fn rejects_invalid_and_numerically_collapsed_inputs() {
    let base = dimensions();
    for input in [
        RoofDimensions { rise: 0.0, ..base },
        RoofDimensions {
            depth: f64::NAN,
            ..base
        },
        RoofDimensions {
            normal_thickness: f64::INFINITY,
            ..base
        },
        RoofDimensions {
            wall_thickness: 3.0,
            ..base
        },
        RoofDimensions {
            width: -8.0,
            ..base
        },
    ] {
        assert_eq!(
            RoofProfile::new(input),
            Err(RoofProfileError::InvalidDimensions)
        );
    }
    assert_eq!(
        RoofProfile::new(RoofDimensions {
            wall_top: 1e30,
            ..base
        }),
        Err(RoofProfileError::UnresolvableProfile)
    );
    assert_eq!(
        derive_house_roof(&Building::new().unwrap()),
        Err(HouseRoofError::IncompatibleHouse)
    );
}

#[test]
fn repeated_generation_derives_identical_profiles() {
    let config = HouseConfig::default();
    let first = generate_house(&config).unwrap();
    let second = generate_house(&config).unwrap();
    assert_ne!(first.id(), second.id());
    let a = derive_house_roof(&first).unwrap();
    let b = derive_house_roof(&second).unwrap();
    assert_eq!(a, b);
    let [[lower, _], [upper, _]] = a.ridge();
    near(lower.y, 4.75, 2e-6);
    near(upper.y, 4.8781601, 2e-6);
    near(a.normal_thickness(), 0.12, 2e-8);
    assert_eq!(lower.x, 0.0);
}
