// tests/cli.rs

//! Process-level checks: exit codes and formats.

mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::fixture;

/// Runs the built binary.
fn run_in(cwd: &Path, args: &[&str]) -> Output {
	Command::new(env!("CARGO_BIN_EXE_flynt"))
		.current_dir(cwd)
		.args(args)
		// Keep output stable across run locations.
		.env("NO_COLOR", "1")
		.output()
		.expect("cannot run the flynt binary")
}

/// Runs against a fixture from elsewhere.
fn run(name: &str, args: &[&str]) -> Output {
	let path = fixture(name);
	let path = path.to_str().expect("the fixture path is UTF-8");
	let mut all = vec![path];
	all.extend_from_slice(args);
	run_in(Path::new(env!("CARGO_MANIFEST_DIR")), &all)
}

fn code(output: &Output) -> i32 {
	output
		.status
		.code()
		.expect("the process was not killed by a signal")
}

fn stdout(output: &Output) -> String {
	String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
	String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

#[test]
fn clean_tree_exits_zero() {
	let output = run("clean", &[]);
	assert_eq!(code(&output), 0, "{}", stderr(&output));
	assert!(stdout(&output).contains("All translation keys validated"));
}

#[test]
fn findings_exit_one() {
	for fixture in ["missing_key", "inconsistent", "duplicate", "bad_ftl"] {
		let output = run(fixture, &[]);
		assert_eq!(code(&output), 1, "{fixture}: {}", stdout(&output));
		assert!(
			stdout(&output).contains("Validation failed"),
			"{fixture}: {}",
			stdout(&output)
		);
	}
}

#[test]
fn configuration_problem_exits_two() {
	for fixture in ["no_locales", "empty_locales"] {
		let output = run(fixture, &[]);
		assert_eq!(code(&output), 2, "{fixture}: {}", stderr(&output));
		assert!(
			stderr(&output).starts_with("flynt: error:"),
			"{fixture}: {}",
			stderr(&output)
		);
	}
}

#[test]
fn missing_path_exits_two() {
	let output = run_in(
		Path::new(env!("CARGO_MANIFEST_DIR")),
		&["./no-such-directory-xyz"],
	);
	assert_eq!(code(&output), 2);
	assert!(stderr(&output).contains("cannot read the directory to lint"));
}

#[test]
fn warning_alone_exits_zero() {
	let output = run("unused", &[]);
	assert_eq!(code(&output), 0, "{}", stderr(&output));
	assert!(stdout(&output).contains("Warning:"));
	assert!(!stdout(&output).contains("Validation failed"));
}

#[test]
fn unused_promotes_to_error() {
	let output = run("unused", &["--unused", "error"]);
	assert_eq!(code(&output), 1);
	assert!(stdout(&output).contains("Error:"));
}

#[test]
fn json_format_carries_findings() {
	let output = run("missing_key", &["--format", "json"]);
	assert_eq!(code(&output), 1);

	let value: serde_json::Value =
		serde_json::from_str(&stdout(&output)).expect("the output is valid JSON");
	assert_eq!(value["schema_version"], 1);
	assert_eq!(value["missing_keys"][0]["key"], "tpl-orphan");
	assert_eq!(value["missing_keys"][0]["missing_in"][0], "fr");
	assert_eq!(
		value["missing_keys"][0]["usages"][0]["at"]["file"],
		"templates/home.html"
	);
	assert_eq!(value["summary"]["reference_locale"], "en");
}

#[test]
fn config_file_root_used() {
	let output = run("config_file", &[]);
	assert_eq!(code(&output), 0, "{}", stderr(&output));
}

#[test]
fn flag_beats_config_file() {
	let output = run("config_file", &["--locales-dir", "locales"]);
	assert_eq!(code(&output), 2, "{}", stdout(&output));
	assert!(stderr(&output).contains("locales directory does not exist"));
}

#[test]
fn no_config_skips_file() {
	let output = run("config_file", &["--no-config"]);
	assert_eq!(code(&output), 2, "{}", stdout(&output));
}

#[test]
fn config_file_named_explicitly() {
	let path = fixture("config_file").join(".flynt.toml");
	let output = run(
		"config_file",
		&["--config", path.to_str().expect("the path is UTF-8")],
	);
	assert_eq!(code(&output), 0, "{}", stderr(&output));
}

#[test]
fn missing_named_config_errors() {
	let output = run("clean", &["--config", "does-not-exist.toml"]);
	assert_eq!(code(&output), 2);
	assert!(stderr(&output).contains("cannot read the config file"));
}

#[test]
fn result_ignores_cwd() {
	let fixture_path = fixture("missing_key");
	let absolute = fixture_path.to_str().expect("the path is UTF-8");
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

	let from_manifest = run_in(manifest, &[absolute]);
	let from_fixture = run_in(&fixture_path, &[absolute]);
	let from_temp = run_in(&std::env::temp_dir(), &[absolute]);

	assert_eq!(stdout(&from_manifest), stdout(&from_fixture));
	assert_eq!(stdout(&from_manifest), stdout(&from_temp));
	assert_eq!(code(&from_manifest), code(&from_temp));
}

#[test]
fn default_path_is_cwd() {
	let output = run_in(&fixture("clean"), &[]);
	assert_eq!(code(&output), 0, "{}", stderr(&output));
}

#[test]
fn quiet_says_nothing() {
	let output = run("clean", &["--quiet"]);
	assert_eq!(code(&output), 0);
	assert!(stdout(&output).is_empty(), "{:?}", stdout(&output));
}

#[test]
fn help_and_version_succeed() {
	for flag in ["--help", "--version"] {
		let output = run_in(Path::new(env!("CARGO_MANIFEST_DIR")), &[flag]);
		assert_eq!(code(&output), 0, "{flag}: {}", stderr(&output));
		assert!(!stdout(&output).is_empty(), "{flag} printed nothing");
	}
}

#[test]
fn help_documents_the_overrides() {
	let output = run_in(Path::new(env!("CARGO_MANIFEST_DIR")), &["--help"]);
	let help = stdout(&output);
	for flag in [
		"--locales-dir",
		"--locales",
		"--src",
		"--add-src",
		"--templates",
		"--filters",
		"--functions",
		"--unused",
		"--format",
	] {
		assert!(
			help.contains(flag),
			"--help does not mention {flag}:\n{help}"
		);
	}
}

#[test]
fn unknown_flag_rejected() {
	let output = run_in(Path::new(env!("CARGO_MANIFEST_DIR")), &["--nope"]);
	assert_ne!(code(&output), 0);
	assert!(stderr(&output).contains("--nope"));
}

#[test]
fn extra_src_dirs_added() {
	let output = run("member_without_src", &["--add-src", "b/lib"]);
	assert_eq!(code(&output), 1, "{}", stdout(&output));
	assert!(
		stdout(&output).contains("b-key-not-scanned"),
		"{}",
		stdout(&output)
	);
}
