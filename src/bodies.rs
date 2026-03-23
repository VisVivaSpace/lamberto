use anise::constants::frames::{
    EARTH_MOON_BARYCENTER_J2000, JUPITER_BARYCENTER_J2000, MARS_BARYCENTER_J2000, MERCURY_J2000,
    NEPTUNE_BARYCENTER_J2000, SATURN_BARYCENTER_J2000, URANUS_BARYCENTER_J2000, VENUS_J2000,
};
use anise::prelude::Frame;

use crate::config::BodySpec;

/// Resolve a body specification (name or NAIF ID) to an anise Frame.
pub fn resolve_frame(body: &BodySpec) -> Result<Frame, String> {
    match body {
        BodySpec::Name(name) => frame_from_name(name),
        BodySpec::Id(id) => frame_from_naif_id(*id),
    }
}

fn frame_from_name(name: &str) -> Result<Frame, String> {
    match name.to_lowercase().as_str() {
        "mercury" => Ok(MERCURY_J2000),
        "venus" => Ok(VENUS_J2000),
        "earth" => Ok(EARTH_MOON_BARYCENTER_J2000),
        "mars" => Ok(MARS_BARYCENTER_J2000),
        "jupiter" => Ok(JUPITER_BARYCENTER_J2000),
        "saturn" => Ok(SATURN_BARYCENTER_J2000),
        "uranus" => Ok(URANUS_BARYCENTER_J2000),
        "neptune" => Ok(NEPTUNE_BARYCENTER_J2000),
        _ => Err(format!("Unknown body name: {name}")),
    }
}

fn frame_from_naif_id(id: i32) -> Result<Frame, String> {
    match id {
        1 => Ok(MERCURY_J2000),
        2 => Ok(VENUS_J2000),
        3 => Ok(EARTH_MOON_BARYCENTER_J2000),
        4 => Ok(MARS_BARYCENTER_J2000),
        5 => Ok(JUPITER_BARYCENTER_J2000),
        6 => Ok(SATURN_BARYCENTER_J2000),
        7 => Ok(URANUS_BARYCENTER_J2000),
        8 => Ok(NEPTUNE_BARYCENTER_J2000),
        _ => Err(format!("Unknown NAIF barycenter ID: {id}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_name_case_insensitive() {
        let earth = resolve_frame(&BodySpec::Name("Earth".to_string())).unwrap();
        let earth_lower = resolve_frame(&BodySpec::Name("earth".to_string())).unwrap();
        let earth_upper = resolve_frame(&BodySpec::Name("EARTH".to_string())).unwrap();
        assert_eq!(earth, earth_lower);
        assert_eq!(earth, earth_upper);
        assert_eq!(earth, EARTH_MOON_BARYCENTER_J2000);
    }

    #[test]
    fn test_resolve_naif_id() {
        assert_eq!(
            resolve_frame(&BodySpec::Id(3)).unwrap(),
            EARTH_MOON_BARYCENTER_J2000
        );
        assert_eq!(
            resolve_frame(&BodySpec::Id(4)).unwrap(),
            MARS_BARYCENTER_J2000
        );
    }

    #[test]
    fn test_resolve_all_planets() {
        for id in 1..=8 {
            assert!(
                resolve_frame(&BodySpec::Id(id)).is_ok(),
                "NAIF ID {id} should resolve"
            );
        }
        for name in [
            "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
        ] {
            assert!(
                resolve_frame(&BodySpec::Name(name.to_string())).is_ok(),
                "{name} should resolve"
            );
        }
    }

    #[test]
    fn test_unknown_body() {
        assert!(resolve_frame(&BodySpec::Name("Pluto".to_string())).is_err());
        assert!(resolve_frame(&BodySpec::Id(99)).is_err());
    }
}
