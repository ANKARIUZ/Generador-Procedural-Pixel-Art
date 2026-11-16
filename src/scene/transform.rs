use std::error::Error;
use std::fmt::{Display, Formatter};

use glam::{Mat4, Quat, Vec3};

/// Error de validación de una transformación local.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransformError {
    /// Una posición contiene NaN o infinito.
    NonFiniteTranslation,
    /// La rotación contiene NaN, infinito o no tiene longitud representable.
    InvalidRotation,
    /// La escala debe ser finita, positiva y no subnormal.
    InvalidScale,
    /// La composición no produjo una matriz finita.
    NonFiniteMatrix,
}

impl Display for TransformError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteTranslation => write!(f, "la traslación debe ser finita"),
            Self::InvalidRotation => write!(f, "la rotación debe ser un cuaternión finito no nulo"),
            Self::InvalidScale => write!(
                f,
                "la escala debe ser finita, positiva y al menos el mínimo normal de f32"
            ),
            Self::NonFiniteMatrix => write!(f, "la matriz de modelo no es finita"),
        }
    }
}

impl Error for TransformError {}

/// Transformación local de un objeto en unidades de mundo y radianes.
///
/// La matriz resultante es `Translation × Rotation × Scale`. La escala mínima
/// aceptada es `f32::MIN_POSITIVE`: se admiten objetos pequeños, pero se evitan
/// valores cero y subnormales que no representan una escala estable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    /// Crea la transformación identidad.
    pub const fn identity() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Crea una transformación validada y normaliza la rotación.
    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Result<Self, TransformError> {
        let mut transform = Self::identity();
        transform.set_translation(translation)?;
        transform.set_rotation(rotation)?;
        transform.set_scale(scale)?;
        transform.model_matrix()?;
        Ok(transform)
    }

    /// Reconstruye una rotación unitaria almacenada sin volver a redondearla.
    pub(crate) fn from_normalized_parts(
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) -> Result<Self, TransformError> {
        if !rotation.is_finite() || (rotation.length_squared() - 1.0).abs() > 4.0 * f32::EPSILON {
            return Err(TransformError::InvalidRotation);
        }
        let mut transform = Self {
            rotation,
            ..Self::identity()
        };
        transform.set_translation(translation)?;
        transform.set_scale(scale)?;
        transform.model_matrix()?;
        Ok(transform)
    }

    /// Posición en coordenadas de mundo.
    pub const fn translation(&self) -> Vec3 {
        self.translation
    }

    /// Rotación local normalizada; los ángulos de construcción usan radianes.
    pub const fn rotation(&self) -> Quat {
        self.rotation
    }

    /// Escala local positiva por eje.
    pub const fn scale(&self) -> Vec3 {
        self.scale
    }

    /// Establece una posición finita.
    pub fn set_translation(&mut self, translation: Vec3) -> Result<(), TransformError> {
        if !translation.is_finite() {
            return Err(TransformError::NonFiniteTranslation);
        }
        self.translation = translation;
        Ok(())
    }

    /// Establece una rotación finita no nula y la normaliza.
    pub fn set_rotation(&mut self, rotation: Quat) -> Result<(), TransformError> {
        let length_squared = rotation.length_squared();
        if !rotation.is_finite()
            || !length_squared.is_finite()
            || length_squared <= f32::MIN_POSITIVE
        {
            return Err(TransformError::InvalidRotation);
        }
        let rotation = rotation.normalize();
        if !rotation.is_finite() {
            return Err(TransformError::InvalidRotation);
        }
        self.rotation = rotation;
        Ok(())
    }

    /// Establece una escala positiva, finita y no subnormal por eje.
    pub fn set_scale(&mut self, scale: Vec3) -> Result<(), TransformError> {
        if !scale.is_finite() || scale.min_element() < f32::MIN_POSITIVE {
            return Err(TransformError::InvalidScale);
        }
        self.scale = scale;
        Ok(())
    }

    /// Calcula la matriz de modelo con orden `Translation × Rotation × Scale`.
    pub fn model_matrix(&self) -> Result<Mat4, TransformError> {
        let model = Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale);
        model
            .is_finite()
            .then_some(model)
            .ok_or(TransformError::NonFiniteMatrix)
    }
}
