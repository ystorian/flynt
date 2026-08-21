// src/lib.rs

//! Lint Fluent translation keys against their use in Rust code and Askama templates.
//!
//! Keys used in templates `{{ "key" | t(&lang) }}` and in Rust `loc("key", &lang)` are collected
//! and compared against the `.ftl` files. A typo then fails the build instead of shipping the key
//! name as user-facing text.
//!
//! flynt infers what to scan from the target repository. Workspace members come from its
//! `Cargo.toml`. Locales come from the subdirectories of its locales directory. Every default can
//! be overridden, on the command line or in a `.flynt.toml`.
//!
//! # Checks
//!
//! - coverage: a used key is defined in every locale.
//! - consistency: every locale defines the same set of keys.
//! - duplicates: no locale defines a key twice.
//! - unused: every defined key is referenced somewhere. This is a warning by default.
//! - syntax: every `.ftl` file parses.
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

/// Runs every check and returns the report. Performs no output.
///
/// Rendering is [`report::render`]'s job. This lets a caller embedding flynt act on the findings
/// instead of parsing text.
///
/// # Errors
///
/// Returns an error for tool-level problems only. This includes: a directory that cannot be walked,
/// a file that is not valid UTF-8, an invalid glob, a missing locales directory (unless
/// `require_locales` is off), or a tree with nothing at all to check. Lint findings are carried in
/// the [`Report`], not returned as errors.
pub fn check(config: &Config) -> anyhow::Result<Report> {
	check::run(config)
}
