// tests/support/mod.rs

//! Shared helpers for the integration tests.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, io};

use flynt::config::{self, Config, PartialConfig};
use flynt::model::Report;

/// Fixture file names (repository and working copy).
const RENAMED: &[(&str, &str)] = &[
	("_Cargo.toml", "Cargo.toml"),
	("_flynt.toml", ".flynt.toml"),
];

/// Marks a directory that must stay empty.
const KEEP_DIR: &str = "_gitkeep";

/// Fixture trees already copied this run.
static MATERIALIZED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Absolute path of a usable fixture copy.
pub fn fixture(name: &str) -> PathBuf {
	// A scratch directory under target.
	let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
		.join(env!("CARGO_CRATE_NAME"))
		.join(name);

	// The lock is held for the copy.
	let mut done = MATERIALIZED
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	if done.insert(name.to_owned()) {
		let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("tests/fixtures")
			.join(name);
		assert!(
			source.is_dir(),
			"there is no {name:?} fixture at {}",
			source.display()
		);
		materialize(&source, &target).unwrap_or_else(|e| {
			panic!(
				"cannot copy the {name:?} fixture to {}: {e}",
				target.display()
			)
		});
	}

	target
}

/// Copies a fixture tree, renaming destination files.
fn materialize(source: &Path, target: &Path) -> io::Result<()> {
	if target.exists() {
		fs::remove_dir_all(target)?;
	}
	copy_tree(source, target)
}

fn copy_tree(source: &Path, target: &Path) -> io::Result<()> {
	fs::create_dir_all(target)?;
	for entry in fs::read_dir(source)? {
		let entry = entry?;
		let name = entry.file_name();
		if name == KEEP_DIR {
			continue;
		}
		let name = RENAMED
			.iter()
			.find(|(stored, _)| name == *stored)
			.map_or_else(|| name.clone(), |(_, used)| (*used).into());
		let to = target.join(name);
		if entry.file_type()?.is_dir() {
			copy_tree(&entry.path(), &to)?;
		} else {
			fs::copy(entry.path(), &to)?;
		}
	}
	Ok(())
}

/// Loads the configuration for a fixture.
pub fn config_with(name: &str, adjust: impl FnOnce(&mut PartialConfig)) -> Config {
	let mut cli = PartialConfig {
		root: Some(fixture(name)),
		..PartialConfig::default()
	};
	adjust(&mut cli);
	config::load(&cli).unwrap_or_else(|e| panic!("cannot configure the {name:?} fixture: {e:#}"))
}

/// Loads a fixture's configuration with only defaults.
pub fn config(name: &str) -> Config {
	config_with(name, |_| {})
}

/// Runs every check against a fixture.
pub fn check(name: &str) -> Report {
	let config = config(name);
	flynt::check(&config).unwrap_or_else(|e| panic!("cannot check the {name:?} fixture: {e:#}"))
}

/// Runs every check with an adjusted configuration.
pub fn check_with(name: &str, adjust: impl FnOnce(&mut PartialConfig)) -> Report {
	let config = config_with(name, adjust);
	flynt::check(&config).unwrap_or_else(|e| panic!("cannot check the {name:?} fixture: {e:#}"))
}

/// Asserts a report has no findings.
pub fn assert_clean(report: &Report) {
	assert!(
		report.missing_keys.is_empty(),
		"unexpected missing keys: {:?}",
		report.missing_keys
	);
	assert!(
		report.inconsistent_keys.is_empty(),
		"unexpected inconsistent keys: {:?}",
		report.inconsistent_keys
	);
	assert!(
		report.duplicate_keys.is_empty(),
		"unexpected duplicate keys: {:?}",
		report.duplicate_keys
	);
	assert!(
		report.unused_keys.is_empty(),
		"unexpected unused keys: {:?}",
		report.unused_keys
	);
	assert!(
		report.parse_errors.is_empty(),
		"unexpected parse errors: {:?}",
		report.parse_errors
	);
	assert!(!report.has_errors());
	assert_eq!(report.exit_code(), 0);
}
