use std::collections::BTreeSet;

use glam::{Quat, Vec3};
use procedural_city_engine::building::{
    Building, BuildingComponent, BuildingError, BuildingMaterialId, ComponentKind, ComponentRef,
    Dimensions,
};
use procedural_city_engine::scene::{Transform, TransformError};

fn dimensions() -> Dimensions {
    Dimensions::new(2.0, 2.5, 0.3).expect("dimensiones válidas")
}

fn transform(position: Vec3) -> Transform {
    Transform::new(position, Quat::IDENTITY, Vec3::ONE).expect("transformación válida")
}

fn add(building: &mut Building, kind: ComponentKind, position: Vec3) -> ComponentRef {
    building
        .add_component(
            kind,
            dimensions(),
            transform(position),
            BuildingMaterialId(7),
        )
        .expect("componente válido")
}

#[test]
fn building_registers_all_component_kinds_with_stable_unique_ids() {
    let mut building = Building::new().unwrap();
    let ids = [
        add(&mut building, ComponentKind::Foundation, Vec3::ZERO),
        add(&mut building, ComponentKind::Floor, Vec3::Y),
        add(&mut building, ComponentKind::Wall, Vec3::X),
        add(&mut building, ComponentKind::Column, Vec3::Z),
        add(&mut building, ComponentKind::Roof, Vec3::Y * 3.0),
        add(&mut building, ComponentKind::Door, Vec3::ZERO),
        add(&mut building, ComponentKind::Window, Vec3::ONE),
    ];

    assert_eq!(building.component_count(), 7);
    assert_eq!(
        building.component(ids[0]).expect("fundación").kind(),
        ComponentKind::Foundation
    );
    assert_eq!(
        building.component(ids[6]).expect("ventana").material(),
        BuildingMaterialId(7)
    );
    let local_ids = ids.map(ComponentRef::component_id);
    assert_eq!(
        local_ids.into_iter().collect::<BTreeSet<_>>().len(),
        ids.len()
    );
    assert!(local_ids.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(ids
        .iter()
        .all(|reference| reference.building_id() == building.id()));
}

#[test]
fn dimensions_and_transformations_are_validated_before_model_changes() {
    assert!(matches!(
        Dimensions::new(0.0, 1.0, 1.0),
        Err(BuildingError::InvalidDimensions(_))
    ));
    assert!(matches!(
        Dimensions::new(1.0, f32::NAN, 1.0),
        Err(BuildingError::InvalidDimensions(_))
    ));
    assert_eq!(
        Transform::new(Vec3::ZERO, Quat::from_xyzw(0.0, 0.0, 0.0, 0.0), Vec3::ONE),
        Err(TransformError::InvalidRotation)
    );

    let building = Building::new().unwrap();
    assert_eq!(building.component_count(), 0);
}

#[test]
fn compatible_architectural_relations_are_queryable_and_deterministic() {
    let mut building = Building::new().unwrap();
    let foundation = add(&mut building, ComponentKind::Foundation, Vec3::ZERO);
    let floor = add(&mut building, ComponentKind::Floor, Vec3::Y);
    let wall = add(&mut building, ComponentKind::Wall, Vec3::X);
    let column = add(&mut building, ComponentKind::Column, Vec3::Z);
    let roof = add(&mut building, ComponentKind::Roof, Vec3::Y * 3.0);
    let door = add(&mut building, ComponentKind::Door, Vec3::ZERO);
    let window = add(&mut building, ComponentKind::Window, Vec3::ONE);

    building.attach(floor, foundation).expect("piso válido");
    building.attach(wall, floor).expect("pared válida");
    building.attach(column, foundation).expect("columna válida");
    building.attach(roof, wall).expect("techo válido");
    building.attach(door, wall).expect("puerta válida");
    building.attach(window, wall).expect("ventana válida");

    assert_eq!(building.parent_of(roof).expect("techo"), Some(wall));
    let wall_children = building
        .children_of(wall)
        .expect("pared válida")
        .collect::<Vec<_>>();
    assert_eq!(wall_children, vec![roof, door, window]);
}

#[test]
fn invalid_missing_and_duplicate_relations_preserve_the_building() {
    let mut building = Building::new().unwrap();
    let foundation = add(&mut building, ComponentKind::Foundation, Vec3::ZERO);
    let wall = add(&mut building, ComponentKind::Wall, Vec3::X);
    let door = add(&mut building, ComponentKind::Door, Vec3::ZERO);
    let removed = add(&mut building, ComponentKind::Column, Vec3::Z);
    building
        .remove_component(removed)
        .expect("sin dependientes");

    let before = snapshot(&building);
    for (child, parent, expected) in [
        (
            door,
            foundation,
            BuildingError::InvalidRelation {
                child: ComponentKind::Door,
                parent: ComponentKind::Foundation,
            },
        ),
        (door, removed, BuildingError::ComponentNotFound(removed)),
        (removed, wall, BuildingError::ComponentNotFound(removed)),
        (door, door, BuildingError::SelfRelation(door)),
    ] {
        assert_eq!(building.attach(child, parent), Err(expected));
        assert_eq!(snapshot(&building), before);
    }
    building.attach(wall, foundation).expect("pared válida");
    building.attach(door, wall).expect("puerta válida");
    let before = snapshot(&building);
    assert!(matches!(
        building.attach(door, foundation),
        Err(BuildingError::ComponentAlreadyHasParent(id)) if id == door
    ));

    assert_eq!(snapshot(&building), before);
}

#[test]
fn removal_rejects_dependents_and_never_reuses_component_ids() {
    let mut building = Building::new().unwrap();
    let foundation = add(&mut building, ComponentKind::Foundation, Vec3::ZERO);
    let floor = add(&mut building, ComponentKind::Floor, Vec3::Y);
    building.attach(floor, foundation).expect("piso válido");

    let before = snapshot(&building);
    assert!(matches!(
        building.remove_component(foundation),
        Err(BuildingError::ComponentHasDependents(id)) if id == foundation
    ));
    assert_eq!(snapshot(&building), before);
    building.remove_component(floor).expect("hijo eliminado");
    assert_eq!(building.children_of(foundation).unwrap().count(), 0);
    building
        .remove_component(foundation)
        .expect("padre liberado");
    let replacement = add(&mut building, ComponentKind::Foundation, Vec3::ZERO);

    assert!(replacement.component_id() > floor.component_id());
    assert!(replacement.component_id() > foundation.component_id());
    for id in [foundation, floor] {
        assert_eq!(
            building.component(id),
            Err(BuildingError::ComponentNotFound(id))
        );
    }
    assert_eq!(
        building
            .components()
            .map(BuildingComponent::reference)
            .collect::<Vec<_>>(),
        vec![replacement]
    );
}

fn snapshot(building: &Building) -> Vec<(BuildingComponent, Option<ComponentRef>)> {
    building
        .components()
        .map(|component| {
            (
                component.clone(),
                building.parent_of(component.reference()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn same_operations_reproduce_local_ids_and_relationships_in_two_buildings() {
    fn build() -> (Building, ComponentRef) {
        let mut building = Building::new().unwrap();
        let wall = add(&mut building, ComponentKind::Wall, Vec3::ZERO);
        let door = add(&mut building, ComponentKind::Door, Vec3::X);
        building.attach(door, wall).unwrap();
        building.remove_component(door).unwrap();
        let window = add(&mut building, ComponentKind::Window, Vec3::ONE);
        building.attach(window, wall).unwrap();
        (building, wall)
    }

    let (mut first, first_wall) = build();
    let (second, second_wall) = build();
    assert_ne!(first.id(), second.id());
    assert_ne!(first_wall, second_wall);
    assert_eq!(first_wall.component_id(), second_wall.component_id());
    let local_snapshot = |building: &Building| {
        snapshot(building)
            .into_iter()
            .map(|(component, parent)| {
                (
                    component.id(),
                    component.kind(),
                    component.dimensions(),
                    component.transform(),
                    component.material(),
                    parent.map(ComponentRef::component_id),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(local_snapshot(&first), local_snapshot(&second));

    let second_before = snapshot(&second);
    assert!(first.remove_component(first_wall).is_err());
    add(&mut first, ComponentKind::Wall, Vec3::Z);
    assert_eq!(snapshot(&second), second_before);
    assert_ne!(snapshot(&first), second_before);
}

#[test]
fn foreign_references_with_matching_local_ids_are_rejected_without_changes() {
    let mut a = Building::new().unwrap();
    let mut b = Building::new().unwrap();
    let wall_a = add(&mut a, ComponentKind::Wall, Vec3::ZERO);
    let wall_b = add(&mut b, ComponentKind::Wall, Vec3::ZERO);
    let door_b = add(&mut b, ComponentKind::Door, Vec3::X);
    assert_ne!(a.id(), b.id());
    assert_eq!(wall_a.component_id(), wall_b.component_id());
    assert_ne!(wall_a, wall_b);

    let before_a = snapshot(&a);
    let before_b = snapshot(&b);
    let foreign = BuildingError::ForeignComponent {
        expected: b.id(),
        reference: wall_a,
    };
    assert_eq!(b.component(wall_a), Err(foreign.clone()));
    assert_eq!(b.parent_of(wall_a), Err(foreign.clone()));
    assert_eq!(b.children_of(wall_a).err(), Some(foreign.clone()));
    assert_eq!(b.attach(door_b, wall_a), Err(foreign.clone()));
    assert_eq!(b.attach(wall_a, wall_b), Err(foreign.clone()));
    assert_eq!(b.remove_component(wall_a), Err(foreign));
    assert_eq!(b.parent_of(door_b), Ok(None));
    assert_eq!(b.children_of(wall_b).unwrap().count(), 0);
    assert_eq!(snapshot(&a), before_a);
    assert_eq!(snapshot(&b), before_b);

    let mut control = Building::new().unwrap();
    add(&mut control, ComponentKind::Wall, Vec3::ZERO);
    add(&mut control, ComponentKind::Door, Vec3::X);
    let next = add(&mut b, ComponentKind::Window, Vec3::Y);
    let expected = add(&mut control, ComponentKind::Window, Vec3::Y);
    assert_eq!(next.component_id(), expected.component_id());
}

#[test]
fn building_ids_remain_unique_across_threads_and_after_drop() {
    use std::sync::Barrier;
    use std::thread;

    const THREADS: usize = 8;
    const PER_THREAD: usize = 64;
    let barrier = Barrier::new(THREADS);
    let ids = thread::scope(|scope| {
        let tasks = (0..THREADS)
            .map(|_| {
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    // Cada instancia se descarta al obtener su ID.
                    (0..PER_THREAD)
                        .map(|_| Building::new().unwrap().id())
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        tasks
            .into_iter()
            .flat_map(|task| task.join().unwrap())
            .collect::<BTreeSet<_>>()
    });
    assert_eq!(ids.len(), THREADS * PER_THREAD);
    assert!(!ids.contains(&Building::new().unwrap().id()));
}
