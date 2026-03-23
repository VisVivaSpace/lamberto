//! Interplanetary trajectory scanner.
//!
//! Lamberto sweeps departure/arrival date grids, solves Lambert's problem
//! for each pair using the Gooding method, and outputs trajectory catalogs
//! and pork-chop plot data for mission design.
//!
//! # Modules
//!
//! - [`config`] — YAML configuration parsing
//! - [`scan`] — sweep execution and solution collection
//! - [`output`] — CSV and YAML output generation
//! - [`transfer`] — transfer angle computation and type classification
//! - [`bodies`] — celestial body name/NAIF ID resolution
//! - [`error`] — error types

pub mod bodies;
pub mod config;
pub mod error;
pub mod output;
pub mod scan;
pub mod transfer;

pub use error::LambertoError;

/// DE440 planetary ephemeris (shortened, 2000–2075), embedded at compile time.
pub const EMBEDDED_SPK: &[u8] = include_bytes!("../assets/de440-shorter.bsp");

/// Load an Almanac with the embedded ephemeris, optionally loading
/// an additional SPK file on top.
pub fn load_almanac(extra_spk: Option<&str>) -> Result<anise::prelude::Almanac, LambertoError> {
    let almanac = anise::prelude::Almanac::default().load_from_bytes(
        bytes::BytesMut::from(EMBEDDED_SPK),
    )
    .map_err(|e| LambertoError::Ephemeris(format!("embedded SPK: {e}")))?;

    match extra_spk {
        Some(path) => almanac
            .load(path)
            .map_err(|e| LambertoError::Ephemeris(format!("{path}: {e}"))),
        None => Ok(almanac),
    }
}
