use serde::{Deserialize, Serialize};

/// A single entry in a driver championship standings table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverStanding {
    /// Current championship position (1-based)
    pub position: u32,
    /// Driver's full name (e.g., "Max Verstappen")
    pub driver_name: String,
    /// Driver's short abbreviation (e.g., "VER"), if available
    pub driver_code: Option<String>,
    /// Driver's permanent number (e.g., 1), if applicable
    pub driver_number: Option<u32>,
    /// Team/constructor name (e.g., "Red Bull Racing")
    pub team: String,
    /// Total championship points
    pub points: f64,
    /// Number of wins this season
    pub wins: u32,
}

/// A single entry in a constructor/manufacturer/team championship standings table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorStanding {
    /// Current championship position (1-based)
    pub position: u32,
    /// Constructor/team/manufacturer name (e.g., "Red Bull Racing")
    pub name: String,
    /// Total championship points
    pub points: f64,
    /// Number of wins this season
    pub wins: u32,
}

/// Complete championship standings for a single series in a single season.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonStandings {
    /// Series ID (e.g., "f1", "nascar_cup")
    pub series_id: String,
    /// Season year (e.g., 2026)
    pub season: u32,
    /// Driver championship standings (always present)
    pub drivers: Vec<DriverStanding>,
    /// Constructor/manufacturer standings (empty if the series doesn't have them)
    pub constructors: Vec<ConstructorStanding>,
    /// When this data was last fetched (UTC timestamp)
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

impl SeasonStandings {
    /// Returns true if this series has constructor/manufacturer standings.
    pub fn has_constructor_standings(&self) -> bool {
        !self.constructors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_driver_standing_serialization_roundtrip() {
        let standing = DriverStanding {
            position: 1,
            driver_name: "Max Verstappen".to_string(),
            driver_code: Some("VER".to_string()),
            driver_number: Some(1),
            team: "Red Bull Racing".to_string(),
            points: 575.0,
            wins: 19,
        };

        let json = serde_json::to_string(&standing).unwrap();
        let deserialized: DriverStanding = serde_json::from_str(&json).unwrap();
        assert_eq!(standing, deserialized);
    }

    #[test]
    fn test_constructor_standing_serialization_roundtrip() {
        let standing = ConstructorStanding {
            position: 1,
            name: "Red Bull Racing".to_string(),
            points: 860.0,
            wins: 21,
        };

        let json = serde_json::to_string(&standing).unwrap();
        let deserialized: ConstructorStanding = serde_json::from_str(&json).unwrap();
        assert_eq!(standing, deserialized);
    }

    #[test]
    fn test_season_standings_has_constructor_standings() {
        let with_constructors = SeasonStandings {
            series_id: "f1".to_string(),
            season: 2026,
            drivers: vec![DriverStanding {
                position: 1,
                driver_name: "Max Verstappen".to_string(),
                driver_code: Some("VER".to_string()),
                driver_number: Some(1),
                team: "Red Bull Racing".to_string(),
                points: 25.0,
                wins: 1,
            }],
            constructors: vec![ConstructorStanding {
                position: 1,
                name: "Red Bull Racing".to_string(),
                points: 25.0,
                wins: 1,
            }],
            fetched_at: Utc::now(),
        };
        assert!(with_constructors.has_constructor_standings());

        let without_constructors = SeasonStandings {
            series_id: "indycar".to_string(),
            season: 2026,
            drivers: vec![DriverStanding {
                position: 1,
                driver_name: "Alex Palou".to_string(),
                driver_code: None,
                driver_number: Some(10),
                team: "Chip Ganassi Racing".to_string(),
                points: 50.0,
                wins: 1,
            }],
            constructors: vec![],
            fetched_at: Utc::now(),
        };
        assert!(!without_constructors.has_constructor_standings());
    }

    #[test]
    fn test_season_standings_json_roundtrip() {
        let standings = SeasonStandings {
            series_id: "f1".to_string(),
            season: 2026,
            drivers: vec![DriverStanding {
                position: 1,
                driver_name: "Lando Norris".to_string(),
                driver_code: Some("NOR".to_string()),
                driver_number: Some(4),
                team: "McLaren".to_string(),
                points: 26.0,
                wins: 1,
            }],
            constructors: vec![ConstructorStanding {
                position: 1,
                name: "McLaren".to_string(),
                points: 40.0,
                wins: 1,
            }],
            fetched_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&standings).unwrap();
        let deserialized: SeasonStandings = serde_json::from_str(&json).unwrap();
        assert_eq!(standings.series_id, deserialized.series_id);
        assert_eq!(standings.season, deserialized.season);
        assert_eq!(standings.drivers, deserialized.drivers);
        assert_eq!(standings.constructors, deserialized.constructors);
    }
}
