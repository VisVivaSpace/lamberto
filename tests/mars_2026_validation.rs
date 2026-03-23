//! Integration tests validating lamberto's full pipeline against NASA TM-2010-216764 Table 2.
//!
//! Reference: Sergeyevsky, Snyder, Cunniff, "Interplanetary Mission Design Handbook,
//! Volume I, Part 2: Earth to Mars Ballistic Mission Opportunities, 2026-2045",
//! NASA TM-2010-216764, JPL, 2010.
//!
//! Table 2 provides energy minima for the 2026 Earth-to-Mars opportunity:
//!
//! | Type | Optimized for | Departure   | Arrival  | C3 (km^2/s^2) | V_inf_arr (km/s) |
//! |------|---------------|-------------|----------|----------------|-------------------|
//! | I    | min C3        | 11/14/2026  | 8/9/2027 | 11.11          | 2.915             |
//! | II   | min C3        | 10/31/2026  | 8/19/2027| 9.144          | 2.729             |
//! | I    | min V_inf_arr | 11/14/2026  | 8/9/2027 | 11.11          | 2.915             |
//! | II   | min V_inf_arr | 11/6/2026   | 9/8/2027 | 9.646          | 2.565             |
//!
//! ## Tolerance rationale
//!
//! C3: +/- 0.8 km^2/s^2. Three factors compound:
//!   1. Grid discretization: 2-day steps can miss the true optimum by ~1 day,
//!      causing up to ~0.3 km^2/s^2 error near the shallow C3 minimum.
//!   2. Ephemeris version: the handbook likely used DE421; we use DE440s.
//!      Planetary position differences of ~1-10 km shift the optimum slightly.
//!   3. Reference frame: lamberto uses Sun-centered positions (heliocentric),
//!      which is the standard frame for interplanetary Lambert problems.
//!      Combined, these effects justify a 0.8 km^2/s^2 tolerance.
//!
//! V_inf_arrival: +/- 0.15 km/s. Same discretization and ephemeris factors
//!   apply, but arrival V-infinity is less sensitive than C3 near its minimum.
//!   0.15 km/s is conservative while still meaningful validation.
//!
//! Dates: +/- 5 days. The 2-day grid step limits date accuracy to +/- 2 days,
//!   and ephemeris/frame differences can shift optima by another 1-2 days.

mod common;
use common::*;

use lamberto::scan::{self, SolutionRow, SweepResult};
use lamberto::transfer::Direction;

// ─── Tolerances ──────────────────────────────────────────────────────────────

const C3_TOLERANCE: f64 = 0.8; // km^2/s^2
const VINF_ARR_TOLERANCE: f64 = 0.15; // km/s
const DATE_TOLERANCE_DAYS: f64 = 5.0; // days

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Filter solutions to only those with transfer angle in the Type I range (< 180 deg).
fn type_i_solutions(result: &SweepResult) -> Vec<&SolutionRow> {
    result
        .solutions
        .iter()
        .filter(|s| s.transfer_angle_deg < 180.0)
        .collect()
}

/// Filter solutions to only those with transfer angle in the Type II range (> 180 deg).
fn type_ii_solutions(result: &SweepResult) -> Vec<&SolutionRow> {
    result
        .solutions
        .iter()
        .filter(|s| s.transfer_angle_deg > 180.0)
        .collect()
}

/// Find the solution with minimum C3 from a filtered slice.
fn min_c3_of<'a>(solutions: &[&'a SolutionRow]) -> &'a SolutionRow {
    solutions
        .iter()
        .min_by(|a, b| a.c3_departure_km2s2.total_cmp(&b.c3_departure_km2s2))
        .expect("should have at least one solution")
}

/// Find the solution with minimum arrival V-infinity from a filtered slice.
fn min_vinf_arr_of<'a>(solutions: &[&'a SolutionRow]) -> &'a SolutionRow {
    solutions
        .iter()
        .min_by(|a, b| a.v_inf_arrival_kms.total_cmp(&b.v_inf_arrival_kms))
        .expect("should have at least one solution")
}

/// Parse a date string like "11/14/2026" into an Epoch for comparison.
fn parse_date(date_str: &str) -> anise::prelude::Epoch {
    let parts: Vec<&str> = date_str.split('/').collect();
    assert_eq!(parts.len(), 3, "Date must be MM/DD/YYYY format");
    let tdb_str = format!("{}-{}-{} 00:00:00 TDB", parts[2], parts[0], parts[1]);
    tdb_str
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse date '{tdb_str}': {e}"))
}

/// Assert that a solution's date is within tolerance of the expected date.
fn assert_date_within(
    label: &str,
    actual: anise::prelude::Epoch,
    expected_str: &str,
    tolerance_days: f64,
) {
    let expected = parse_date(expected_str);
    let diff_days = (actual - expected).to_seconds().abs() / 86400.0;
    assert!(
        diff_days <= tolerance_days,
        "{label}: date mismatch -- expected {expected_str}, got {actual}, \
         difference {diff_days:.1} days exceeds tolerance of {tolerance_days} days"
    );
}

/// Assert arrival V-infinity is within tolerance, with a descriptive message.
fn assert_vinf_arr(label: &str, actual: f64, expected: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= VINF_ARR_TOLERANCE,
        "{label}: V_inf_arr mismatch -- expected {expected:.3} +/- {VINF_ARR_TOLERANCE}, \
         got {actual:.3} (diff {diff:.3})"
    );
}

// ─── Type I tests ────────────────────────────────────────────────────────────

/// Validate Type I minimum C3 against Table 2.
///
/// Expected: departure 11/14/2026, arrival 8/9/2027, C3 = 11.11 km^2/s^2,
/// V_inf_arr = 2.915 km/s.
///
/// For Type I, the min-C3 and min-V_inf_arr optima coincide per the handbook.
#[test]
fn type_i_min_c3() {
    let almanac = load_almanac();

    // Sweep window centered on the expected Type I optimum with comfortable margins.
    // nrev=0 returns both Type I and Type II solutions; we filter post-hoc.
    let sweep = earth_mars_sweep(
        "Type I min-C3 validation",
        "2026-10-01 00:00:00 TDB",
        "2026-12-31 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        2.0, // 2-day steps
        0,   // nrev=0
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("Type I sweep should succeed");
    let t1_sols = type_i_solutions(&result);
    assert!(!t1_sols.is_empty(), "Should have Type I solutions");

    let best = min_c3_of(&t1_sols);

    assert_c3(
        "Type I min C3",
        best.c3_departure_km2s2,
        11.11,
        C3_TOLERANCE,
    );
    assert_vinf_arr("Type I min C3", best.v_inf_arrival_kms, 2.915);
    assert_date_within(
        "Type I min C3 departure",
        best.departure_date,
        "11/14/2026",
        DATE_TOLERANCE_DAYS,
    );
    assert_date_within(
        "Type I min C3 arrival",
        best.arrival_date,
        "08/09/2027",
        DATE_TOLERANCE_DAYS,
    );

    eprintln!(
        "Type I min C3: dep={}, arr={}, C3={:.3}, V_inf_arr={:.3}",
        best.departure_date, best.arrival_date, best.c3_departure_km2s2, best.v_inf_arrival_kms
    );
}

/// Validate Type I minimum arrival V-infinity against Table 2.
///
/// Per the handbook, for Type I the min-C3 and min-V_inf_arr optima coincide:
/// departure 11/14/2026, arrival 8/9/2027, C3 = 11.11, V_inf_arr = 2.915.
///
/// On a discrete 2-day grid, the grid point that minimizes V_inf_arr may not
/// be the same grid point that minimizes C3. The C3 at the min-V_inf_arr grid
/// point can be up to ~1 km^2/s^2 higher than the true continuous minimum,
/// because the C3 surface has a shallow, elongated valley in this region.
/// We therefore apply a wider C3 cross-check tolerance of 1.0 km^2/s^2 for
/// this test (primary validation is on V_inf_arr itself).
#[test]
fn type_i_min_vinf_arrival() {
    let almanac = load_almanac();

    let sweep = earth_mars_sweep(
        "Type I min-V_inf_arr validation",
        "2026-10-01 00:00:00 TDB",
        "2026-12-31 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        2.0,
        0, // nrev=0
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("Type I sweep should succeed");
    let t1_sols = type_i_solutions(&result);
    assert!(!t1_sols.is_empty(), "Should have Type I solutions");

    let best = min_vinf_arr_of(&t1_sols);

    // Primary: V_inf_arr should be close to handbook value.
    assert_vinf_arr("Type I min V_inf_arr", best.v_inf_arrival_kms, 2.915);

    // Cross-check: C3 at the min-V_inf_arr grid point. Use wider tolerance
    // because on a discrete grid the min-V_inf_arr and min-C3 solutions may
    // land on different grid points, even though they coincide in the continuous
    // case. The shallow C3 valley means a 1-2 day offset can shift C3 by ~1.
    let c3_cross_check_tol = 1.0;
    let c3_diff = (best.c3_departure_km2s2 - 11.11).abs();
    assert!(
        c3_diff <= c3_cross_check_tol,
        "Type I min V_inf_arr C3 cross-check: expected 11.11 +/- {c3_cross_check_tol}, \
         got {:.3} (diff {c3_diff:.3})",
        best.c3_departure_km2s2
    );

    assert_date_within(
        "Type I min V_inf_arr departure",
        best.departure_date,
        "11/14/2026",
        DATE_TOLERANCE_DAYS,
    );
    assert_date_within(
        "Type I min V_inf_arr arrival",
        best.arrival_date,
        "08/09/2027",
        DATE_TOLERANCE_DAYS,
    );

    eprintln!(
        "Type I min V_inf_arr: dep={}, arr={}, C3={:.3}, V_inf_arr={:.3}",
        best.departure_date, best.arrival_date, best.c3_departure_km2s2, best.v_inf_arrival_kms
    );
}

// ─── Type II tests ───────────────────────────────────────────────────────────

/// Validate Type II minimum C3 against Table 2.
///
/// Expected: departure 10/31/2026, arrival 8/19/2027, C3 = 9.144 km^2/s^2,
/// V_inf_arr = 2.729 km/s.
#[test]
fn type_ii_min_c3() {
    let almanac = load_almanac();

    // nrev=0 returns both Type I and Type II; we filter to Type II post-hoc.
    let sweep = earth_mars_sweep(
        "Type II min-C3 validation",
        "2026-09-01 00:00:00 TDB",
        "2026-12-31 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-11-01 00:00:00 TDB",
        2.0,
        0, // nrev=0
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("Type II sweep should succeed");
    let t2_sols = type_ii_solutions(&result);
    assert!(!t2_sols.is_empty(), "Should have Type II solutions");

    let best = min_c3_of(&t2_sols);

    assert_c3(
        "Type II min C3",
        best.c3_departure_km2s2,
        9.144,
        C3_TOLERANCE,
    );
    assert_vinf_arr("Type II min C3", best.v_inf_arrival_kms, 2.729);
    assert_date_within(
        "Type II min C3 departure",
        best.departure_date,
        "10/31/2026",
        DATE_TOLERANCE_DAYS,
    );
    assert_date_within(
        "Type II min C3 arrival",
        best.arrival_date,
        "08/19/2027",
        DATE_TOLERANCE_DAYS,
    );

    eprintln!(
        "Type II min C3: dep={}, arr={}, C3={:.3}, V_inf_arr={:.3}",
        best.departure_date, best.arrival_date, best.c3_departure_km2s2, best.v_inf_arrival_kms
    );
}

/// Validate Type II minimum arrival V-infinity against Table 2.
///
/// Expected: departure 11/6/2026, arrival 9/8/2027, C3 = 9.646 km^2/s^2,
/// V_inf_arr = 2.565 km/s.
///
/// Note: the min-V_inf_arr optimum differs from the min-C3 optimum for Type II.
/// The min-V_inf_arr solution has a later arrival and slightly higher C3 but
/// lower arrival V-infinity.
#[test]
fn type_ii_min_vinf_arrival() {
    let almanac = load_almanac();

    let sweep = earth_mars_sweep(
        "Type II min-V_inf_arr validation",
        "2026-09-01 00:00:00 TDB",
        "2026-12-31 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-11-01 00:00:00 TDB",
        2.0,
        0, // nrev=0
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("Type II sweep should succeed");
    let t2_sols = type_ii_solutions(&result);
    assert!(!t2_sols.is_empty(), "Should have Type II solutions");

    let best = min_vinf_arr_of(&t2_sols);

    assert_vinf_arr("Type II min V_inf_arr", best.v_inf_arrival_kms, 2.565);
    assert_c3(
        "Type II min V_inf_arr",
        best.c3_departure_km2s2,
        9.646,
        C3_TOLERANCE,
    );
    assert_date_within(
        "Type II min V_inf_arr departure",
        best.departure_date,
        "11/06/2026",
        DATE_TOLERANCE_DAYS,
    );
    assert_date_within(
        "Type II min V_inf_arr arrival",
        best.arrival_date,
        "09/08/2027",
        DATE_TOLERANCE_DAYS,
    );

    eprintln!(
        "Type II min V_inf_arr: dep={}, arr={}, C3={:.3}, V_inf_arr={:.3}",
        best.departure_date, best.arrival_date, best.c3_departure_km2s2, best.v_inf_arrival_kms
    );
}
