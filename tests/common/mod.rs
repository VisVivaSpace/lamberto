//! Shared helpers for lamberto integration tests.

#![allow(dead_code)]

use anise::prelude::Almanac;
use lamberto::config::{BodySpec, Sweep};
use lamberto::transfer::Direction;

/// Load the almanac using the embedded ephemeris.
/// If `LAMBERTO_SPK_PATH` is set, loads that as an additional kernel.
pub fn load_almanac() -> Almanac {
    let extra = std::env::var("LAMBERTO_SPK_PATH").ok();
    lamberto::load_almanac(extra.as_deref()).expect("Failed to load ephemeris")
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
