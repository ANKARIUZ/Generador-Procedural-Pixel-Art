//! Documentos estructurales de vivienda y paquetes de exportación.

mod manifest;

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    path::Path,
};
use thiserror::Error;

use crate::{
    building::{Building, BuildingError, ComponentRef},
    meshing::{mesh_building, BuildingMesh, MeshingError},
    procedural::{generate_house, HouseConfig, HouseGenerationError, HOUSE_ALGORITHM_VERSION},
    render::{render_sprite, FacePalette, OffscreenError},
    scene::TransformError,
};

pub use manifest::{AssetManifest, SpriteSettings, FORMAT_VERSION, IMAGE_FILE, MANIFEST_FILE};

/// Tamaño máximo del JSON de versión 1.
pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;
/// Límite de piezas para acotar también el costo del mallado de entradas externas.
pub const MAX_COMPONENTS: usize = 128;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("el documento supera {MAX_DOCUMENT_BYTES} bytes")]
    DocumentTooLarge,
    #[error("JSON de asset inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("versión de formato incompatible: {0}")]
    Version(u32),
    #[error("versión del algoritmo no disponible: {0}")]
    AlgorithmVersion(u32),
    #[error(
        "el nombre lógico debe tener de 1 a 64 letras ASCII, dígitos, guiones o guiones bajos"
    )]
    Name,
    #[error("el archivo asociado debe llamarse sprite.png")]
    AssociatedFile,
    #[error("metadatos de presentación o escala incompatibles")]
    Visual,
    #[error("configuración procedural fuera del contrato de la versión 1")]
    Recipe,
    #[error("el límite es de 1 a {MAX_COMPONENTS} componentes, se recibieron {0}")]
    Count(usize),
    #[error("identificador de componente duplicado: {0}")]
    DuplicateId(u32),
    #[error("el componente {child} referencia al padre inexistente {parent}")]
    MissingParent { child: u32, parent: u32 },
    #[error("el componente {0} requiere un padre")]
    Orphan(u32),
    #[error("el componente {id} no cumple el contrato: {source}")]
    Component { id: u32, source: BuildingError },
    #[error("transformación inválida del componente {id}: {source}")]
    Transform { id: u32, source: TransformError },
    #[error("material {0} ausente de la paleta o con un color inválido o ambiguo")]
    Material(u32),
    #[error("los límites declarados no corresponden a la malla")]
    Bounds,
    #[error(transparent)]
    Building(#[from] BuildingError),
    #[error(transparent)]
    Generation(#[from] HouseGenerationError),
    #[error(transparent)]
    Meshing(#[from] MeshingError),
    #[error(transparent)]
    Render(#[from] OffscreenError),
    #[error("no se pudo codificar PNG: {0}")]
    Png(#[from] png::EncodingError),
    #[error("no se pudo acceder al paquete: {0}")]
    Io(#[from] io::Error),
}

/// Vivienda validada con una sola estructura como fuente de la malla y del manifiesto.
pub struct HouseAsset {
    manifest: AssetManifest,
    building: Building,
    mesh: BuildingMesh,
    references: BTreeMap<u32, ComponentRef>,
}

impl HouseAsset {
    /// Genera una vivienda y prepara su representación de exportación. La paleta
    /// debe abarcar los IDs indicados en la receta.
    pub fn generate(
        name: &str,
        config: &HouseConfig,
        palette: &FacePalette,
        sprite: SpriteSettings,
    ) -> Result<Self, AssetError> {
        let maximum_windows = MAX_COMPONENTS - 9;
        if config.window_count > maximum_windows {
            return Err(AssetError::Count(config.window_count.saturating_add(9)));
        }
        sprite.output_size()?;
        Self::from_building(
            name,
            HOUSE_ALGORITHM_VERSION,
            config,
            generate_house(config)?,
            palette,
            sprite,
        )
    }

    /// Prepara una vivienda existente con su receta y versión de procedencia.
    /// El edificio suministrado se usa como fuente de verdad.
    pub fn from_building(
        name: &str,
        algorithm_version: u32,
        config: &HouseConfig,
        building: Building,
        palette: &FacePalette,
        sprite: SpriteSettings,
    ) -> Result<Self, AssetError> {
        manifest::check_count(building.component_count())?;
        sprite.output_size()?;
        let mesh = mesh_building(&building)?;
        let manifest = AssetManifest::capture(
            name,
            algorithm_version,
            config,
            &building,
            &mesh.mesh,
            palette,
            sprite,
        )?;
        let references = building
            .components()
            .enumerate()
            .map(|(index, part)| (index as u32, part.reference()))
            .collect();
        Ok(Self {
            manifest,
            building,
            mesh,
            references,
        })
    }

    /// Lee un documento acotado. Restaura la estructura mediante la API
    /// arquitectónica, comprueba que pueda mallarse y verifica sus límites.
    pub fn import_json(reader: impl Read) -> Result<Self, AssetError> {
        let mut bytes = Vec::new();
        reader
            .take(MAX_DOCUMENT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err(AssetError::DocumentTooLarge);
        }
        #[derive(serde::Deserialize)]
        struct Header {
            format_version: u32,
        }
        let Header { format_version } = serde_json::from_slice(&bytes)?;
        if format_version != FORMAT_VERSION {
            return Err(AssetError::Version(format_version));
        }
        let mut manifest: AssetManifest = serde_json::from_slice(&bytes)?;
        let (building, references) = manifest.restore()?;
        let mesh = mesh_building(&building)?;
        manifest.check_bounds(&mesh.mesh)?;
        Ok(Self {
            manifest,
            building,
            mesh,
            references,
        })
    }

    pub fn manifest(&self) -> &AssetManifest {
        &self.manifest
    }

    pub fn building(&self) -> &Building {
        &self.building
    }

    pub fn mesh(&self) -> &BuildingMesh {
        &self.mesh
    }

    /// Resuelve un ID local al documento en la instancia de edificio actual.
    pub fn component_ref(&self, document_id: u32) -> Option<ComponentRef> {
        self.references.get(&document_id).copied()
    }

    /// Exporta JSON en el orden canónico de componentes, sin iniciar GPU.
    pub fn export_json(&self, mut writer: impl Write) -> Result<(), AssetError> {
        serde_json::to_writer_pretty(&mut writer, &self.manifest)?;
        writer.write_all(b"\n")?;
        Ok(())
    }

    /// Exporta RGBA sRGB con alfa recto. La ampliación entera replica téxeles.
    pub fn export_png(&self, writer: impl Write) -> Result<(), AssetError> {
        let settings = self.manifest.sprite_settings();
        let (size, [width, height]) = settings.validated()?;
        let rgba = render_sprite(
            &self.mesh.mesh,
            &self.manifest.palette(),
            size,
            settings.margin,
        )?;
        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut png = encoder.write_header()?;
        let [logical_width, _] = size.dimensions();
        let scale = settings.scale as usize;
        if scale == 1 {
            png.write_image_data(&rgba)?;
        } else {
            let mut stream = png.stream_writer()?;
            let mut expanded = Vec::with_capacity(width as usize * 4);
            for row in rgba.chunks_exact(logical_width as usize * 4) {
                expanded.clear();
                for pixel in row.as_chunks::<4>().0 {
                    for _ in 0..scale {
                        expanded.extend_from_slice(pixel);
                    }
                }
                for _ in 0..scale {
                    stream.write_all(&expanded)?;
                }
            }
            stream.finish()?;
        }
        png.finish()?;
        Ok(())
    }

    /// Publica en un directorio nuevo. El manifiesto se publica al final y
    /// actúa como marcador de paquete completo.
    pub fn export_package(&self, directory: impl AsRef<Path>) -> Result<(), AssetError> {
        let directory = directory.as_ref();
        if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir(directory)?;
        let image = directory.join(IMAGE_FILE);
        let pending = directory.join("asset.json.tmp");
        let result = (|| {
            let mut png =
                BufWriter::new(File::options().write(true).create_new(true).open(&image)?);
            self.export_png(&mut png)?;
            png.flush()?;
            png.get_ref().sync_all()?;
            let mut json = BufWriter::new(
                File::options()
                    .write(true)
                    .create_new(true)
                    .open(&pending)?,
            );
            self.export_json(&mut json)?;
            json.flush()?;
            json.get_ref().sync_all()?;
            drop(json);
            fs::rename(&pending, directory.join(MANIFEST_FILE))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&pending);
            let _ = fs::remove_file(&image);
            let _ = fs::remove_dir(directory);
        }
        result
    }
}
