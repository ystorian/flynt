// src/config/partial.rs

//! The CLI and `.flynt.toml` schema.

use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

use super::{ColorChoice, OutputFormat, Severity};

/// A partially specified configuration.
#[allow(clippy::doc_markdown)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Parser, Deserialize)]
#[command(
	name = "flynt",
	version,
	about = "Fluent keys linter for Askama templates",
	after_help = "Defaults come from the repository, can be overriden in .flynt.toml."
)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct PartialConfig {
	// ---- CLI only ----
	/// Directory to lint [default: the current directory]
	#[arg(value_name = "PATH")]
	#[serde(skip)]
	pub root: Option<PathBuf>,

	/// Read this configuration file instead of `<PATH>/.flynt.toml`
	#[arg(long, value_name = "FILE")]
	#[serde(skip)]
	pub config: Option<PathBuf>,

	/// Ignore `.flynt.toml` entirely
	#[arg(long, num_args = 0..=1, default_missing_value = "true", value_name = "BOOL")]
	#[serde(skip)]
	pub no_config: Option<bool>,

	// ---- anchors ----
	/// Path to the root Cargo.toml [default: <PATH>/Cargo.toml]
	#[arg(long, value_name = "FILE")]
	pub manifest_path: Option<PathBuf>,

	// ---- locales ----
	/// Directory holding locales [default: locales]
	#[arg(long, value_name = "DIR")]
	pub locales_dir: Option<PathBuf>,

	/// Locale to check; repeatable [default: every locale]
	#[arg(long, value_name = "LOCALE")]
	pub locales: Option<Vec<String>>,

	/// Reference locale for the unused-key check [default: en]
	#[arg(long, value_name = "LOCALE")]
	pub reference_locale: Option<String>,

	// ---- sources ----
	/// Rust source directories to scan (replaces defaults)
	#[arg(long = "src", value_name = "DIR")]
	pub src: Option<Vec<PathBuf>>,

	/// Extra Rust source directory (adds to defaults)
	#[arg(long, value_name = "DIR")]
	pub add_src: Option<Vec<PathBuf>>,

	// ---- templates ----
	/// Template directories to scan (replaces defaults)
	#[arg(long = "templates", value_name = "DIR")]
	pub templates: Option<Vec<PathBuf>>,

	/// Extra template directory (adds to defaults)
	#[arg(long, value_name = "DIR")]
	pub add_templates: Option<Vec<PathBuf>>,

	/// Template file extension; repeatable [default: html]
	#[arg(long = "template-ext", value_name = "EXT")]
	pub template_ext: Option<Vec<String>>,

	// ---- helper names ----
	/// Askama filter taking a key [default: t, tn]
	#[arg(long, value_name = "NAME")]
	pub filters: Option<Vec<String>>,

	/// Rust function taking a key [default: loc, loc_with_args]
	#[arg(long, value_name = "NAME")]
	pub functions: Option<Vec<String>>,

	// ---- checks ----
	/// Severity for unused keys [default: warn]
	#[arg(long, value_enum, value_name = "LEVEL")]
	pub unused: Option<Severity>,

	/// Key-name glob never reported as unused; repeatable
	#[arg(long, value_name = "GLOB")]
	pub ignore_unused: Option<Vec<String>>,

	/// Treat `key.attribute` as defined [default: true]
	#[arg(long, num_args = 0..=1, default_missing_value = "true", value_name = "BOOL")]
	pub attributes: Option<bool>,

	// ---- walking ----
	/// Path glob to skip [default: target/**]
	#[arg(long, value_name = "GLOB")]
	pub exclude: Option<Vec<String>>,

	/// Follow symbolic links [default: true]
	#[arg(long, num_args = 0..=1, default_missing_value = "true", value_name = "BOOL")]
	pub follow_links: Option<bool>,

	// ---- output ----
	/// Output format [default: text]
	#[arg(long, value_enum, value_name = "FORMAT")]
	pub format: Option<OutputFormat>,

	/// Coloring [default: auto]
	#[arg(long, value_enum, value_name = "WHEN")]
	pub color: Option<ColorChoice>,

	/// Silence is a virtue
	#[arg(long, short = 'q', num_args = 0..=1, default_missing_value = "true", value_name = "BOOL")]
	pub quiet: Option<bool>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cli_and_toml_agree() {
		let cli = PartialConfig::parse_from([
			"flynt",
			"--locales-dir",
			"i18n",
			"--locales",
			"en",
			"--locales",
			"de",
			"--template-ext",
			"j2",
			"--filters",
			"tr",
			"--unused",
			"error",
			"--attributes=false",
			"--format",
			"json",
		]);
		let toml: PartialConfig = toml::from_str(
			r#"
			locales-dir = "i18n"
			locales = ["en", "de"]
			template-ext = ["j2"]
			filters = ["tr"]
			unused = "error"
			attributes = false
			format = "json"
			"#,
		)
		.expect("the TOML keys must match the long flags");

		assert_eq!(cli, toml);
	}

	#[test]
	fn absent_flags_stay_none() {
		let cli = PartialConfig::parse_from(["flynt"]);
		assert_eq!(cli, PartialConfig::default());
		assert!(cli.attributes.is_none());
		assert!(cli.locales.is_none());
	}

	#[test]
	fn bare_bool_flag_is_true() {
		assert_eq!(
			PartialConfig::parse_from(["flynt", "--attributes"]).attributes,
			Some(true)
		);
		assert_eq!(
			PartialConfig::parse_from(["flynt", "--attributes=false"]).attributes,
			Some(false)
		);
		assert_eq!(PartialConfig::parse_from(["flynt", "-q"]).quiet, Some(true));
	}

	#[test]
	fn empty_list_different() {
		let absent: PartialConfig = toml::from_str("").expect("empty TOML is valid");
		assert!(absent.locales.is_none());

		let empty: PartialConfig = toml::from_str("locales = []").expect("an empty list is valid");
		assert_eq!(empty.locales, Some(Vec::new()));
	}

	#[test]
	fn unknown_toml_key_errors() {
		let err = toml::from_str::<PartialConfig>("locale-dir = \"i18n\"")
			.expect_err("deny_unknown_fields must reject a misspelled key");
		assert!(err.to_string().contains("locale-dir"), "{err}");
	}

	#[test]
	fn root_is_cli_only() {
		let err =
			toml::from_str::<PartialConfig>("root = \"..\"").expect_err("root must be CLI-only");
		assert!(err.to_string().contains("root"), "{err}");
	}

	#[test]
	fn cli_definition_is_consistent() {
		use clap::CommandFactory;
		PartialConfig::command().debug_assert();
	}
}
