use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    building::{Building, BuildingMaterialId, ComponentKind, ComponentRef, Dimensions},
    geometry::Mesh,
    procedural::{generate_house, HouseConfig, HouseMaterials, HOUSE_ALGORITHM_VERSION},
    render::{FacePalette, SpriteSize},
    scene::Transform,
};

use super::{AssetError, MAX_COMPONENTS};

/// Versión del esquema JSON y del perfil de presentación de este sprint.
pub const FORMAT_VERSION: u32 = 1;
pub const IMAGE_FILE: &str = "sprite.png";
pub const MANIFEST_FILE: &str = "asset.json";

/// Resolución lógica, ampliación entera y margen en téxeles antes de ampliar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteSettings {
    pub logical_size: [u32; 2],
    pub scale: u32,
    pub margin: u32,
}

impl Default for SpriteSettings {
    fn default() -> Self {
        Self {
            logical_size: [128, 128],
            scale: 2,
            margin: 4,
        }
    }
}

impl SpriteSettings {
    /// Valida los tamaños y devuelve dimensiones finales en píxeles.
    pub fn output_size(self) -> Result<[u32; 2], AssetError> {
        self.validated().map(|(_, output)| output)
    }

    pub(super) fn validated(self) -> Result<(SpriteSize, [u32; 2]), AssetError> {
        let [w, h] = self.logical_size;
        let size = SpriteSize::new(w, h)?;
        if !(1..=8).contains(&self.scale)
            || w.max(h) > SpriteSize::MAX_EDGE / self.scale
            || self.margin == 0
            || self.margin > (w.min(h) - 1) / 2
        {
            return Err(AssetError::Visual);
        }
        Ok((size, [w * self.scale, h * self.scale]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AssetKind {
    House,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Orientation {
    Isometric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Style {
    PixelV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Visual {
    orientation: Orientation,
    azimuth_degrees: f32,
    elevation_degrees: f32,
    style: Style,
    sprite: SpriteSettings,
    output_size: [u32; 2],
    palette: [PaletteColor; 6],
}

const AZIMUTH: f32 = 45.0;
const ELEVATION: f32 = 35.264_39;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaletteColor {
    material: u32,
    rgb_linear: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    width: f32,
    depth: f32,
    wall_height: f32,
    roof_rise: f32,
    door_width: f32,
    door_height: f32,
    window_width: f32,
    window_height: f32,
    window_count: u32,
    materials: RecipeMaterials,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeMaterials {
    foundation: u32,
    floor: u32,
    wall: u32,
    door: u32,
    window: u32,
    roof: u32,
}

impl RecipeMaterials {
    fn ids(self) -> [u32; 6] {
        [
            self.foundation,
            self.floor,
            self.wall,
            self.door,
            self.window,
            self.roof,
        ]
    }
}

impl Recipe {
    fn new(config: &HouseConfig) -> Result<Self, AssetError> {
        let m = config.materials;
        let recipe = Self {
            width: config.width,
            depth: config.depth,
            wall_height: config.wall_height,
            roof_rise: config.roof_rise,
            door_width: config.door_width,
            door_height: config.door_height,
            window_width: config.window_width,
            window_height: config.window_height,
            window_count: u32::try_from(config.window_count).map_err(|_| AssetError::Recipe)?,
            materials: RecipeMaterials {
                foundation: m.foundation.0,
                floor: m.floor.0,
                wall: m.wall.0,
                door: m.door.0,
                window: m.window.0,
                roof: m.roof.0,
            },
        };
        recipe.validate()?;
        Ok(recipe)
    }

    fn validate(self) -> Result<(), AssetError> {
        if self.window_count as usize > MAX_COMPONENTS - 9
            || [
                self.width,
                self.depth,
                self.wall_height,
                self.roof_rise,
                self.door_width,
                self.door_height,
                self.window_width,
                self.window_height,
            ]
            .into_iter()
            .any(|value| !(0.001..=1024.0).contains(&value))
        {
            return Err(AssetError::Recipe);
        }
        Ok(())
    }

    fn config(self, seed: u64) -> HouseConfig {
        let [foundation, floor, wall, door, window, roof] =
            self.materials.ids().map(BuildingMaterialId);
        HouseConfig {
            width: self.width,
            depth: self.depth,
            wall_height: self.wall_height,
            roof_rise: self.roof_rise,
            door_width: self.door_width,
            door_height: self.door_height,
            window_width: self.window_width,
            window_height: self.window_height,
            window_count: self.window_count as usize,
            seed,
            materials: HouseMaterials {
                foundation,
                floor,
                wall,
                door,
                window,
                roof,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PartKind {
    Foundation,
    Wall,
    Floor,
    Column,
    Roof,
    Door,
    Window,
}

impl From<ComponentKind> for PartKind {
    fn from(kind: ComponentKind) -> Self {
        match kind {
            ComponentKind::Foundation => Self::Foundation,
            ComponentKind::Wall => Self::Wall,
            ComponentKind::Floor => Self::Floor,
            ComponentKind::Column => Self::Column,
            ComponentKind::Roof => Self::Roof,
            ComponentKind::Door => Self::Door,
            ComponentKind::Window => Self::Window,
        }
    }
}

impl From<PartKind> for ComponentKind {
    fn from(kind: PartKind) -> Self {
        match kind {
            PartKind::Foundation => Self::Foundation,
            PartKind::Wall => Self::Wall,
            PartKind::Floor => Self::Floor,
            PartKind::Column => Self::Column,
            PartKind::Roof => Self::Roof,
            PartKind::Door => Self::Door,
            PartKind::Window => Self::Window,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Part {
    id: u32,
    kind: PartKind,
    parent: Option<u32>,
    dimensions: [f32; 3],
    translation: [f32; 3],
    rotation_xyzw: [f32; 4],
    scale: [f32; 3],
    material: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bounds {
    min: [f32; 3],
    max: [f32; 3],
}

impl Bounds {
    fn from_mesh(mesh: &Mesh) -> Result<Self, AssetError> {
        let bounds = mesh.bounds().ok_or(AssetError::Bounds)?;
        Ok(Self {
            min: bounds.min.to_array(),
            max: bounds.max.to_array(),
        })
    }

    fn matches(self, other: Self) -> bool {
        self.min
            .into_iter()
            .chain(self.max)
            .zip(other.min.into_iter().chain(other.max))
            .all(|(a, b)| {
                a.is_finite()
                    && b.is_finite()
                    && (a - b).abs() <= 8.0 * f32::EPSILON * a.abs().max(b.abs()).max(1.0)
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Files {
    sprite: String,
}

/// Documento de vivienda versionado. Sus IDs de componentes solo referencian este documento.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetManifest {
    format_version: u32,
    algorithm_version: u32,
    kind: AssetKind,
    name: String,
    seed: u64,
    recipe: Recipe,
    bounds: Bounds,
    components: Vec<Part>,
    visual: Visual,
    files: Files,
}

impl AssetManifest {
    pub(super) fn capture(
        name: &str,
        algorithm_version: u32,
        config: &HouseConfig,
        building: &Building,
        mesh: &Mesh,
        palette: &FacePalette,
        sprite: SpriteSettings,
    ) -> Result<Self, AssetError> {
        check_count(building.component_count())?;
        let ids: BTreeMap<_, _> = building
            .components()
            .enumerate()
            .map(|(index, part)| (part.id(), index as u32))
            .collect();
        let mut components = Vec::with_capacity(ids.len());
        for part in building.components() {
            let transform = part.transform();
            let parent = building
                .parent_of(part.reference())?
                .map(|parent| ids[&parent.component_id()]);
            components.push(Part {
                id: ids[&part.id()],
                kind: part.kind().into(),
                parent,
                dimensions: part.dimensions().as_vec3().to_array(),
                translation: transform.translation().to_array(),
                rotation_xyzw: transform.rotation().to_array(),
                scale: transform.scale().to_array(),
                material: part.material().0,
            });
        }
        let manifest = Self {
            format_version: FORMAT_VERSION,
            algorithm_version,
            kind: AssetKind::House,
            name: name.to_owned(),
            seed: config.seed,
            recipe: Recipe::new(config)?,
            bounds: Bounds::from_mesh(mesh)?,
            components,
            visual: Visual {
                orientation: Orientation::Isometric,
                azimuth_degrees: AZIMUTH,
                elevation_degrees: ELEVATION,
                style: Style::PixelV1,
                sprite,
                output_size: sprite.output_size()?,
                palette: palette
                    .entries()
                    .map(|(material, rgb_linear)| PaletteColor {
                        material,
                        rgb_linear,
                    }),
            },
            files: Files {
                sprite: IMAGE_FILE.to_owned(),
            },
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Nombre lógico controlado. Su unicidad corresponde al catálogo que consuma el asset.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn algorithm_version(&self) -> u32 {
        self.algorithm_version
    }

    pub fn sprite_settings(&self) -> SpriteSettings {
        self.visual.sprite
    }

    /// Ejecuta la receta solo si corresponde al algoritmo disponible.
    pub fn regenerate(&self) -> Result<Building, AssetError> {
        self.validate()?;
        if self.algorithm_version != HOUSE_ALGORITHM_VERSION {
            return Err(AssetError::AlgorithmVersion(self.algorithm_version));
        }
        Ok(generate_house(&self.recipe.config(self.seed))?)
    }

    pub(super) fn palette(&self) -> FacePalette {
        FacePalette::from_entries(
            self.visual
                .palette
                .map(|color| (color.material, color.rgb_linear)),
        )
    }

    fn validate(&self) -> Result<(), AssetError> {
        if self.format_version != FORMAT_VERSION {
            return Err(AssetError::Version(self.format_version));
        }
        if self.algorithm_version == 0 {
            return Err(AssetError::AlgorithmVersion(0));
        }
        if self.name.is_empty()
            || self.name.len() > 64
            || !self
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(AssetError::Name);
        }
        if self.files.sprite != IMAGE_FILE {
            return Err(AssetError::AssociatedFile);
        }
        if self.visual.sprite.output_size()? != self.visual.output_size
            || self.visual.azimuth_degrees != AZIMUTH
            || self.visual.elevation_degrees != ELEVATION
        {
            return Err(AssetError::Visual);
        }
        self.recipe.validate()?;
        check_count(self.components.len())?;
        let mut colors = BTreeMap::new();
        for entry in self.visual.palette {
            if entry
                .rgb_linear
                .into_iter()
                .any(|value| !(0.0..=1.0).contains(&value))
            {
                return Err(AssetError::Material(entry.material));
            }
            if let Some(previous) = colors.insert(entry.material, entry.rgb_linear) {
                if previous != entry.rgb_linear {
                    return Err(AssetError::Material(entry.material));
                }
            }
        }
        for id in self
            .recipe
            .materials
            .ids()
            .into_iter()
            .chain(self.components.iter().map(|part| part.material))
        {
            if !colors.contains_key(&id) {
                return Err(AssetError::Material(id));
            }
        }
        for part in &self.components {
            if part.parent.is_none() && part.kind != PartKind::Foundation {
                return Err(AssetError::Orphan(part.id));
            }
        }
        Ok(())
    }

    pub(super) fn restore(
        &mut self,
    ) -> Result<(Building, BTreeMap<u32, ComponentRef>), AssetError> {
        self.validate()?;
        self.components.sort_unstable_by_key(|part| part.id);
        for pair in self.components.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(AssetError::DuplicateId(pair[0].id));
            }
        }
        let mut building = Building::new()?;
        let mut references = BTreeMap::new();
        for part in &self.components {
            let [w, h, d] = part.dimensions;
            let dimensions = Dimensions::new(w, h, d).map_err(|source| AssetError::Component {
                id: part.id,
                source,
            })?;
            let transform = Transform::from_normalized_parts(
                Vec3::from_array(part.translation),
                Quat::from_array(part.rotation_xyzw),
                Vec3::from_array(part.scale),
            )
            .map_err(|source| AssetError::Transform {
                id: part.id,
                source,
            })?;
            let reference = building
                .add_component(
                    part.kind.into(),
                    dimensions,
                    transform,
                    BuildingMaterialId(part.material),
                )
                .map_err(|source| AssetError::Component {
                    id: part.id,
                    source,
                })?;
            references.insert(part.id, reference);
        }
        for part in &self.components {
            if let Some(id) = part.parent {
                let parent = references.get(&id).ok_or(AssetError::MissingParent {
                    child: part.id,
                    parent: id,
                })?;
                building
                    .attach(references[&part.id], *parent)
                    .map_err(|source| AssetError::Component {
                        id: part.id,
                        source,
                    })?;
            }
        }
        Ok((building, references))
    }

    pub(super) fn check_bounds(&self, mesh: &Mesh) -> Result<(), AssetError> {
        if self.bounds.matches(Bounds::from_mesh(mesh)?) {
            Ok(())
        } else {
            Err(AssetError::Bounds)
        }
    }
}

pub(super) fn check_count(count: usize) -> Result<(), AssetError> {
    if (1..=MAX_COMPONENTS).contains(&count) {
        Ok(())
    } else {
        Err(AssetError::Count(count))
    }
}
