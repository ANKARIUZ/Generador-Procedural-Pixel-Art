//! Conversión CPU de una vivienda estructural a una envolvente y sus insertos.

mod envelope;
mod surface;

use std::{error::Error, fmt, ops::Range};

use crate::{
    building::{Building, ComponentRef},
    geometry::{GeometryError, Mesh},
    procedural::HouseRoofError,
};

/// Geometría agrupada y procedencia por triángulo en el mismo orden de las caras.
pub struct BuildingMesh {
    pub mesh: Mesh,
    pub sources: Vec<ComponentRef>,
    /// Caras de la envolvente. Las caras restantes son insertos cerrados.
    pub envelope_faces: Range<usize>,
}

#[derive(Debug, PartialEq)]
pub enum MeshingError {
    Roof(HouseRoofError),
    UnsupportedLayout,
    InvalidOpening(ComponentRef),
    Geometry(GeometryError),
    InvalidTopology,
}

impl fmt::Display for MeshingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Roof(error) => write!(f, "cubierta incompatible: {error}"),
            Self::UnsupportedLayout => f.write_str("planta o cimentación incompatible"),
            Self::InvalidOpening(reference) => write!(f, "abertura incompatible: {reference:?}"),
            Self::Geometry(error) => write!(f, "geometría inválida: {error}"),
            Self::InvalidTopology => {
                f.write_str("la frontera tiene caras duplicadas o aristas sin pareja opuesta")
            }
        }
    }
}

impl Error for MeshingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Roof(error) => Some(error),
            Self::Geometry(error) => Some(error),
            _ => None,
        }
    }
}

impl From<GeometryError> for MeshingError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

/// Convierte la disposición de vivienda del generador, con materiales originales
/// y coordenadas en el marco de la casa. El edificio se consulta por referencia.
pub fn mesh_building(building: &Building) -> Result<BuildingMesh, MeshingError> {
    envelope::mesh(building)
}

/// Comprueba índices, áreas, duplicados y dos incidencias opuestas por arista.
/// El contrato es topológico y no comprueba autointersecciones arbitrarias.
pub fn validate_closed_mesh(mesh: &Mesh) -> Result<(), MeshingError> {
    surface::validate_closed(mesh)
}
