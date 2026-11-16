use std::{error::Error, fmt};

use crate::building::{BuildingError, BuildingMaterialId};

/// Referencias de material para los seis tipos usados por la vivienda.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseMaterials {
    pub foundation: BuildingMaterialId,
    pub floor: BuildingMaterialId,
    pub wall: BuildingMaterialId,
    pub door: BuildingMaterialId,
    pub window: BuildingMaterialId,
    pub roof: BuildingMaterialId,
}

impl Default for HouseMaterials {
    fn default() -> Self {
        Self {
            foundation: BuildingMaterialId(0),
            floor: BuildingMaterialId(1),
            wall: BuildingMaterialId(2),
            door: BuildingMaterialId(3),
            window: BuildingMaterialId(4),
            roof: BuildingMaterialId(5),
        }
    }
}

/// Parámetros de una vivienda de una planta, expresados en unidades de mundo.
///
/// La puerta se centra en la fachada frontal. Las ventanas ocupan la pared
/// trasera y las laterales. Se admite un recuento de cero ventanas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HouseConfig {
    pub width: f32,
    pub depth: f32,
    pub wall_height: f32,
    /// Desnivel vertical de la cumbrera respecto a la coronación de las paredes.
    pub roof_rise: f32,
    pub door_width: f32,
    pub door_height: f32,
    pub window_width: f32,
    pub window_height: f32,
    pub window_count: usize,
    pub seed: u64,
    pub materials: HouseMaterials,
}

impl Default for HouseConfig {
    fn default() -> Self {
        Self {
            width: 8.0,
            depth: 6.0,
            wall_height: 2.8,
            roof_rise: 1.5,
            door_width: 0.9,
            door_height: 2.1,
            window_width: 1.1,
            window_height: 1.0,
            window_count: 4,
            seed: 0,
            materials: HouseMaterials::default(),
        }
    }
}

/// Error de configuración o de construcción del modelo estructural.
#[derive(Debug, PartialEq)]
pub enum HouseGenerationError {
    InvalidDimension { field: &'static str, value: f32 },
    FootprintTooSmall,
    DoorDoesNotFit,
    WindowTooTall,
    TooManyWindows { requested: usize, capacity: usize },
    Building(BuildingError),
}

// El rango conserva holgura numérica en posiciones f32 frente a los márgenes.
const MIN_DIMENSION: f32 = 0.001;
const MAX_DIMENSION: f32 = 1024.0;

impl HouseConfig {
    pub(super) fn validate_dimensions(&self) -> Result<(), HouseGenerationError> {
        for (field, value) in [
            ("width", self.width),
            ("depth", self.depth),
            ("wall_height", self.wall_height),
            ("roof_rise", self.roof_rise),
            ("door_width", self.door_width),
            ("door_height", self.door_height),
            ("window_width", self.window_width),
            ("window_height", self.window_height),
        ] {
            if !(MIN_DIMENSION..=MAX_DIMENSION).contains(&value) {
                return Err(HouseGenerationError::InvalidDimension { field, value });
            }
        }
        Ok(())
    }
}

impl fmt::Display for HouseGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension { field, value } => write!(
                f, "la dimensión {field} debe estar entre {MIN_DIMENSION} y {MAX_DIMENSION}: {value}"
            ),
            Self::FootprintTooSmall => write!(f, "la planta debe dejar un interior entre las paredes"),
            Self::DoorDoesNotFit => write!(f, "la puerta no cabe con sus márgenes en la pared frontal"),
            Self::WindowTooTall => write!(f, "la ventana no cabe entre el antepecho y la coronación"),
            Self::TooManyWindows { requested, capacity } => write!(
                f, "se solicitaron {requested} ventanas y caben {capacity} con los márgenes y la separación exigidos"
            ),
            Self::Building(error) => write!(f, "no se pudo construir la vivienda: {error}"),
        }
    }
}

impl Error for HouseGenerationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Building(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BuildingError> for HouseGenerationError {
    fn from(error: BuildingError) -> Self {
        Self::Building(error)
    }
}
