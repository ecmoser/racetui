mod config;
mod data;

use std::path::Path;

fn main() {
    // Test config
    let config = config::UserConfig::load().expect("Failed to load config");
    println!("Config loaded. Favorites: {:?}", config.favorites);

    // Test series registry
    let registry = data::series_registry::load_series_registry(Path::new("data/series.toml"))
        .expect("Failed to load series registry");
    println!("Loaded {} series", registry.len());
}



