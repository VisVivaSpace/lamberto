use serde::Deserialize;
use std::borrow::Cow;
use std::f64::consts::PI;
use std::fmt;

/// Transfer orbit direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Prograde (short-way) transfer.
    Prograde,
    /// Retrograde (long-way) transfer.
    Retrograde,
}

/// Compute transfer angle between two position vectors.
/// Returns angle in radians in (0, 2π).
/// For prograde: angle < π when cross product z-component ≥ 0.
/// For retrograde: opposite sense.
///
/// Note: inline vector magnitude/cross-product math is duplicated from
/// `scan::vec3_magnitude` / `scan::vec3_sub`. Kept separate to avoid
/// coupling transfer geometry to the scan module.
pub fn transfer_angle(r1: &[f64; 3], r2: &[f64; 3], direction: Direction) -> f64 {
    let dot = r1[0] * r2[0] + r1[1] * r2[1] + r1[2] * r2[2];
    let r1_mag = (r1[0] * r1[0] + r1[1] * r1[1] + r1[2] * r1[2]).sqrt();
    let r2_mag = (r2[0] * r2[0] + r2[1] * r2[1] + r2[2] * r2[2]).sqrt();
    let cos_theta = (dot / (r1_mag * r2_mag)).clamp(-1.0, 1.0);

    // Cross product z-component determines which half-plane
    let cross_z = r1[0] * r2[1] - r1[1] * r2[0];

    let angle = cos_theta.acos(); // [0, π]
    match direction {
        Direction::Prograde => {
            if cross_z >= 0.0 {
                angle
            } else {
                2.0 * PI - angle
            }
        }
        Direction::Retrograde => {
            if cross_z >= 0.0 {
                2.0 * PI - angle
            } else {
                angle
            }
        }
    }
}

/// Classify transfer angle into type number.
/// Type I: (0°, 180°), Type II: (180°, 360°), etc.
pub fn classify_type(angle_rad: f64) -> u32 {
    let half_revs = angle_rad / PI;
    half_revs.ceil().max(1.0) as u32
}

/// Check if two position vectors are nearly collinear (singularity for Lambert).
/// Computes unit vectors, then checks if the norm of their cross product is < 1e-15.
pub fn is_near_singularity(r1: &[f64; 3], r2: &[f64; 3]) -> bool {
    let r1_mag = (r1[0] * r1[0] + r1[1] * r1[1] + r1[2] * r1[2]).sqrt();
    let r2_mag = (r2[0] * r2[0] + r2[1] * r2[1] + r2[2] * r2[2]).sqrt();

    // Degenerate zero-magnitude vectors: treat as singularity to avoid NaN
    // from the division below.
    if r1_mag < 1e-10 || r2_mag < 1e-10 {
        return true;
    }

    let r1_hat = [r1[0] / r1_mag, r1[1] / r1_mag, r1[2] / r1_mag];
    let r2_hat = [r2[0] / r2_mag, r2[1] / r2_mag, r2[2] / r2_mag];

    let cross = [
        r1_hat[1] * r2_hat[2] - r1_hat[2] * r2_hat[1],
        r1_hat[2] * r2_hat[0] - r1_hat[0] * r2_hat[2],
        r1_hat[0] * r2_hat[1] - r1_hat[1] * r2_hat[0],
    ];
    let cross_norm = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    // Near machine-precision threshold. Only filters cases that would
    // produce NaN or numerically meaningless results from the unit-vector
    // math above. The gooding-lambert solver has its own singularity guard
    // (at ~1e-10 relative to r1_mag*r2_mag) and will return an error for
    // near-singular cases it can't handle — those get counted as
    // skipped_solver, which is the honest classification.
    cross_norm < 1e-15
}

/// Transfer type classification (e.g. Type I prograde, Type II-R retrograde).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferType {
    /// Numeric type (1 = I, 2 = II, etc.) based on half-revolutions of transfer angle.
    pub type_num: u32,
    /// Prograde or retrograde direction.
    pub direction: Direction,
}

impl fmt::Display for TransferType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", type_label(self.type_num, self.direction))
    }
}

/// Build a human-readable type label from the numeric type and direction.
///
/// Type I (0-180 deg), II (180-360 deg), III (360-540 deg), IV (540-720 deg).
/// Retrograde transfers get a "-R" suffix: "I-R", "II-R", etc.
pub fn type_label(type_num: u32, direction: Direction) -> Cow<'static, str> {
    let roman = match type_num {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        n => return Cow::Owned(format!("{n}{}", if matches!(direction, Direction::Retrograde) { "-R" } else { "" })),
    };
    match direction {
        Direction::Prograde => Cow::Borrowed(roman),
        Direction::Retrograde => Cow::Owned(format!("{roman}-R")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_angle_90_deg_prograde() {
        let r1 = [1.0, 0.0, 0.0];
        let r2 = [0.0, 1.0, 0.0];
        let angle = transfer_angle(&r1, &r2, Direction::Prograde);
        assert!((angle - PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_transfer_angle_90_deg_retrograde() {
        let r1 = [1.0, 0.0, 0.0];
        let r2 = [0.0, 1.0, 0.0];
        let angle = transfer_angle(&r1, &r2, Direction::Retrograde);
        assert!((angle - 3.0 * PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_classify_type() {
        assert_eq!(classify_type(0.5), 1); // Type I
        assert_eq!(classify_type(PI - 0.1), 1); // Type I
        assert_eq!(classify_type(PI + 0.1), 2); // Type II
        assert_eq!(classify_type(2.0 * PI - 0.1), 2); // Type II
        assert_eq!(classify_type(2.0 * PI + 0.1), 3); // Type III
    }

    #[test]
    fn test_near_singularity() {
        // Exactly collinear (0°) — singularity (cross product is exactly zero)
        assert!(is_near_singularity(&[1.0, 0.0, 0.0], &[2.0, 0.0, 0.0]));
        // Exactly anti-parallel (180°) — singularity
        assert!(is_near_singularity(&[1.0, 0.0, 0.0], &[-1.0, 0.0, 0.0]));
        // Zero-magnitude vector — singularity (degenerate)
        assert!(is_near_singularity(&[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]));
        assert!(is_near_singularity(&[1.0, 0.0, 0.0], &[0.0, 0.0, 0.0]));
        // Not near singularity — 90 degrees
        assert!(!is_near_singularity(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]));
        // Not near singularity — very small angle (~5.7e-7 degrees)
        // The threshold is 1e-15, so angles as small as ~1e-8 radians pass through.
        // The solver decides what it can handle.
        assert!(!is_near_singularity(&[1.0, 0.0, 0.0], &[1.0, 1e-8, 0.0]));
        // Not near singularity — small angle (~0.006 degrees)
        assert!(!is_near_singularity(&[1.0, 0.0, 0.0], &[1.0, 1e-4, 0.0]));
    }

    #[test]
    fn singularity_threshold_is_near_machine_precision() {
        // The threshold (1e-15) should only catch cases that are
        // indistinguishable from collinear at double precision.
        // The solver has its own guards for "close but not exact."

        // Cross product norm of ~1e-16 (below threshold) — singularity
        assert!(is_near_singularity(&[1.0, 0.0, 0.0], &[1.0, 1e-16, 0.0]));

        // Cross product norm of ~1e-14 (above threshold) — not singularity
        assert!(!is_near_singularity(&[1.0, 0.0, 0.0], &[1.0, 1e-14, 0.0]));
    }

    #[test]
    fn test_type_label() {
        assert_eq!(type_label(1, Direction::Prograde), "I");
        assert_eq!(type_label(2, Direction::Prograde), "II");
        assert_eq!(type_label(1, Direction::Retrograde), "I-R");
        assert_eq!(type_label(3, Direction::Retrograde), "III-R");
    }

    // --- LA-13: Round-trip and boundary tests for transfer_angle ---

    #[test]
    fn roundtrip_prograde_plus_retrograde_equals_2pi() {
        // For any r1/r2 pair, prograde_angle + retrograde_angle = 2*pi
        let cases: Vec<([f64; 3], [f64; 3])> = vec![
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),       // 90 deg
            ([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),       // 180 deg (cross_z = 0)
            ([1.0, 0.0, 0.0], [1.0, 1.0, 0.0]),        // 45 deg
            ([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),       // 270 deg prograde
            ([3.0, 4.0, 0.0], [-4.0, 3.0, 0.0]),       // 90 deg, different magnitudes
            ([1.0, 1.0, 0.0], [-1.0, 1.0, 0.0]),       // 90 deg rotated
            ([1.0, 0.0, 0.0], [1.0, 0.001, 0.0]),      // near-zero angle
            ([1.0, 0.0, 0.0], [1.0, -0.001, 0.0]),     // near-2pi prograde
            ([2.0, 3.0, 1.0], [-1.0, 4.0, -2.0]),      // 3D vectors
        ];

        for (r1, r2) in &cases {
            let pro = transfer_angle(r1, r2, Direction::Prograde);
            let retro = transfer_angle(r1, r2, Direction::Retrograde);
            let sum = pro + retro;
            assert!(
                (sum - 2.0 * PI).abs() < 1e-10,
                "prograde ({pro:.6}) + retrograde ({retro:.6}) = {sum:.10}, \
                 expected 2*pi for r1={r1:?}, r2={r2:?}"
            );
        }
    }

    #[test]
    fn roundtrip_symmetry_swapped_vectors() {
        // Swapping r1 and r2 should give complementary angles for prograde
        let r1 = [1.0, 0.0, 0.0];
        let r2 = [0.0, 1.0, 0.0];

        let angle_12 = transfer_angle(&r1, &r2, Direction::Prograde);
        let angle_21 = transfer_angle(&r2, &r1, Direction::Prograde);
        assert!(
            (angle_12 + angle_21 - 2.0 * PI).abs() < 1e-10,
            "angle(r1->r2) + angle(r2->r1) should equal 2*pi for prograde"
        );
    }

    #[test]
    fn boundary_collinear_same_direction() {
        // r1 and r2 point in the same direction => angle is 0 (or 2*pi depending on convention)
        // cross_z = 0, cos_theta = 1 => acos = 0
        // Prograde: cross_z >= 0 (0.0 >= 0.0 is true) => angle = 0
        let r1 = [1.0, 0.0, 0.0];
        let r2 = [2.0, 0.0, 0.0];

        let pro = transfer_angle(&r1, &r2, Direction::Prograde);
        assert!(
            pro.abs() < 1e-10,
            "collinear same-direction prograde angle should be 0, got {pro}"
        );

        let retro = transfer_angle(&r1, &r2, Direction::Retrograde);
        assert!(
            (retro - 2.0 * PI).abs() < 1e-10,
            "collinear same-direction retrograde angle should be 2*pi, got {retro}"
        );
    }

    #[test]
    fn boundary_collinear_opposite_direction() {
        // r1 and r2 point in opposite directions => 180 deg
        // cross_z = 0, cos_theta = -1 => acos = pi
        // Prograde: cross_z >= 0 (0.0 >= 0.0 is true) => angle = pi
        let r1 = [1.0, 0.0, 0.0];
        let r2 = [-1.0, 0.0, 0.0];

        let pro = transfer_angle(&r1, &r2, Direction::Prograde);
        assert!(
            (pro - PI).abs() < 1e-10,
            "anti-parallel prograde angle should be pi, got {pro}"
        );

        let retro = transfer_angle(&r1, &r2, Direction::Retrograde);
        assert!(
            (retro - PI).abs() < 1e-10,
            "anti-parallel retrograde angle should be pi, got {retro}"
        );
    }

    #[test]
    fn boundary_180_deg_known_geometry() {
        // 180-degree transfer: r2 = (-r1_x, -r1_y, z) where z doesn't affect cross_z
        // For any vector pair where cos_theta = -1, angle = pi regardless of direction
        let r1 = [3.0, 4.0, 0.0];
        let r2 = [-3.0, -4.0, 0.0];

        let pro = transfer_angle(&r1, &r2, Direction::Prograde);
        let retro = transfer_angle(&r1, &r2, Direction::Retrograde);

        assert!(
            (pro - PI).abs() < 1e-10,
            "180-deg prograde should be pi, got {pro}"
        );
        assert!(
            (retro - PI).abs() < 1e-10,
            "180-deg retrograde should be pi, got {retro}"
        );
    }

    #[test]
    fn boundary_near_zero_angle() {
        // r2 is nearly aligned with r1, small positive cross_z
        let r1 = [1.0, 0.0, 0.0];
        let r2 = [1.0, 1e-8, 0.0];

        let pro = transfer_angle(&r1, &r2, Direction::Prograde);
        assert!(
            pro < 1e-6 && pro >= 0.0,
            "near-zero prograde angle should be very small positive, got {pro}"
        );

        let retro = transfer_angle(&r1, &r2, Direction::Retrograde);
        assert!(
            (retro - 2.0 * PI).abs() < 1e-6,
            "near-zero retrograde angle should be near 2*pi, got {retro}"
        );
    }

    #[test]
    fn transfer_angle_with_3d_vectors() {
        // Verify that z-components of r1/r2 don't break the invariant
        // (cross_z only depends on x,y components)
        let r1 = [1.0, 0.0, 5.0];
        let r2 = [0.0, 1.0, -3.0];

        let pro = transfer_angle(&r1, &r2, Direction::Prograde);
        let retro = transfer_angle(&r1, &r2, Direction::Retrograde);

        assert!(
            (pro + retro - 2.0 * PI).abs() < 1e-10,
            "round-trip should hold for 3D vectors"
        );
        // Angle should be between 0 and 2*pi
        assert!(pro > 0.0 && pro < 2.0 * PI);
        assert!(retro > 0.0 && retro < 2.0 * PI);
    }
}
