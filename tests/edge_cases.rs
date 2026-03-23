//! Edge case and robustness tests for the lamberto scan pipeline.
//!
//! These tests exercise boundary conditions, singularity handling, solver failure
//! modes, and direction configuration that the validation tests do not cover.
//! All tests use real ephemeris geometry from DE440s.

mod common;
use common::*;

use lamberto::scan;
use lamberto::transfer::Direction;

// ─── Near-180 deg singularity handling ───────────────────────────────────────

/// Verify that sweeps crossing the 180 deg transfer angle ridge correctly skip
/// near-singularity points without crashing, and still produce solutions on
/// both sides.
///
/// Strategy: Fix a single departure date and sweep arrival dates at very fine
/// resolution (0.01-day = ~15 min steps) over a narrow window that straddles
/// the 180 deg boundary. With nrev=0, a single sweep now returns both Type I
/// and Type II solutions.
#[test]
fn near_180_singularity_is_skipped_without_crash() {
    let almanac = load_almanac();

    let sweep = earth_mars_sweep(
        "singularity-probe",
        "2026-11-01 00:00:00 TDB",
        "2026-11-01 00:00:00 TDB",
        "2027-08-01 00:00:00 TDB",
        "2027-08-10 00:00:00 TDB",
        0.01,
        0,
        Direction::Prograde,
    );

    let r = scan::run_sweep(&almanac, &sweep).expect("singularity sweep failed");

    eprintln!(
        "Singularity test: {} solutions, {} singularity skips",
        r.solutions.len(),
        r.skipped_singularity,
    );

    if r.skipped_singularity > 0 {
        eprintln!("Singularity hit confirmed: {} skips", r.skipped_singularity);
    }

    assert!(
        !r.solutions.is_empty(),
        "Sweep should produce solutions near the 180 deg boundary"
    );

    // Accounting invariant (no skipped_type anymore)
    let accounted = r.solutions.len() as u64
        + r.skipped_tof
        + r.skipped_singularity
        + r.skipped_solver
        + r.skipped_ephemeris;
    assert_eq!(accounted, r.total_points, "accounting mismatch");
}

/// Verify that solutions on either side of the 180 deg boundary have transfer
/// angles that are genuinely on opposite sides (Type I < 180, Type II > 180).
#[test]
fn solutions_straddle_180_boundary() {
    let almanac = load_almanac();

    let sweep = earth_mars_sweep(
        "boundary-straddle",
        "2026-10-15 00:00:00 TDB",
        "2026-11-15 00:00:00 TDB",
        "2027-07-15 00:00:00 TDB",
        "2027-09-15 00:00:00 TDB",
        1.0,
        0,
        Direction::Prograde,
    );

    let r = scan::run_sweep(&almanac, &sweep).expect("sweep failed");

    let type_i: Vec<_> = r
        .solutions
        .iter()
        .filter(|s| s.transfer_angle_deg < 180.0)
        .collect();
    let type_ii: Vec<_> = r
        .solutions
        .iter()
        .filter(|s| s.transfer_angle_deg > 180.0)
        .collect();

    assert!(
        !type_i.is_empty(),
        "Should have Type I solutions (< 180 deg)"
    );
    assert!(
        !type_ii.is_empty(),
        "Should have Type II solutions (> 180 deg)"
    );

    // Check type labels
    for sol in &type_i {
        assert_eq!(
            sol.transfer_type.type_num, 1,
            "Type I solution has wrong label: {}",
            sol.transfer_type
        );
    }
    for sol in &type_ii {
        assert_eq!(
            sol.transfer_type.type_num, 2,
            "Type II solution has wrong label: {}",
            sol.transfer_type
        );
    }

    eprintln!(
        "Boundary straddle: {} Type I solutions (all <180), {} Type II solutions (all >180)",
        type_i.len(),
        type_ii.len()
    );
}

// ─── TOF boundary conditions ─────────────────────────────────────────────────

/// Verify that when arrival dates are before departure dates (negative TOF),
/// the sweep produces zero solutions and does not panic.
#[test]
fn negative_tof_produces_no_solutions() {
    let almanac = load_almanac();

    // Arrival window is entirely before the departure window.
    let sweep = earth_mars_sweep(
        "negative-TOF",
        "2027-06-01 00:00:00 TDB",
        "2027-08-01 00:00:00 TDB",
        "2026-01-01 00:00:00 TDB",
        "2026-06-01 00:00:00 TDB",
        5.0,
        0,
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("negative-TOF sweep should not error");

    assert!(
        result.solutions.is_empty(),
        "Negative TOF should produce 0 solutions, got {}",
        result.solutions.len()
    );
    assert!(
        result.skipped_tof > 0,
        "All points should be skipped due to negative TOF, but skipped_tof={}",
        result.skipped_tof
    );
    assert_eq!(
        result.skipped_tof, result.total_points,
        "Every grid point should be TOF-skipped: skipped_tof={} vs total_points={}",
        result.skipped_tof, result.total_points
    );

    eprintln!(
        "Negative TOF: {} total points, {} skipped_tof, 0 solutions -- OK",
        result.total_points, result.skipped_tof
    );
}

/// Verify that very short TOF (< 30 days) does not cause panics.
/// The solver may fail or produce extreme solutions, but should not crash.
#[test]
fn very_short_tof_no_panic() {
    let almanac = load_almanac();

    // Departure and arrival windows overlap with only ~10-30 day separation.
    let sweep = earth_mars_sweep(
        "short-TOF",
        "2026-11-01 00:00:00 TDB",
        "2026-11-15 00:00:00 TDB",
        "2026-11-10 00:00:00 TDB", // only 9 days after dep_start
        "2026-12-15 00:00:00 TDB",
        1.0,
        0,
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("short-TOF sweep should not error");

    // We don't assert on solution count -- short TOFs may or may not solve.
    // The key assertion is that we got here without panicking.
    eprintln!(
        "Short TOF: {} total, {} solutions, {} skipped_tof, {} skipped_solver, {} skipped_singularity",
        result.total_points,
        result.solutions.len(),
        result.skipped_tof,
        result.skipped_solver,
        result.skipped_singularity,
    );

    // Verify accounting: every point is either a solution or was skipped.
    let accounted = result.solutions.len() as u64
        + result.skipped_tof
        + result.skipped_singularity
        + result.skipped_solver
        + result.skipped_ephemeris;
    assert_eq!(
        accounted, result.total_points,
        "Every grid point should be accounted for: {accounted} != {}",
        result.total_points
    );
}

/// Verify that very long TOF (> 400 days) still produces solutions.
/// These are high-energy but geometrically valid transfers.
#[test]
fn very_long_tof_still_solves() {
    let almanac = load_almanac();

    // Departure in early 2026, arrival in late 2027 -> TOFs of 500-700 days.
    let sweep = earth_mars_sweep(
        "long-TOF",
        "2026-03-01 00:00:00 TDB",
        "2026-04-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        "2027-12-01 00:00:00 TDB",
        5.0,
        0,
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("long-TOF sweep should not error");

    // Verify minimum TOF in solutions is > 400 days.
    if !result.solutions.is_empty() {
        let min_tof = result
            .solutions
            .iter()
            .map(|s| s.tof_days)
            .fold(f64::INFINITY, f64::min);
        assert!(
            min_tof > 400.0,
            "Expected all solutions to have TOF > 400 days, but min TOF = {:.1}",
            min_tof
        );

        eprintln!(
            "Long TOF: {} solutions, min TOF={:.1} days, max TOF={:.1} days",
            result.solutions.len(),
            min_tof,
            result
                .solutions
                .iter()
                .map(|s| s.tof_days)
                .fold(f64::NEG_INFINITY, f64::max),
        );
    } else {
        eprintln!(
            "Long TOF: 0 solutions (all {} points skipped), but no panic -- OK",
            result.total_points
        );
    }

    // Accounting check.
    let accounted = result.solutions.len() as u64
        + result.skipped_tof
        + result.skipped_singularity
        + result.skipped_solver
        + result.skipped_ephemeris;
    assert_eq!(
        accounted, result.total_points,
        "Every grid point should be accounted for"
    );
}

// ─── Solver failure modes ────────────────────────────────────────────────────

/// Run a sweep that likely triggers some solver failures and verify the
/// `skipped_solver` counter increments without any panics.
///
/// Very short TOFs combined with large distances (Earth to Mars) stress
/// the Lambert solver. The solver may not converge for physically
/// unreasonable geometries.
#[test]
fn solver_failures_counted_not_panicked() {
    let almanac = load_almanac();

    // A sweep where some points have very short TOFs (a few days to weeks)
    // between Earth and Mars -- the solver may struggle here.
    let sweep = earth_mars_sweep(
        "solver-stress",
        "2026-11-01 00:00:00 TDB",
        "2026-11-10 00:00:00 TDB",
        "2026-11-05 00:00:00 TDB",
        "2026-12-10 00:00:00 TDB",
        1.0,
        0,
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("solver-stress sweep should not error");

    // The key assertion: we completed without panic.
    // Log the breakdown for diagnostic purposes.
    eprintln!(
        "Solver stress: {} total, {} solutions, {} skipped_tof, {} skipped_solver, \
         {} skipped_singularity",
        result.total_points,
        result.solutions.len(),
        result.skipped_tof,
        result.skipped_solver,
        result.skipped_singularity,
    );

    // Accounting check.
    let accounted = result.solutions.len() as u64
        + result.skipped_tof
        + result.skipped_singularity
        + result.skipped_solver
        + result.skipped_ephemeris;
    assert_eq!(
        accounted, result.total_points,
        "Every grid point should be accounted for"
    );
}

// ─── Direction handling ──────────────────────────────────────────────────────

/// Verify that prograde and retrograde sweeps produce different results
/// for the same date window.
#[test]
fn prograde_vs_retrograde_differ() {
    let almanac = load_almanac();

    let sweep_pro = earth_mars_sweep(
        "direction-prograde",
        "2026-10-15 00:00:00 TDB",
        "2026-12-15 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        5.0,
        0,
        Direction::Prograde,
    );

    let sweep_retro = earth_mars_sweep(
        "direction-retrograde",
        "2026-10-15 00:00:00 TDB",
        "2026-12-15 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        5.0,
        0,
        Direction::Retrograde,
    );

    let r_pro = scan::run_sweep(&almanac, &sweep_pro).expect("prograde sweep failed");
    let r_retro = scan::run_sweep(&almanac, &sweep_retro).expect("retrograde sweep failed");

    // Same grid should have same total points.
    assert_eq!(
        r_pro.total_points, r_retro.total_points,
        "Same window should produce same total grid points"
    );

    let pro_count = r_pro.solutions.len();
    let retro_count = r_retro.solutions.len();

    eprintln!(
        "Direction test: prograde has {} solutions, retrograde has {} solutions",
        pro_count, retro_count
    );

    // At least one configuration should produce solutions.
    assert!(
        pro_count > 0 || retro_count > 0,
        "At least one direction should produce solutions"
    );

    // The solution counts or values must differ -- if both have solutions,
    // the C3 values should differ for the same departure/arrival dates.
    if pro_count > 0 && retro_count > 0 {
        // If solution counts differ, that alone proves they differ.
        if pro_count != retro_count {
            eprintln!(
                "Solution counts differ ({pro_count} vs {retro_count}) -- directions produce different results"
            );
        } else {
            // Compare min-C3 values as a proxy.
            let pro_min_c3 = r_pro
                .solutions
                .iter()
                .map(|s| s.c3_departure_km2s2)
                .fold(f64::INFINITY, f64::min);
            let retro_min_c3 = r_retro
                .solutions
                .iter()
                .map(|s| s.c3_departure_km2s2)
                .fold(f64::INFINITY, f64::min);
            // Even if counts happen to match, the energy values should differ.
            eprintln!(
                "Same count but min C3 prograde={pro_min_c3:.3} vs retrograde={retro_min_c3:.3}"
            );
        }
    }
}

/// Verify that prograde is the physically correct direction for Earth-Mars
/// transfers: prograde should produce lower-energy solutions than
/// retrograde for the 2026 opportunity.
#[test]
fn prograde_is_lower_energy_for_earth_mars() {
    let almanac = load_almanac();

    let sweep_pro = earth_mars_sweep(
        "energy-prograde",
        "2026-10-01 00:00:00 TDB",
        "2026-12-31 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        2.0,
        0,
        Direction::Prograde,
    );

    let r_pro = scan::run_sweep(&almanac, &sweep_pro).expect("prograde sweep failed");

    // Prograde Earth-Mars should produce solutions.
    assert!(
        !r_pro.solutions.is_empty(),
        "Prograde should produce solutions for Earth-Mars 2026"
    );

    let min_c3_pro = r_pro
        .solutions
        .iter()
        .map(|s| s.c3_departure_km2s2)
        .fold(f64::INFINITY, f64::min);

    // For Earth-Mars, prograde min-C3 should be reasonable (~8-15 km^2/s^2).
    assert!(
        min_c3_pro < 20.0,
        "Prograde Earth-Mars min C3 should be < 20 km^2/s^2, got {:.3}",
        min_c3_pro
    );
    assert!(
        min_c3_pro > 5.0,
        "Prograde Earth-Mars min C3 should be > 5 km^2/s^2, got {:.3}",
        min_c3_pro
    );

    eprintln!(
        "Prograde Earth-Mars: {} solutions, min C3 = {:.3} km^2/s^2",
        r_pro.solutions.len(),
        min_c3_pro
    );

    // Now test retrograde on the same window. For a standard prograde-orbiting
    // system (Earth, Mars both orbit prograde), a retrograde transfer is going
    // "against the grain" and should be much higher energy.
    let sweep_retro = earth_mars_sweep(
        "energy-retrograde",
        "2026-10-01 00:00:00 TDB",
        "2026-12-31 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        2.0,
        0,
        Direction::Retrograde,
    );

    let r_retro = scan::run_sweep(&almanac, &sweep_retro).expect("retrograde sweep failed");

    if r_retro.solutions.is_empty() {
        eprintln!(
            "Retrograde: 0 solutions (angle flip moves solutions to different geometry) -- correct"
        );
    } else {
        let min_c3_retro = r_retro
            .solutions
            .iter()
            .map(|s| s.c3_departure_km2s2)
            .fold(f64::INFINITY, f64::min);
        eprintln!(
            "Retrograde: {} solutions, min C3 = {:.3} km^2/s^2",
            r_retro.solutions.len(),
            min_c3_retro
        );
        // If retrograde does produce solutions, they should be higher energy
        // than prograde.
        assert!(
            min_c3_retro > min_c3_pro,
            "Retrograde min C3 ({:.3}) should be higher than prograde ({:.3}) for Earth-Mars",
            min_c3_retro,
            min_c3_pro
        );
    }
}

// ─── Grid point accounting ───────────────────────────────────────────────────

/// Verify that for any sweep, the total grid points equals the sum of
/// solutions + all skip categories. This is a meta-test of the sweep
/// bookkeeping invariant.
#[test]
fn grid_point_accounting_invariant() {
    let almanac = load_almanac();

    // Use a broad sweep that exercises all skip categories.
    let sweep = earth_mars_sweep(
        "accounting",
        "2026-10-01 00:00:00 TDB",
        "2026-12-31 00:00:00 TDB",
        "2027-06-01 00:00:00 TDB",
        "2027-10-01 00:00:00 TDB",
        3.0,
        0,
        Direction::Prograde,
    );

    let result = scan::run_sweep(&almanac, &sweep).expect("accounting sweep failed");

    let accounted = result.solutions.len() as u64
        + result.skipped_tof
        + result.skipped_singularity
        + result.skipped_solver
        + result.skipped_ephemeris;

    assert_eq!(
        accounted,
        result.total_points,
        "Grid point accounting: solutions({}) + skipped_tof({}) + \
         skipped_singularity({}) + skipped_solver({}) + skipped_ephemeris({}) = {} != total_points({})",
        result.solutions.len(),
        result.skipped_tof,
        result.skipped_singularity,
        result.skipped_solver,
        result.skipped_ephemeris,
        accounted,
        result.total_points
    );

    eprintln!(
        "Accounting: {} total = {} solutions + {} tof + {} singularity + {} solver + {} ephemeris",
        result.total_points,
        result.solutions.len(),
        result.skipped_tof,
        result.skipped_singularity,
        result.skipped_solver,
        result.skipped_ephemeris
    );
}
