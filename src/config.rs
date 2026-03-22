use serde::Deserialize;

use crate::error::LambertoError;
use crate::transfer::Direction;

/// Top-level configuration loaded from a YAML file.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Path to the NAIF SPK ephemeris file.
    pub spk_file: String,
    /// List of trajectory sweeps to execute.
    pub sweeps: Vec<Sweep>,
}

/// A celestial body specified by name or NAIF ID.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BodySpec {
    /// Body name (e.g. `"Earth"`, `"Mars"`).
    Name(String),
    /// NAIF barycenter ID (1-8).
    Id(i32),
}

fn default_direction() -> Direction {
    Direction::Prograde
}

fn default_target_v_inf() -> f64 {
    0.0
}

/// Parameters for a single departure/arrival date-grid sweep.
#[derive(Debug, Clone, Deserialize)]
pub struct Sweep {
    /// Human-readable sweep label (also used for output filenames).
    pub name: String,
    /// Departure body (planet name or NAIF ID).
    pub departure_body: BodySpec,
    /// Arrival body (planet name or NAIF ID).
    pub arrival_body: BodySpec,
    /// Start of the departure date window (TDB epoch string).
    pub departure_start: String,
    /// End of the departure date window (TDB epoch string).
    pub departure_end: String,
    /// Departure date grid step size in days.
    pub departure_step_days: f64,
    /// Start of the arrival date window (TDB epoch string).
    pub arrival_start: String,
    /// End of the arrival date window (TDB epoch string).
    pub arrival_end: String,
    /// Arrival date grid step size in days.
    pub arrival_step_days: f64,
    /// Number of complete revolutions (default: 0).
    #[serde(default)]
    pub nrev: u32,
    /// Transfer direction: prograde or retrograde (default: prograde).
    #[serde(default = "default_direction")]
    pub direction: Direction,
    /// Target departure v-infinity (km/s) for best-solution ranking.
    #[serde(default = "default_target_v_inf")]
    pub target_v_inf_departure: f64,
    /// Target arrival v-infinity (km/s) for best-solution ranking.
    #[serde(default = "default_target_v_inf")]
    pub target_v_inf_arrival: f64,
}

/// Load and parse a sweep configuration from a YAML file.
pub fn load_config(path: impl AsRef<std::path::Path>) -> Result<Config, LambertoError> {
    let contents = std::fs::read_to_string(path.as_ref())
        .map_err(|e| LambertoError::Config(e.to_string()))?;
    let config: Config = serde_yaml_ng::from_str(&contents)
        .map_err(|e| LambertoError::Config(e.to_string()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
spk_file: "assets/de440s.bsp"
sweeps:
  - name: "Earth-Mars Type I"
    departure_body: "Earth"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
"#;
        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.spk_file, "assets/de440s.bsp");
        assert_eq!(config.sweeps.len(), 1);
        let s = &config.sweeps[0];
        assert!(matches!(s.departure_body, BodySpec::Name(ref n) if n == "Earth"));
        assert!(matches!(s.arrival_body, BodySpec::Name(ref n) if n == "Mars"));
        assert_eq!(s.nrev, 0);
        assert!(matches!(s.direction, Direction::Prograde));
        assert_eq!(s.target_v_inf_departure, 0.0);
        assert_eq!(s.target_v_inf_arrival, 0.0);
    }

    #[test]
    fn test_parse_naif_ids() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - name: "Test"
    departure_body: 3
    arrival_body: 4
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
    nrev: 1
    direction: retrograde
    target_v_inf_departure: 3.5
    target_v_inf_arrival: 2.0
"#;
        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
        let s = &config.sweeps[0];
        assert!(matches!(s.departure_body, BodySpec::Id(3)));
        assert!(matches!(s.arrival_body, BodySpec::Id(4)));
        assert!(matches!(s.direction, Direction::Retrograde));
        assert_eq!(s.nrev, 1);
        assert_eq!(s.target_v_inf_departure, 3.5);
        assert_eq!(s.target_v_inf_arrival, 2.0);
    }

    // --- LA-15: Config error-path tests ---

    #[test]
    fn load_config_missing_file() {
        let result = load_config("/nonexistent/path/config.yaml");
        assert!(result.is_err(), "missing file should return Err");
    }

    #[test]
    fn parse_missing_spk_file_field() {
        let yaml = r#"
sweeps:
  - name: "Test"
    departure_body: "Earth"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "missing spk_file should fail");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("missing field") || err_msg.contains("spk_file"),
            "error should mention the missing field, got: {err_msg}"
        );
    }

    #[test]
    fn parse_missing_sweeps_field() {
        let yaml = r#"
spk_file: "de440s.bsp"
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "missing sweeps should fail");
    }

    #[test]
    fn parse_missing_required_sweep_fields() {
        // Missing name
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - departure_body: "Earth"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "missing sweep 'name' should fail");
    }

    #[test]
    fn parse_missing_departure_body() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - name: "Test"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "missing departure_body should fail");
    }

    #[test]
    fn parse_missing_step_days() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - name: "Test"
    departure_body: "Earth"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "missing departure_step_days should fail");
    }

    #[test]
    fn parse_malformed_yaml() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - name: "Test"
    departure_body: [invalid yaml structure
    this is not valid
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "malformed YAML should fail");
    }

    #[test]
    fn parse_completely_invalid_yaml() {
        let yaml = ":::not yaml at all{{{";
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "completely invalid YAML should fail");
    }

    #[test]
    fn parse_empty_input() {
        let yaml = "";
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "empty input should fail");
    }

    #[test]
    fn parse_wrong_type_for_step_days() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - name: "Test"
    departure_body: "Earth"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: "not_a_number"
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "string for step_days should fail");
    }

    #[test]
    fn parse_invalid_direction_value() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - name: "Test"
    departure_body: "Earth"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
    direction: "diagonal"
"#;
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "invalid direction value should fail");
    }

    #[test]
    fn parse_negative_nrev_fails() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps:
  - name: "Test"
    departure_body: "Earth"
    arrival_body: "Mars"
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
    nrev: -1
"#;
        // u32 cannot represent -1; serde_yaml_ng should reject this
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_err(), "negative nrev (u32) should fail");
    }

    #[test]
    fn parse_empty_sweeps_list() {
        let yaml = r#"
spk_file: "de440s.bsp"
sweeps: []
"#;
        // Empty sweeps is structurally valid (Vec<Sweep> can be empty)
        let result: Result<Config, _> = serde_yaml_ng::from_str(yaml);
        assert!(result.is_ok(), "empty sweeps list should parse");
        assert_eq!(result.unwrap().sweeps.len(), 0);
    }

    #[test]
    fn load_config_with_malformed_file() {
        let dir = std::env::temp_dir().join("lamberto_test_config_malformed");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_config.yaml");
        std::fs::write(&path, ":::not valid yaml{{{").unwrap();

        let result = load_config(path.to_str().unwrap());
        assert!(result.is_err(), "load_config with malformed file should return Err");

        std::fs::remove_dir_all(&dir).ok();
    }
}
