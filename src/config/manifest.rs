// src/config/manifest.rs

//! Discovers crate layout from Cargo.toml.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Askama template directory name.
const TEMPLATE_DIR: &str = "templates";

#[derive(Debug, Deserialize)]
struct Manifest {
	workspace: Option<Workspace>,
	package: Option<Package>,
}

#[derive(Debug, Default, Deserialize)]
struct Workspace {
	#[serde(default)]
	members: Vec<String>,
	#[serde(default)]
	exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
	#[allow(dead_code)]
	name: Option<String>,
}

/// Lists crate directories from the manifest.
///
/// # Errors
///
/// - Fails if the manifest is missing.
/// - Fails if the manifest is invalid.
/// - Fails if no package or workspace exists.
///
pub fn members(root: &Path, manifest_path: &Path) -> Result<Vec<PathBuf>> {
	let text = std::fs::read_to_string(manifest_path).with_context(|| {
		format!(
			"cannot read the manifest: {}. Pass --manifest-path.",
			manifest_path.display()
		)
	})?;
	let manifest: Manifest = toml::from_str(&text)
		.with_context(|| format!("cannot parse the manifest: {}", manifest_path.display()))?;

	let workspace = manifest.workspace.unwrap_or_default();
	let mut found = BTreeSet::new();

	if !workspace.members.is_empty() {
		let excluded = patterns(&workspace.exclude, manifest_path)?;
		for pattern in &workspace.members {
			for member in expand(root, pattern, manifest_path)? {
				if is_excluded(root, &member, &excluded, &workspace.exclude) {
					continue;
				}
				found.insert(member);
			}
		}
	}

	// The root may also be a package.
	if manifest.package.is_some() {
		found.insert(root.to_path_buf());
	}

	anyhow::ensure!(
		!found.is_empty(),
		"{} declares neither [package] nor [workspace]. Use --src instead.",
		manifest_path.display()
	);

	Ok(found.into_iter().collect())
}

/// Each member's `src` directory.
///
/// Skips members without a `src` directory.
#[must_use]
pub fn src_dirs(members: &[PathBuf]) -> Vec<PathBuf> {
	subdirs(members, "src")
}

/// Each member's `templates` directory.
#[must_use]
pub fn template_dirs(members: &[PathBuf]) -> Vec<PathBuf> {
	subdirs(members, TEMPLATE_DIR)
}

fn subdirs(members: &[PathBuf], name: &str) -> Vec<PathBuf> {
	members
		.iter()
		.map(|m| m.join(name))
		.filter(|p| p.is_dir())
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect()
}

/// Locale names from the locales subdirectories.
///
/// # Errors
///
/// - Fails if the directory is unreadable.
pub fn discover_locales(locales_dir: &Path) -> Result<Vec<String>> {
	if !locales_dir.is_dir() {
		return Ok(Vec::new());
	}

	let entries = std::fs::read_dir(locales_dir).with_context(|| {
		format!(
			"cannot read the locales directory: {}",
			locales_dir.display()
		)
	})?;

	let mut locales = BTreeSet::new();
	for entry in entries {
		let entry = entry.with_context(|| {
			format!(
				"cannot read a locales directory entry: {}",
				locales_dir.display()
			)
		})?;
		if !entry.path().is_dir() {
			continue;
		}
		let name = entry.file_name().to_string_lossy().into_owned();
		// Skip dotted directories.
		if name.starts_with('.') {
			continue;
		}
		locales.insert(name);
	}

	Ok(locales.into_iter().collect())
}

/// Expands a member pattern into paths.
fn expand(root: &Path, pattern: &str, manifest_path: &Path) -> Result<Vec<PathBuf>> {
	let joined = root.join(pattern);

	if !pattern.contains(['*', '?', '[']) {
		return Ok(if joined.is_dir() {
			vec![joined]
		} else {
			Vec::new()
		});
	}

	let as_str = joined.to_str().with_context(|| {
		format!(
			"member pattern {pattern:?} in {} is not valid UTF-8",
			manifest_path.display()
		)
	})?;
	let paths = glob::glob(as_str).with_context(|| {
		format!(
			"member pattern {pattern:?} in {} is not a valid glob",
			manifest_path.display()
		)
	})?;

	let mut found = Vec::new();
	for path in paths {
		let Ok(path) = path else { continue };
		// A glob must only match a crate.
		if path.is_dir() && path.join("Cargo.toml").is_file() {
			found.push(path);
		}
	}
	Ok(found)
}

/// Compiles the `workspace.exclude` entries into globs.
fn patterns(raw: &[String], manifest_path: &Path) -> Result<Vec<glob::Pattern>> {
	raw.iter()
		.map(|p| {
			glob::Pattern::new(p).with_context(|| {
				format!(
					"exclude pattern {p:?} in {} is not a valid glob",
					manifest_path.display()
				)
			})
		})
		.collect()
}

/// Whether a member is excluded.
fn is_excluded(root: &Path, member: &Path, compiled: &[glob::Pattern], raw: &[String]) -> bool {
	let Ok(relative) = member.strip_prefix(root) else {
		return false;
	};
	compiled.iter().any(|p| p.matches_path(relative))
		|| raw.iter().any(|r| relative.starts_with(Path::new(r)))
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Builds a throwaway tree and returns its root.
	fn scratch(name: &str, files: &[(&str, &str)]) -> PathBuf {
		let root = std::env::temp_dir().join(format!("flynt-{}-{name}", std::process::id()));
		let _ = std::fs::remove_dir_all(&root);
		for (path, contents) in files {
			let full = root.join(path);
			std::fs::create_dir_all(full.parent().expect("a file has a parent"))
				.expect("cannot create the scratch directory");
			std::fs::write(&full, contents).expect("cannot write the scratch file");
		}
		root
	}

	/// Renders paths relative to the root.
	fn names(root: &Path, found: &[PathBuf]) -> Vec<String> {
		found
			.iter()
			.map(|p| match p.strip_prefix(root) {
				Ok(relative) => {
					let text = relative.to_string_lossy().replace('\\', "/");
					if text.is_empty() {
						".".to_owned()
					} else {
						text
					}
				}
				Err(_) => ".".to_owned(),
			})
			.collect()
	}

	#[test]
	fn workspace_yields_members() {
		let root = scratch(
			"members",
			&[
				("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
				("a/Cargo.toml", ""),
				("a/src/lib.rs", ""),
				("b/Cargo.toml", ""),
				("b/src/lib.rs", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec!["a", "b"]);
		assert_eq!(names(&root, &src_dirs(&found)), vec!["a/src", "b/src"]);
	}

	#[test]
	fn member_globs_respect_exclude() {
		let root = scratch(
			"globs",
			&[
				(
					"Cargo.toml",
					"[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/skipped\"]\n",
				),
				("crates/one/Cargo.toml", ""),
				("crates/one/src/lib.rs", ""),
				("crates/two/Cargo.toml", ""),
				("crates/skipped/Cargo.toml", ""),
				// Non-crate files must be skipped.
				("crates/notacrate/README.md", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec!["crates/one", "crates/two"]);
	}

	#[test]
	fn single_crate_yields_root() {
		let root = scratch(
			"single",
			&[
				("Cargo.toml", "[package]\nname = \"solo\"\n"),
				("src/main.rs", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec!["."]);
		assert_eq!(names(&root, &src_dirs(&found)), vec!["src"]);
	}

	#[test]
	fn empty_workspace_yields_root() {
		let root = scratch(
			"empty-workspace",
			&[
				("Cargo.toml", "[package]\nname = \"solo\"\n[workspace]\n"),
				("src/lib.rs", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec!["."]);
	}

	#[test]
	fn root_yields_both_entries() {
		let root = scratch(
			"both",
			&[
				(
					"Cargo.toml",
					"[package]\nname = \"top\"\n[workspace]\nmembers = [\"sub\"]\n",
				),
				("src/lib.rs", ""),
				("sub/Cargo.toml", ""),
				("sub/src/lib.rs", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec![".", "sub"]);
	}

	#[test]
	fn default_members_is_ignored() {
		let root = scratch(
			"default-members",
			&[
				(
					"Cargo.toml",
					"[workspace]\nmembers = [\"app\", \"types\"]\ndefault-members = [\"app\"]\n",
				),
				("app/Cargo.toml", ""),
				("app/src/main.rs", ""),
				("types/Cargo.toml", ""),
				("types/src/lib.rs", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec!["app", "types"]);
	}

	#[test]
	fn member_without_src_skipped() {
		let root = scratch(
			"no-src",
			&[
				("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
				("a/Cargo.toml", ""),
				("a/src/lib.rs", ""),
				("b/Cargo.toml", ""),
				("b/lib/other.rs", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec!["a", "b"]);
		assert_eq!(names(&root, &src_dirs(&found)), vec!["a/src"]);
	}

	#[test]
	fn templates_found_per_member() {
		let root = scratch(
			"templates",
			&[
				("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
				("a/Cargo.toml", ""),
				("a/templates/home.html", ""),
				("b/Cargo.toml", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &template_dirs(&found)), vec!["a/templates"]);
	}

	#[test]
	fn missing_manifest_names_flag() {
		let root = scratch("missing", &[("src/lib.rs", "")]);
		let err =
			members(&root, &root.join("Cargo.toml")).expect_err("a missing manifest must fail");
		let text = format!("{err:#}");
		assert!(text.contains("cannot read the manifest"), "{text}");
		assert!(text.contains("--manifest-path"), "{text}");
	}

	#[test]
	fn malformed_manifest_errors() {
		let root = scratch("malformed", &[("Cargo.toml", "[package\nname =")]);
		let err = members(&root, &root.join("Cargo.toml")).expect_err("invalid TOML must fail");
		assert!(format!("{err:#}").contains("cannot parse the manifest"));
	}

	#[test]
	fn manifest_without_tables_errors() {
		let root = scratch(
			"neither",
			&[("Cargo.toml", "[dependencies]\nanyhow = \"1\"\n")],
		);
		let err = members(&root, &root.join("Cargo.toml"))
			.expect_err("a manifest with nothing to scan must fail");
		let text = format!("{err:#}");
		assert!(text.contains("neither [package] nor [workspace]"), "{text}");
		assert!(text.contains("--src"), "{text}");
	}

	#[test]
	fn locales_from_subdirectories() {
		let root = scratch(
			"locales",
			&[
				("locales/en/app.ftl", ""),
				("locales/fr/app.ftl", ""),
				("locales/.hidden/app.ftl", ""),
				// A stray file, not a locale.
				("locales/README.md", ""),
			],
		);
		let found = discover_locales(&root.join("locales")).expect("the directory is readable");
		assert_eq!(found, vec!["en".to_owned(), "fr".to_owned()]);
	}

	#[test]
	fn missing_locales_dir_empty() {
		let root = scratch("no-locales", &[("Cargo.toml", "[package]\nname = \"x\"\n")]);
		let found = discover_locales(&root.join("locales"))
			.expect("a missing directory is not an error here");
		assert!(found.is_empty());
	}
}
