use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use glam::Vec3;

use crate::scene::{Transform, TransformError};

use super::{
    BuildingComponent, BuildingId, BuildingMaterialId, ComponentId, ComponentKind, ComponentRef,
    Dimensions,
};

/// Error de construcción o modificación del modelo arquitectónico.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildingError {
    InvalidDimensions(Vec3),
    InvalidTransform(TransformError),
    BuildingIdExhausted,
    ForeignComponent {
        expected: BuildingId,
        reference: ComponentRef,
    },
    ComponentNotFound(ComponentRef),
    ComponentIdExhausted,
    SelfRelation(ComponentRef),
    InvalidRelation {
        child: ComponentKind,
        parent: ComponentKind,
    },
    ComponentAlreadyHasParent(ComponentRef),
    ComponentHasDependents(ComponentRef),
}

impl Display for BuildingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions(dimensions) => write!(
                f,
                "las dimensiones deben ser positivas y finitas, se recibió {:?}",
                dimensions
            ),
            Self::InvalidTransform(error) => write!(f, "transformación inválida: {error}"),
            Self::BuildingIdExhausted => write!(f, "se agotaron los identificadores de edificio"),
            Self::ForeignComponent {
                expected,
                reference,
            } => write!(
                f,
                "la referencia {reference:?} no pertenece al edificio {expected:?}"
            ),
            Self::ComponentNotFound(id) => write!(f, "el componente {:?} no existe", id),
            Self::ComponentIdExhausted => {
                write!(f, "se agotaron los identificadores de componente")
            }
            Self::SelfRelation(id) => write!(
                f,
                "el componente {:?} no puede relacionarse consigo mismo",
                id
            ),
            Self::InvalidRelation { child, parent } => {
                write!(
                    f,
                    "un componente {:?} no puede depender de {:?}",
                    child, parent
                )
            }
            Self::ComponentAlreadyHasParent(id) => {
                write!(
                    f,
                    "el componente {:?} ya tiene una relación de pertenencia",
                    id
                )
            }
            Self::ComponentHasDependents(id) => write!(
                f,
                "el componente {:?} tiene dependientes y no puede eliminarse",
                id
            ),
        }
    }
}

impl Error for BuildingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTransform(error) => Some(error),
            Self::InvalidDimensions(_)
            | Self::BuildingIdExhausted
            | Self::ForeignComponent { .. }
            | Self::ComponentNotFound(_)
            | Self::ComponentIdExhausted
            | Self::SelfRelation(_)
            | Self::InvalidRelation { .. }
            | Self::ComponentAlreadyHasParent(_)
            | Self::ComponentHasDependents(_) => None,
        }
    }
}

/// Modelo estructural de un edificio que posee sus componentes y relaciones.
///
/// Cada instancia tiene una identidad propia y todas las consultas y mutaciones
/// por referencia comprueban su procedencia. El edificio carece de `Clone`.
///
/// ```compile_fail,E0599
/// use procedural_city_engine::building::Building;
/// let building = Building::new().unwrap();
/// let duplicate = building.clone();
/// ```
pub struct Building {
    id: BuildingId,
    components: BTreeMap<ComponentId, BuildingComponent>,
    /// Cada componente hijo tiene, como máximo, un componente arquitectónico padre.
    parents: BTreeMap<ComponentId, ComponentId>,
    next_component_id: u64,
}

impl Building {
    /// Crea un edificio vacío o devuelve un error si se agotaron las identidades.
    pub fn new() -> Result<Self, BuildingError> {
        Ok(Self {
            id: BuildingId::allocate()?,
            components: BTreeMap::new(),
            parents: BTreeMap::new(),
            next_component_id: 0,
        })
    }

    /// Identidad de esta instancia de edificio.
    pub const fn id(&self) -> BuildingId {
        self.id
    }

    /// Registra un componente conceptual sin crear geometría ni recursos GPU.
    pub fn add_component(
        &mut self,
        kind: ComponentKind,
        dimensions: Dimensions,
        transform: Transform,
        material: BuildingMaterialId,
    ) -> Result<ComponentRef, BuildingError> {
        transform
            .model_matrix()
            .map_err(BuildingError::InvalidTransform)?;
        let id = ComponentId(self.next_component_id);
        self.next_component_id = self
            .next_component_id
            .checked_add(1)
            .ok_or(BuildingError::ComponentIdExhausted)?;
        let reference = ComponentRef::new(self.id, id);
        self.components.insert(
            id,
            BuildingComponent::new(reference, kind, dimensions, transform, material),
        );
        Ok(reference)
    }

    fn local_id(&self, reference: ComponentRef) -> Result<ComponentId, BuildingError> {
        if reference.building_id() != self.id {
            return Err(BuildingError::ForeignComponent {
                expected: self.id,
                reference,
            });
        }
        Ok(reference.component_id())
    }

    /// Consulta un componente mediante una referencia emitida por este edificio.
    pub fn component(&self, reference: ComponentRef) -> Result<&BuildingComponent, BuildingError> {
        self.components
            .get(&self.local_id(reference)?)
            .ok_or(BuildingError::ComponentNotFound(reference))
    }

    /// Relaciona un componente hijo con un padre arquitectónicamente compatible.
    pub fn attach(
        &mut self,
        child_ref: ComponentRef,
        parent_ref: ComponentRef,
    ) -> Result<(), BuildingError> {
        let child_id = self.local_id(child_ref)?;
        let parent_id = self.local_id(parent_ref)?;
        if child_id == parent_id {
            return Err(BuildingError::SelfRelation(child_ref));
        }
        let child = self.component(child_ref)?;
        let parent = self.component(parent_ref)?;
        if self.parents.contains_key(&child_id) {
            return Err(BuildingError::ComponentAlreadyHasParent(child_ref));
        }
        if !relation_is_allowed(child.kind(), parent.kind()) {
            return Err(BuildingError::InvalidRelation {
                child: child.kind(),
                parent: parent.kind(),
            });
        }
        self.parents.insert(child_id, parent_id);
        Ok(())
    }

    /// Retira un componente sin dependientes; sus identificadores nunca se reutilizan.
    pub fn remove_component(
        &mut self,
        reference: ComponentRef,
    ) -> Result<BuildingComponent, BuildingError> {
        let id = self.local_id(reference)?;
        if self.parents.values().any(|parent| *parent == id) {
            return Err(BuildingError::ComponentHasDependents(reference));
        }
        let component = self
            .components
            .remove(&id)
            .ok_or(BuildingError::ComponentNotFound(reference))?;
        self.parents.remove(&id);
        Ok(component)
    }

    /// Número de componentes registrados.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Componente padre, si el hijo tiene una relación.
    pub fn parent_of(&self, child: ComponentRef) -> Result<Option<ComponentRef>, BuildingError> {
        let id = self.component(child)?.id();
        Ok(self
            .parents
            .get(&id)
            .map(|&parent| ComponentRef::new(self.id, parent)))
    }

    /// Recorre los componentes por identificador para resultados deterministas.
    pub fn components(&self) -> impl Iterator<Item = &BuildingComponent> {
        self.components.values()
    }

    /// Recorre los hijos de un componente por identificador.
    pub fn children_of(
        &self,
        parent: ComponentRef,
    ) -> Result<impl Iterator<Item = ComponentRef> + '_, BuildingError> {
        let parent_id = self.component(parent)?.id();
        Ok(self.parents.iter().filter_map(move |(&child, &candidate)| {
            (candidate == parent_id).then_some(ComponentRef::new(self.id, child))
        }))
    }
}

fn relation_is_allowed(child: ComponentKind, parent: ComponentKind) -> bool {
    match child {
        ComponentKind::Foundation => false,
        ComponentKind::Floor => parent == ComponentKind::Foundation,
        ComponentKind::Wall | ComponentKind::Column => {
            matches!(parent, ComponentKind::Foundation | ComponentKind::Floor)
        }
        ComponentKind::Roof => matches!(parent, ComponentKind::Wall | ComponentKind::Column),
        ComponentKind::Door | ComponentKind::Window => parent == ComponentKind::Wall,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausting_component_ids_preserves_components_and_links() {
        let mut building = Building {
            next_component_id: u64::MAX - 2,
            ..Building::new().expect("identidad disponible")
        };
        let dimensions = Dimensions::new(1.0, 1.0, 1.0).expect("dimensiones válidas");
        let material = BuildingMaterialId(0);
        let wall = building
            .add_component(
                ComponentKind::Wall,
                dimensions,
                Transform::identity(),
                material,
            )
            .expect("penúltima asignación válida");
        let window = building
            .add_component(
                ComponentKind::Window,
                dimensions,
                Transform::identity(),
                material,
            )
            .expect("última asignación válida");
        building.attach(window, wall).expect("relación válida");

        assert_eq!(wall.component_id(), ComponentId(u64::MAX - 2));
        assert_eq!(window.component_id(), ComponentId(u64::MAX - 1));
        for _ in 0..2 {
            assert_eq!(
                building.add_component(
                    ComponentKind::Door,
                    dimensions,
                    Transform::identity(),
                    material
                ),
                Err(BuildingError::ComponentIdExhausted)
            );
            assert_eq!(building.next_component_id, u64::MAX);
            assert!(building
                .components()
                .map(BuildingComponent::reference)
                .eq([wall, window]));
            assert_eq!(building.parent_of(window), Ok(Some(wall)));
        }
    }
}
