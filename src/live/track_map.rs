use serde::{Deserialize, Serialize};

/// A single coordinate point on the circuit track outline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackPoint {
    /// X coordinate (e.g., in meters or normalized units)
    pub x: f64,
    /// Y coordinate (e.g., in meters or normalized units)
    pub y: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_point_serialization() {
        let point = TrackPoint { x: 123.45, y: -67.89 };
        let json = serde_json::to_string(&point).expect("serialize TrackPoint");
        let parsed: TrackPoint = serde_json::from_str(&json).expect("deserialize TrackPoint");
        assert_eq!(point, parsed);
    }
}
