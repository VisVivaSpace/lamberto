//! Shared helpers for lamberto integration tests.

#![allow(dead_code)]

use anise::prelude::Almanac;
use lamberto::config::{BodySpec, Sweep};
use lamberto::transfer::Direction;

/// Load the DE440s almanac from the project assets directory.
pub fn load_almanac() -> Almanac {
    // Integration tests run with cwd = crate root (code/lamberto/).
    // The SPK file is at ../../assets/de440s.bsp relative to that.
    let spk_path = "../../assets/de440s.bsp";
    Almanac::default()
        .load(spk_path)
        .unwrap_or_else(|e| panic!("Failed to load SPK file at '{spk_path}': {e}"))
}

/// Build a Sweep config for Earth-to-Mars with the given parameters.
pub fn earth_mars_sweep(
    name: &str,
    dep_start: &str,
    dep_end: &str,
    arr_start: &str,
    arr_end: &str,
    step_days: f64,
    nrev: u32,
    direction: Direction,
) -> Sweep {
    Sweep {
        name: name.to_string(),
        departure_body: BodySpec::Name("Earth".to_string()),
        arrival_body: BodySpec::Name("Mars".to_string()),
        departure_start: dep_start.to_string(),
        departure_end: dep_end.to_string(),
        departure_step_days: step_days,
        arrival_start: arr_start.to_string(),
        arrival_end: arr_end.to_string(),
        arrival_step_days: step_days,
        nrev,
        direction,
        target_v_inf_departure: 0.0,
        target_v_inf_arrival: 0.0,
    }
}

/// Assert C3 is within tolerance, with a descriptive message.
pub fn assert_c3(label: &str, actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "{label}: C3 mismatch -- expected {expected:.3} +/- {tolerance}, \
         got {actual:.3} (diff {diff:.3})"
    );
}
