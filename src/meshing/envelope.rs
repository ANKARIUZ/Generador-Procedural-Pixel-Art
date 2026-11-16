use glam::DVec3;

use crate::{
    building::{Building, BuildingComponent, ComponentKind},
    procedural::derive_house_roof,
};

use super::{
    surface::{Polygon, Surface},
    BuildingMesh, MeshingError,
};

type Part<'a> = &'a BuildingComponent;

struct Opening<'a> {
    x: [f64; 2],
    y: [f64; 2],
    z: [f64; 2],
    part: Part<'a>,
    side: bool,
}

struct Wall<'a> {
    part: Part<'a>,
    x: [f64; 2],
    z: [f64; 2],
    openings: Vec<Opening<'a>>,
}

#[derive(Clone, Copy)]
enum Level {
    Flat(f64),
    Roof { ridge: f64, edge: f64, run: f64 },
}

impl Level {
    fn at(self, x: f64) -> f64 {
        match self {
            Self::Flat(y) => y,
            Self::Roof { ridge, edge, run } => {
                if x.abs() == run {
                    edge
                } else {
                    ridge + (edge - ridge) * (x.abs() / run)
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Band<'a> {
    low: Level,
    high: Level,
    part: Part<'a>,
}

impl<'a> Band<'a> {
    fn flat(low: f64, high: f64, part: Part<'a>) -> Self {
        Self {
            low: Level::Flat(low),
            high: Level::Flat(high),
            part,
        }
    }
}

struct Strip<'a> {
    z: [f64; 2],
    cuts: Vec<f64>,
    cells: Vec<Vec<Band<'a>>>,
}

fn sorted(mut cuts: Vec<f64>) -> Vec<f64> {
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    cuts
}

fn midpoint(range: [f64; 2]) -> f64 {
    (range[0] + range[1]) / 2.0
}

fn inside(value: f64, range: [f64; 2]) -> bool {
    value > range[0] && value < range[1]
}

fn overlaps(a: [f64; 2], b: [f64; 2]) -> bool {
    a[0].max(b[0]) < a[1].min(b[1])
}

pub(super) fn mesh(building: &Building) -> Result<BuildingMesh, MeshingError> {
    use ComponentKind::*;
    let profile = derive_house_roof(building).map_err(MeshingError::Roof)?;
    let only = |kind| {
        let mut items = building.components().filter(|part| part.kind() == kind);
        match (items.next(), items.next()) {
            (Some(part), None) => Ok(part),
            _ => Err(MeshingError::UnsupportedLayout),
        }
    };
    let floor = only(Floor)?;
    let foundation = only(Foundation)?;
    let [left, right] = profile.panels();
    let run = right.profile[0].x;
    let wall_top = right.profile[0].y;
    let [rear, front] = profile.gables();
    let [low_z, high_z] = [*rear.z.start(), *front.z.end()];
    let [inner_low_z, inner_high_z] = [*rear.z.end(), *front.z.start()];
    let [left_cap, right_cap] = profile.side_caps();
    let [inner_left, inner_right] = [left_cap.profile[1].x, right_cap.profile[1].x];
    let floor_size = floor.dimensions();
    let base_top =
        f64::from(floor.transform().translation().y) - f64::from(floor_size.height()) / 2.0;
    let floor_top = f64::from(floor.transform().translation().y + floor_size.height() / 2.0);
    let base_pose = foundation.transform();
    let base_size = foundation.dimensions();
    let base_bottom = f64::from(base_pose.translation().y) - f64::from(base_size.height()) / 2.0;
    let tolerance = 8.0 * f64::from(f32::EPSILON);
    if base_pose.scale() != glam::Vec3::ONE
        || !(base_pose.rotation() * glam::Vec3::X).abs_diff_eq(glam::Vec3::X, tolerance as f32)
        || !(base_pose.rotation() * glam::Vec3::Y).abs_diff_eq(glam::Vec3::Y, tolerance as f32)
        || base_size.width() != floor_size.width()
        || base_size.depth() != floor_size.depth()
        || base_pose.translation().x != 0.0
        || base_pose.translation().z != 0.0
        || (base_bottom + f64::from(base_size.height()) - base_top).abs()
            > tolerance * base_top.abs().max(1.0)
        || building.parent_of(floor.reference()) != Ok(Some(foundation.reference()))
        || building.parent_of(foundation.reference()) != Ok(None)
    {
        return Err(MeshingError::UnsupportedLayout);
    }

    let mut walls = Vec::new();
    for part in building.components().filter(|part| part.kind() == Wall) {
        let normal = part.transform().rotation() * glam::Vec3::Z;
        let (x, z) = if normal.z > 0.5 {
            ([-run, run], [inner_high_z, high_z])
        } else if normal.z < -0.5 {
            ([-run, run], [low_z, inner_low_z])
        } else if normal.x > 0.5 {
            ([inner_right, run], [inner_low_z, inner_high_z])
        } else {
            ([-run, inner_left], [inner_low_z, inner_high_z])
        };
        let mut wall = self::Wall {
            part,
            x,
            z,
            openings: Vec::new(),
        };
        for child in building
            .children_of(part.reference())
            .map_err(|_| MeshingError::UnsupportedLayout)?
        {
            let opening = building
                .component(child)
                .map_err(|_| MeshingError::UnsupportedLayout)?;
            if matches!(opening.kind(), Door | Window) {
                wall.add_opening(opening, [floor_top, wall_top])?;
            }
        }
        walls.push(wall);
    }
    if building.components().any(|part| {
        part.kind() == Column
            || (matches!(part.kind(), Door | Window)
                && !matches!(building.parent_of(part.reference()), Ok(Some(_))))
    }) {
        return Err(MeshingError::UnsupportedLayout);
    }

    let mut roofs = building.components().filter(|part| part.kind() == Roof);
    let a = roofs.next().ok_or(MeshingError::UnsupportedLayout)?;
    let b = roofs.next().ok_or(MeshingError::UnsupportedLayout)?;
    let roofs = if a.transform().translation().x < b.transform().translation().x {
        [a, b]
    } else {
        [b, a]
    };
    let bottom = Level::Roof {
        ridge: left.profile[1].y,
        edge: wall_top,
        run,
    };
    let top = Level::Roof {
        ridge: left.profile[2].y,
        edge: left.profile[3].y,
        run,
    };
    let base = [
        Band::flat(base_bottom, base_top, foundation),
        Band::flat(base_top, floor_top, floor),
    ];
    let z_cuts = sorted(
        [low_z, inner_low_z, inner_high_z, high_z]
            .into_iter()
            .chain(
                walls
                    .iter()
                    .flat_map(|wall| wall.openings.iter().flat_map(|opening| opening.z)),
            )
            .collect(),
    );
    let mut strips = Vec::new();
    for z in z_cuts.windows(2).map(|pair| [pair[0], pair[1]]) {
        let middle = midpoint(z);
        let active: Vec<_> = walls.iter().filter(|wall| inside(middle, wall.z)).collect();
        let cuts = sorted(
            [-run, inner_left, 0.0, inner_right, run]
                .into_iter()
                .chain(active.iter().flat_map(|wall| {
                    wall.openings
                        .iter()
                        .filter(move |hole| inside(middle, hole.z))
                        .flat_map(|hole| hole.x)
                }))
                .collect(),
        );
        let cells = cuts
            .windows(2)
            .map(|pair| {
                let x = midpoint([pair[0], pair[1]]);
                let mut bands = base.to_vec();
                if let Some(wall) = active.iter().find(|wall| inside(x, wall.x)) {
                    let mut holes: Vec<_> = wall
                        .openings
                        .iter()
                        .filter(|hole| inside(x, hole.x) && inside(middle, hole.z))
                        .collect();
                    holes.sort_by(|a, b| a.y[0].total_cmp(&b.y[0]));
                    let mut y = floor_top;
                    for hole in holes {
                        if y < hole.y[0] {
                            bands.push(Band::flat(y, hole.y[0], wall.part));
                        }
                        y = hole.y[1];
                    }
                    bands.push(Band {
                        low: Level::Flat(y),
                        high: bottom,
                        part: wall.part,
                    });
                }
                bands.push(Band {
                    low: bottom,
                    high: top,
                    part: roofs[usize::from(x > 0.0)],
                });
                bands
            })
            .collect();
        strips.push(Strip { z, cuts, cells });
    }

    let mut polygons = Vec::new();
    for strip in &strips {
        for (index, bands) in strip.cells.iter().enumerate() {
            let x = [strip.cuts[index], strip.cuts[index + 1]];
            for (n, band) in bands.iter().enumerate() {
                let mid = midpoint(x);
                if n == 0 || bands[n - 1].high.at(mid) < band.low.at(mid) {
                    horizontal(&mut polygons, x, strip.z, band.low, false, band.part);
                }
                if n + 1 == bands.len() || band.high.at(mid) < bands[n + 1].low.at(mid) {
                    horizontal(&mut polygons, x, strip.z, band.high, true, band.part);
                }
            }
        }
        for (index, &x) in strip.cuts.iter().enumerate() {
            let before = if index > 0 {
                &strip.cells[index - 1][..]
            } else {
                &[]
            };
            let after = strip.cells.get(index).map_or(&[][..], Vec::as_slice);
            vertical(&mut polygons, before, after, Boundary::X { x, z: strip.z });
        }
    }
    for index in 0..=strips.len() {
        let before = index.checked_sub(1).and_then(|i| strips.get(i));
        let after = strips.get(index);
        let z = after.map_or_else(|| before.unwrap().z[1], |strip| strip.z[0]);
        let cuts = sorted(
            before
                .into_iter()
                .chain(after)
                .flat_map(|strip| strip.cuts.iter().copied())
                .collect(),
        );
        for pair in cuts.windows(2) {
            let x = [pair[0], pair[1]];
            vertical(
                &mut polygons,
                bands_at(before, midpoint(x)),
                bands_at(after, midpoint(x)),
                Boundary::Z { z, x },
            );
        }
    }
    let mut surface = Surface::default();
    surface.polygons(polygons)?;
    let envelope_faces = 0..surface.mesh.faces.len();
    for opening in walls.iter().flat_map(|wall| &wall.openings) {
        let mut x = opening.x;
        let mut z = opening.z;
        // Un inserto ocupa el centro de su hueco con holgura proporcional
        // en el perímetro y la mitad del espesor del muro.
        let inset = |values: &mut [f64; 2], fraction| {
            let margin = (values[1] - values[0]) * fraction;
            values[0] += margin;
            values[1] -= margin;
        };
        inset(&mut x, if opening.side { 0.25 } else { 0.02 });
        inset(&mut z, if opening.side { 0.02 } else { 0.25 });
        let mut y = opening.y;
        inset(&mut y, 0.02);
        let [x0, x1] = x;
        let [y0, y1] = y;
        let [z0, z1] = z;
        let corners = [
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ]
        .map(DVec3::from_array);
        surface.insert(corners, opening.part)?;
    }
    super::validate_closed_mesh(&surface.mesh)?;
    Ok(BuildingMesh {
        mesh: surface.mesh,
        sources: surface.sources,
        envelope_faces,
    })
}

impl<'a> Wall<'a> {
    fn add_opening(&mut self, part: Part<'a>, height: [f64; 2]) -> Result<(), MeshingError> {
        let error = MeshingError::InvalidOpening(part.reference());
        let wall_pose = self.part.transform();
        let pose = part.transform();
        let local = wall_pose.rotation().inverse() * (pose.translation() - wall_pose.translation());
        let size = part.dimensions();
        let epsilon = 8.0
            * f64::from(f32::EPSILON)
            * f64::from(
                wall_pose
                    .translation()
                    .abs()
                    .max_element()
                    .max(self.part.dimensions().as_vec3().max_element()),
            );
        if pose.rotation() != wall_pose.rotation()
            || pose.scale() != glam::Vec3::ONE
            || f64::from(local.z.abs()) > epsilon
            || size.depth() != self.part.dimensions().depth()
        {
            return Err(error);
        }
        let normal = wall_pose.rotation() * glam::Vec3::Z;
        let side = normal.x.abs() > 0.5;
        let tangent_sign = if side {
            -normal.x.signum()
        } else {
            normal.z.signum()
        };
        let center = f64::from(tangent_sign * local.x);
        let span = if side { self.z } else { self.x };
        let origin = midpoint(span);
        let half = f64::from(size.width()) / 2.0;
        let interval = [origin + center - half, origin + center + half];
        let half_height = f64::from(size.height()) / 2.0;
        let mut y = [
            f64::from(pose.translation().y) - half_height,
            f64::from(pose.translation().y) + half_height,
        ];
        if part.kind() == ComponentKind::Door {
            if (y[0] - height[0]).abs() > epsilon {
                return Err(error);
            }
            y[0] = height[0];
        }
        if interval[0] <= span[0]
            || interval[1] >= span[1]
            || y[0] < height[0]
            || y[1] >= height[1]
            || y[0] >= y[1]
        {
            return Err(error);
        }
        let (x, z) = if side {
            (self.x, interval)
        } else {
            (interval, self.z)
        };
        if self
            .openings
            .iter()
            .any(|other| overlaps(x, other.x) && overlaps(y, other.y) && overlaps(z, other.z))
        {
            return Err(error);
        }
        self.openings.push(Opening {
            x,
            y,
            z,
            part,
            side,
        });
        Ok(())
    }
}

fn bands_at<'a, 'b>(strip: Option<&'b Strip<'a>>, x: f64) -> &'b [Band<'a>] {
    strip.map_or(&[], |strip| {
        &strip.cells[strip.cuts.partition_point(|cut| *cut <= x) - 1]
    })
}

fn horizontal<'a>(
    out: &mut Vec<Polygon<'a>>,
    [x0, x1]: [f64; 2],
    [z0, z1]: [f64; 2],
    level: Level,
    up: bool,
    source: Part<'a>,
) {
    let mut points = vec![
        DVec3::new(x0, level.at(x0), z0),
        DVec3::new(x0, level.at(x0), z1),
        DVec3::new(x1, level.at(x1), z1),
        DVec3::new(x1, level.at(x1), z0),
    ];
    if !up {
        points.reverse();
    }
    out.push(Polygon { points, source });
}

enum Boundary {
    X { x: f64, z: [f64; 2] },
    Z { z: f64, x: [f64; 2] },
}

fn vertical<'a>(
    out: &mut Vec<Polygon<'a>>,
    before: &[Band<'a>],
    after: &[Band<'a>],
    boundary: Boundary,
) {
    let middle = match boundary {
        Boundary::X { x, .. } => x,
        Boundary::Z { x, .. } => midpoint(x),
    };
    let mut levels: Vec<_> = before
        .iter()
        .chain(after)
        .flat_map(|band| [band.low, band.high])
        .collect();
    levels.sort_by(|a, b| a.at(middle).total_cmp(&b.at(middle)));
    levels.dedup_by(|a, b| a.at(middle) == b.at(middle));
    for pair in levels.windows(2) {
        let [low, high] = [pair[0], pair[1]];
        let y = midpoint([low.at(middle), high.at(middle)]);
        let covering = |bands: &[Band<'a>]| {
            bands
                .iter()
                .find(|band| y > band.low.at(middle) && y < band.high.at(middle))
                .map(|band| band.part)
        };
        let (source, positive) = match (covering(before), covering(after)) {
            (Some(source), None) => (source, true),
            (None, Some(source)) => (source, false),
            _ => continue,
        };
        let mut points = match boundary {
            Boundary::X { x, z: [z0, z1] } => vec![
                DVec3::new(x, low.at(x), z0),
                DVec3::new(x, high.at(x), z0),
                DVec3::new(x, high.at(x), z1),
                DVec3::new(x, low.at(x), z1),
            ],
            Boundary::Z { z, x: [x0, x1] } => vec![
                DVec3::new(x0, low.at(x0), z),
                DVec3::new(x1, low.at(x1), z),
                DVec3::new(x1, high.at(x1), z),
                DVec3::new(x0, high.at(x0), z),
            ],
        };
        if !positive {
            points.reverse();
        }
        points.dedup();
        if points.first() == points.last() {
            points.pop();
        }
        if points.len() >= 3 {
            out.push(Polygon { points, source });
        }
    }
}
