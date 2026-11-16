use std::collections::{BTreeMap, BTreeSet};

use glam::{DVec3, Vec3};

use crate::{
    building::BuildingComponent,
    geometry::{Face, Mesh, Vertex},
};

use super::MeshingError;

pub(super) struct Polygon<'a> {
    pub points: Vec<DVec3>,
    pub source: &'a BuildingComponent,
}

#[derive(Default)]
pub(super) struct Surface {
    pub mesh: Mesh,
    pub sources: Vec<crate::building::ComponentRef>,
    vertices: BTreeMap<[u32; 3], u32>,
}

fn key(point: Vec3) -> [u32; 3] {
    point
        .to_array()
        .map(|axis| if axis == 0.0 { 0 } else { axis.to_bits() })
}

impl Surface {
    fn vertex(&mut self, point: DVec3) -> Result<u32, MeshingError> {
        let position = point.as_vec3();
        let id = key(position);
        if let Some(&index) = self.vertices.get(&id) {
            return Ok(index);
        }
        let index = self.mesh.add_vertex(Vertex::new(position))?;
        self.vertices.insert(id, index);
        Ok(index)
    }

    fn triangle(
        &mut self,
        points: [DVec3; 3],
        source: &BuildingComponent,
    ) -> Result<(), MeshingError> {
        let indices = [
            self.vertex(points[0])?,
            self.vertex(points[1])?,
            self.vertex(points[2])?,
        ];
        self.mesh
            .add_face(Face::new(indices, source.material().0))?;
        self.sources.push(source.reference());
        Ok(())
    }

    // Todas las aristas de una sección discurren en X, Y o Z. Las inclinadas
    // avanzan en X con Z fijo. Se comparten los puntos colineales antes de triangular.
    pub fn polygons(&mut self, polygons: Vec<Polygon<'_>>) -> Result<(), MeshingError> {
        let mut lines: [BTreeMap<[u64; 2], Vec<DVec3>>; 3] =
            std::array::from_fn(|_| BTreeMap::new());
        for polygon in &polygons {
            for &point in &polygon.points {
                for (axis, lines) in lines.iter_mut().enumerate() {
                    lines.entry(line_key(point, axis)).or_default().push(point);
                }
            }
        }
        for (axis, lines) in lines.iter_mut().enumerate() {
            for points in lines.values_mut() {
                points.sort_by(|a, b| a[axis].total_cmp(&b[axis]));
                points.dedup();
            }
        }
        for polygon in polygons {
            let mut perimeter = Vec::new();
            for (&a, &b) in polygon
                .points
                .iter()
                .zip(polygon.points.iter().cycle().skip(1))
            {
                let axis = if a.x != b.x {
                    0
                } else if a.z != b.z {
                    2
                } else {
                    1
                };
                let candidates = &lines[axis][&line_key(a, axis)];
                let low = a[axis].min(b[axis]);
                let high = a[axis].max(b[axis]);
                let start = candidates.partition_point(|p| p[axis] <= low);
                let end = candidates.partition_point(|p| p[axis] < high);
                perimeter.push(a);
                let offset = perimeter.len();
                if start < end {
                    for &point in &candidates[start..end] {
                        if axis == 0 {
                            let y = a.y + (b.y - a.y) * ((point.x - a.x) / (b.x - a.x));
                            let tolerance =
                                16.0 * f64::EPSILON * y.abs().max(point.y.abs()).max(1.0);
                            if (point.y - y).abs() > tolerance {
                                continue;
                            }
                        }
                        perimeter.push(point);
                    }
                }
                if a[axis] > b[axis] {
                    perimeter[offset..].reverse();
                }
            }
            perimeter.dedup_by(|a, b| a.as_vec3() == b.as_vec3());
            if perimeter.first().map(|p| p.as_vec3()) == perimeter.last().map(|p| p.as_vec3()) {
                perimeter.pop();
            }
            if perimeter.len() < 3 {
                continue;
            }
            if perimeter.len() == 3 {
                self.triangle([perimeter[0], perimeter[1], perimeter[2]], polygon.source)?;
            } else if perimeter.len() == 4 && polygon.points.len() == 4 {
                self.triangle([perimeter[0], perimeter[1], perimeter[2]], polygon.source)?;
                self.triangle([perimeter[0], perimeter[2], perimeter[3]], polygon.source)?;
            } else {
                let center =
                    polygon.points.iter().copied().sum::<DVec3>() / polygon.points.len() as f64;
                for (&a, &b) in perimeter.iter().zip(perimeter.iter().cycle().skip(1)) {
                    self.triangle([a, b, center], polygon.source)?;
                }
            }
        }
        Ok(())
    }

    pub fn insert(
        &mut self,
        corners: [DVec3; 8],
        source: &BuildingComponent,
    ) -> Result<(), MeshingError> {
        for [a, b, c, d] in [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 4, 7, 3],
            [1, 2, 6, 5],
            [0, 1, 5, 4],
            [3, 7, 6, 2],
        ] {
            self.triangle([corners[a], corners[b], corners[c]], source)?;
            self.triangle([corners[a], corners[c], corners[d]], source)?;
        }
        Ok(())
    }
}

fn line_key(point: DVec3, axis: usize) -> [u64; 2] {
    let bits = |value: f64| if value == 0.0 { 0 } else { value.to_bits() };
    match axis {
        0 => [bits(point.z), 0],
        1 => [bits(point.x), bits(point.z)],
        _ => [bits(point.x), bits(point.y)],
    }
}

pub(super) fn validate_closed(mesh: &Mesh) -> Result<(), MeshingError> {
    mesh.validate()?;
    if mesh.faces.is_empty() {
        return Err(MeshingError::InvalidTopology);
    }
    let mut triangles = BTreeSet::new();
    let mut edges = BTreeMap::new();
    for face in &mesh.faces {
        let [a, b, c] = face
            .indices
            .map(|index| key(mesh.vertices[index as usize].position));
        let mut triangle = [a, b, c];
        triangle.sort_unstable();
        if !triangles.insert(triangle) {
            return Err(MeshingError::InvalidTopology);
        }
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let (edge, orientation) = if from < to {
                ((from, to), 1)
            } else {
                ((to, from), -1)
            };
            let (count, balance) = edges.entry(edge).or_insert((0, 0));
            *count += 1;
            *balance += orientation;
        }
    }
    if edges.values().any(|entry| *entry != (2, 0)) {
        return Err(MeshingError::InvalidTopology);
    }
    Ok(())
}
