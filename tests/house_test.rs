use glam::Vec4;
use procedural_city_engine::{
    building::{Building, BuildingMaterialId, ComponentKind, ComponentRef},
    procedural::{generate_house, HouseConfig, HouseGenerationError, HouseMaterials},
};

fn canonical(building: &Building) -> Vec<impl PartialEq + std::fmt::Debug> {
    building
        .components()
        .map(|component| {
            (
                component.id(),
                component.kind(),
                component.dimensions(),
                component.transform(),
                component.material(),
                building
                    .parent_of(component.reference())
                    .unwrap()
                    .map(ComponentRef::component_id),
            )
        })
        .collect()
}

fn near(value: f32, expected: f32) {
    assert!((value - expected).abs() < 0.00002, "{value} != {expected}");
}

#[test]
fn generated_house_has_coherent_dimensions_materials_and_relationships() {
    use ComponentKind::*;
    let config = HouseConfig {
        materials: HouseMaterials {
            foundation: BuildingMaterialId(10),
            floor: BuildingMaterialId(11),
            wall: BuildingMaterialId(12),
            door: BuildingMaterialId(13),
            window: BuildingMaterialId(14),
            roof: BuildingMaterialId(15),
        },
        ..HouseConfig::default()
    };
    let house = generate_house(&config).unwrap();
    assert_eq!(house.component_count(), 9 + config.window_count);
    let components: Vec<_> = house.components().collect();
    for (kind, count) in [
        (Foundation, 1),
        (Floor, 1),
        (Wall, 4),
        (Door, 1),
        (Window, config.window_count),
        (Roof, 2),
    ] {
        assert_eq!(
            components.iter().filter(|part| part.kind() == kind).count(),
            count,
            "{kind:?}"
        );
    }
    for part in &components {
        let ref_id = part.reference();
        assert_eq!(ref_id.building_id(), house.id());
        assert_eq!(house.component(ref_id).unwrap().id(), part.id());
        let size = part.dimensions().as_vec3();
        assert!(size.is_finite() && size.min_element() > 0.0);
        assert!(part.transform().model_matrix().unwrap().is_finite());
        assert!(part.transform().rotation().is_normalized());
        let parent = house.parent_of(ref_id).unwrap();
        if let Some(parent) = parent {
            assert_eq!(parent.building_id(), house.id());
            assert!(house
                .children_of(parent)
                .unwrap()
                .any(|child| child == ref_id));
        }
        let parent_kind = parent.map(|reference| house.component(reference).unwrap().kind());
        let (expected_parent, material) = match part.kind() {
            Foundation => (None, config.materials.foundation),
            Floor => (Some(Foundation), config.materials.floor),
            Wall => (Some(Floor), config.materials.wall),
            Door => (Some(Wall), config.materials.door),
            Window => (Some(Wall), config.materials.window),
            Roof => (Some(Wall), config.materials.roof),
            Column => panic!("columna inesperada"),
        };
        assert_eq!(parent_kind, expected_parent);
        assert_eq!(part.material(), material);
    }
    let foundation = components[0];
    let floor = components[1];
    near(
        foundation.transform().translation().y - foundation.dimensions().height() / 2.0,
        0.0,
    );
    near(floor.dimensions().width(), config.width);
    near(floor.dimensions().depth(), config.depth);
    near(
        foundation.transform().translation().y + foundation.dimensions().height() / 2.0,
        floor.transform().translation().y - floor.dimensions().height() / 2.0,
    );
    let floor_top = floor.transform().translation().y + floor.dimensions().height() / 2.0;
    let walls = &components[2..6];
    let thickness = walls[0].dimensions().depth();
    for wall in walls {
        near(
            wall.transform().translation().y - wall.dimensions().height() / 2.0,
            floor_top,
        );
    }
    near(
        walls[0].transform().translation().z + thickness / 2.0,
        config.depth / 2.0,
    );
    near(
        walls[1].transform().translation().z - thickness / 2.0,
        -config.depth / 2.0,
    );
    near(
        walls[2].transform().translation().x - thickness / 2.0,
        -config.width / 2.0,
    );
    near(
        walls[3].transform().translation().x + thickness / 2.0,
        config.width / 2.0,
    );
    near(
        walls[2].dimensions().width(),
        config.depth - 2.0 * thickness,
    );
    near(
        walls[3].dimensions().width(),
        config.depth - 2.0 * thickness,
    );
    let door = components[6];
    assert_eq!(
        house.parent_of(door.reference()).unwrap(),
        Some(walls[0].reference())
    );
    near(door.transform().translation().x, 0.0);
    near(
        door.transform().translation().y - door.dimensions().height() / 2.0,
        floor_top,
    );

    let eave_y = floor_top + config.wall_height;
    for (roof, sign, support) in [
        (components[components.len() - 2], -1.0, walls[2]),
        (components[components.len() - 1], 1.0, walls[3]),
    ] {
        assert_eq!(
            house.parent_of(roof.reference()).unwrap(),
            Some(support.reference())
        );
        let matrix = roof.transform().model_matrix().unwrap();
        let half = roof.dimensions().width() / 2.0;
        let ridge = matrix * Vec4::new(-sign * half, 0.0, 0.0, 1.0);
        let eave = matrix * Vec4::new(sign * half, 0.0, 0.0, 1.0);
        near(ridge.x, 0.0);
        near(ridge.y, eave_y + config.roof_rise);
        near(eave.x, sign * config.width / 2.0);
        near(eave.y, eave_y);
        near(roof.dimensions().depth(), config.depth);
    }
}

#[test]
fn windows_stay_inside_wall_margins_and_clear_other_openings() {
    for count in [0, 1, 4, 11] {
        let config = HouseConfig {
            window_count: count,
            seed: 20,
            ..HouseConfig::default()
        };
        let house = generate_house(&config).unwrap();
        let mut total = 0;
        for wall in house
            .components()
            .filter(|part| part.kind() == ComponentKind::Wall)
        {
            let pose = wall.transform();
            let mut intervals = Vec::new();
            for child in house.children_of(wall.reference()).unwrap() {
                let part = house.component(child).unwrap();
                if part.kind() != ComponentKind::Window {
                    continue;
                }
                total += 1;
                assert!(pose.translation().z <= 0.0);
                let local = pose.rotation().inverse()
                    * (part.transform().translation() - pose.translation());
                near(local.z, 0.0);
                near(part.dimensions().width(), config.window_width);
                near(part.dimensions().height(), config.window_height);
                let half = part.dimensions().width() / 2.0;
                let bottom = local.y + config.wall_height / 2.0 - part.dimensions().height() / 2.0;
                near(bottom, 0.65);
                assert!(bottom + part.dimensions().height() <= config.wall_height - 0.2);
                assert!(local.x.abs() + half <= wall.dimensions().width() / 2.0 - 0.25 + 0.00001);
                intervals.push((local.x - half, local.x + half));
            }
            intervals.sort_by(|left, right| left.0.total_cmp(&right.0));
            assert!(intervals
                .windows(2)
                .all(|pair| pair[1].0 - pair[0].1 >= 0.3 - 0.00001));
        }
        assert_eq!(total, count);
    }
}

#[test]
fn seed_is_repeatable_after_interleaved_generations_and_allows_variation() {
    let config = HouseConfig {
        seed: 42,
        window_count: 1,
        ..HouseConfig::default()
    };
    let a = generate_house(&config).unwrap();
    let different = generate_house(&HouseConfig { seed: 43, ..config }).unwrap();
    let b = generate_house(&config).unwrap();
    assert_ne!(a.id(), b.id());
    assert_eq!(canonical(&a), canonical(&b));
    assert_ne!(canonical(&a), canonical(&different));
    let reference = a.components().next().unwrap().reference();
    assert!(matches!(
        b.component(reference),
        Err(procedural_city_engine::building::BuildingError::ForeignComponent { .. })
    ));
}

#[test]
fn invalid_dimensions_and_opening_layouts_return_typed_errors() {
    use HouseGenerationError::*;
    let base = HouseConfig::default();
    for value in [
        0.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN_POSITIVE,
    ] {
        assert!(matches!(
            generate_house(&HouseConfig {
                roof_rise: value,
                ..base
            }),
            Err(InvalidDimension {
                field: "roof_rise",
                ..
            })
        ));
    }
    for config in [
        HouseConfig {
            door_width: base.width,
            ..base
        },
        HouseConfig {
            door_height: base.wall_height,
            ..base
        },
    ] {
        assert!(matches!(generate_house(&config), Err(DoorDoesNotFit)));
    }
    assert!(matches!(
        generate_house(&HouseConfig { width: 0.4, ..base }),
        Err(FootprintTooSmall)
    ));
    assert!(matches!(
        generate_house(&HouseConfig {
            window_height: base.wall_height,
            ..base
        }),
        Err(WindowTooTall)
    ));
    for config in [
        HouseConfig {
            window_width: base.width,
            window_count: 1,
            ..base
        },
        HouseConfig {
            window_count: usize::MAX,
            ..base
        },
        // A ojo cabrían dos ventanas por pared, pero se perdería la separación.
        HouseConfig {
            width: 2.5,
            depth: 2.5,
            window_width: 1.0,
            window_count: 4,
            ..base
        },
    ] {
        assert!(matches!(
            generate_house(&config),
            Err(TooManyWindows { .. })
        ));
    }
}

#[test]
fn compact_and_near_limit_houses_keep_positive_dimensions_and_finite_poses() {
    let compact = HouseConfig {
        width: 0.5011,
        depth: 0.4001,
        wall_height: 0.2011,
        roof_rise: 0.001,
        door_width: 0.001,
        door_height: 0.001,
        window_count: 0,
        ..HouseConfig::default()
    };
    let large = HouseConfig {
        width: 1024.0,
        depth: 1024.0,
        wall_height: 1024.0,
        roof_rise: 1024.0,
        window_width: 0.001,
        ..HouseConfig::default()
    };
    for config in [compact, large] {
        let house = generate_house(&config).unwrap();
        assert_eq!(house.component_count(), 9 + config.window_count);
        for component in house.components() {
            assert!(component.dimensions().as_vec3().min_element() > 0.0);
            assert!(component.transform().model_matrix().unwrap().is_finite());
        }
    }
}
