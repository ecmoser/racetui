mod data;

use std::path::Path;

fn main() {
    let registry = data::series_registry::load_series_registry(Path::new("data/series.toml"))
        .expect("Failed to load series registry");
    println!("Loaded {} series:", registry.len());
    for (id, series) in &registry {
        println!(
            "  {} ({}) - {} - color: ({},{},{})",
            series.name, id, series.car_style, series.color.0, series.color.1, series.color.2
        );
    }
}


