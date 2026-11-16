use glam::Vec3;

use super::{BuildingError, ComponentId, ComponentRef, Transform};

/// Referencia opaca a un material arquitectónico futuro.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildingMaterialId(pub u32);

/// Clase conceptual de un componente de edificio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentKind {
    Foundation,
    Wall,
    Floor,
    Column,
    Roof,
    Door,
    Window,
}

/// Medidas locales de ancho, alto y profundidad en unidades de mundo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dimensions(Vec3);

impl Dimensions {
    /// Crea dimensiones estrictamente positivas y finitas.
    pub fn new(width: f32, height: f32, depth: f32) -> Result<Self, BuildingError> {
        let value = Vec3::new(width, height, depth);
        if !value.is_finite() || value.min_element() <= 0.0 {
            return Err(BuildingError::InvalidDimensions(value));
        }
        Ok(Self(value))
    }

    /// Ancho local.
    pub const fn width(self) -> f32 {
        self.0.x
    }

    /// Alto local.
    pub const fn height(self) -> f32 {
        self.0.y
    }

    /// Profundidad local.
    pub const fn depth(self) -> f32 {
        self.0.z
    }

    /// Vector de dimensiones para futuras conversiones geométricas.
    pub const fn as_vec3(self) -> Vec3 {
        self.0
    }
}

/// Componente arquitectónico conceptual, independiente de GPU y de geometría final.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingComponent {
    reference: ComponentRef,
    kind: ComponentKind,
    dimensions: Dimensions,
    transform: Transform,
    material: BuildingMaterialId,
}

impl BuildingComponent {
    pub(crate) const fn new(
        reference: ComponentRef,
        kind: ComponentKind,
        dimensions: Dimensions,
        transform: Transform,
        material: BuildingMaterialId,
    ) -> Self {
        Self {
            reference,
            kind,
            dimensions,
            transform,
            material,
        }
    }

    /// Identidad local estable del componente.
    pub const fn id(&self) -> ComponentId {
        self.reference.component_id()
    }

    /// Referencia que conserva la identidad del edificio de origen.
    pub const fn reference(&self) -> ComponentRef {
        self.reference
    }

    /// Tipo arquitectónico.
    pub const fn kind(&self) -> ComponentKind {
        self.kind
    }

    /// Dimensiones conceptuales locales.
    pub const fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    /// Transformación local validada.
    pub const fn transform(&self) -> Transform {
        self.transform
    }

    /// Referencia de material arquitectónico.
    pub const fn material(&self) -> BuildingMaterialId {
        self.material
    }
}
