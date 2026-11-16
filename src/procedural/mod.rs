//! Generación de viviendas estructurales en CPU.

mod config;
mod house;
mod roof;

pub use config::{HouseConfig, HouseGenerationError, HouseMaterials};
pub use house::{generate_house, HOUSE_ALGORITHM_VERSION};
pub use roof::{derive_house_roof, HouseRoofError};
