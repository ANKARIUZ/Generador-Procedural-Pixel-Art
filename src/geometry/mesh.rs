use std::error::Error;
use std::fmt::{Display, Formatter};

use glam::{DVec3, Vec3};

use super::{Face, Vertex};

/// Margen relativo para el error de redondeo de las operaciones de planitud en
/// `f64`. Se escala con el cubo de las aristas, por lo que no impone una unidad
/// mínima de mundo ni descarta cuadriláteros pequeños matemáticamente válidos.
const COPLANARITY_ROUNDING_FACTOR: f64 = 32.0 * f64::EPSILON;

/// Caja alineada a ejes que encierra una malla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    /// Menor coordenada de cada eje.
    pub min: Vec3,
    /// Mayor coordenada de cada eje.
    pub max: Vec3,
}

/// Error de construcción o validación geométrica.
#[derive(Clone, Debug, PartialEq)]
pub enum GeometryError {
    /// Una dimensión debe ser estrictamente positiva y finita.
    InvalidDimension { name: &'static str, value: f32 },
    /// Una posición de entrada contiene NaN o infinito.
    NonFiniteVertex { index: usize },
    /// Una cara hace referencia a un vértice inexistente.
    IndexOutOfBounds {
        face_index: usize,
        vertex_index: u32,
        vertex_count: usize,
    },
    /// El índice solicitado no identifica una cara existente.
    FaceIndexOutOfBounds {
        face_index: usize,
        face_count: usize,
    },
    /// Los tres índices de un triángulo deben ser diferentes.
    RepeatedTriangleIndex { face_index: usize },
    /// La cantidad de vértices no cabe en el tipo de índice público `u32`.
    VertexIndexOverflow { vertex_count: usize },
    /// Un cálculo geométrico produjo NaN o infinito.
    NonFiniteComputation {
        face_index: usize,
        operation: &'static str,
    },
    /// Los tres puntos de una cara son colineales en precisión `f64`.
    ///
    /// No se usa una tolerancia absoluta de área: cualquier área representable
    /// distinta de cero es válida, incluso para geometría de escala pequeña.
    DegenerateTriangle { face_index: usize },
    /// La normal de una cara válida no puede representarse como `glam::Vec3`.
    NormalOutOfRange { face_index: usize },
    /// Un cuadrilátero no puede usar dos veces el mismo vértice.
    InvalidQuad,
    /// El cuadrilátero no es coplanar dentro del error de redondeo relativo.
    NonCoplanarQuad,
    /// El perímetro del cuadrilátero se cruza a sí mismo.
    SelfIntersectingQuad,
    /// El cuadrilátero no es estrictamente convexo o contiene tres vértices colineales.
    NonConvexQuad,
    /// Los triángulos generados por la diagonal tienen orientaciones incompatibles.
    InconsistentQuadWinding,
}

impl Display for GeometryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimension { name, value } => {
                write!(f, "la dimensión {name} debe ser positiva y finita, se recibió {value}")
            }
            Self::NonFiniteVertex { index } => write!(f, "el vértice {index} no es finito"),
            Self::IndexOutOfBounds {
                face_index,
                vertex_index,
                vertex_count,
            } => write!(
                f,
                "la cara {face_index} referencia el vértice {vertex_index}, pero solo hay {vertex_count}"
            ),
            Self::FaceIndexOutOfBounds {
                face_index,
                face_count,
            } => write!(f, "la cara {face_index} no existe; hay {face_count} caras"),
            Self::RepeatedTriangleIndex { face_index } => {
                write!(f, "la cara {face_index} repite al menos un índice")
            }
            Self::VertexIndexOverflow { vertex_count } => write!(
                f,
                "no se puede indexar el vértice {vertex_count}: los índices están limitados a u32"
            ),
            Self::NonFiniteComputation {
                face_index,
                operation,
            } => write!(
                f,
                "la cara {face_index} produjo un resultado no finito al calcular {operation}"
            ),
            Self::DegenerateTriangle { face_index } => {
                write!(f, "la cara {face_index} es un triángulo degenerado")
            }
            Self::NormalOutOfRange { face_index } => write!(
                f,
                "la normal de la cara {face_index} no cabe en una Vec3 de precisión simple"
            ),
            Self::InvalidQuad => write!(f, "un cuadrilátero requiere cuatro índices diferentes"),
            Self::NonCoplanarQuad => write!(f, "el cuadrilátero no es coplanar"),
            Self::SelfIntersectingQuad => write!(f, "el cuadrilátero se autointersecta"),
            Self::NonConvexQuad => write!(f, "el cuadrilátero debe ser estrictamente convexo"),
            Self::InconsistentQuadWinding => {
                write!(f, "los triángulos del cuadrilátero tienen orientación incompatible")
            }
        }
    }
}

impl Error for GeometryError {}

/// Malla indexada compuesta exclusivamente por triángulos.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// Vértices compartidos por las caras de la malla.
    pub vertices: Vec<Vertex>,
    /// Triángulos indexados y sus materiales por cara.
    pub faces: Vec<Face>,
}

impl Mesh {
    /// Construye y valida una malla. La validación ocurre al crearla, no durante
    /// el uso normal; puede repetirse tras modificar directamente sus campos públicos.
    pub fn new(vertices: Vec<Vertex>, faces: Vec<Face>) -> Result<Self, GeometryError> {
        let mesh = Self { vertices, faces };
        mesh.validate()?;
        Ok(mesh)
    }

    /// Crea una malla vacía para construirla incrementalmente.
    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    /// Inserta un vértice finito y devuelve su índice sin conversión truncable.
    pub fn add_vertex(&mut self, vertex: Vertex) -> Result<u32, GeometryError> {
        let index = vertex_index_from_count(self.vertices.len())?;
        if !vertex.position.is_finite() {
            return Err(GeometryError::NonFiniteVertex {
                index: self.vertices.len(),
            });
        }
        self.vertices.push(vertex);
        Ok(index)
    }

    /// Inserta un triángulo y valida sus índices, coordenadas y área.
    pub fn add_face(&mut self, face: Face) -> Result<(), GeometryError> {
        self.triangle_cross(self.faces.len(), face)?;
        self.faces.push(face);
        Ok(())
    }

    /// Triangula un cuadrilátero simple, estrictamente convexo y coplanar.
    ///
    /// Los índices deben describir el perímetro en orden consecutivo. No admite
    /// cuadriláteros cóncavos, autointersectados, no coplanares ni degenerados.
    /// Todas las comprobaciones ocurren antes de insertar las dos caras.
    pub fn add_quad(&mut self, indices: [u32; 4], material_id: u32) -> Result<(), GeometryError> {
        if has_repeated_indices(&indices) {
            return Err(GeometryError::InvalidQuad);
        }

        let first = Face::new([indices[0], indices[1], indices[2]], material_id);
        let second = Face::new([indices[0], indices[2], indices[3]], material_id);
        let first_normal = self.triangle_cross(self.faces.len(), first)?;
        let second_normal = self.triangle_cross(self.faces.len() + 1, second)?;
        self.validate_quad(indices, first_normal, second_normal)?;

        self.faces.extend([first, second]);
        Ok(())
    }

    /// Comprueba la integridad completa, útil tras una modificación externa o en depuración.
    pub fn validate(&self) -> Result<(), GeometryError> {
        for (index, vertex) in self.vertices.iter().enumerate() {
            if !vertex.position.is_finite() {
                return Err(GeometryError::NonFiniteVertex { index });
            }
        }
        for (index, face) in self.faces.iter().copied().enumerate() {
            self.triangle_cross(index, face)?;
        }
        Ok(())
    }

    /// Calcula la normal no normalizada de una cara.
    ///
    /// Devuelve `Err` en vez de ocultar una malla modificada con índices inválidos,
    /// coordenadas no finitas o una normal que no puede expresarse en `Vec3`.
    pub fn face_normal(&self, face_index: usize) -> Result<Vec3, GeometryError> {
        let face = *self
            .faces
            .get(face_index)
            .ok_or(GeometryError::FaceIndexOutOfBounds {
                face_index,
                face_count: self.faces.len(),
            })?;
        let normal = self.triangle_cross(face_index, face)?;
        let result = Vec3::new(normal.x as f32, normal.y as f32, normal.z as f32);
        if !result.is_finite() || result == Vec3::ZERO {
            return Err(GeometryError::NormalOutOfRange { face_index });
        }
        Ok(result)
    }

    /// Devuelve la caja mínima alineada a ejes, o `None` si no existen vértices.
    pub fn bounds(&self) -> Option<Aabb> {
        let first = self.vertices.first()?.position;
        Some(self.vertices.iter().skip(1).fold(
            Aabb {
                min: first,
                max: first,
            },
            |bounds, vertex| Aabb {
                min: bounds.min.min(vertex.position),
                max: bounds.max.max(vertex.position),
            },
        ))
    }

    fn triangle_cross(&self, face_index: usize, face: Face) -> Result<DVec3, GeometryError> {
        if has_repeated_indices(&face.indices) {
            return Err(GeometryError::RepeatedTriangleIndex { face_index });
        }

        let [a, b, c] = face
            .indices
            .map(|index| self.vertex_position(face_index, index));
        let (a, b, c) = (a?, b?, c?);
        let cross = (b - a).cross(c - a);
        if !cross.is_finite() || !cross.length_squared().is_finite() {
            return Err(GeometryError::NonFiniteComputation {
                face_index,
                operation: "el producto cruzado",
            });
        }
        if cross.length_squared() == 0.0 {
            return Err(GeometryError::DegenerateTriangle { face_index });
        }
        Ok(cross)
    }

    fn vertex_position(
        &self,
        face_index: usize,
        vertex_index: u32,
    ) -> Result<DVec3, GeometryError> {
        let index = vertex_index as usize;
        let vertex = self
            .vertices
            .get(index)
            .ok_or(GeometryError::IndexOutOfBounds {
                face_index,
                vertex_index,
                vertex_count: self.vertices.len(),
            })?;
        if !vertex.position.is_finite() {
            return Err(GeometryError::NonFiniteVertex { index });
        }
        Ok(DVec3::new(
            f64::from(vertex.position.x),
            f64::from(vertex.position.y),
            f64::from(vertex.position.z),
        ))
    }

    fn validate_quad(
        &self,
        indices: [u32; 4],
        first_normal: DVec3,
        second_normal: DVec3,
    ) -> Result<(), GeometryError> {
        let [a, b, c, d] = indices.map(|index| self.vertex_position(self.faces.len(), index));
        let (a, b, c, d) = (a?, b?, c?, d?);
        let points = [a, b, c, d];
        let edge_scale = [b - a, c - a, d - a]
            .into_iter()
            .map(DVec3::length)
            .fold(0.0_f64, f64::max);
        let volume = (d - a).dot(first_normal);
        let coplanarity_limit = COPLANARITY_ROUNDING_FACTOR * edge_scale.powi(3);
        if !volume.is_finite() || !coplanarity_limit.is_finite() {
            return Err(GeometryError::NonFiniteComputation {
                face_index: self.faces.len(),
                operation: "la coplanaridad del cuadrilátero",
            });
        }
        if volume.abs() > coplanarity_limit {
            return Err(GeometryError::NonCoplanarQuad);
        }

        let projection_axis = dominant_axis(first_normal);
        let projected = points.map(|point| project_to_plane(point, projection_axis));
        if segments_intersect(projected[0], projected[1], projected[2], projected[3])
            || segments_intersect(projected[1], projected[2], projected[3], projected[0])
        {
            return Err(GeometryError::SelfIntersectingQuad);
        }

        if first_normal.dot(second_normal) <= 0.0 {
            return Err(GeometryError::InconsistentQuadWinding);
        }

        let turn_limit = COPLANARITY_ROUNDING_FACTOR * first_normal.length_squared();
        for index in 0..4 {
            let edge = points[(index + 1) % 4] - points[index];
            let next_edge = points[(index + 2) % 4] - points[(index + 1) % 4];
            let turn = edge.cross(next_edge).dot(first_normal);
            if !turn.is_finite() {
                return Err(GeometryError::NonFiniteComputation {
                    face_index: self.faces.len(),
                    operation: "la convexidad del cuadrilátero",
                });
            }
            if turn <= turn_limit {
                return Err(GeometryError::NonConvexQuad);
            }
        }
        Ok(())
    }
}

fn vertex_index_from_count(vertex_count: usize) -> Result<u32, GeometryError> {
    u32::try_from(vertex_count).map_err(|_| GeometryError::VertexIndexOverflow { vertex_count })
}

fn has_repeated_indices<const N: usize>(indices: &[u32; N]) -> bool {
    indices
        .iter()
        .enumerate()
        .any(|(index, value)| indices.iter().skip(index + 1).any(|other| other == value))
}

fn dominant_axis(normal: DVec3) -> usize {
    let absolute = normal.abs();
    if absolute.x >= absolute.y && absolute.x >= absolute.z {
        0
    } else if absolute.y >= absolute.z {
        1
    } else {
        2
    }
}

fn project_to_plane(point: DVec3, discarded_axis: usize) -> (f64, f64) {
    match discarded_axis {
        0 => (point.y, point.z),
        1 => (point.x, point.z),
        _ => (point.x, point.y),
    }
}

fn segments_intersect(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let first = orientation_2d(a, b, c);
    let second = orientation_2d(a, b, d);
    let third = orientation_2d(c, d, a);
    let fourth = orientation_2d(c, d, b);
    (first > 0.0 && second < 0.0 || first < 0.0 && second > 0.0)
        && (third > 0.0 && fourth < 0.0 || third < 0.0 && fourth > 0.0)
}

fn orientation_2d(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

impl Default for Mesh {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{vertex_index_from_count, GeometryError};

    #[test]
    fn vertex_index_accepts_the_last_representable_value() {
        assert_eq!(vertex_index_from_count(u32::MAX as usize), Ok(u32::MAX));
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn vertex_index_rejects_values_after_the_u32_limit() {
        assert!(matches!(
            vertex_index_from_count(u32::MAX as usize + 1),
            Err(GeometryError::VertexIndexOverflow { .. })
        ));
    }
}
