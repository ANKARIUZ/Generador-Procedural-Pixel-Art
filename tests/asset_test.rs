use std::io::Cursor;

use procedural_city_engine::{
    asset::{AssetError, HouseAsset, SpriteSettings, MAX_COMPONENTS, MAX_DOCUMENT_BYTES},
    building::BuildingMaterialId,
    procedural::{generate_house, HouseConfig, HOUSE_ALGORITHM_VERSION},
    render::{FacePalette, OffscreenError},
};
use serde_json::{json, Value};

fn config() -> HouseConfig {
    HouseConfig {
        seed: u64::MAX - 3,
        ..HouseConfig::default()
    }
}

fn asset(config: &HouseConfig, settings: SpriteSettings) -> HouseAsset {
    HouseAsset::generate("house_test", config, &FacePalette::house(), settings).unwrap()
}

fn bytes(asset: &HouseAsset) -> Vec<u8> {
    let mut output = Vec::new();
    asset.export_json(&mut output).unwrap();
    output
}

fn document() -> Value {
    serde_json::from_slice(&bytes(&asset(&config(), SpriteSettings::default()))).unwrap()
}

fn import(document: &Value) -> Result<HouseAsset, AssetError> {
    HouseAsset::import_json(serde_json::to_vec(document).unwrap().as_slice())
}

#[test]
fn structural_round_trip_preserves_geometry_and_rebinds_references() {
    let initial = asset(&config(), SpriteSettings::default());
    let encoded = bytes(&initial);
    let restored = HouseAsset::import_json(encoded.as_slice()).unwrap();
    assert_eq!(bytes(&restored), encoded);
    assert_eq!(restored.manifest().seed(), config().seed);
    assert_eq!(
        restored.manifest().algorithm_version(),
        HOUSE_ALGORITHM_VERSION
    );
    assert_ne!(initial.building().id(), restored.building().id());
    assert_eq!(initial.mesh().mesh.vertices, restored.mesh().mesh.vertices);
    assert_eq!(initial.mesh().mesh.faces, restored.mesh().mesh.faces);

    for id in 0..initial.building().component_count() as u32 {
        let original = initial.component_ref(id).unwrap();
        let rebound = restored.component_ref(id).unwrap();
        let a = initial.building().component(original).unwrap();
        let b = restored.building().component(rebound).unwrap();
        assert_eq!(a.kind(), b.kind());
        assert_eq!(a.material(), b.material());
        assert_eq!(a.dimensions(), b.dimensions());
        assert_eq!(a.transform(), b.transform());
        assert!(restored.building().component(original).is_err());
        assert_eq!(rebound.building_id(), restored.building().id());
        let parent = |building: &procedural_city_engine::building::Building, part| {
            building
                .parent_of(part)
                .unwrap()
                .map(|parent| parent.component_id())
        };
        assert_eq!(
            parent(initial.building(), original),
            parent(restored.building(), rebound)
        );
    }
}

#[test]
fn canonical_json_and_recipe_are_deterministic() {
    let config = config();
    let original = generate_house(&config).unwrap();
    let a = asset(&config, SpriteSettings::default());
    let b = asset(&config, SpriteSettings::default());
    assert_eq!(bytes(&a), bytes(&b));
    let regenerated = a.manifest().regenerate().unwrap();
    let signature = |building: &procedural_city_engine::building::Building| {
        building
            .components()
            .map(|part| {
                (
                    part.kind(),
                    part.dimensions(),
                    part.transform(),
                    part.material(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(signature(&original), signature(&regenerated));
}

#[test]
fn existing_structure_is_authoritative_over_provenance_recipe() {
    let recipe = config();
    let edited_config = HouseConfig {
        width: 13.0,
        ..recipe
    };
    let existing = generate_house(&edited_config).unwrap();
    let original_id = existing.id();
    let asset = HouseAsset::from_building(
        "existing_house",
        HOUSE_ALGORITHM_VERSION,
        &recipe,
        existing,
        &FacePalette::house(),
        SpriteSettings::default(),
    )
    .unwrap();
    assert_eq!(asset.building().id(), original_id);
    let imported = HouseAsset::import_json(bytes(&asset).as_slice()).unwrap();
    assert_eq!(imported.mesh().mesh.bounds(), asset.mesh().mesh.bounds());
    let regenerated = asset.manifest().regenerate().unwrap();
    let width = |building: &procedural_city_engine::building::Building| {
        building.components().next().unwrap().dimensions().width()
    };
    assert_eq!(width(imported.building()), edited_config.width);
    assert_eq!(width(&regenerated), recipe.width);
}

#[test]
fn stored_structure_supports_unknown_algorithm_versions_and_sparse_ids() {
    let mut doc = document();
    doc["algorithm_version"] = json!(99);
    let parts = doc["components"].as_array_mut().unwrap();
    for part in parts.iter_mut() {
        part["id"] = json!(100 + 7 * part["id"].as_u64().unwrap());
        if let Some(parent) = part["parent"].as_u64() {
            part["parent"] = json!(100 + 7 * parent);
        }
    }
    parts.reverse();
    let restored = import(&doc).unwrap();
    assert_eq!(restored.manifest().algorithm_version(), 99);
    assert!(matches!(
        restored.manifest().regenerate(),
        Err(AssetError::AlgorithmVersion(99))
    ));
    assert!(restored.component_ref(0).is_none());
    let floor = restored.component_ref(107).unwrap();
    assert_eq!(
        restored.building().parent_of(floor).unwrap(),
        restored.component_ref(100)
    );
    let again = HouseAsset::import_json(bytes(&restored).as_slice()).unwrap();
    assert_ne!(restored.building().id(), again.building().id());
    assert_eq!(restored.manifest(), again.manifest());
}

#[test]
fn validation_rejects_bad_syntax_versions_and_unbounded_documents() {
    assert!(matches!(
        HouseAsset::import_json(b"{".as_slice()),
        Err(AssetError::Json(_))
    ));
    assert!(matches!(
        HouseAsset::import_json(vec![b' '; MAX_DOCUMENT_BYTES + 1].as_slice()),
        Err(AssetError::DocumentTooLarge)
    ));
    let mut doc = document();
    doc["format_version"] = json!(2);
    doc.as_object_mut().unwrap().remove("components");
    assert!(matches!(import(&doc), Err(AssetError::Version(2))));
    let mut doc = document();
    doc["components"] = json!(vec![doc["components"][0].clone(); MAX_COMPONENTS + 1]);
    assert!(matches!(import(&doc), Err(AssetError::Count(_))));
}

#[test]
fn validation_checks_local_links_known_types_dimensions_and_transforms() {
    let base = document();
    let with = |index: usize, field: &str, value: Value| {
        let mut doc = base.clone();
        doc["components"][index][field] = value;
        import(&doc)
    };
    assert!(matches!(
        with(1, "id", json!(0)),
        Err(AssetError::DuplicateId(0))
    ));
    assert!(matches!(
        with(1, "parent", json!(999)),
        Err(AssetError::MissingParent { .. })
    ));
    assert!(matches!(
        with(1, "parent", json!(1)),
        Err(AssetError::Component { .. })
    ));
    assert!(matches!(
        with(1, "parent", json!(2)),
        Err(AssetError::Component { .. })
    ));
    assert!(matches!(
        with(1, "parent", Value::Null),
        Err(AssetError::Orphan(1))
    ));
    assert!(matches!(
        with(1, "kind", json!("stairs")),
        Err(AssetError::Json(_))
    ));
    assert!(matches!(
        with(1, "dimensions", json!([-1, 1, 1])),
        Err(AssetError::Component { .. })
    ));
    assert!(matches!(
        with(1, "rotation_xyzw", json!([0, 0, 0, 0])),
        Err(AssetError::Transform { .. })
    ));
    assert!(matches!(
        with(1, "scale", json!([1, 0, 1])),
        Err(AssetError::Transform { .. })
    ));
    assert!(matches!(
        with(1, "translation", json!([null, 0, 1])),
        Err(AssetError::Json(_))
    ));
    let nonfinite = serde_json::to_string(&base).unwrap().replacen(
        "\"translation\":[0.0,",
        "\"translation\":[1e100,",
        1,
    );
    assert!(matches!(
        HouseAsset::import_json(nonfinite.as_bytes()),
        Err(AssetError::Transform { .. } | AssetError::Json(_))
    ));
}

#[test]
fn validation_rejects_foreign_paths_invalid_visual_settings_and_missing_materials() {
    let mut doc = document();
    doc["files"]["sprite"] = json!("../other.png");
    assert!(matches!(import(&doc), Err(AssetError::AssociatedFile)));
    let mut doc = document();
    doc["bounds"]["max"][1] = json!(100);
    assert!(matches!(import(&doc), Err(AssetError::Bounds)));
    let mut doc = document();
    doc["visual"]["output_size"] = json!([512, 256]);
    assert!(matches!(import(&doc), Err(AssetError::Visual)));
    let mut doc = document();
    doc["components"][1]["material"] = json!(8000);
    assert!(matches!(import(&doc), Err(AssetError::Material(8000))));
    let mut custom = config();
    custom.materials.wall = BuildingMaterialId(8000);
    assert!(matches!(
        HouseAsset::generate(
            "test",
            &custom,
            &FacePalette::house(),
            SpriteSettings::default()
        ),
        Err(AssetError::Material(8000))
    ));
    assert!(SpriteSettings {
        logical_size: [u32::MAX, 65],
        ..SpriteSettings::default()
    }
    .output_size()
    .is_err());
    assert!(SpriteSettings {
        scale: 0,
        ..SpriteSettings::default()
    }
    .output_size()
    .is_err());
    assert!(SpriteSettings {
        margin: u32::MAX,
        ..SpriteSettings::default()
    }
    .output_size()
    .is_err());
    assert!(matches!(
        HouseAsset::generate(
            "test",
            &HouseConfig {
                window_count: usize::MAX,
                ..config()
            },
            &FacePalette::house(),
            SpriteSettings::default()
        ),
        Err(AssetError::Count(_))
    ));
}

#[test]
fn export_never_reuses_an_existing_directory() {
    let asset = asset(&config(), SpriteSettings::default());
    assert!(
        matches!(asset.export_package("."), Err(AssetError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists)
    );
}

#[test]
#[ignore = "requiere un adaptador GPU"]
fn gpu_asset_round_trip_preserves_transparency_palette_and_integer_pixels() {
    // 65 * 4 bytes obliga a añadir y eliminar relleno de cada fila.
    let settings = SpriteSettings {
        logical_size: [65, 51],
        scale: 3,
        margin: 4,
    };
    let mut custom = config();
    custom.width = 17.25;
    custom.depth = 5.5;
    let asset = asset(&custom, settings);
    let imported = HouseAsset::import_json(bytes(&asset).as_slice()).unwrap();
    let mut png = Vec::new();
    let mut rebuilt_png = Vec::new();
    asset.export_png(&mut png).unwrap();
    imported.export_png(&mut rebuilt_png).unwrap();
    assert_eq!(png, rebuilt_png);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    assert!(reader.info().srgb.is_some());
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!([info.width, info.height], [195, 153]);
    assert_eq!(info.color_type, png::ColorType::Rgba);
    let (width, height) = (info.width as usize, info.height as usize);
    let at = |x, y| -> &[u8] {
        let offset: usize = (y * width + x) * 4;
        &pixels[offset..offset + 4]
    };
    let margin = (settings.margin * settings.scale) as usize;
    let mut colors = std::collections::BTreeSet::new();
    for y in 0..height {
        for x in 0..width {
            let pixel = at(x, y);
            assert_eq!(pixel, at(x / 3 * 3, y / 3 * 3));
            if x < margin
                || y < margin
                || x >= width - margin
                || y >= height - margin
                || pixel[3] == 0
            {
                assert_eq!(pixel, [0; 4]);
            } else {
                assert_eq!(pixel[3], 255);
                colors.insert((pixel[0], pixel[1], pixel[2]));
            }
        }
    }
    assert!(colors.iter().any(|&(r, g, b)| r > g && g.abs_diff(b) < 12)); // Cubierta terracota.
    assert!(colors.iter().any(|&(r, g, b)| b > g && g > r)); // Ventanas azules.
    assert!(colors.iter().any(|&(r, g, b)| r > 170 && r > g && g > b)); // Fachadas crema.
    assert!(colors.len() >= 5);
}

#[test]
fn offscreen_api_rejects_invalid_margins_before_gpu_setup() {
    let asset = asset(&config(), SpriteSettings::default());
    let result = procedural_city_engine::render::render_sprite(
        &asset.mesh().mesh,
        &FacePalette::house(),
        procedural_city_engine::render::SpriteSize::new(8, 8).unwrap(),
        4,
    );
    assert!(matches!(result, Err(OffscreenError::Margin)));
}
