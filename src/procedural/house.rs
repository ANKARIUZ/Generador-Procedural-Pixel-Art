use glam::{Quat, Vec3};

use crate::building::{
    Building, BuildingError, BuildingMaterialId, ComponentKind, ComponentRef, Dimensions,
};
use crate::scene::Transform;

use super::{HouseConfig, HouseGenerationError};

/// Versión de las reglas de distribución y del flujo pseudoaleatorio.
pub const HOUSE_ALGORITHM_VERSION: u32 = 1;

const FOUNDATION_THICKNESS: f32 = 0.3;
const FLOOR_THICKNESS: f32 = 0.15;
const WALL_THICKNESS: f32 = 0.2;
const ROOF_THICKNESS: f32 = 0.12;
const EDGE_MARGIN: f64 = 0.25;
const TOP_MARGIN: f64 = 0.2;
const WINDOW_GAP: f64 = 0.3;
const WINDOW_SILL: f64 = 0.65;

#[derive(Clone, Copy)]
struct Wall {
    span: f32,
    x: f32,
    z: f32,
    yaw: f32,
}

impl Wall {
    fn rotation(self) -> Quat {
        Quat::from_rotation_y(self.yaw)
    }

    fn opening_center(self, offset: f64, height: f64) -> Vec3 {
        Vec3::new(self.x, height as f32, self.z) + self.rotation() * Vec3::X * offset as f32
    }
}

struct Layout {
    // Frontal, trasera, izquierda, derecha.
    walls: [Wall; 4],
    windows: [usize; 3],
    window_offsets: [f64; 3],
}

impl Layout {
    fn plan(config: &HouseConfig) -> Result<Self, HouseGenerationError> {
        use std::f32::consts::{FRAC_PI_2, PI};
        config.validate_dimensions()?;
        let side_span = config.depth - 2.0 * WALL_THICKNESS;
        if side_span <= 0.0 || config.width <= 2.0 * WALL_THICKNESS {
            return Err(HouseGenerationError::FootprintTooSmall);
        }
        let x = (config.width - WALL_THICKNESS) / 2.0;
        let z = (config.depth - WALL_THICKNESS) / 2.0;
        let walls = [
            Wall {
                span: config.width,
                x: 0.0,
                z,
                yaw: 0.0,
            },
            Wall {
                span: config.width,
                x: 0.0,
                z: -z,
                yaw: PI,
            },
            Wall {
                span: side_span,
                x: -x,
                z: 0.0,
                yaw: -FRAC_PI_2,
            },
            Wall {
                span: side_span,
                x,
                z: 0.0,
                yaw: FRAC_PI_2,
            },
        ];
        if f64::from(config.door_width) + 2.0 * EDGE_MARGIN > f64::from(config.width)
            || f64::from(config.door_height) + TOP_MARGIN > f64::from(config.wall_height)
        {
            return Err(HouseGenerationError::DoorDoesNotFit);
        }
        if config.window_count > 0
            && WINDOW_SILL + f64::from(config.window_height) + TOP_MARGIN
                > f64::from(config.wall_height)
        {
            return Err(HouseGenerationError::WindowTooTall);
        }

        let window_width = f64::from(config.window_width);
        let capacity = [walls[1], walls[2], walls[3]].map(|wall| {
            let usable = f64::from(wall.span) - 2.0 * EDGE_MARGIN;
            ((usable + WINDOW_GAP) / (window_width + WINDOW_GAP))
                .floor()
                .max(0.0) as usize
        });
        let total = capacity.iter().sum();
        if config.window_count > total {
            return Err(HouseGenerationError::TooManyWindows {
                requested: config.window_count,
                capacity: total,
            });
        }

        let mut random = SplitMix64(config.seed);
        let mut order = [0, 1, 2];
        for index in (1..order.len()).rev() {
            order.swap(index, (random.next() % (index as u64 + 1)) as usize);
        }
        let mut windows = [0; 3];
        let mut remaining = config.window_count;
        while remaining > 0 {
            for index in order {
                if remaining > 0 && windows[index] < capacity[index] {
                    windows[index] += 1;
                    remaining -= 1;
                }
            }
        }
        let window_offsets = std::array::from_fn(|_| random.fraction());
        Ok(Self {
            walls,
            windows,
            window_offsets,
        })
    }
}

/// Genera una vivienda estructural de una planta mediante la API de `Building`.
///
/// X es el ancho, Y la altura y +Z apunta a la fachada. El origen está en el
/// centro de la cara inferior de la fundación. Todos los componentes se
/// expresan en el mismo marco local de la vivienda.
pub fn generate_house(config: &HouseConfig) -> Result<Building, HouseGenerationError> {
    use ComponentKind::*;

    let layout = Layout::plan(config)?;
    let mut building = Building::new()?;
    let materials = config.materials;
    let floor_top = FOUNDATION_THICKNESS + FLOOR_THICKNESS;
    let foundation = add(
        &mut building,
        Foundation,
        Vec3::new(config.width, FOUNDATION_THICKNESS, config.depth),
        Vec3::Y * (FOUNDATION_THICKNESS / 2.0),
        Quat::IDENTITY,
        materials.foundation,
    )?;
    let floor = add(
        &mut building,
        Floor,
        Vec3::new(config.width, FLOOR_THICKNESS, config.depth),
        Vec3::Y * (FOUNDATION_THICKNESS + FLOOR_THICKNESS / 2.0),
        Quat::IDENTITY,
        materials.floor,
    )?;
    building.attach(floor, foundation)?;

    let mut walls = [floor; 4];
    for (reference, wall) in walls.iter_mut().zip(layout.walls) {
        *reference = add(
            &mut building,
            Wall,
            Vec3::new(wall.span, config.wall_height, WALL_THICKNESS),
            Vec3::new(wall.x, floor_top + config.wall_height / 2.0, wall.z),
            wall.rotation(),
            materials.wall,
        )?;
        building.attach(*reference, floor)?;
    }

    let front = layout.walls[0];
    let door = add(
        &mut building,
        Door,
        Vec3::new(config.door_width, config.door_height, WALL_THICKNESS),
        front.opening_center(
            0.0,
            f64::from(floor_top) + f64::from(config.door_height) / 2.0,
        ),
        front.rotation(),
        materials.door,
    )?;
    building.attach(door, walls[0])?;

    let width = f64::from(config.window_width);
    for (index, count) in layout.windows.into_iter().enumerate() {
        if count == 0 {
            continue;
        }
        let wall = layout.walls[index + 1];
        let span = f64::from(wall.span);
        let group = count as f64 * width + (count - 1) as f64 * WINDOW_GAP;
        let spare = (span - 2.0 * EDGE_MARGIN - group).max(0.0);
        let first = -span / 2.0 + EDGE_MARGIN + width / 2.0 + spare * layout.window_offsets[index];
        let height = f64::from(floor_top) + WINDOW_SILL + f64::from(config.window_height) / 2.0;
        for window_index in 0..count {
            let offset = first + window_index as f64 * (width + WINDOW_GAP);
            let window = add(
                &mut building,
                Window,
                Vec3::new(config.window_width, config.window_height, WALL_THICKNESS),
                wall.opening_center(offset, height),
                wall.rotation(),
                materials.window,
            )?;
            building.attach(window, walls[index + 1])?;
        }
    }

    let run = config.width / 2.0;
    let rise = config.roof_rise;
    let slope = rise.atan2(run);
    let roof_size = Vec3::new(run.hypot(rise), ROOF_THICKNESS, config.depth);
    let roof_center_y = floor_top + config.wall_height + rise / 2.0;
    // Las dos aguas comparten cumbrera en X = 0, paralela a Z.
    for (x, angle, support) in [(-run / 2.0, slope, walls[2]), (run / 2.0, -slope, walls[3])] {
        let roof = add(
            &mut building,
            Roof,
            roof_size,
            Vec3::new(x, roof_center_y, 0.0),
            Quat::from_rotation_z(angle),
            materials.roof,
        )?;
        building.attach(roof, support)?;
    }
    Ok(building)
}

fn add(
    building: &mut Building,
    kind: ComponentKind,
    size: Vec3,
    center: Vec3,
    rotation: Quat,
    material: BuildingMaterialId,
) -> Result<ComponentRef, BuildingError> {
    let dimensions = Dimensions::new(size.x, size.y, size.z)?;
    let transform =
        Transform::new(center, rotation, Vec3::ONE).map_err(BuildingError::InvalidTransform)?;
    building.add_component(kind, dimensions, transform, material)
}

// SplitMix64 usa aritmética modular explícita para fijar el flujo de la versión.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut bits = self.0;
        bits = (bits ^ (bits >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        bits = (bits ^ (bits >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        bits ^ (bits >> 31)
    }

    fn fraction(&mut self) -> f64 {
        const SCALE: f64 = (1_u64 << 53) as f64;
        (self.next() >> 11) as f64 / SCALE
    }
}
