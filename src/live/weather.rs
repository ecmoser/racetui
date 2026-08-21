use crate::live::event::WeatherInfo;

impl WeatherInfo {
    /// Format weather information as a concise single-line display string for the UI.
    /// Example: "Air: 24.5°C  |  Track: 38.2°C  |  Humidity: 55%  |  Wind: 12.0 km/h (180°)  |  ☀️ Dry"
    pub fn display_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(air) = self.air_temp_c {
            parts.push(format!("Air: {:.1}°C", air));
        }

        if let Some(track) = self.track_temp_c {
            parts.push(format!("Track: {:.1}°C", track));
        }

        if let Some(hum) = self.humidity_pct {
            parts.push(format!("Humidity: {:.0}%", hum));
        }

        if let Some(wind) = self.wind_speed_kmh {
            if let Some(dir) = self.wind_direction_deg {
                parts.push(format!("Wind: {:.1} km/h ({}°)", wind, dir as u32));
            } else {
                parts.push(format!("Wind: {:.1} km/h", wind));
            }
        }

        let rain_status = if self.rainfall {
            if let Some(ref intensity) = self.rain_intensity {
                format!("🌧️ Rain ({})", intensity)
            } else {
                "🌧️ Rain".to_string()
            }
        } else if let Some(ref desc) = self.description {
            if desc.to_lowercase().contains("wet") || desc.to_lowercase().contains("rain") {
                format!("🌧️ {}", desc)
            } else {
                format!("☀️ {}", desc)
            }
        } else {
            "☀️ Dry".to_string()
        };
        parts.push(rain_status);

        if parts.is_empty() {
            "Weather data unavailable".to_string()
        } else {
            parts.join("  |  ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_string_full() {
        let weather = WeatherInfo {
            air_temp_c: Some(25.4),
            track_temp_c: Some(40.2),
            humidity_pct: Some(58.0),
            wind_speed_kmh: Some(15.2),
            wind_direction_deg: Some(270.0),
            rainfall: false,
            rain_intensity: None,
            description: Some("Clear".to_string()),
        };
        let s = weather.display_string();
        assert!(s.contains("Air: 25.4°C"));
        assert!(s.contains("Track: 40.2°C"));
        assert!(s.contains("Humidity: 58%"));
        assert!(s.contains("Wind: 15.2 km/h (270°)"));
        assert!(s.contains("☀️ Clear"));
    }

    #[test]
    fn test_display_string_rain() {
        let weather = WeatherInfo {
            air_temp_c: Some(18.0),
            track_temp_c: Some(20.0),
            humidity_pct: Some(92.0),
            wind_speed_kmh: Some(8.0),
            wind_direction_deg: None,
            rainfall: true,
            rain_intensity: Some("Heavy".to_string()),
            description: None,
        };
        let s = weather.display_string();
        assert!(s.contains("Air: 18.0°C"));
        assert!(s.contains("Wind: 8.0 km/h"));
        assert!(s.contains("🌧️ Rain (Heavy)"));
    }

    #[test]
    fn test_display_string_empty() {
        let weather = WeatherInfo::default();
        let s = weather.display_string();
        assert_eq!(s, "☀️ Dry");
    }
}
