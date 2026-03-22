use anise::constants::frames::SUN_J2000;
use anise::prelude::{Almanac, Epoch};

use crate::bodies::resolve_frame;
use crate::config::Sweep;
use crate::error::LambertoError;
use crate::transfer;

/// Sun's GM from DE440 (km³/s²). Source: `assets/gm_de440.tpc`.
/// Hardcoded by design — ensures consistency regardless of which SPK file
/// is loaded, and avoids a runtime query for a value that doesn't change.
const MU_SUN: f64 = 1.327_124_400_412_794_2e11;

/// One valid Lambert solution.
#[derive(Debug, Clone)]
pub struct SolutionRow {
    /// Departure epoch.
    pub departure_date: Epoch,
    /// Arrival epoch.
    pub arrival_date: Epoch,
    /// Time of flight in days.
    pub tof_days: f64,
    /// Transfer angle in degrees (includes full revolutions).
    pub transfer_angle_deg: f64,
    /// Transfer type classification (e.g. Type I, II-R).
    pub transfer_type: transfer::TransferType,
    /// Departure C3 energy (km^2/s^2).
    pub c3_departure_km2s2: f64,
    /// Departure hyperbolic excess velocity (km/s).
    pub v_inf_departure_kms: f64,
    /// Arrival hyperbolic excess velocity (km/s).
    pub v_inf_arrival_kms: f64,
}

/// A diagnostic event recorded during a sweep (no I/O in the computation).
#[derive(Debug, Clone)]
pub enum SweepDiagnostic {
    /// Ephemeris query failed for a body at the given epoch.
    EphemerisError { epoch: Epoch, body: &'static str, error: String },
    /// Departure/arrival positions are nearly collinear (Lambert singularity).
    NearSingularity { dep: Epoch, arr: Epoch, angle_deg: f64 },
    /// Lambert solver returned an error for this grid point.
    SolverFailure { dep: Epoch, arr: Epoch, error: String },
}

/// Statistics and results from one sweep.
#[derive(Debug)]
pub struct SweepResult {
    /// Sweep name from configuration.
    pub name: String,
    /// Valid Lambert solutions found.
    pub solutions: Vec<SolutionRow>,
    /// Diagnostic events (errors, warnings) encountered during the sweep.
    pub diagnostics: Vec<SweepDiagnostic>,
    /// Total grid points evaluated.
    pub total_points: u64,
    /// Points skipped due to non-positive time of flight.
    pub skipped_tof: u64,
    /// Points skipped due to near-collinear geometry.
    pub skipped_singularity: u64,
    /// Points skipped due to solver failure.
    pub skipped_solver: u64,
    /// Points skipped due to ephemeris query failure.
    pub skipped_ephemeris: u64,
}

impl SweepResult {
    /// Find solution closest to target departure v-infinity.
    pub fn best_departure_vinf(&self, target: f64) -> Option<&SolutionRow> {
        self.solutions
            .iter()
            .min_by(|a, b| {
                let da = (a.v_inf_departure_kms - target).abs();
                let db = (b.v_inf_departure_kms - target).abs();
                da.total_cmp(&db)
            })
    }

    /// Find solution closest to target arrival v-infinity.
    pub fn best_arrival_vinf(&self, target: f64) -> Option<&SolutionRow> {
        self.solutions
            .iter()
            .min_by(|a, b| {
                let da = (a.v_inf_arrival_kms - target).abs();
                let db = (b.v_inf_arrival_kms - target).abs();
                da.total_cmp(&db)
            })
    }

    /// Print diagnostics and summary to stderr/stdout.
    /// Separates I/O from the computation in `run_sweep`.
    pub fn print_report(&self) {
        for diag in &self.diagnostics {
            match diag {
                SweepDiagnostic::EphemerisError { epoch, body, error } => {
                    eprintln!("Ephemeris error at {body}={epoch}: {error}");
                }
                SweepDiagnostic::NearSingularity { dep, arr, angle_deg } => {
                    eprintln!("Warning: near-singularity at dep={dep}, arr={arr}");
                    eprintln!("  \u{03b8}={angle_deg:.2}\u{00b0}");
                }
                SweepDiagnostic::SolverFailure { dep, arr, error } => {
                    eprintln!("Lambert solver failed at dep={dep}, arr={arr}: {error}");
                }
            }
        }
        println!(
            "Sweep \"{}\": {} solutions from {} grid points",
            self.name,
            self.solutions.len(),
            self.total_points,
        );
    }
}

fn vec3_magnitude(v: &[f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn vec3_sub(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Iterator over epochs from `start` to `end` (inclusive) with a fixed step.
struct EpochRange {
    current: Epoch,
    end: Epoch,
    step: anise::prelude::Duration,
}

impl EpochRange {
    fn new(start: Epoch, end: Epoch, step_seconds: f64) -> Self {
        Self {
            current: start,
            end,
            step: anise::prelude::Duration::from_seconds(step_seconds),
        }
    }
}

impl Iterator for EpochRange {
    type Item = Epoch;

    fn next(&mut self) -> Option<Epoch> {
        if self.current > self.end {
            return None;
        }
        let epoch = self.current;
        self.current += self.step;
        Some(epoch)
    }
}

/// Execute one sweep: iterate departure/arrival date grid, solve Lambert.
///
/// Pure computation — all diagnostics are collected in `SweepResult::diagnostics`
/// rather than printed. Call `SweepResult::print_report()` for I/O.
pub fn run_sweep(
    almanac: &Almanac,
    sweep: &Sweep,
) -> Result<SweepResult, LambertoError> {
    let dep_frame = resolve_frame(&sweep.departure_body)
        .map_err(|e| LambertoError::Ephemeris(e))?;
    let arr_frame = resolve_frame(&sweep.arrival_body)
        .map_err(|e| LambertoError::Ephemeris(e))?;

    let dep_start: Epoch = sweep.departure_start.parse()
        .map_err(|e| LambertoError::Config(format!("{e}")))?;
    let dep_end: Epoch = sweep.departure_end.parse()
        .map_err(|e| LambertoError::Config(format!("{e}")))?;
    let arr_start: Epoch = sweep.arrival_start.parse()
        .map_err(|e| LambertoError::Config(format!("{e}")))?;
    let arr_end: Epoch = sweep.arrival_end.parse()
        .map_err(|e| LambertoError::Config(format!("{e}")))?;

    if sweep.departure_step_days <= 0.0 {
        return Err(LambertoError::Config(format!(
            "departure_step_days must be > 0, got {}",
            sweep.departure_step_days
        )));
    }
    if sweep.arrival_step_days <= 0.0 {
        return Err(LambertoError::Config(format!(
            "arrival_step_days must be > 0, got {}",
            sweep.arrival_step_days
        )));
    }

    let dep_step = sweep.departure_step_days * 86400.0; // seconds per day
    let arr_step = sweep.arrival_step_days * 86400.0; // seconds per day

    let direction = sweep.direction;

    let mut result = SweepResult {
        name: sweep.name.clone(),
        solutions: Vec::new(),
        diagnostics: Vec::new(),
        total_points: 0,
        skipped_tof: 0,
        skipped_singularity: 0,
        skipped_solver: 0,
        skipped_ephemeris: 0,
    };

    for dep_epoch in EpochRange::new(dep_start, dep_end, dep_step) {
        for arr_epoch in EpochRange::new(arr_start, arr_end, arr_step) {
            result.total_points += 1;

            // TOF check
            let tof_seconds = (arr_epoch - dep_epoch).to_seconds();
            if tof_seconds <= 0.0 {
                result.skipped_tof += 1;
                continue;
            }
            let tof_days = tof_seconds / 86400.0;

            // Query planet states relative to the Sun
            let dep_state = match almanac.translate_geometric(dep_frame, SUN_J2000, dep_epoch) {
                Ok(s) => s,
                Err(e) => {
                    result.diagnostics.push(SweepDiagnostic::EphemerisError {
                        epoch: dep_epoch,
                        body: "dep",
                        error: format!("{e:?}"),
                    });
                    result.skipped_ephemeris += 1;
                    continue;
                }
            };
            let arr_state = match almanac.translate_geometric(arr_frame, SUN_J2000, arr_epoch) {
                Ok(s) => s,
                Err(e) => {
                    result.diagnostics.push(SweepDiagnostic::EphemerisError {
                        epoch: arr_epoch,
                        body: "arr",
                        error: format!("{e:?}"),
                    });
                    result.skipped_ephemeris += 1;
                    continue;
                }
            };

            let r1 = [dep_state.radius_km.x, dep_state.radius_km.y, dep_state.radius_km.z];
            let r2 = [arr_state.radius_km.x, arr_state.radius_km.y, arr_state.radius_km.z];
            let v_planet1 = [
                dep_state.velocity_km_s.x,
                dep_state.velocity_km_s.y,
                dep_state.velocity_km_s.z,
            ];
            let v_planet2 = [
                arr_state.velocity_km_s.x,
                arr_state.velocity_km_s.y,
                arr_state.velocity_km_s.z,
            ];

            // Singularity check
            if transfer::is_near_singularity(&r1, &r2) {
                let theta = transfer::transfer_angle(&r1, &r2, direction);
                result.diagnostics.push(SweepDiagnostic::NearSingularity {
                    dep: dep_epoch,
                    arr: arr_epoch,
                    angle_deg: theta.to_degrees(),
                });
                result.skipped_singularity += 1;
                continue;
            }

            // Transfer angle
            let theta = transfer::transfer_angle(&r1, &r2, direction);

            // Solver direction: short-way when θ < π, long-way when θ > π
            let lambert_dir = if theta <= std::f64::consts::PI {
                gooding_lambert::Direction::Prograde
            } else {
                gooding_lambert::Direction::Retrograde
            };

            // Use nrev directly from config — no type-to-nrev mapping
            match gooding_lambert::lambert(MU_SUN, r1, r2, tof_seconds, sweep.nrev, lambert_dir, gooding_lambert::MultiRevPeriod::LongPeriod) {
                Ok(sol) => {
                    let v_inf_dep = vec3_sub(&sol.v1, &v_planet1);
                    let v_inf_arr = vec3_sub(&sol.v2, &v_planet2);
                    let v_inf_dep_mag = vec3_magnitude(&v_inf_dep);
                    let v_inf_arr_mag = vec3_magnitude(&v_inf_arr);
                    let c3 = v_inf_dep_mag * v_inf_dep_mag;

                    // Derive type classification for output
                    let full_theta = 2.0 * std::f64::consts::PI * sweep.nrev as f64 + theta;
                    let t_type_num = transfer::classify_type(full_theta);

                    result.solutions.push(SolutionRow {
                        departure_date: dep_epoch,
                        arrival_date: arr_epoch,
                        tof_days,
                        transfer_angle_deg: full_theta.to_degrees(),
                        transfer_type: transfer::TransferType {
                            type_num: t_type_num,
                            direction,
                        },
                        c3_departure_km2s2: c3,
                        v_inf_departure_kms: v_inf_dep_mag,
                        v_inf_arrival_kms: v_inf_arr_mag,
                    });
                }
                Err(e) => {
                    result.diagnostics.push(SweepDiagnostic::SolverFailure {
                        dep: dep_epoch,
                        arr: arr_epoch,
                        error: format!("{e:?}"),
                    });
                    result.skipped_solver += 1;
                }
            }
        }
    }

    Ok(result)
}
