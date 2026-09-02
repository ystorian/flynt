// src/lib.rs

//! Lints Fluent translation keys against their use in Rust code and Askama templates.
//!
//! Keys used in templates `{{ "key" | t(&lang) }}` and in Rust `loc("key", &lang)` are collected
//! and compared against the `.ftl` files in the `locales` directory.
//!
//! # Example
//!
//! ```no_run
//! use flynt::config::{self, PartialConfig};
//!
//! let cli = PartialConfig {
//!     root: Some("../my-project".into()),
//!     ..PartialConfig::default()
//! };
//! let config = config::load(&cli)?;
//! let report = flynt::check(&config)?;
//!
//! for finding in &report.missing_keys {
//!     println!("{} is missing in {}", finding.key, finding.missing_in.join(", "));
//! }
//! assert_eq!(report.exit_code(), 0);
//! # Ok::<(), anyhow::Error>(())
//! ```

#![deny(unsafe_code)]
#![warn(clippy::pedantic, missing_docs)]

mod check;
pub mod config;
mod extract;
pub mod model;
mod parse;
pub mod report;

pub use config::{ColorChoice, Config, OutputFormat, PartialConfig, Severity};
pub use model::{
	DuplicateKey, InconsistentKey, KeyDefinition, KeyUsage, Location, MissingKey, ParseError,
	Report, SCHEMA_VERSION, Summary, UnusedKey, UsageType,
};

/// Runs every check and returns the report.
///
/// # Errors
///
/// - The directory cannot be walked.
/// - A file is not valid UTF-8.
/// - A glob pattern is invalid.
/// - The `locales` directory is missing.
/// - Nothing exists to check.
pub fn check(config: &Config) -> anyhow::Result<Report> {
	check::run(config)
}
