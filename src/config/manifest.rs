// src/config/manifest.rs

//! Discovery from `Cargo.toml` and the filesystem.
//!
//! The manifest is parsed directly rather than by shelling out to `cargo metadata`.
//!
//! Flynt only needs directory names.
//!
//! Note that `workspace.default-members` is deliberately ignored.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Directory name conventionally holding Askama templates.
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

/// Lists the crate directories to consider, from the root manifest.
///
/// A workspace yields its `members`. Globs expand, and `exclude` applies. The root itself also
/// counts, when the root manifest is also a package.
///
/// A manifest with no `[workspace]` table, or an empty one, yields just the root.
///
/// # Errors
///
/// Returns an error when the manifest is missing, unreadable, or not valid TOML. It also returns an
/// error when the manifest declares neither `[package]` nor `[workspace]`, or when a member pattern
/// is not a valid glob.
pub fn members(root: &Path, manifest_path: &Path) -> Result<Vec<PathBuf>> {
	let text = std::fs::read_to_string(manifest_path).with_context(|| {
		format!(
			"cannot read the manifest: {}\nPass --manifest-path to point at it, or run flynt from \
			 the directory that holds it.",
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

	// A root manifest can be both the workspace root and a package of its own.
	if manifest.package.is_some() {
		found.insert(root.to_path_buf());
	}

	anyhow::ensure!(
		!found.is_empty(),
		"{} declares neither [package] nor [workspace] members, so there is nothing to scan.\n\
		 Use --src to say which directories hold the Rust code.",
		manifest_path.display()
	);

	Ok(found.into_iter().collect())
}

/// The `src` directory of every member that has one.
///
/// A member without a `src` directory is skipped.
#[must_use]
pub fn src_dirs(members: &[PathBuf]) -> Vec<PathBuf> {
	subdirs(members, "src")
}

/// The `templates` directory of every member that has one.
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

/// Locale names, taken from the subdirectories of the locales directory.
///
/// A missing directory yields an empty list. This lets `--require-locales=false` work as intended.
///
/// # Errors
///
/// Returns an error when the directory exists but cannot be read.
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
				"cannot read an entry of the locales directory: {}",
				locales_dir.display()
			)
		})?;
		if !entry.path().is_dir() {
			continue;
		}
		let name = entry.file_name().to_string_lossy().into_owned();
		// Skip dotted directories such as `.git` or `.DS_Store` leftovers.
		if name.starts_with('.') {
			continue;
		}
		locales.insert(name);
	}

	Ok(locales.into_iter().collect())
}

/// Expands one member pattern into the directories it names.
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
			"the member pattern {pattern:?} in {} does not form a valid UTF-8 path",
			manifest_path.display()
		)
	})?;
	let paths = glob::glob(as_str).with_context(|| {
		format!(
			"the member pattern {pattern:?} in {} is not a valid glob",
			manifest_path.display()
		)
	})?;

	let mut found = Vec::new();
	for path in paths {
		let Ok(path) = path else { continue };
		// A glob must only ever match a crate.
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
					"the exclude pattern {p:?} in {} is not a valid glob",
					manifest_path.display()
				)
			})
		})
		.collect()
}

/// Whether a member is excluded, by glob or as a plain directory prefix.
///
/// Cargo accepts a bare directory name in `exclude`. As a glob, that name would only match the
/// exact path.
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

	/// Builds a throwaway tree under the temp directory. Returns its root.
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

	/// Renders the found paths relative to the root, using `.` for the root itself.
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
	fn a_workspace_yields_every_member() {
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
	fn member_globs_expand_and_exclude_applies() {
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
				// Not a crate. A glob must not pick it up.
				("crates/notacrate/README.md", ""),
			],
		);
		let found = members(&root, &root.join("Cargo.toml")).expect("the manifest is valid");
		assert_eq!(names(&root, &found), vec!["crates/one", "crates/two"]);
	}

	#[test]
	fn a_single_crate_yields_its_own_root() {
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
	fn an_empty_workspace_table_means_a_single_crate() {
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
	fn a_root_that_is_both_package_and_workspace_yields_both() {
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
	fn a_member_without_a_src_directory_is_skipped() {
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
	fn template_directories_are_found_next_to_each_member() {
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
	fn a_missing_manifest_is_an_error_naming_the_flag() {
		let root = scratch("missing", &[("src/lib.rs", "")]);
		let err =
			members(&root, &root.join("Cargo.toml")).expect_err("a missing manifest must fail");
		let text = format!("{err:#}");
		assert!(text.contains("cannot read the manifest"), "{text}");
		assert!(text.contains("--manifest-path"), "{text}");
	}

	#[test]
	fn a_malformed_manifest_is_an_error() {
		let root = scratch("malformed", &[("Cargo.toml", "[package\nname =")]);
		let err = members(&root, &root.join("Cargo.toml")).expect_err("invalid TOML must fail");
		assert!(format!("{err:#}").contains("cannot parse the manifest"));
	}

	#[test]
	fn a_manifest_with_neither_table_is_an_error() {
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
	fn locales_come_from_the_subdirectories() {
		let root = scratch(
			"locales",
			&[
				("locales/en/app.ftl", ""),
				("locales/fr/app.ftl", ""),
				("locales/.hidden/app.ftl", ""),
				// A stray file must not be mistaken for a locale.
				("locales/README.md", ""),
			],
		);
		let found = discover_locales(&root.join("locales")).expect("the directory is readable");
		assert_eq!(found, vec!["en".to_owned(), "fr".to_owned()]);
	}

	#[test]
	fn a_missing_locales_directory_yields_no_locales_rather_than_an_error() {
		let root = scratch("no-locales", &[("Cargo.toml", "[package]\nname = \"x\"\n")]);
		let found = discover_locales(&root.join("locales"))
			.expect("a missing directory is not an error here");
		assert!(found.is_empty());
	}
}
