use serde::Serialize;
use std::io::Write;
use std::path::Path;

use crate::config::Config;
use crate::error::LambertoError;
use crate::scan::{SolutionRow, SweepResult};

/// Write CSV for each sweep and a YAML summary.
pub fn write_all(
    config: &Config,
    results: &[SweepResult],
    output_dir: &Path,
) -> Result<(), LambertoError> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).map_err(|e| LambertoError::Output(e.to_string()))?;

    let mut summaries = Vec::new();

    for (sweep, result) in config.sweeps.iter().zip(results.iter()) {
        // Write CSV
        let csv_path = output_dir.join(format!("{}.csv", sweep.name));
        let csv_path_str = csv_path
            .to_str()
            .ok_or_else(|| LambertoError::Output("invalid UTF-8 in output path".to_string()))?;
        write_csv(csv_path_str, &result.solutions)?;
        println!(
            "Wrote {} ({} rows)",
            csv_path.display(),
            result.solutions.len()
        );

        // Build summary entry
        let best_dep = result.best_departure_vinf(sweep.target_v_inf_departure);
        let best_arr = result.best_arrival_vinf(sweep.target_v_inf_arrival);

        summaries.push(SweepSummary {
            name: result.name.clone(),
            total_grid_points: result.total_points,
            valid_solutions: result.solutions.len() as u64,
            skipped_tof: result.skipped_tof,
            skipped_singularity: result.skipped_singularity,
            skipped_solver_failure: result.skipped_solver,
            skipped_ephemeris: result.skipped_ephemeris,
            best_departure_v_inf: best_dep.map(BestEntry::from_row),
            best_arrival_v_inf: best_arr.map(BestEntry::from_row),
        });
    }

    // Write YAML summary
    let summary_path = output_dir.join("summary.yaml");
    let summary_yaml =
        serde_yaml_ng::to_string(&summaries).map_err(|e| LambertoError::Output(e.to_string()))?;
    let mut f =
        std::fs::File::create(&summary_path).map_err(|e| LambertoError::Output(e.to_string()))?;
    f.write_all(summary_yaml.as_bytes())
        .map_err(|e| LambertoError::Output(e.to_string()))?;
    println!("Wrote {}", summary_path.display());

    Ok(())
}

fn write_csv(path: &str, solutions: &[SolutionRow]) -> Result<(), LambertoError> {
    let mut wtr = csv::Writer::from_path(path).map_err(|e| LambertoError::Output(e.to_string()))?;
    wtr.write_record([
        "departure_date",
        "arrival_date",
        "tof_days",
        "transfer_angle_deg",
        "type",
        "c3_departure_km2s2",
        "v_inf_departure_kms",
        "v_inf_arrival_kms",
    ])
    .map_err(|e| LambertoError::Output(e.to_string()))?;
    for row in solutions {
        wtr.write_record([
            row.departure_date.to_string(),
            row.arrival_date.to_string(),
            format!("{:.4}", row.tof_days),
            format!("{:.4}", row.transfer_angle_deg),
            row.transfer_type.to_string(),
            format!("{:.6}", row.c3_departure_km2s2),
            format!("{:.6}", row.v_inf_departure_kms),
            format!("{:.6}", row.v_inf_arrival_kms),
        ])
        .map_err(|e| LambertoError::Output(e.to_string()))?;
    }
    wtr.flush()
        .map_err(|e| LambertoError::Output(e.to_string()))?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct SweepSummary {
    name: String,
    total_grid_points: u64,
    valid_solutions: u64,
    skipped_tof: u64,
    skipped_singularity: u64,
    skipped_solver_failure: u64,
    skipped_ephemeris: u64,
    best_departure_v_inf: Option<BestEntry>,
    best_arrival_v_inf: Option<BestEntry>,
}

// NOTE: BestEntry copies scalar fields from SolutionRow and formats Epoch
// values to strings.  A reference-based approach (BestEntry<'a>) would avoid
// the copy but would require lifetime annotations to propagate through
// SweepSummary, the serialization call, and the write_all API — not worth
// the complexity for a small summary struct.  If SolutionRow gains large or
// heap-allocated fields in the future, revisit this decision.
#[derive(Debug, Serialize)]
struct BestEntry {
    departure_date: String,
    arrival_date: String,
    tof_days: f64,
    transfer_angle_deg: f64,
    transfer_type: String,
    c3_departure_km2s2: f64,
    v_inf_departure_kms: f64,
    v_inf_arrival_kms: f64,
}

impl BestEntry {
    fn from_row(row: &SolutionRow) -> Self {
        BestEntry {
            departure_date: row.departure_date.to_string(),
            arrival_date: row.arrival_date.to_string(),
            tof_days: row.tof_days,
            transfer_angle_deg: row.transfer_angle_deg,
            transfer_type: row.transfer_type.to_string(),
            c3_departure_km2s2: row.c3_departure_km2s2,
            v_inf_departure_kms: row.v_inf_departure_kms,
            v_inf_arrival_kms: row.v_inf_arrival_kms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::{Direction, TransferType};
    use anise::prelude::Epoch;

    /// Helper: build a SolutionRow with known values for deterministic testing.
    /// The `ttype` string is parsed into a TransferType: roman numeral gives
    /// the type_num, and a "-R" suffix selects Retrograde direction.
    fn make_row(
        dep: &str,
        arr: &str,
        tof: f64,
        angle: f64,
        ttype: &str,
        c3: f64,
        v_dep: f64,
        v_arr: f64,
    ) -> SolutionRow {
        let (direction, roman) = if let Some(r) = ttype.strip_suffix("-R") {
            (Direction::Retrograde, r)
        } else {
            (Direction::Prograde, ttype)
        };
        let type_num = match roman {
            "I" => 1,
            "II" => 2,
            "III" => 3,
            "IV" => 4,
            "V" => 5,
            "VI" => 6,
            other => other.parse::<u32>().unwrap_or(1),
        };
        SolutionRow {
            departure_date: dep.parse::<Epoch>().unwrap(),
            arrival_date: arr.parse::<Epoch>().unwrap(),
            tof_days: tof,
            transfer_angle_deg: angle,
            transfer_type: TransferType {
                type_num,
                direction,
            },
            c3_departure_km2s2: c3,
            v_inf_departure_kms: v_dep,
            v_inf_arrival_kms: v_arr,
        }
    }

    #[test]
    fn csv_header_matches_expected_fields() {
        let dir = std::env::temp_dir().join("lamberto_test_csv_header");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("header_test.csv");
        let path_str = path.to_str().unwrap();

        write_csv(path_str, &[]).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let first_line = contents.lines().next().unwrap();
        assert_eq!(
            first_line,
            "departure_date,arrival_date,tof_days,transfer_angle_deg,type,\
             c3_departure_km2s2,v_inf_departure_kms,v_inf_arrival_kms"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn csv_field_order_and_delimiter() {
        let dir = std::env::temp_dir().join("lamberto_test_csv_fields");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fields_test.csv");
        let path_str = path.to_str().unwrap();

        let row = make_row(
            "2026-06-01 00:00:00 TDB",
            "2027-01-15 00:00:00 TDB",
            228.0,
            145.3456,
            "I",
            10.5,
            3.240_000,
            2.850_000,
        );
        write_csv(path_str, &[row]).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "expect header + 1 data row");

        let fields: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(fields.len(), 8, "each row must have 8 fields");

        // Field order: departure, arrival, tof, angle, type, c3, v_dep, v_arr
        assert_eq!(fields[4], "I", "5th field is transfer type");
        // Verify comma delimiter (no tabs, no pipes)
        assert!(!lines[1].contains('\t'), "no tab characters");
        assert!(!lines[1].contains('|'), "no pipe characters");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn csv_numeric_precision_tof_and_angle() {
        let dir = std::env::temp_dir().join("lamberto_test_csv_precision_tof");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("precision_test.csv");
        let path_str = path.to_str().unwrap();

        let row = make_row(
            "2026-06-01 00:00:00 TDB",
            "2027-01-15 00:00:00 TDB",
            228.123_456_789,
            145.987_654_321,
            "II",
            10.123_456_789,
            3.141_592_653,
            2.718_281_828,
        );
        write_csv(path_str, &[row]).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let data_line = contents.lines().nth(1).unwrap();
        let fields: Vec<&str> = data_line.split(',').collect();

        // tof_days: 4 decimal places
        assert_eq!(
            fields[2], "228.1235",
            "tof_days should have 4 decimal places"
        );
        // transfer_angle_deg: 4 decimal places
        assert_eq!(
            fields[3], "145.9877",
            "transfer_angle_deg should have 4 decimal places"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn csv_numeric_precision_c3_and_vinf() {
        let dir = std::env::temp_dir().join("lamberto_test_csv_precision_c3");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("precision_c3_test.csv");
        let path_str = path.to_str().unwrap();

        let row = make_row(
            "2026-06-01 00:00:00 TDB",
            "2027-01-15 00:00:00 TDB",
            200.0,
            90.0,
            "I",
            10.123_456_789,
            3.141_592_653,
            2.718_281_828,
        );
        write_csv(path_str, &[row]).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let data_line = contents.lines().nth(1).unwrap();
        let fields: Vec<&str> = data_line.split(',').collect();

        // c3, v_inf_departure, v_inf_arrival: 6 decimal places
        assert_eq!(fields[5], "10.123457", "c3 should have 6 decimal places");
        assert_eq!(
            fields[6], "3.141593",
            "v_inf_departure should have 6 decimal places"
        );
        assert_eq!(
            fields[7], "2.718282",
            "v_inf_arrival should have 6 decimal places"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn csv_multiple_rows_all_written() {
        let dir = std::env::temp_dir().join("lamberto_test_csv_multi");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("multi_test.csv");
        let path_str = path.to_str().unwrap();

        let rows = vec![
            make_row(
                "2026-06-01 00:00:00 TDB",
                "2027-01-15 00:00:00 TDB",
                228.0,
                90.0,
                "I",
                10.0,
                3.0,
                2.0,
            ),
            make_row(
                "2026-07-01 00:00:00 TDB",
                "2027-02-15 00:00:00 TDB",
                229.0,
                180.0,
                "II",
                11.0,
                3.5,
                2.5,
            ),
            make_row(
                "2026-08-01 00:00:00 TDB",
                "2027-03-15 00:00:00 TDB",
                226.0,
                270.0,
                "II",
                12.0,
                4.0,
                3.0,
            ),
        ];
        write_csv(path_str, &rows).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 4, "header + 3 data rows");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn yaml_summary_structure() {
        let summary = SweepSummary {
            name: "Earth-Mars Type I".to_string(),
            total_grid_points: 1000,
            valid_solutions: 850,
            skipped_tof: 50,
            skipped_singularity: 10,
            skipped_solver_failure: 90,
            skipped_ephemeris: 0,
            best_departure_v_inf: None,
            best_arrival_v_inf: None,
        };

        let yaml = serde_yaml_ng::to_string(&[&summary]).unwrap();

        assert!(yaml.contains("name:"), "YAML must contain 'name' field");
        assert!(
            yaml.contains("total_grid_points:"),
            "YAML must contain 'total_grid_points'"
        );
        assert!(
            yaml.contains("valid_solutions:"),
            "YAML must contain 'valid_solutions'"
        );
        assert!(
            yaml.contains("skipped_tof:"),
            "YAML must contain 'skipped_tof'"
        );
        assert!(
            yaml.contains("skipped_singularity:"),
            "YAML must contain 'skipped_singularity'"
        );
        assert!(
            yaml.contains("skipped_solver_failure:"),
            "YAML must contain 'skipped_solver_failure'"
        );
        assert!(
            yaml.contains("skipped_ephemeris:"),
            "YAML must contain 'skipped_ephemeris'"
        );
        assert!(
            yaml.contains("best_departure_v_inf:"),
            "YAML must contain 'best_departure_v_inf'"
        );
        assert!(
            yaml.contains("best_arrival_v_inf:"),
            "YAML must contain 'best_arrival_v_inf'"
        );

        assert!(yaml.contains("Earth-Mars Type I"));
        assert!(yaml.contains("1000"));
        assert!(yaml.contains("850"));
    }

    #[test]
    fn yaml_summary_with_best_entries() {
        let best = BestEntry {
            departure_date: "2026-06-15".to_string(),
            arrival_date: "2027-01-20".to_string(),
            tof_days: 219.0,
            transfer_angle_deg: 150.1234,
            transfer_type: "I".to_string(),
            c3_departure_km2s2: 8.5,
            v_inf_departure_kms: 2.915,
            v_inf_arrival_kms: 2.345,
        };

        let summary = SweepSummary {
            name: "Test".to_string(),
            total_grid_points: 100,
            valid_solutions: 80,
            skipped_tof: 5,
            skipped_singularity: 3,
            skipped_solver_failure: 12,
            skipped_ephemeris: 0,
            best_departure_v_inf: Some(best),
            best_arrival_v_inf: None,
        };

        let yaml = serde_yaml_ng::to_string(&[&summary]).unwrap();

        assert!(
            yaml.contains("departure_date:"),
            "best entry must include departure_date"
        );
        assert!(
            yaml.contains("tof_days:"),
            "best entry must include tof_days"
        );
        assert!(
            yaml.contains("transfer_type:"),
            "best entry must include transfer_type"
        );
        assert!(
            yaml.contains("2026-06-15"),
            "departure_date value must appear"
        );
    }

    #[test]
    fn yaml_summary_numeric_precision() {
        let best = BestEntry {
            departure_date: "2026-06-15".to_string(),
            arrival_date: "2027-01-20".to_string(),
            tof_days: 219.123_456_789,
            transfer_angle_deg: 150.987_654_321,
            transfer_type: "I".to_string(),
            c3_departure_km2s2: 8.123_456_789,
            v_inf_departure_kms: 2.915_432_1,
            v_inf_arrival_kms: 2.345_678_9,
        };

        let summary = SweepSummary {
            name: "Precision".to_string(),
            total_grid_points: 1,
            valid_solutions: 1,
            skipped_tof: 0,
            skipped_singularity: 0,
            skipped_solver_failure: 0,
            skipped_ephemeris: 0,
            best_departure_v_inf: Some(best),
            best_arrival_v_inf: None,
        };

        let yaml = serde_yaml_ng::to_string(&[&summary]).unwrap();

        let parsed: Vec<serde_yaml_ng::Value> = serde_yaml_ng::from_str(&yaml).unwrap();
        let entry = &parsed[0]["best_departure_v_inf"];
        let tof = entry["tof_days"].as_f64().unwrap();
        assert!(
            (tof - 219.123_456_789).abs() < 1e-9,
            "YAML must preserve full f64 precision for tof_days"
        );
        let c3 = entry["c3_departure_km2s2"].as_f64().unwrap();
        assert!(
            (c3 - 8.123_456_789).abs() < 1e-9,
            "YAML must preserve full f64 precision for c3"
        );
    }

    #[test]
    fn best_entry_from_row_round_trip() {
        let row = make_row(
            "2026-06-01 00:00:00 TDB",
            "2027-01-15 00:00:00 TDB",
            228.5,
            145.0,
            "I-R",
            10.5,
            3.24,
            2.85,
        );
        let entry = BestEntry::from_row(&row);

        assert_eq!(entry.tof_days, row.tof_days);
        assert_eq!(entry.transfer_angle_deg, row.transfer_angle_deg);
        assert_eq!(entry.transfer_type, row.transfer_type.to_string());
        assert_eq!(entry.c3_departure_km2s2, row.c3_departure_km2s2);
        assert_eq!(entry.v_inf_departure_kms, row.v_inf_departure_kms);
        assert_eq!(entry.v_inf_arrival_kms, row.v_inf_arrival_kms);
    }
}
