//! Modelo arquitectónico de edificios, independiente de geometría final y GPU.
//!
//! Un edificio posee componentes conceptuales con dimensiones, transformación
//! y material. Las relaciones tipadas validan pertenencias arquitectónicas sin
//! implementar todavía estabilidad, combustión ni destrucción.

mod component;
mod identity;
#[path = "building.rs"]
mod model;

pub use component::{BuildingComponent, BuildingMaterialId, ComponentKind, Dimensions};
pub use identity::{BuildingId, ComponentId, ComponentRef};
pub use model::{Building, BuildingError};

use crate::scene::Transform;
