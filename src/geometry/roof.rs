//! Perfil simétrico de cubierta, independiente del modelo de edificios.

use std::{error::Error, fmt, ops::RangeInclusive};

use glam::{DVec2, DVec3};

/// Medidas de la envolvente en el marco local de la vivienda.
#[derive(Clone, Copy, Debug)]
pub struct RoofDimensions {
    pub width: f64,
    pub depth: f64,
    pub wall_top: f64,
    pub rise: f64,
    pub normal_thickness: f64,
    pub wall_thickness: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoofProfileError {
    InvalidDimensions,
    UnresolvableProfile,
}

impl fmt::Display for RoofProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions => f.write_str("dimensiones inválidas para la cubierta"),
            Self::UnresolvableProfile => {
                f.write_str("el perfil excede la resolución numérica disponible")
            }
        }
    }
}

impl Error for RoofProfileError {}

/// Sección XY que se prolonga entre dos planos Z.
///
/// Los puntos se entregan por el perímetro. Los intervalos Z son inclusivos
/// para que las piezas contiguas compartan las mismas secciones de contacto.
#[derive(Clone, Debug, PartialEq)]
pub struct RoofSection<const N: usize> {
    pub profile: [DVec2; N],
    pub z: RangeInclusive<f64>,
}

impl<const N: usize> RoofSection<N> {
    /// Caras transversales en orden de Z creciente, sin triangulación.
    pub fn ends(&self) -> [[DVec3; N]; 2] {
        [*self.z.start(), *self.z.end()].map(|z| self.profile.map(|point| point.extend(z)))
    }
}

/// Aguas simétricas con cortes verticales en cumbrera y fachadas.
///
/// El origen XY está centrado horizontalmente y conserva la cota Y de la
/// vivienda. La cara inferior coincide con la cota de pared en los bordes
/// exteriores. El espesor se mide perpendicularmente a cada plano inclinado.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoofProfile {
    run: f64,
    half_depth: f64,
    wall_top: f64,
    rise: f64,
    normal_thickness: f64,
    wall_thickness: f64,
    vertical_offset: f64,
    side_rise: f64,
}

impl RoofProfile {
    /// Valida dimensiones, espacio interior y resolución de las secciones.
    pub fn new(dimensions: RoofDimensions) -> Result<Self, RoofProfileError> {
        let RoofDimensions {
            width,
            depth,
            wall_top,
            rise,
            normal_thickness,
            wall_thickness,
        } = dimensions;
        if ![width, depth, rise, normal_thickness, wall_thickness]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
            || !wall_top.is_finite()
            || wall_thickness >= width / 2.0
            || wall_thickness >= depth / 2.0
        {
            return Err(RoofProfileError::InvalidDimensions);
        }
        let run = width / 2.0;
        let half_depth = depth / 2.0;
        let vertical_offset = normal_thickness * (run.hypot(rise) / run);
        let side_rise = rise * (wall_thickness / run);
        let top = wall_top + rise + vertical_offset;
        let scale = width.max(depth).max(top.abs()).max(wall_top.abs());
        let tolerance = 16.0 * f64::EPSILON * scale;
        if !top.is_finite()
            || [
                run,
                half_depth,
                rise,
                vertical_offset,
                side_rise,
                wall_thickness,
                half_depth - wall_thickness,
                run - wall_thickness,
            ]
            .into_iter()
            .any(|size| !size.is_finite() || size <= tolerance)
        {
            return Err(RoofProfileError::UnresolvableProfile);
        }
        Ok(Self {
            run,
            half_depth,
            wall_top,
            rise,
            normal_thickness,
            wall_thickness,
            vertical_offset,
            side_rise,
        })
    }

    pub fn slope_radians(&self) -> f64 {
        self.rise.atan2(self.run)
    }

    pub fn normal_thickness(&self) -> f64 {
        self.normal_thickness
    }

    /// Bordes de cumbrera inferior y superior, cada uno de atrás hacia delante.
    pub fn ridge(&self) -> [[DVec3; 2]; 2] {
        [
            self.wall_top + self.rise,
            self.wall_top + self.rise + self.vertical_offset,
        ]
        .map(|y| [-self.half_depth, self.half_depth].map(|z| DVec3::new(0.0, y, z)))
    }

    /// Aguas izquierda y derecha, con la junta común en X = 0.
    pub fn panels(&self) -> [RoofSection<4>; 2] {
        let [[lower, _], [upper, _]] = self.ridge();
        [-self.run, self.run].map(|x| RoofSection {
            profile: [
                DVec2::new(x, self.wall_top),
                lower.truncate(),
                upper.truncate(),
                DVec2::new(x, self.wall_top + self.vertical_offset),
            ],
            z: -self.half_depth..=self.half_depth,
        })
    }

    /// Hastiales trasero y frontal. La base toca la pared y el vértice toca
    /// la cumbrera inferior. Cada sección ocupa la banda Z de la pared.
    pub fn gables(&self) -> [RoofSection<3>; 2] {
        let profile = [
            DVec2::new(-self.run, self.wall_top),
            DVec2::new(self.run, self.wall_top),
            self.ridge()[0][0].truncate(),
        ];
        let inner = self.half_depth - self.wall_thickness;
        [-self.half_depth..=-inner, inner..=self.half_depth].map(|z| RoofSection { profile, z })
    }

    /// Rellenos de coronación sobre las paredes laterales, entre los hastiales.
    pub fn side_caps(&self) -> [RoofSection<3>; 2] {
        let inner = self.half_depth - self.wall_thickness;
        [-1.0, 1.0].map(|sign| {
            let outer_x = sign * self.run;
            let inner_x = sign * (self.run - self.wall_thickness);
            RoofSection {
                profile: [
                    DVec2::new(outer_x, self.wall_top),
                    DVec2::new(inner_x, self.wall_top),
                    DVec2::new(inner_x, self.wall_top + self.side_rise),
                ],
                z: -inner..=inner,
            }
        })
    }
}
