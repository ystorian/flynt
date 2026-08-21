// src/check/mod.rs

//! Lint checks and orchestration.

pub mod consistency;
pub mod coverage;
pub mod duplicates;
pub mod unused;

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use crate::config::{Config, Severity};
use crate::extract::{self, Found, scan::Walker};
use crate::model::{Report, SCHEMA_VERSION, Summary};
use crate::parse::fluent::{self, Parsed};

/// Runs every check and returns the report.
///
/// Performs no output.
///
/// # Errors
///
/// Returns an error for tool-level problems only.
pub fn run(config: &Config) -> Result<Report> {
	let walker = Walker::new(config)?;

	let mut found = Found::default();
	found.absorb(extract::templates::keys(config, &walker)?);
	found.absorb(extract::rust::keys(config, &walker)?);
	found.usages.sort();

	require_locale_directory(config)?;
	let parsed = fluent::parse_locales(config, &walker)?;
	require_locale_contents(config, &parsed)?;

	let used: BTreeSet<String> = found.usages.iter().map(|u| u.key.clone()).collect();
	let defined: BTreeSet<&String> = parsed
		.locales
		.values()
		.flat_map(|locale| locale.keys.keys())
		.collect();

	// A linter that found nothing to look at has been pointed at the wrong place.
	ensure!(
		!(used.is_empty() && defined.is_empty()),
		"nothing to check: no keys used under {} and none defined under {}.\n\
		 Check the path, or set --src, --templates and --locales-dir.",
		describe(&config.src, &config.templates),
		config.locales_dir.display()
	);

	let unused_keys = if config.unused == Severity::Allow {
		Vec::new()
	} else {
		let reference = parsed.locales.get(&config.reference_locale);
		match reference {
			Some(reference) => unused::unused(reference, &used, &config.ignore_unused)?,
			None => Vec::new(),
		}
	};

	Ok(Report {
		schema_version: SCHEMA_VERSION,
		summary: Summary {
			used: used.len(),
			defined: defined.len(),
			defined_per_locale: parsed
				.locales
				.iter()
				.map(|(name, locale)| (name.clone(), locale.keys.len()))
				.collect(),
			locales: parsed.locales.keys().cloned().collect(),
			reference_locale: config.reference_locale.clone(),
			files_scanned: found.files + parsed.files,
		},
		missing_keys: coverage::missing(&found.usages, &parsed.locales),
		inconsistent_keys: consistency::inconsistent(&parsed.locales),
		duplicate_keys: duplicates::duplicates(&parsed.locales),
		unused_keys,
		parse_errors: parsed.errors,
		unused_severity: config.unused,
	})
}

/// Refuses to run when there is no locale set to check against.
fn require_locale_directory(config: &Config) -> Result<()> {
	if !config.require_locales {
		return Ok(());
	}

	ensure!(
		config.locales_dir.is_dir(),
		"the locales directory does not exist: {}\n\
		 Pass --locales-dir to point at it, or --require-locales=false to allow its absence.",
		config.locales_dir.display()
	);
	ensure!(
		!config.locales.is_empty(),
		"no locale found in {}: it has no subdirectories.\n\
		 A locale is a subdirectory holding .ftl files, such as {}/en.",
		config.locales_dir.display(),
		config.locales_dir.display()
	);

	Ok(())
}

/// Refuses to run when a requested locale is empty or absent.
fn require_locale_contents(config: &Config, parsed: &Parsed) -> Result<()> {
	if !config.require_locales {
		return Ok(());
	}

	for locale in &config.locales {
		let dir = config.locales_dir.join(locale);
		ensure!(
			dir.is_dir(),
			"the locale {locale:?} has no directory: {}",
			dir.display()
		);
		ensure!(
			parsed.files_per_locale.get(locale).copied().unwrap_or(0) > 0,
			"the locale {locale:?} has no .ftl file under {}",
			dir.display()
		);
	}

	Ok(())
}

/// Describes the scanned directories for an error message.
fn describe(src: &[std::path::PathBuf], templates: &[std::path::PathBuf]) -> String {
	let mut all: Vec<String> = src
		.iter()
		.chain(templates)
		.map(|p| p.display().to_string())
		.collect();
	all.sort();
	if all.is_empty() {
		"(no source or template directory)".to_owned()
	} else {
		all.join(", ")
	}
}

#[cfg(test)]
pub(crate) mod tests {
	use std::path::PathBuf;

	use crate::model::{KeyDefinition, KeyUsage, LocaleKeys, Location, UsageType};

	pub fn locale(name: &str, keys: &[&str]) -> LocaleKeys {
		locale_with_terms(name, keys, &[])
	}

	pub fn locale_with_terms(name: &str, keys: &[&str], terms: &[&str]) -> LocaleKeys {
		LocaleKeys {
			locale: name.to_owned(),
			keys: keys
				.iter()
				.map(|k| {
					(
						(*k).to_owned(),
						vec![KeyDefinition {
							at: Location {
								file: PathBuf::from(format!("locales/{name}/a.ftl")),
								line: 1,
								column: 1,
							},
						}],
					)
				})
				.collect(),
			terms: terms.iter().map(|t| (*t).to_owned()).collect(),
		}
	}

	/// A locale with explicit definition sites.
	pub fn locale_with(name: &str, keys: &[(&str, &[(&str, usize)])]) -> LocaleKeys {
		LocaleKeys {
			locale: name.to_owned(),
			keys: keys
				.iter()
				.map(|(key, sites)| {
					(
						(*key).to_owned(),
						sites
							.iter()
							.map(|(file, line)| KeyDefinition {
								at: Location {
									file: PathBuf::from(format!("locales/{name}/{file}")),
									line: *line,
									column: 1,
								},
							})
							.collect(),
					)
				})
				.collect(),
			terms: std::collections::BTreeSet::new(),
		}
	}

	/// A template usage of `key`.
	pub fn usage(key: &str) -> KeyUsage {
		KeyUsage {
			key: key.to_owned(),
			at: Location {
				file: PathBuf::from("templates/home.html"),
				line: 1,
				column: 1,
			},
			kind: UsageType::Template,
		}
	}
}
