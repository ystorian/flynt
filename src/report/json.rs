// src/report/json.rs

//! The machine-readable report.
//!
//! Paths are relative to the linted root.

use std::io::Write;

use crate::model::Report;

/// Writes the report as pretty-printed JSON, with a trailing newline.
///
/// # Errors
///
/// Propagates write failures from `out`.
pub fn render(report: &Report, out: &mut impl Write) -> std::io::Result<()> {
	serde_json::to_writer_pretty(&mut *out, report).map_err(std::io::Error::other)?;
	writeln!(out)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use std::path::PathBuf;

	use serde_json::{Value, json};

	use super::*;
	use crate::config::Severity;
	use crate::model::{
		DuplicateKey, InconsistentKey, KeyDefinition, KeyUsage, Location, MissingKey, ParseError,
		SCHEMA_VERSION, Summary, UnusedKey, UsageType,
	};

	fn at(file: &str, line: usize, column: usize) -> Location {
		Location {
			file: PathBuf::from(file),
			line,
			column,
		}
	}

	fn full() -> Report {
		Report {
			schema_version: SCHEMA_VERSION,
			summary: Summary {
				used: 2,
				defined: 3,
				defined_per_locale: BTreeMap::from([("en".to_owned(), 3), ("fr".to_owned(), 2)]),
				locales: vec!["en".to_owned(), "fr".to_owned()],
				reference_locale: "en".to_owned(),
				files_scanned: 5,
			},
			missing_keys: vec![MissingKey {
				key: "tpl-ghost".to_owned(),
				usages: vec![KeyUsage {
					key: "tpl-ghost".to_owned(),
					at: at("types/templates/home.html", 42, 8),
					kind: UsageType::Template,
				}],
				missing_in: vec!["fr".to_owned()],
			}],
			inconsistent_keys: vec![InconsistentKey {
				key: "only-en".to_owned(),
				present_in: vec!["en".to_owned()],
				missing_in: vec!["fr".to_owned()],
			}],
			duplicate_keys: vec![DuplicateKey {
				key: "dup".to_owned(),
				locale: "en".to_owned(),
				definitions: vec![KeyDefinition {
					at: at("locales/en/a.ftl", 3, 1),
				}],
			}],
			unused_keys: vec![UnusedKey {
				key: "stale".to_owned(),
				locale: "en".to_owned(),
				definition: KeyDefinition {
					at: at("locales/en/app.ftl", 7, 1),
				},
			}],
			parse_errors: vec![ParseError {
				at: at("locales/en/broken.ftl", 12, 7),
				message: "Expected a token starting with \"=\"".to_owned(),
			}],
			unused_severity: Severity::Warn,
		}
	}

	fn value(report: &Report) -> Value {
		let mut out = Vec::new();
		render(report, &mut out).expect("writing to a Vec cannot fail");
		serde_json::from_slice(&out).expect("the output is valid JSON")
	}

	#[test]
	fn the_shape_is_exactly_as_documented() {
		assert_eq!(
			value(&full()),
			json!({
				"schema_version": 1,
				"summary": {
					"used": 2,
					"defined": 3,
					"defined_per_locale": { "en": 3, "fr": 2 },
					"locales": ["en", "fr"],
					"reference_locale": "en",
					"files_scanned": 5
				},
				"missing_keys": [{
					"key": "tpl-ghost",
					"usages": [{
						"key": "tpl-ghost",
						"at": { "file": "types/templates/home.html", "line": 42, "column": 8 },
						"kind": "template"
					}],
					"missing_in": ["fr"]
				}],
				"inconsistent_keys": [{
					"key": "only-en",
					"present_in": ["en"],
					"missing_in": ["fr"]
				}],
				"duplicate_keys": [{
					"key": "dup",
					"locale": "en",
					"definitions": [{ "at": { "file": "locales/en/a.ftl", "line": 3, "column": 1 } }]
				}],
				"unused_keys": [{
					"key": "stale",
					"locale": "en",
					"definition": { "at": { "file": "locales/en/app.ftl", "line": 7, "column": 1 } }
				}],
				"parse_errors": [{
					"at": { "file": "locales/en/broken.ftl", "line": 12, "column": 7 },
					"message": "Expected a token starting with \"=\""
				}]
			})
		);
	}

	#[test]
	fn the_severity_is_not_serialized() {
		// The severity decides the exit code. It is not part of the report data.
		let object = value(&full());
		assert!(object.get("unused_severity").is_none());
	}

	#[test]
	fn the_output_ends_with_a_newline() {
		let mut out = Vec::new();
		render(&full(), &mut out).expect("writing to a Vec cannot fail");
		assert_eq!(out.last(), Some(&b'\n'));
	}

	#[test]
	fn rendering_twice_gives_identical_bytes() {
		let report = full();
		let mut a = Vec::new();
		let mut b = Vec::new();
		render(&report, &mut a).expect("writing to a Vec cannot fail");
		render(&report, &mut b).expect("writing to a Vec cannot fail");
		assert_eq!(a, b);
	}
}
