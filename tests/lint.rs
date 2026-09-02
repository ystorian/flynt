// tests/lint.rs

//! Library-level checks over the fixture trees.

mod support;

use support::{assert_clean, check, check_with, config, fixture};

#[test]
fn clean_workspace_reports_nothing() {
	let report = check("clean");
	assert_clean(&report);
	assert_eq!(report.summary.used, 4);
	assert_eq!(report.summary.defined, 4);
	assert_eq!(
		report.summary.locales,
		vec!["en".to_owned(), "fr".to_owned()]
	);
	assert_eq!(report.summary.reference_locale, "en");
}

#[test]
fn every_member_is_scanned() {
	// Both `app` and `types` contribute.
	let report = check("clean");
	let mut keys: Vec<&str> = report
		.summary
		.defined_per_locale
		.keys()
		.map(String::as_str)
		.collect();
	keys.sort_unstable();
	assert_eq!(keys, vec!["en", "fr"]);
	assert_clean(&report);
}

#[test]
fn missing_key_reported() {
	let report = check("missing_key");
	assert_eq!(report.missing_keys.len(), 1);
	let finding = &report.missing_keys[0];
	assert_eq!(finding.key, "tpl-orphan");
	assert_eq!(finding.missing_in, vec!["fr".to_owned()]);
	assert_eq!(finding.usages.len(), 1);
	assert_eq!(finding.usages[0].at.line, 2);
	assert!(report.has_errors());
	assert_eq!(report.exit_code(), 1);
}

#[test]
fn four_byte_char_column() {
	// Every file holds a 4-byte emoji.
	let report = check("unicode");
	let keys: Vec<&str> = report.missing_keys.iter().map(|k| k.key.as_str()).collect();
	assert_eq!(keys, vec!["rust-key", "tpl-key"]);

	let rust = &report.missing_keys[0].usages[0];
	assert_eq!(rust.at.line, 2);
	assert_eq!(rust.at.column, 45);

	let template = &report.missing_keys[1].usages[0];
	assert_eq!(template.at.line, 1);
	assert_eq!(template.at.column, 11);

	// The emoji does not disturb parsing.
	assert_eq!(report.summary.defined, 3);
	assert_eq!(report.summary.used, 3);
}

#[test]
fn drifting_locales_reported() {
	let report = check("inconsistent");
	assert_eq!(report.inconsistent_keys.len(), 1);
	let finding = &report.inconsistent_keys[0];
	assert_eq!(finding.key, "only-fr");
	assert_eq!(finding.present_in, vec!["fr".to_owned()]);
	assert_eq!(finding.missing_in, vec!["en".to_owned()]);
	assert_eq!(report.exit_code(), 1);
}

#[test]
fn duplicate_key_reported_once() {
	let report = check("duplicate");
	assert_eq!(report.duplicate_keys.len(), 1);
	let finding = &report.duplicate_keys[0];
	assert_eq!(finding.key, "dup-key");
	assert_eq!(finding.locale, "en");
	let files: Vec<String> = finding
		.definitions
		.iter()
		.map(|d| d.at.file.display().to_string().replace('\\', "/"))
		.collect();
	assert_eq!(files, vec!["locales/en/a.ftl", "locales/en/b.ftl"]);
	assert_eq!(report.exit_code(), 1);
}

#[test]
fn duplicate_lines_one_file() {
	let report = check("duplicate_same_file");
	assert_eq!(report.duplicate_keys.len(), 1);
	let lines: Vec<usize> = report.duplicate_keys[0]
		.definitions
		.iter()
		.map(|d| d.at.line)
		.collect();
	assert_eq!(lines, vec![1, 3]);
}

#[test]
fn unused_key_warns_only() {
	let report = check("unused");
	let keys: Vec<&str> = report.unused_keys.iter().map(|k| k.key.as_str()).collect();
	assert_eq!(keys, vec!["err-404", "never-used-key"]);
	assert!(report.has_warnings());
	assert!(!report.has_errors());
	assert_eq!(report.exit_code(), 0);
}

#[test]
fn unused_keys_become_errors() {
	let report = check_with("unused", |cli| {
		cli.unused = Some(flynt::Severity::Error);
	});
	assert_eq!(report.unused_keys.len(), 2);
	assert!(report.has_errors());
	assert_eq!(report.exit_code(), 1);
}

#[test]
fn unused_keys_switch_off() {
	let report = check_with("unused", |cli| {
		cli.unused = Some(flynt::Severity::Allow);
	});
	assert!(report.unused_keys.is_empty());
	assert_eq!(report.exit_code(), 0);
}

#[test]
fn ignore_glob_silences_keys() {
	let report = check_with("unused", |cli| {
		cli.ignore_unused = Some(vec!["err-*".to_owned()]);
	});
	let keys: Vec<&str> = report.unused_keys.iter().map(|k| k.key.as_str()).collect();
	assert_eq!(keys, vec!["never-used-key"]);
}

#[test]
fn lone_crate_scans_src() {
	let report = check("single_crate");
	assert_clean(&report);
	assert_eq!(report.summary.used, 2);
	assert_eq!(report.summary.locales, vec!["en".to_owned()]);
}

#[test]
fn member_globs_respect_excludes() {
	// `crates/skipped` is excluded.
	let report = check("glob_members");
	assert_clean(&report);
	assert_eq!(report.summary.used, 2);
}

#[test]
fn member_without_src_skipped() {
	let report = check("member_without_src");
	assert_clean(&report);
	assert_eq!(report.summary.used, 1);
}

#[test]
fn missing_locales_dir_refused() {
	let config = config("no_locales");
	let error = flynt::check(&config).expect_err("a missing locales directory must fail");
	let text = format!("{error:#}");
	assert!(text.contains("locales directory does not exist"), "{text}");
	assert!(text.contains("--locales-dir"), "{text}");
}

#[test]
fn empty_locales_dir_refused() {
	let config = config("empty_locales");
	let error = flynt::check(&config).expect_err("an empty locales directory must fail");
	assert!(format!("{error:#}").contains("no locale found"));
}

#[test]
fn broken_file_keeps_keys() {
	let report = check("bad_ftl");
	assert_eq!(report.parse_errors.len(), 1);
	let error = &report.parse_errors[0];
	assert_eq!(
		error.at.file.display().to_string().replace('\\', "/"),
		"locales/en/broken.ftl"
	);
	assert_eq!(error.at.line, 2);
	assert!(
		!error.message.is_empty(),
		"the parser message is carried through"
	);
	// Recovery keeps the good entries.
	assert_eq!(report.summary.defined, 2);
	assert!(report.has_errors());
	assert_eq!(report.exit_code(), 1);
}

#[test]
fn custom_names_are_honoured() {
	let report = check_with("custom_names", |cli| {
		cli.filters = Some(vec!["x".to_owned()]);
		cli.functions = Some(vec!["translate".to_owned()]);
		cli.template_ext = Some(vec!["j2".to_owned()]);
	});
	assert_clean(&report);
	assert_eq!(report.summary.used, 2);
}

#[test]
fn doc_comments_not_usages() {
	let report = check("comments");
	assert_clean(&report);
	assert_eq!(report.summary.used, 1);
}

#[test]
fn config_file_reads_root() {
	let report = check("config_file");
	assert_clean(&report);
	assert_eq!(report.summary.used, 2);
}

#[test]
fn cli_overrides_config_file() {
	let cli = flynt::PartialConfig {
		root: Some(fixture("config_file")),
		locales_dir: Some("locales".into()),
		..flynt::PartialConfig::default()
	};
	let config = flynt::config::load(&cli).expect("the configuration loads");
	// The flag overrides the file's value.
	let error = flynt::check(&config).expect_err("the CLI value must win");
	assert!(format!("{error:#}").contains("locales directory does not exist"));
}

#[test]
fn config_file_is_ignorable() {
	let cli = flynt::PartialConfig {
		root: Some(fixture("config_file")),
		no_config: Some(true),
		..flynt::PartialConfig::default()
	};
	let config = flynt::config::load(&cli).expect("the configuration loads");
	assert!(config.locales_dir.ends_with("locales"), "{config:?}");
	assert_eq!(config.filters, vec!["t".to_owned(), "tn".to_owned()]);
}

#[test]
fn findings_paths_are_relative() {
	let report = check("missing_key");
	let file = &report.missing_keys[0].usages[0].at.file;
	assert!(file.is_relative(), "{file:?}");
	assert_eq!(
		file.display().to_string().replace('\\', "/"),
		"templates/home.html"
	);
}
