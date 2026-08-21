// src/model.rs

//! Data model: key usages, key definitions, findings, and the report.
//!
//! Every collection in this module is ordered. This module uses `BTreeMap`, `BTreeSet`, or an
//! explicitly sorted `Vec`. Two runs over the same tree then produce byte-identical output.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::config::Severity;

/// Where a key was used or defined.
///
/// `file` is always relative to the linted root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Location {
	/// Path relative to the linted root.
	#[serde(serialize_with = "serialize_path")]
	pub file: PathBuf,
	/// One-based line number.
	pub line: usize,
	/// One-based column number.
	pub column: usize,
}

/// Serializes a path with forward slashes for the JSON report (platform agnostic).
fn serialize_path<S: serde::Serializer>(path: &std::path::Path, out: S) -> Result<S::Ok, S::Error> {
	let text = path.to_string_lossy();
	if std::path::MAIN_SEPARATOR == '/' {
		out.serialize_str(&text)
	} else {
		out.serialize_str(&text.replace(std::path::MAIN_SEPARATOR, "/"))
	}
}

impl std::fmt::Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{}", self.file.display(), self.line, self.column)
	}
}

/// What kind of source a key was used from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageType {
	/// An Askama template filter, such as `{{ "key" | t(&lang) }}`.
	Template,
	/// A Rust function call, such as `loc("key", &lang)`.
	Rust,
}

impl UsageType {
	/// Human-readable label used in the text report.
	#[must_use]
	pub fn label(self) -> &'static str {
		match self {
			Self::Template => "template",
			Self::Rust => "rust",
		}
	}
}

/// A single site where a translation key is used.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct KeyUsage {
	/// The translation key.
	pub key: String,
	/// Where the key is used.
	pub at: Location,
	/// Whether the usage came from a template or from Rust code.
	pub kind: UsageType,
}

/// A single site where a translation key is defined.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct KeyDefinition {
	/// Where the key is defined.
	pub at: Location,
}

/// Every key defined by one locale.
///
/// Keys map to all of their definitionss. Duplicate detection needs no second map because of this.
#[derive(Debug, Clone, Default)]
pub struct LocaleKeys {
	/// The locale name, which is also its directory name.
	pub locale: String,
	/// Keys, each with every definition found for it.
	pub keys: BTreeMap<String, Vec<KeyDefinition>>,
	/// Keys that are Fluent terms (`-brand = ...`), a subset of `keys`.
	pub terms: std::collections::BTreeSet<String>,
}

/// A key that is used but not defined in every locale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingKey {
	/// The translation key.
	pub key: String,
	/// Every site that uses the key, sorted.
	pub usages: Vec<KeyUsage>,
	/// Locales that do not define the key, sorted.
	pub missing_in: Vec<String>,
}

/// A key that some locales define and others do not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InconsistentKey {
	/// The translation key.
	pub key: String,
	/// Locales that define the key, sorted.
	pub present_in: Vec<String>,
	/// Locales that do not define the key, sorted.
	pub missing_in: Vec<String>,
}

/// A key defined more than once within a single locale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateKey {
	/// The translation key.
	pub key: String,
	/// The locale that defines it more than once.
	pub locale: String,
	/// Every definition, sorted.
	pub definitions: Vec<KeyDefinition>,
}

/// A key that is defined but never used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnusedKey {
	/// The translation key.
	pub key: String,
	/// The locale the finding was computed against, the reference locale.
	pub locale: String,
	/// Where the key is defined.
	pub definition: KeyDefinition,
}

/// A syntax error in a Fluent file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParseError {
	/// Where the error occurred.
	pub at: Location,
	/// The message reported by the Fluent parser.
	pub message: String,
}

/// Counts and context for the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
	/// Number of distinct keys used across templates and Rust code.
	pub used: usize,
	/// Number of distinct keys defined, as the union over every locale.
	pub defined: usize,
	/// Number of keys defined by each locale.
	pub defined_per_locale: BTreeMap<String, usize>,
	/// Locales that were checked, sorted.
	pub locales: Vec<String>,
	/// Locale used for the unused-key check.
	pub reference_locale: String,
	/// Number of files read, across sources, templates, and locales.
	pub files_scanned: usize,
}

/// The result of a full run. It contains findings and performs no output.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
	/// Version of the JSON shape, bumped on any breaking change.
	pub schema_version: u32,
	/// Counts and context.
	pub summary: Summary,
	/// Keys used but missing from at least one locale.
	pub missing_keys: Vec<MissingKey>,
	/// Keys that some locales define and others do not.
	pub inconsistent_keys: Vec<InconsistentKey>,
	/// Keys defined more than once in one locale.
	pub duplicate_keys: Vec<DuplicateKey>,
	/// Keys defined but never used.
	pub unused_keys: Vec<UnusedKey>,
	/// Fluent syntax errors.
	pub parse_errors: Vec<ParseError>,
	/// Severity configured for `unused_keys`, which decides the exit code.
	#[serde(skip)]
	pub unused_severity: Severity,
}

/// The current [`Report::schema_version`].
pub const SCHEMA_VERSION: u32 = 1;

impl Report {
	/// Whether the run found anything that should fail the build.
	#[must_use]
	pub fn has_errors(&self) -> bool {
		!self.missing_keys.is_empty()
			|| !self.inconsistent_keys.is_empty()
			|| !self.duplicate_keys.is_empty()
			|| !self.parse_errors.is_empty()
			|| (self.unused_severity == Severity::Error && !self.unused_keys.is_empty())
	}

	/// Whether the run found anything worth mentioning but not failing on.
	#[must_use]
	pub fn has_warnings(&self) -> bool {
		self.is_vacuous()
			|| (self.unused_severity == Severity::Warn && !self.unused_keys.is_empty())
	}

	/// Whether no locale was checked. When this is true, the coverage, and consistency checks are
	/// inapplicable.
	///
	/// This only happens with `--require-locales=false`. The run must never be reported as clean in
	/// this case.
	#[must_use]
	pub fn is_vacuous(&self) -> bool {
		self.summary.locales.is_empty()
	}

	/// Process exit code: `0` when clean, `1` when there are errors.
	#[must_use]
	pub fn exit_code(&self) -> u8 {
		u8::from(self.has_errors())
	}

	/// Total number of findings, at any severity.
	#[must_use]
	pub fn finding_count(&self) -> usize {
		self.missing_keys.len()
			+ self.inconsistent_keys.len()
			+ self.duplicate_keys.len()
			+ self.unused_keys.len()
			+ self.parse_errors.len()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn report(unused_severity: Severity, unused: usize) -> Report {
		Report {
			schema_version: SCHEMA_VERSION,
			summary: Summary {
				used: 0,
				defined: 0,
				defined_per_locale: BTreeMap::from([("en".to_owned(), 0)]),
				locales: vec!["en".to_owned()],
				reference_locale: "en".to_owned(),
				files_scanned: 0,
			},
			missing_keys: Vec::new(),
			inconsistent_keys: Vec::new(),
			duplicate_keys: Vec::new(),
			unused_keys: (0..unused)
				.map(|i| UnusedKey {
					key: format!("k{i}"),
					locale: "en".to_owned(),
					definition: KeyDefinition {
						at: Location {
							file: PathBuf::from("locales/en/a.ftl"),
							line: 1,
							column: 1,
						},
					},
				})
				.collect(),
			parse_errors: Vec::new(),
			unused_severity,
		}
	}

	#[test]
	fn clean_report_exits_zero() {
		let r = report(Severity::Warn, 0);
		assert!(!r.has_errors());
		assert!(!r.has_warnings());
		assert_eq!(r.exit_code(), 0);
	}

	#[test]
	fn unused_severity_decides_the_exit_code() {
		let warn = report(Severity::Warn, 3);
		assert!(!warn.has_errors());
		assert!(warn.has_warnings());
		assert_eq!(warn.exit_code(), 0);

		let deny = report(Severity::Error, 3);
		assert!(deny.has_errors());
		assert!(!deny.has_warnings());
		assert_eq!(deny.exit_code(), 1);

		let allow = report(Severity::Allow, 3);
		assert!(!allow.has_errors());
		assert!(!allow.has_warnings());
	}

	#[test]
	fn a_run_with_no_locale_is_never_reported_as_clean() {
		let mut r = report(Severity::Warn, 0);
		r.summary.locales.clear();
		assert!(r.is_vacuous());
		assert!(r.has_warnings(), "an inapplicable run must not look clean");
		// The user asked for it with `--require-locales=false`.
		assert!(!r.has_errors());
		assert_eq!(r.exit_code(), 0);
	}

	#[test]
	fn location_displays_as_file_line_column() {
		let at = Location {
			file: PathBuf::from("types/templates/home.html"),
			line: 42,
			column: 8,
		};
		assert_eq!(at.to_string(), "types/templates/home.html:42:8");
	}

	#[test]
	fn a_serialized_path_always_uses_forward_slashes() {
		let at = Location {
			file: PathBuf::from("templates").join("home.html"),
			line: 1,
			column: 1,
		};
		let json = serde_json::to_value(&at).expect("the location serializes");
		assert_eq!(json["file"], "templates/home.html");
	}
}
