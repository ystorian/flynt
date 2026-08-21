// src/report/text.rs

//! The human-readable report.

use std::io::Write;

use super::{Paint, plural};
use crate::config::{Config, Severity};
use crate::model::Report;

/// Writes the report as text.
///
/// # Errors
///
/// Propagates write failures from `out`.
pub fn render(
	report: &Report,
	config: &Config,
	color: bool,
	out: &mut impl Write,
) -> std::io::Result<()> {
	let paint = Paint::new(color);

	if !report.has_errors() && !report.has_warnings() {
		if config.quiet {
			return Ok(());
		}
		let _ = summary(report, &paint, out);
		writeln!(out)?;
		writeln!(out, "✅ {}", paint.green("All translation keys validated"))?;
		return Ok(());
	}

	vacuous(report, &paint, out)?;
	missing(report, &paint, out)?;
	inconsistent(report, &paint, out)?;
	duplicates(report, &paint, out)?;
	parse_errors(report, &paint, out)?;
	unused(report, &paint, out)?;

	summary(report, &paint, out)?;

	if report.has_errors() {
		writeln!(out)?;
		writeln!(out, "❌ {}", paint.red("Validation failed"))?;
	}

	Ok(())
}

fn heading(paint: &Paint, level: Severity, text: &str) -> String {
	match level {
		Severity::Error => format!("\n{} {text}", paint.red("Error:")),
		Severity::Warn => format!("\n{} {text}", paint.yellow("Warning:")),
		Severity::Allow => text.to_owned(),
	}
}

fn vacuous(report: &Report, paint: &Paint, out: &mut impl Write) -> std::io::Result<()> {
	if !report.is_vacuous() {
		return Ok(());
	}

	writeln!(
		out,
		"{}",
		heading(
			paint,
			Severity::Warn,
			"no locale was checked, key coverage was not verified"
		)
	)?;
	writeln!(
		out,
		"  {}",
		paint.dim("Point --locales-dir at a directory holding one subdirectory per locale.")
	)?;
	Ok(())
}

fn missing(report: &Report, paint: &Paint, out: &mut impl Write) -> std::io::Result<()> {
	if report.missing_keys.is_empty() {
		return Ok(());
	}

	writeln!(
		out,
		"{}",
		heading(
			paint,
			Severity::Error,
			&format!(
				"{} used but not defined",
				plural(report.missing_keys.len(), "translation key")
			)
		)
	)?;

	for finding in &report.missing_keys {
		writeln!(out, " {}", paint.yellow(&finding.key))?;
		for usage in &finding.usages {
			writeln!(
				out,
				"  used at {} {}",
				paint.link(&usage.at.to_string()),
				paint.dim(&format!("({})", usage.kind.label()))
			)?;
		}
		writeln!(
			out,
			"  {}",
			paint.yellow(&format!("missing in {}", finding.missing_in.join(", ")))
		)?;
	}
	Ok(())
}

fn inconsistent(report: &Report, paint: &Paint, out: &mut impl Write) -> std::io::Result<()> {
	if report.inconsistent_keys.is_empty() {
		return Ok(());
	}

	writeln!(
		out,
		"{}",
		heading(
			paint,
			Severity::Error,
			&format!(
				"{} not defined in every locale",
				plural(report.inconsistent_keys.len(), "key")
			)
		)
	)?;

	for finding in &report.inconsistent_keys {
		writeln!(out, " {}", paint.yellow(&finding.key))?;
		writeln!(out, "  defined in {}", finding.present_in.join(", "))?;
		writeln!(
			out,
			"  {}",
			paint.yellow(&format!("missing in {}", finding.missing_in.join(", ")))
		)?;
	}
	Ok(())
}

fn duplicates(report: &Report, paint: &Paint, out: &mut impl Write) -> std::io::Result<()> {
	if report.duplicate_keys.is_empty() {
		return Ok(());
	}

	writeln!(
		out,
		"{}",
		heading(
			paint,
			Severity::Error,
			&format!(
				"{} defined more than once",
				plural(report.duplicate_keys.len(), "key")
			)
		)
	)?;
	writeln!(out)?;

	for finding in &report.duplicate_keys {
		writeln!(
			out,
			" {} {}",
			paint.yellow(&finding.key),
			paint.dim(&format!("({})", finding.locale))
		)?;
		for definition in &finding.definitions {
			writeln!(out, "  {}", definition.at)?;
		}
	}
	writeln!(out)
}

fn parse_errors(report: &Report, paint: &Paint, out: &mut impl Write) -> std::io::Result<()> {
	if report.parse_errors.is_empty() {
		return Ok(());
	}

	writeln!(
		out,
		"{}",
		heading(
			paint,
			Severity::Error,
			&format!(
				"{} in the locale files",
				plural(report.parse_errors.len(), "syntax error")
			)
		)
	)?;

	for finding in &report.parse_errors {
		writeln!(out, "  {}: {}", finding.at, finding.message)?;
	}
	Ok(())
}

fn unused(report: &Report, paint: &Paint, out: &mut impl Write) -> std::io::Result<()> {
	if report.unused_keys.is_empty() {
		return Ok(());
	}

	writeln!(
		out,
		"{}",
		heading(
			paint,
			report.unused_severity,
			&format!(
				"{} defined but never used",
				plural(report.unused_keys.len(), "key")
			)
		)
	)?;

	for finding in &report.unused_keys {
		writeln!(out, " {}", paint.yellow(&finding.key))?;
		writeln!(out, "  defined at {}", finding.definition.at)?;
	}
	writeln!(
		out,
		"  {}",
		paint.dim("Use --ignore-unused <GLOB> for keys built at runtime.")
	)?;
	Ok(())
}

fn summary(report: &Report, paint: &Paint, out: &mut impl Write) -> std::io::Result<()> {
	let s = &report.summary;

	let locales = if s.locales.is_empty() {
		"none".to_owned()
	} else {
		s.locales.join(", ")
	};
	let locale_count = s.locales.len();
	let locale_word = if locale_count == 1 {
		"locale"
	} else {
		"locales"
	};
	writeln!(
		out,
		"{} {locales}",
		paint.green_bold(
			"\nChecked ",
			&locale_count.to_string(),
			&format!(" {locale_word}:")
		)
	)?;

	let mut problems = Vec::new();
	push_count(&mut problems, report.missing_keys.len(), "missing");
	push_count(
		&mut problems,
		report.inconsistent_keys.len(),
		"inconsistent",
	);
	push_count(&mut problems, report.duplicate_keys.len(), "duplicate");
	push_count(&mut problems, report.parse_errors.len(), "syntax error");
	if !problems.is_empty() {
		writeln!(out, "{} {}", paint.red("Problems:"), problems.join(", "))?;
	}

	if report.unused_severity == Severity::Warn && !report.unused_keys.is_empty() {
		writeln!(
			out,
			"  {} {} unused",
			paint.yellow("⚠"),
			report.unused_keys.len()
		)?;
	}

	writeln!(out)?;
	writeln!(out, "{}", paint.bold("Summary:"))?;
	writeln!(out, "  {} used", plural(s.used, "key"))?;
	writeln!(out, "  {} defined", plural(s.defined, "key"))?;

	Ok(())
}

fn push_count(into: &mut Vec<String>, count: usize, label: &str) {
	if count > 0 {
		into.push(if label.contains(' ') {
			plural(count, label)
		} else {
			format!("{count} {label}")
		});
	}
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use std::path::PathBuf;

	use super::*;
	use crate::config::{ColorChoice, OutputFormat, PartialConfig, resolve};
	use crate::model::{
		DuplicateKey, InconsistentKey, KeyDefinition, KeyUsage, Location, MissingKey, ParseError,
		Report, SCHEMA_VERSION, Summary, UnusedKey, UsageType,
	};

	fn config() -> Config {
		resolve(
			&PartialConfig::default(),
			&PartialConfig::default(),
			&crate::config::Discovered::default(),
			std::path::Path::new("/repo"),
		)
	}

	fn at(file: &str, line: usize, column: usize) -> Location {
		Location {
			file: PathBuf::from(file),
			line,
			column,
		}
	}

	fn clean() -> Report {
		Report {
			schema_version: SCHEMA_VERSION,
			summary: Summary {
				used: 136,
				defined: 136,
				defined_per_locale: BTreeMap::from([
					("en".to_owned(), 136),
					("fr".to_owned(), 136),
				]),
				locales: vec!["en".to_owned(), "fr".to_owned()],
				reference_locale: "en".to_owned(),
				files_scanned: 91,
			},
			missing_keys: Vec::new(),
			inconsistent_keys: Vec::new(),
			duplicate_keys: Vec::new(),
			unused_keys: Vec::new(),
			parse_errors: Vec::new(),
			unused_severity: Severity::Warn,
		}
	}

	fn rendered(report: &Report, config: &Config) -> String {
		let mut out = Vec::new();
		render(report, config, false, &mut out).expect("writing to a Vec cannot fail");
		String::from_utf8(out).expect("the output is UTF-8")
	}

	#[test]
	fn a_clean_run_reports_the_counts() {
		let out = rendered(&clean(), &config());
		assert_eq!(
			out,
			"\nChecked 2 locales: en, fr\n\nSummary:\n  136 keys used\n  136 keys defined\n\n✅ All translation keys validated\n"
		);
	}

	#[test]
	fn a_clean_run_prints_nothing_when_quiet() {
		let mut config = config();
		config.quiet = true;
		assert_eq!(rendered(&clean(), &config), "");
	}

	#[test]
	fn quiet_still_reports_findings() {
		let mut config = config();
		config.quiet = true;
		let mut report = clean();
		report.missing_keys = vec![MissingKey {
			key: "tpl-ghost".to_owned(),
			usages: vec![KeyUsage {
				key: "tpl-ghost".to_owned(),
				at: at("types/templates/home.html", 42, 8),
				kind: UsageType::Template,
			}],
			missing_in: vec!["fr".to_owned()],
		}];
		let out = rendered(&report, &config);
		assert!(out.contains("tpl-ghost"), "{out}");
	}

	#[test]
	fn a_missing_key_names_its_sites_and_locales() {
		let mut report = clean();
		report.missing_keys = vec![MissingKey {
			key: "tpl-ghost".to_owned(),
			usages: vec![
				KeyUsage {
					key: "tpl-ghost".to_owned(),
					at: at("types/templates/home.html", 42, 8),
					kind: UsageType::Template,
				},
				KeyUsage {
					key: "tpl-ghost".to_owned(),
					at: at("types/src/lib.rs", 7, 12),
					kind: UsageType::Rust,
				},
			],
			missing_in: vec!["en".to_owned(), "fr".to_owned()],
		}];

		let out = rendered(&report, &config());
		assert!(
			out.contains("Error: 1 translation key used but not defined"),
			"{out}"
		);
		assert!(
			out.contains("used at types/templates/home.html:42:8 (template)"),
			"{out}"
		);
		assert!(
			out.contains("used at types/src/lib.rs:7:12 (rust)"),
			"{out}"
		);
		assert!(out.contains("missing in en, fr"), "{out}");
		assert!(out.contains("Validation failed"), "{out}");
	}

	#[test]
	fn an_inconsistent_key_names_both_sides() {
		let mut report = clean();
		report.inconsistent_keys = vec![InconsistentKey {
			key: "only-en".to_owned(),
			present_in: vec!["en".to_owned()],
			missing_in: vec!["fr".to_owned()],
		}];
		let out = rendered(&report, &config());
		assert!(
			out.contains("Error: 1 key not defined in every locale"),
			"{out}"
		);
		assert!(out.contains("defined in en"), "{out}");
		assert!(out.contains("missing in fr"), "{out}");
	}

	#[test]
	fn a_duplicate_key_names_its_locale_and_every_site() {
		let mut report = clean();
		report.duplicate_keys = vec![DuplicateKey {
			key: "dup".to_owned(),
			locale: "en".to_owned(),
			definitions: vec![
				KeyDefinition {
					at: at("locales/en/a.ftl", 3, 1),
				},
				KeyDefinition {
					at: at("locales/en/b.ftl", 9, 1),
				},
			],
		}];
		let out = rendered(&report, &config());
		assert!(out.contains("Error: 1 key defined more than once"), "{out}");
		assert!(out.contains("dup (en)"), "{out}");
		assert!(out.contains("locales/en/a.ftl:3:1"), "{out}");
		assert!(out.contains("locales/en/b.ftl:9:1"), "{out}");
	}

	#[test]
	fn a_parse_error_is_shown_with_the_parser_message() {
		let mut report = clean();
		report.parse_errors = vec![ParseError {
			at: at("locales/en/broken.ftl", 12, 7),
			message: "Expected a token starting with \"=\"".to_owned(),
		}];
		let out = rendered(&report, &config());
		assert!(
			out.contains("Error: 1 syntax error in the locale files"),
			"{out}"
		);
		assert!(
			out.contains("locales/en/broken.ftl:12:7: Expected a token starting with \"=\""),
			"{out}"
		);
	}

	#[test]
	fn an_unused_key_is_a_warning_by_default_and_does_not_fail() {
		let mut report = clean();
		report.unused_keys = vec![UnusedKey {
			key: "stale".to_owned(),
			locale: "en".to_owned(),
			definition: KeyDefinition {
				at: at("locales/en/app.ftl", 7, 1),
			},
		}];
		let out = rendered(&report, &config());
		assert!(
			out.contains("Warning: 1 key defined but never used"),
			"{out}"
		);
		assert!(out.contains("defined at locales/en/app.ftl:7:1"), "{out}");
		assert!(
			out.contains("--ignore-unused"),
			"the escape hatch is mentioned: {out}"
		);
		assert!(!out.contains("Validation failed"), "{out}");
		assert!(out.contains("⚠ 1 unused"), "{out}");
	}

	#[test]
	fn an_unused_key_is_an_error_when_configured_so() {
		let mut report = clean();
		report.unused_severity = Severity::Error;
		report.unused_keys = vec![UnusedKey {
			key: "stale".to_owned(),
			locale: "en".to_owned(),
			definition: KeyDefinition {
				at: at("locales/en/app.ftl", 7, 1),
			},
		}];
		let out = rendered(&report, &config());
		assert!(out.contains("Error: 1 key defined but never used"), "{out}");
		assert!(out.contains("Validation failed"), "{out}");
	}

	#[test]
	fn a_run_with_no_locale_warns_instead_of_reporting_success() {
		let mut report = clean();
		report.summary.locales.clear();
		report.summary.defined = 0;
		report.summary.defined_per_locale.clear();

		let out = rendered(&report, &config());
		assert!(
			out.contains("Warning: no locale was checked"),
			"an inapplicable run must say so: {out}"
		);
		assert!(
			!out.contains("All translation keys validated"),
			"it must not claim success: {out}"
		);
		assert!(out.contains("Checked 0 locales: none"), "{out}");
		// Nothing failed. No failure line follows.
		assert!(!out.contains("Validation failed"), "{out}");
	}

	#[test]
	fn colour_is_emitted_only_when_asked_for() {
		let mut out = Vec::new();
		render(&clean(), &config(), true, &mut out).expect("writing to a Vec cannot fail");
		let text = String::from_utf8(out).expect("the output is UTF-8");
		assert!(text.contains("\x1b[32m"), "{text:?}");

		assert!(!rendered(&clean(), &config()).contains('\x1b'));
	}

	#[test]
	fn rendering_is_reproducible() {
		let report = clean();
		let config = config();
		assert_eq!(rendered(&report, &config), rendered(&report, &config));
	}

	#[test]
	fn the_json_format_is_dispatched_to() {
		let mut config = config();
		config.format = OutputFormat::Json;
		config.color = ColorChoice::Never;
		let mut out = Vec::new();
		super::super::render(&clean(), &config, false, &mut out)
			.expect("writing to a Vec cannot fail");
		let text = String::from_utf8(out).expect("the output is UTF-8");
		assert!(text.starts_with('{'), "{text}");
	}
}
