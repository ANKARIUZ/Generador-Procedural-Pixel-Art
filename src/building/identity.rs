use std::sync::atomic::{AtomicU64, Ordering};

use super::BuildingError;

static NEXT_BUILDING_ID: AtomicU64 = AtomicU64::new(0);

/// Identidad de una instancia de edificio durante la ejecución del proceso.
///
/// Su valor puede repetirse en otro proceso o tras reiniciar la aplicación.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildingId(u64);

impl BuildingId {
    pub(super) fn allocate() -> Result<Self, BuildingError> {
        Self::allocate_from(&NEXT_BUILDING_ID)
    }

    fn allocate_from(counter: &AtomicU64) -> Result<Self, BuildingError> {
        let mut current = counter.load(Ordering::Relaxed);
        loop {
            let next = current
                .checked_add(1)
                .ok_or(BuildingError::BuildingIdExhausted)?;
            match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(id) => return Ok(Self(id)),
                Err(observed) => current = observed,
            }
        }
    }
}

/// Identificador monotónico local a un edificio, útil para comparar su estructura.
///
/// ```compile_fail,E0308
/// use procedural_city_engine::building::{Building, ComponentId};
/// fn lookup(building: &Building, id: ComponentId) {
///     building.component(id);
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(pub(super) u64);

/// Referencia de instancia a un componente, emitida por su edificio.
///
/// Su identidad incluye la procedencia y tiene el mismo ámbito que `BuildingId`.
///
/// ```compile_fail,E0624
/// use procedural_city_engine::building::{BuildingId, ComponentId, ComponentRef};
/// fn rebind(building: BuildingId, component: ComponentId) -> ComponentRef {
///     ComponentRef::new(building, component)
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentRef {
    building_id: BuildingId,
    component_id: ComponentId,
}

impl ComponentRef {
    pub(super) const fn new(building_id: BuildingId, component_id: ComponentId) -> Self {
        Self {
            building_id,
            component_id,
        }
    }

    /// Identidad de la instancia que emitió la referencia.
    pub const fn building_id(self) -> BuildingId {
        self.building_id
    }

    /// Identidad local para comparar estructuras de distintos edificios.
    pub const fn component_id(self) -> ComponentId {
        self.component_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_identity_exhaustion_does_not_wrap_or_advance() {
        let counter = AtomicU64::new(u64::MAX - 1);
        assert_eq!(
            BuildingId::allocate_from(&counter),
            Ok(BuildingId(u64::MAX - 1))
        );
        for _ in 0..2 {
            assert_eq!(
                BuildingId::allocate_from(&counter),
                Err(BuildingError::BuildingIdExhausted)
            );
            assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        }
    }
}
