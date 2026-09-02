// tests/determinism.rs

//! Two runs must produce identical bytes.

mod support;

use flynt::config::OutputFormat;
use support::config;

/// Renders a fixture's report as JSON.
fn json(name: &str) -> String {
	let mut config = config(name);
	config.format = OutputFormat::Json;
	let report = flynt::check(&config).unwrap_or_else(|e| panic!("cannot check {name:?}: {e:#}"));

	let mut out = Vec::new();
	flynt::report::render(&report, &config, false, &mut out).expect("writing to a Vec cannot fail");
	String::from_utf8(out).expect("the output is UTF-8")
}

#[test]
fn clean_tree_renders_identically() {
	assert_eq!(json("clean"), json("clean"));
}

#[test]
fn findings_render_identically() {
	for fixture in [
		"missing_key",
		"inconsistent",
		"duplicate",
		"duplicate_same_file",
		"unused",
		"bad_ftl",
	] {
		assert_eq!(
			json(fixture),
			json(fixture),
			"{fixture} is not deterministic"
		);
	}
}

#[test]
fn missing_locales_order_stable() {
	let report = {
		let config = config("missing_key");
		flynt::check(&config).expect("the fixture checks")
	};
	let again = {
		let config = config("missing_key");
		flynt::check(&config).expect("the fixture checks")
	};
	assert_eq!(
		report.missing_keys[0].missing_in,
		again.missing_keys[0].missing_in
	);
}

#[test]
fn finding_lists_are_sorted() {
	let config = config("unicode");
	let report = flynt::check(&config).expect("the fixture checks");

	let keys: Vec<&str> = report.missing_keys.iter().map(|k| k.key.as_str()).collect();
	let mut sorted = keys.clone();
	sorted.sort_unstable();
	assert_eq!(keys, sorted, "missing keys are not sorted");
}

#[test]
fn defined_count_order_independent() {
	let config = config("inconsistent");
	let report = flynt::check(&config).expect("the fixture checks");
	assert_eq!(report.summary.defined, 2);
	assert_eq!(report.summary.defined_per_locale["en"], 1);
	assert_eq!(report.summary.defined_per_locale["fr"], 2);
}
