use std::{error::Error, ffi::OsStr, fs::File, path::PathBuf};

use procedural_city_engine::{
    asset::{HouseAsset, SpriteSettings, IMAGE_FILE, MANIFEST_FILE},
    procedural::HouseConfig,
    render::FacePalette,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            let config = HouseConfig { seed: 42, ..HouseConfig::default() };
            let asset =
                HouseAsset::generate("house_0001", &config, &FacePalette::house(), SpriteSettings::default())?;
            export(&asset, PathBuf::from("output/assets/house_0001"))?;
        }
        [operation, output] if operation == OsStr::new("export") => {
            let config = HouseConfig { seed: 42, ..HouseConfig::default() };
            let asset =
                HouseAsset::generate("house_0001", &config, &FacePalette::house(), SpriteSettings::default())?;
            export(&asset, PathBuf::from(output))?;
        }
        [operation, json] if operation == OsStr::new("import") => {
            let asset = HouseAsset::import_json(File::open(json)?)?;
            println!(
                "Importado {} · semilla {} · {} componentes · {} triángulos",
                asset.manifest().name(), asset.manifest().seed(),
                asset.building().component_count(), asset.mesh().mesh.faces.len()
            );
        }
        [operation, json, output] if operation == OsStr::new("render") => {
            let asset = HouseAsset::import_json(File::open(json)?)?;
            export(&asset, PathBuf::from(output))?;
        }
        _ => return Err("Uso: export_house [export <directorio> | import <asset.json> | render <asset.json> <directorio>]".into()),
    }
    Ok(())
}

fn export(asset: &HouseAsset, directory: PathBuf) -> Result<(), Box<dyn Error>> {
    asset.export_package(&directory)?;
    let [width, height] = asset.manifest().sprite_settings().output_size()?;
    println!(
        "Exportación correcta · semilla {} · {width} × {height}",
        asset.manifest().seed()
    );
    println!("{}", directory.join(MANIFEST_FILE).display());
    println!("{}", directory.join(IMAGE_FILE).display());
    Ok(())
}
