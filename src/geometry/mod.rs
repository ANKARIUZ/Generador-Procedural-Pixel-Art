//! Geometría CPU independiente del renderizador.
//!
//! Convenciones: sistema de mano derecha; X positivo hacia la derecha, Y positivo
//! hacia arriba y Z positivo hacia delante. Las unidades son unidades de mundo.
//! Los triángulos usan orden antihorario vistos desde el exterior, por lo que la
//! normal geométrica es `(b - a).cross(c - a)`. No se almacenan normales porque
//! pueden derivarse de la posición y la topología cuando el renderizador las necesite.

mod face;
mod mesh;
pub mod primitives;
pub mod roof;
mod vertex;

pub use face::Face;
pub use mesh::{Aabb, GeometryError, Mesh};
pub use vertex::Vertex;
