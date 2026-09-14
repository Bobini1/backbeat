//! # Rhythm Game Formats
//!
//! [![github]](https://github.com/zkldi/rg_formats)&ensp;[![crates-io]](https://crates.io/crates/rg_formats)&ensp;[![docs-rs]](https://docs.rs/rg_formats)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! Various processors for various rhythm game formats.
//!
//! These are the currently available modules:
//!
//! - [`bms`] for parsing `.bms` / `.bme` / `.bml` / `.pms` files.
//! - [`bmson`] for parsing `.bmson` files.
//! - [`clone_hero_chart`] for parsing Clone Hero `.chart` files.
//! - [`dtx`] for parsing `.dtx` (DTXMania) files.
//! - [`dwi`] for parsing `.dwi` files.
//! - [`ksh`] for parsing `.ksh` files.
//! - [`kson`] for parsing `.kson` files.
//! - [`sm`] for parsing `.sm` files.
//! - [`ssc`] for parsing `.ssc` files.
//! - [`sm_msd`] for parsing `.msd` files, or generally working with the raw underpinnings of the
//!   `.sm` and `.ssc` formats.
//! - [`tja`] for parsing `.tja` (TJAPlayer3 / Taiko) files.
//!
//! Clicking on any of these modules will tell you more.
//!
//! # Usage
//!
//! ```rust,no_run
//! let sm_charts = rg_formats::sm::from_path("my_sm_file.sm")
//!     // an outer io::Error<> indicates whether the file could be read
//!     .expect("failed to open file")
//!     // the inner error indicates whether the contents of the file were valid SM.
//!     .expect("sm file was invalid");
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![forbid(rustdoc::all)]

pub mod bms;
pub mod bmson;
pub mod clone_hero_chart;
pub mod dtx;
pub mod dwi;
#[allow(missing_docs)]
pub mod ksh;
pub mod kson;
pub mod sm;
pub mod sm_msd;
pub mod ssc;
mod test_utils;
pub mod tja;
mod utils;
