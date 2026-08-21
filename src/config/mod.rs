// src/config/mod.rs

//! Configuration: CLI, `.flynt.toml`, and manifest discovery.
//!
//! Precedence, highest first:
//!
//! 1. command line arguments
//! 2. `.flynt.toml` at the root
//! 3. discovery from `Cargo.toml` and the filesystem
//! 4. built-in defaults

pub mod manifest;
mod partial;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::Deserialize;

pub use partial::PartialConfig;

/// Default name of the configuration file.
pub const CONFIG_FILE: &str = ".flynt.toml";

/// Built-in default for [`Config::locales_dir`].
pub const DEFAULT_LOCALES_DIR: &str = "locales";
/// Built-in default for [`Config::template_ext`].
pub const DEFAULT_TEMPLATE_EXT: &[&str] = &["html"];
/// Built-in default for [`Config::filters`].
pub const DEFAULT_FILTERS: &[&str] = &["t", "tn"];
/// Built-in default for [`Config::functions`].
pub const DEFAULT_FUNCTIONS: &[&str] = &["loc", "loc_with_args"];
/// Built-in default for [`Config::exclude`].
pub const DEFAULT_EXCLUDE: &[&str] = &["target/**"];
/// Locale preferred as the reference when it exists.
pub const PREFERRED_REFERENCE_LOCALE: &str = "en";

/// How a check reports its findings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
	/// Report and fail the run.
	Error,
	/// Report but do not fail the run.
	#[default]
	Warn,
	/// Do not report at all, and skip the check.
	Allow,
}

/// Rendered report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
	/// Human-readable text.
	#[default]
	Text,
	/// Machine-readable JSON.
	Json,
}

/// When to emit ANSI color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorChoice {
	/// Color when the stream is a terminal and `NO_COLOR` is unset.
	#[default]
	Auto,
	/// Always color.
	Always,
	/// Never color.
	Never,
}

/// Manifest and filesystem inspection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovered {
	/// Rust source directories, one per workspace member that has a `src`.
	pub src: Vec<PathBuf>,
	/// Template directories found next to each member.
	pub templates: Vec<PathBuf>,
	/// Locale names, from the subdirectories of the locales directory.
	pub locales: Vec<String>,
}

/// A fully resolved configuration. Every path is absolute.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
	/// The directory being linted. Paths in the report are relative to it.
	pub root: PathBuf,
	/// The root `Cargo.toml`.
	pub manifest_path: PathBuf,
	/// Directory holding one subdirectory per locale.
	pub locales_dir: PathBuf,
	/// Locales to check, sorted.
	pub locales: Vec<String>,
	/// Locale the unused-key check runs against.
	pub reference_locale: String,
	/// Rust source directories to scan.
	pub src: Vec<PathBuf>,
	/// Template directories to scan.
	pub templates: Vec<PathBuf>,
	/// Template file extensions, lowercase and without a leading dot.
	pub template_ext: Vec<String>,
	/// Askama filter names that take a key.
	pub filters: Vec<String>,
	/// Rust function names that take a key.
	pub functions: Vec<String>,
	/// Severity for the unused-key check.
	pub unused: Severity,
	/// Key-name globs never reported as unused.
	pub ignore_unused: Vec<String>,
	/// Whether `key.attribute` counts as a defined key.
	pub attributes: bool,
	/// Whether to match keys inside `//` comment lines.
	pub include_comments: bool,
	/// Whether a missing or empty locales directory is an error.
	pub require_locales: bool,
	/// Path globs to skip, relative to the root.
	pub exclude: Vec<String>,
	/// Whether to follow symbolic links while walking.
	pub follow_links: bool,
	/// Shape of the rendered report.
	pub format: OutputFormat,
	/// When to emit color.
	pub color: ColorChoice,
	/// Whether to stay silent when there is nothing to report.
	pub quiet: bool,
}

/// Loads the configuration for a CLI invocation.
///
/// Canonicalizes the root first. Every later path is then absolute.
///
/// # Errors
///
/// Returns an error when the root does not exist, when the configuration file is unreadable or
/// malformed, or when the manifest cannot be understood.
pub fn load(cli: &PartialConfig) -> Result<Config> {
	let requested = cli.root.clone().unwrap_or_else(|| PathBuf::from("."));
	let root = std::fs::canonicalize(&requested)
		.with_context(|| format!("cannot read the directory to lint: {}", requested.display()))?;
	anyhow::ensure!(
		root.is_dir(),
		"the path to lint is not a directory: {}",
		root.display()
	);

	let file = read_config_file(cli, &root)?;

	// The manifest anchor has to be settled before discovery.
	let manifest_path = first_path(&[cli.manifest_path.as_deref(), file.manifest_path.as_deref()])
		.map_or_else(|| root.join("Cargo.toml"), |p| absolutize(&root, p));

	let locales_dir = first_path(&[cli.locales_dir.as_deref(), file.locales_dir.as_deref()])
		.map_or_else(|| root.join(DEFAULT_LOCALES_DIR), |p| absolutize(&root, p));

	let members = manifest::members(&root, &manifest_path)?;
	let discovered = Discovered {
		src: manifest::src_dirs(&members),
		templates: manifest::template_dirs(&members),
		locales: manifest::discover_locales(&locales_dir)?,
	};

	Ok(resolve(cli, &file, &discovered, &root))
}

/// Reads and parses the configuration file.
///
/// # Errors
///
/// Returns an error when `--config` names a file that cannot be read, or when the file is not a
/// valid TOML.
fn read_config_file(cli: &PartialConfig, root: &Path) -> Result<PartialConfig> {
	if cli.no_config == Some(true) {
		return Ok(PartialConfig::default());
	}

	// An explicit `--config` must exist.
	let (path, required) = match cli.config.as_deref() {
		Some(p) => (absolutize(root, p), true),
		None => (root.join(CONFIG_FILE), false),
	};

	if !required && !path.exists() {
		return Ok(PartialConfig::default());
	}

	let text = std::fs::read_to_string(&path)
		.with_context(|| format!("cannot read the config file: {}", path.display()))?;
	toml::from_str(&text).with_context(|| format!("cannot parse {}", path.display()))
}

/// Merges the configuration layers into a final configuration.
#[must_use]
pub fn resolve(
	cli: &PartialConfig,
	file: &PartialConfig,
	discovered: &Discovered,
	root: &Path,
) -> Config {
	// Replace-lists: the first layer that supplies a value wins.
	let src = pick_paths(
		&[cli.src.as_deref(), file.src.as_deref()],
		&discovered.src,
		root,
	);
	let add_src = collect_paths(&[cli.add_src.as_deref(), file.add_src.as_deref()], root);

	let templates = pick_paths(
		&[cli.templates.as_deref(), file.templates.as_deref()],
		&discovered.templates,
		root,
	);
	let add_templates = collect_paths(
		&[cli.add_templates.as_deref(), file.add_templates.as_deref()],
		root,
	);

	let locales = pick_strings(
		&[cli.locales.as_deref(), file.locales.as_deref()],
		&discovered.locales,
	);
	let reference_locale = first_string(&[
		cli.reference_locale.as_deref(),
		file.reference_locale.as_deref(),
	])
	.map_or_else(|| default_reference_locale(&locales), ToOwned::to_owned);

	Config {
		root: root.to_path_buf(),
		manifest_path: first_path(&[cli.manifest_path.as_deref(), file.manifest_path.as_deref()])
			.map_or_else(|| root.join("Cargo.toml"), |p| absolutize(root, p)),
		locales_dir: first_path(&[cli.locales_dir.as_deref(), file.locales_dir.as_deref()])
			.map_or_else(|| root.join(DEFAULT_LOCALES_DIR), |p| absolutize(root, p)),
		locales,
		reference_locale,
		src: merge_paths(src, add_src),
		templates: merge_paths(templates, add_templates),
		template_ext: pick_strings(
			&[cli.template_ext.as_deref(), file.template_ext.as_deref()],
			&defaults(DEFAULT_TEMPLATE_EXT),
		)
		.into_iter()
		.map(|e| e.trim_start_matches('.').to_lowercase())
		.collect(),
		filters: pick_strings(
			&[cli.filters.as_deref(), file.filters.as_deref()],
			&defaults(DEFAULT_FILTERS),
		),
		functions: pick_strings(
			&[cli.functions.as_deref(), file.functions.as_deref()],
			&defaults(DEFAULT_FUNCTIONS),
		),
		unused: cli.unused.or(file.unused).unwrap_or_default(),
		ignore_unused: pick_strings(
			&[cli.ignore_unused.as_deref(), file.ignore_unused.as_deref()],
			&[],
		),
		attributes: cli.attributes.or(file.attributes).unwrap_or(true),
		include_comments: cli
			.include_comments
			.or(file.include_comments)
			.unwrap_or(false),
		require_locales: cli.require_locales.or(file.require_locales).unwrap_or(true),
		exclude: pick_strings(
			&[cli.exclude.as_deref(), file.exclude.as_deref()],
			&defaults(DEFAULT_EXCLUDE),
		),
		follow_links: cli.follow_links.or(file.follow_links).unwrap_or(true),
		format: cli.format.or(file.format).unwrap_or_default(),
		color: cli.color.or(file.color).unwrap_or_default(),
		quiet: cli.quiet.or(file.quiet).unwrap_or(false),
	}
}

impl Config {
	/// Makes `path` relative to the root.
	#[must_use]
	pub fn relative(&self, path: &Path) -> PathBuf {
		path.strip_prefix(&self.root).unwrap_or(path).to_path_buf()
	}

	/// Whether `ext` is one of the configured template extensions.
	#[must_use]
	pub fn is_template_ext(&self, ext: &str) -> bool {
		self.template_ext.contains(&ext.to_lowercase())
	}
}

/// Picks the reference locale. `en` wins when present. Otherwise, the first sorted locale wins.
fn default_reference_locale(locales: &[String]) -> String {
	locales
		.iter()
		.find(|l| *l == PREFERRED_REFERENCE_LOCALE)
		.or_else(|| locales.first())
		.cloned()
		.unwrap_or_else(|| PREFERRED_REFERENCE_LOCALE.to_owned())
}

/// Joins `path` onto `root` unless it is already absolute.
fn absolutize(root: &Path, path: &Path) -> PathBuf {
	if path.is_absolute() {
		path.to_path_buf()
	} else {
		root.join(path)
	}
}

fn first_path<'a>(layers: &[Option<&'a Path>]) -> Option<&'a Path> {
	layers.iter().copied().flatten().next()
}

fn first_string<'a>(layers: &[Option<&'a str>]) -> Option<&'a str> {
	layers.iter().copied().flatten().next()
}

fn defaults(values: &[&str]) -> Vec<String> {
	values.iter().map(|v| (*v).to_owned()).collect()
}

/// The first layer that supplies a list wins. Otherwise, the fallback applies.
fn pick_strings(layers: &[Option<&[String]>], fallback: &[String]) -> Vec<String> {
	layers
		.iter()
		.copied()
		.flatten()
		.next()
		.unwrap_or(fallback)
		.to_vec()
}

/// As `pick_strings` with path from root.
fn pick_paths(layers: &[Option<&[PathBuf]>], fallback: &[PathBuf], root: &Path) -> Vec<PathBuf> {
	layers.iter().copied().flatten().next().map_or_else(
		|| fallback.to_vec(),
		|paths| paths.iter().map(|p| absolutize(root, p)).collect(),
	)
}

/// Unions every layer.
fn collect_paths(layers: &[Option<&[PathBuf]>], root: &Path) -> Vec<PathBuf> {
	layers
		.iter()
		.copied()
		.flatten()
		.flat_map(|paths| paths.iter().map(|p| absolutize(root, p)))
		.collect()
}

/// Concatenates and dedups the paths.
fn merge_paths(base: Vec<PathBuf>, extra: Vec<PathBuf>) -> Vec<PathBuf> {
	base.into_iter()
		.chain(extra)
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn root() -> PathBuf {
		PathBuf::from("/repo")
	}

	fn discovered() -> Discovered {
		Discovered {
			src: vec![PathBuf::from("/repo/types/src")],
			templates: vec![PathBuf::from("/repo/types/templates")],
			locales: vec!["en".to_owned(), "fr".to_owned()],
		}
	}

	fn resolved(cli: &PartialConfig, file: &PartialConfig) -> Config {
		resolve(cli, file, &discovered(), &root())
	}

	#[test]
	fn built_in_defaults_apply_when_nothing_is_specified() {
		let c = resolved(&PartialConfig::default(), &PartialConfig::default());
		assert_eq!(c.locales_dir, PathBuf::from("/repo/locales"));
		assert_eq!(c.manifest_path, PathBuf::from("/repo/Cargo.toml"));
		assert_eq!(c.template_ext, vec!["html".to_owned()]);
		assert_eq!(c.filters, vec!["t".to_owned(), "tn".to_owned()]);
		assert_eq!(
			c.functions,
			vec!["loc".to_owned(), "loc_with_args".to_owned()]
		);
		assert_eq!(c.exclude, vec!["target/**".to_owned()]);
		assert_eq!(c.unused, Severity::Warn);
		assert!(c.attributes);
		assert!(!c.include_comments);
		assert!(c.require_locales);
		assert!(c.follow_links);
		assert!(!c.quiet);
		assert_eq!(c.format, OutputFormat::Text);
	}

	#[test]
	fn discovery_fills_the_lists_that_have_no_built_in_default() {
		let c = resolved(&PartialConfig::default(), &PartialConfig::default());
		assert_eq!(c.src, vec![PathBuf::from("/repo/types/src")]);
		assert_eq!(c.templates, vec![PathBuf::from("/repo/types/templates")]);
		assert_eq!(c.locales, vec!["en".to_owned(), "fr".to_owned()]);
	}

	#[test]
	fn the_cli_beats_the_config_file() {
		let cli = PartialConfig {
			locales_dir: Some(PathBuf::from("from-cli")),
			unused: Some(Severity::Error),
			..PartialConfig::default()
		};
		let file = PartialConfig {
			locales_dir: Some(PathBuf::from("from-file")),
			unused: Some(Severity::Allow),
			quiet: Some(true),
			..PartialConfig::default()
		};
		let c = resolved(&cli, &file);
		assert_eq!(c.locales_dir, PathBuf::from("/repo/from-cli"));
		assert_eq!(c.unused, Severity::Error);
		// A key only the file sets still applies.
		assert!(c.quiet);
	}

	#[test]
	fn the_config_file_beats_discovery() {
		let file = PartialConfig {
			src: Some(vec![PathBuf::from("only/this")]),
			locales: Some(vec!["de".to_owned()]),
			..PartialConfig::default()
		};
		let c = resolved(&PartialConfig::default(), &file);
		assert_eq!(c.src, vec![PathBuf::from("/repo/only/this")]);
		assert_eq!(c.locales, vec!["de".to_owned()]);
	}

	#[test]
	fn a_replace_list_wins_outright_with_no_union() {
		let cli = PartialConfig {
			src: Some(vec![PathBuf::from("a")]),
			..PartialConfig::default()
		};
		let file = PartialConfig {
			src: Some(vec![PathBuf::from("b")]),
			..PartialConfig::default()
		};
		let c = resolved(&cli, &file);
		assert_eq!(c.src, vec![PathBuf::from("/repo/a")]);
	}

	#[test]
	fn an_add_list_unions_every_layer_and_extends_the_base() {
		let cli = PartialConfig {
			src: Some(vec![PathBuf::from("base")]),
			add_src: Some(vec![PathBuf::from("from-cli")]),
			..PartialConfig::default()
		};
		let file = PartialConfig {
			add_src: Some(vec![PathBuf::from("from-file")]),
			..PartialConfig::default()
		};
		let c = resolved(&cli, &file);
		assert_eq!(
			c.src,
			vec![
				PathBuf::from("/repo/base"),
				PathBuf::from("/repo/from-cli"),
				PathBuf::from("/repo/from-file"),
			]
		);
	}

	#[test]
	fn add_without_replace_extends_what_was_discovered() {
		let cli = PartialConfig {
			add_src: Some(vec![PathBuf::from("extra/src")]),
			..PartialConfig::default()
		};
		let c = resolved(&cli, &PartialConfig::default());
		assert_eq!(
			c.src,
			vec![
				PathBuf::from("/repo/extra/src"),
				PathBuf::from("/repo/types/src"),
			]
		);
	}

	#[test]
	fn duplicate_paths_collapse() {
		let cli = PartialConfig {
			src: Some(vec![PathBuf::from("a"), PathBuf::from("a")]),
			add_src: Some(vec![PathBuf::from("a")]),
			..PartialConfig::default()
		};
		let c = resolved(&cli, &PartialConfig::default());
		assert_eq!(c.src, vec![PathBuf::from("/repo/a")]);
	}

	#[test]
	fn an_absolute_override_is_left_alone() {
		let cli = PartialConfig {
			locales_dir: Some(PathBuf::from("/elsewhere/i18n")),
			..PartialConfig::default()
		};
		let c = resolved(&cli, &PartialConfig::default());
		assert_eq!(c.locales_dir, PathBuf::from("/elsewhere/i18n"));
	}

	#[test]
	fn the_reference_locale_prefers_en_then_the_first_sorted() {
		let c = resolved(&PartialConfig::default(), &PartialConfig::default());
		assert_eq!(c.reference_locale, "en");

		let cli = PartialConfig {
			locales: Some(vec!["de".to_owned(), "fr".to_owned()]),
			..PartialConfig::default()
		};
		assert_eq!(
			resolved(&cli, &PartialConfig::default()).reference_locale,
			"de"
		);

		let cli = PartialConfig {
			reference_locale: Some("fr".to_owned()),
			..PartialConfig::default()
		};
		assert_eq!(
			resolved(&cli, &PartialConfig::default()).reference_locale,
			"fr"
		);
	}

	#[test]
	fn a_template_extension_is_normalized() {
		let cli = PartialConfig {
			template_ext: Some(vec![".J2".to_owned(), "HTML".to_owned()]),
			..PartialConfig::default()
		};
		let c = resolved(&cli, &PartialConfig::default());
		assert_eq!(c.template_ext, vec!["j2".to_owned(), "html".to_owned()]);
		assert!(c.is_template_ext("j2"));
		assert!(c.is_template_ext("HtMl"));
		assert!(!c.is_template_ext("txt"));
	}

	#[test]
	fn an_explicitly_empty_list_overrides_the_default() {
		let cli = PartialConfig {
			exclude: Some(Vec::new()),
			..PartialConfig::default()
		};
		assert!(resolved(&cli, &PartialConfig::default()).exclude.is_empty());
	}

	#[test]
	fn paths_are_reported_relative_to_the_root() {
		let c = resolved(&PartialConfig::default(), &PartialConfig::default());
		assert_eq!(
			c.relative(Path::new("/repo/types/src/lib.rs")),
			PathBuf::from("types/src/lib.rs")
		);
		// Outside the root, the path is left as it is rather than mangled.
		assert_eq!(
			c.relative(Path::new("/elsewhere/lib.rs")),
			PathBuf::from("/elsewhere/lib.rs")
		);
	}

	#[test]
	fn resolving_twice_gives_the_same_answer() {
		let a = resolved(&PartialConfig::default(), &PartialConfig::default());
		let b = resolved(&PartialConfig::default(), &PartialConfig::default());
		assert_eq!(a, b);
	}
}
