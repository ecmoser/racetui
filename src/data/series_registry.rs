use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use super::models::{CarStyle, Series};

/// Intermediate deserialization struct matching the TOML format.
/// This is needed because TOML arrays of tables deserialize differently
/// than the final `Series` struct.
#[derive(Debug, Deserialize)]
struct SeriesToml {
    series: Vec<SeriesTomlEntry>,
}

#[derive(Debug, Deserialize)]
struct SeriesTomlEntry {
    id: String,
    name: String,
    short_name: String,
    car_style: String,
    region: String,
    color: [u8; 3],
    calendar_url: String,
    requires_js: bool,
}

impl SeriesTomlEntry {
    fn into_series(self) -> Series {
        let car_style = match self.car_style.as_str() {
            "OpenWheel" => CarStyle::OpenWheel,
            "SportsCar" => CarStyle::SportsCar,
            "StockCar" => CarStyle::StockCar,
            "Touring" => CarStyle::Touring,
            "Rally" => CarStyle::Rally,
            "Motorcycle" => CarStyle::Motorcycle,
            other => panic!("Unknown car style in series.toml: '{}'", other),
        };
        Series {
            id: self.id,
            name: self.name,
            short_name: self.short_name,
            car_style,
            color: (self.color[0], self.color[1], self.color[2]),
            region: self.region,
            calendar_url: self.calendar_url,
            requires_js: self.requires_js,
        }
    }
}

/// Load all series definitions from a `series.toml` file.
/// Returns a HashMap keyed by series ID for fast lookup.
pub fn load_series_registry(path: &Path) -> Result<HashMap<String, Series>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read series registry at {}", path.display()))?;
    let parsed: SeriesToml = toml::from_str(&content)
        .with_context(|| "Failed to parse series.toml")?;
    let mut map = HashMap::new();
    for entry in parsed.series {
        let id = entry.id.clone();
        map.insert(id, entry.into_series());
    }
    Ok(map)
}
