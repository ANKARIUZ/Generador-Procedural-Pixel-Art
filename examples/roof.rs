use procedural_city_engine::procedural::{derive_house_roof, generate_house, HouseConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let house = generate_house(&HouseConfig::default())?;
    let roof = derive_house_roof(&house)?;
    println!(
        "Espesor normal {:.6}, pendiente {:.6}°",
        roof.normal_thickness(),
        roof.slope_radians().to_degrees()
    );
    let [lower, upper] = roof.ridge();
    println!("Cumbrera inferior {lower:?}");
    println!("Cumbrera superior {upper:?}");
    for (label, panel) in ["izquierda", "derecha"].into_iter().zip(roof.panels()) {
        println!(
            "Agua {label}: sección {:?}, límites Z {:?}",
            panel.profile, panel.z
        );
    }
    for (label, cap) in ["izquierdo", "derecho"].into_iter().zip(roof.side_caps()) {
        println!(
            "Apoyo {label}: sección {:?}, límites Z {:?}",
            cap.profile, cap.z
        );
    }
    for (label, gable) in ["trasero", "frontal"].into_iter().zip(roof.gables()) {
        println!(
            "Hastial {label}: sección {:?}, límites Z {:?}",
            gable.profile, gable.z
        );
    }
    Ok(())
}
