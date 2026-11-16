use std::{error::Error, fmt};

use glam::Vec3;

use crate::{
    building::{Building, BuildingComponent, ComponentKind, ComponentRef},
    geometry::roof::{RoofDimensions, RoofProfile, RoofProfileError},
};

/// Error al adaptar la estructura de una vivienda a un perfil de cubierta.
#[derive(Debug, PartialEq, Eq)]
pub enum HouseRoofError {
    IncompatibleHouse,
    Profile(RoofProfileError),
}

impl fmt::Display for HouseRoofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleHouse => f.write_str(
                "la estructura no coincide con la planta y cubierta simétricas de la vivienda",
            ),
            Self::Profile(error) => error.fmt(f),
        }
    }
}

impl Error for HouseRoofError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Profile(error) => Some(error),
            Self::IncompatibleHouse => None,
        }
    }
}

/// Deriva la cubierta de una vivienda con la disposición de `generate_house`.
///
/// Usa el desnivel y la pendiente de los componentes estructurales. El perfil
/// derivado ancla la cara inferior en la coronación de la pared exterior.
pub fn derive_house_roof(house: &Building) -> Result<RoofProfile, HouseRoofError> {
    use ComponentKind::{Floor, Roof, Wall};
    let [floor] = components(house, Floor)?;
    let [first, second] = components(house, Roof)?;
    let walls: [&BuildingComponent; 4] = components(house, Wall)?;
    let size = floor.dimensions();
    let (width, depth) = (size.width(), size.depth());
    let thickness = walls[0].dimensions().depth();
    let wall_height = walls[0].dimensions().height();
    let floor_top = floor.transform().translation().y + size.height() / 2.0;
    let wall_top = f64::from(floor_top) + f64::from(wall_height);
    let relative_tolerance = 8.0 * f32::EPSILON;
    let wall_epsilon = relative_tolerance * width.max(depth).max(wall_height);
    let floor_pose = floor.transform();
    if floor_pose.scale() != Vec3::ONE
        || !(floor_pose.rotation() * Vec3::Y).abs_diff_eq(Vec3::Y, relative_tolerance)
        || !(floor_pose.rotation() * Vec3::X).abs_diff_eq(Vec3::X, relative_tolerance)
        || floor_pose.translation().x.abs() > wall_epsilon
        || floor_pose.translation().z.abs() > wall_epsilon
    {
        return Err(HouseRoofError::IncompatibleHouse);
    }
    let x = (width - thickness) / 2.0;
    let z = (depth - thickness) / 2.0;
    let side_span = depth - 2.0 * thickness;
    let mut side_walls = [None; 2];
    let mut matched = [false; 4];
    let expected_walls = [
        (0.0, z, width, Vec3::Z),
        (0.0, -z, width, -Vec3::Z),
        (-x, 0.0, side_span, -Vec3::X),
        (x, 0.0, side_span, Vec3::X),
    ];
    for wall in walls {
        let pose = wall.transform();
        let size = wall.dimensions();
        let up = pose.rotation() * Vec3::Y;
        if pose.scale() != Vec3::ONE
            || !up.abs_diff_eq(Vec3::Y, relative_tolerance)
            || house.parent_of(wall.reference()) != Ok(Some(floor.reference()))
            || size.depth() != thickness
            || size.height() != wall_height
        {
            return Err(HouseRoofError::IncompatibleHouse);
        }
        let Some(index) = expected_walls.iter().position(|&(x, z, span, normal)| {
            (pose.rotation() * Vec3::Z).abs_diff_eq(normal, relative_tolerance)
                && pose
                    .translation()
                    .abs_diff_eq(Vec3::new(x, floor_top + wall_height / 2.0, z), wall_epsilon)
                && (size.width() - span).abs() <= relative_tolerance * span.abs()
        }) else {
            return Err(HouseRoofError::IncompatibleHouse);
        };
        if matched[index] {
            return Err(HouseRoofError::IncompatibleHouse);
        }
        matched[index] = true;
        if index >= 2 {
            side_walls[index - 2] = Some(wall.reference());
        }
    }
    let normal_thickness = first.dimensions().height();
    let rise = |part: &BuildingComponent| {
        let along_panel = part.transform().rotation() * Vec3::X;
        f64::from(along_panel.y.abs()) * f64::from(part.dimensions().width())
    };
    let rise = (rise(first) + rise(second)) / 2.0;
    let run = f64::from(width) / 2.0;
    let slope = rise.atan2(run);
    let panel_length = run.hypot(rise);
    let roof_epsilon =
        f64::from(relative_tolerance) * f64::from(width.max(depth)).max(wall_top.abs() + rise);
    for (roof, support) in [
        (first, house.parent_of(first.reference())),
        (second, house.parent_of(second.reference())),
    ] {
        let Ok(Some(support)) = support else {
            return Err(HouseRoofError::IncompatibleHouse);
        };
        let sign = side_sign(side_walls, support)?;
        let pose = roof.transform();
        let direction = (pose.rotation() * Vec3::X).as_dvec3();
        let center = pose.translation().as_dvec3();
        let size = roof.dimensions();
        if !direction.abs_diff_eq(
            glam::DVec3::new(slope.cos(), -sign * slope.sin(), 0.0),
            f64::from(relative_tolerance),
        ) || !(pose.rotation() * Vec3::Z).abs_diff_eq(Vec3::Z, relative_tolerance)
            || !center.abs_diff_eq(
                glam::DVec3::new(sign * run / 2.0, wall_top + rise / 2.0, 0.0),
                roof_epsilon,
            )
            || pose.scale() != Vec3::ONE
            || (f64::from(size.width()) - panel_length).abs()
                > f64::from(relative_tolerance) * panel_length
            || size.depth() != depth
            || size.height() != normal_thickness
        {
            return Err(HouseRoofError::IncompatibleHouse);
        }
    }
    if house.parent_of(first.reference()) == house.parent_of(second.reference()) {
        return Err(HouseRoofError::IncompatibleHouse);
    }
    RoofProfile::new(RoofDimensions {
        width: f64::from(width),
        depth: f64::from(depth),
        wall_top,
        rise,
        normal_thickness: f64::from(normal_thickness),
        wall_thickness: f64::from(thickness),
    })
    .map_err(HouseRoofError::Profile)
}

fn side_sign(
    walls: [Option<ComponentRef>; 2],
    reference: ComponentRef,
) -> Result<f64, HouseRoofError> {
    if walls[0] == Some(reference) {
        Ok(-1.0)
    } else if walls[1] == Some(reference) {
        Ok(1.0)
    } else {
        Err(HouseRoofError::IncompatibleHouse)
    }
}

fn components<const N: usize>(
    house: &Building,
    kind: ComponentKind,
) -> Result<[&BuildingComponent; N], HouseRoofError> {
    let mut components = house.components().filter(|part| part.kind() == kind);
    let items: [Option<&BuildingComponent>; N] = std::array::from_fn(|_| components.next());
    if components.next().is_some() || items.iter().any(Option::is_none) {
        return Err(HouseRoofError::IncompatibleHouse);
    }
    Ok(items.map(|item| item.expect("se comprobó el número de componentes")))
}
