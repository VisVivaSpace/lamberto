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
