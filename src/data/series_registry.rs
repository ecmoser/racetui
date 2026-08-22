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

const DEFAULT_SERIES_TOML: &str = include_str!("../../data/series.toml");

/// Parse series definitions from a TOML string.
pub fn parse_series_registry(content: &str) -> Result<HashMap<String, Series>> {
    let parsed: SeriesToml =
        toml::from_str(content).with_context(|| "Failed to parse series.toml")?;
    let mut map = HashMap::new();
    for entry in parsed.series {
        let id = entry.id.clone();
        map.insert(id, entry.into_series());
    }
    Ok(map)
}

/// Load series definitions automatically.
/// Checks user override at `~/.config/racetui/series.toml`,
/// then local `./data/series.toml`, and falls back to embedded `series.toml`.
pub fn load_series_registry_auto() -> Result<HashMap<String, Series>> {
    // 1. User config override: ~/.config/racetui/series.toml
    if let Some(config_dir) = dirs::config_dir() {
        let user_override = config_dir.join("racetui").join("series.toml");
        if user_override.exists() {
            return load_series_registry(&user_override);
        }
    }

    // 2. Working directory relative: ./data/series.toml
    let local_path = Path::new("data/series.toml");
    if local_path.exists() {
        return load_series_registry(local_path);
    }

    // 3. Embedded fallback
    parse_series_registry(DEFAULT_SERIES_TOML)
}

/// Load all series definitions from a `series.toml` file.
/// Returns a HashMap keyed by series ID for fast lookup.
pub fn load_series_registry(path: &Path) -> Result<HashMap<String, Series>> {
    if !path.exists() {
        return parse_series_registry(DEFAULT_SERIES_TOML);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read series registry at {}", path.display()))?;
    parse_series_registry(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_series_toml() {
        let registry = load_series_registry(Path::new("data/series.toml"))
            .expect("Should load data/series.toml");
        assert_eq!(registry.len(), 36);

        let f1 = registry.get("f1").expect("F1 should exist");
        assert_eq!(f1.name, "Formula 1");
        assert_eq!(f1.short_name, "F1");
        assert_eq!(f1.car_style, CarStyle::OpenWheel);
        assert_eq!(f1.color, (255, 24, 1));
        assert!(!f1.requires_js);

        let dtm = registry.get("dtm").expect("DTM should exist");
        assert_eq!(dtm.car_style, CarStyle::Touring);
        assert!(dtm.requires_js);
    }
}
