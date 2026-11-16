use procedural_city_engine::procedural::{generate_house, HouseConfig, HOUSE_ALGORITHM_VERSION};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = HouseConfig {
        seed: 42,
        window_count: 3,
        ..HouseConfig::default()
    };
    let house = generate_house(&config)?;
    println!(
        "Versión {HOUSE_ALGORITHM_VERSION}, semilla {}, {} componentes",
        config.seed,
        house.component_count()
    );
    for component in house.components() {
        let parent = house
            .parent_of(component.reference())?
            .map(|reference| house.component(reference))
            .transpose()?;
        println!(
            "{:?} {:?} {:?} → {:?}",
            component.id(),
            component.kind(),
            component.transform().translation(),
            parent.map(|part| (part.id(), part.kind()))
        );
    }
    Ok(())
}
