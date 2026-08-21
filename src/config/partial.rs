// src/config/partial.rs

//! The struct that is both the CLI and the `.flynt.toml` schema.
//!
//! Derives `clap::Parser` and `serde::Deserialize` on a single type to keep a flag and its
//! configuration-file key in sync. Every field is an `Option`. `None` means "not specified at this
//! layer".

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
	about = "Lint Fluent translation keys against their use in Rust code and Askama templates",
	after_help = "Defaults are inferred from the target repository: workspace members from \
	              Cargo.toml, locales from the subdirectories of the locales directory. Any \
	              default can be overridden here or in a .flynt.toml at the root."
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
	/// Directory holding one subdirectory per locale [default: locales]
	#[arg(long, value_name = "DIR")]
	pub locales_dir: Option<PathBuf>,

	/// Locale to check; repeatable [default: every subdirectory of the locales directory]
	#[arg(long = "locale", value_name = "LOCALE")]
	#[serde(alias = "locale")]
	pub locales: Option<Vec<String>>,

	/// Locale the unused-key check runs against [default: en, else the first sorted]
	#[arg(long, value_name = "LOCALE")]
	pub reference_locale: Option<String>,

	// ---- sources ----
	/// Rust source directory to scan; repeatable, replaces the inferred set
	#[arg(long = "src", value_name = "DIR")]
	pub src: Option<Vec<PathBuf>>,

	/// Extra Rust source directory; repeatable, added to the inferred set
	#[arg(long, value_name = "DIR")]
	pub add_src: Option<Vec<PathBuf>>,

	// ---- templates ----
	/// Template directory to scan; repeatable, replaces the inferred set
	#[arg(long = "templates", value_name = "DIR")]
	pub templates: Option<Vec<PathBuf>>,

	/// Extra template directory; repeatable, added to the inferred set
	#[arg(long, value_name = "DIR")]
	pub add_templates: Option<Vec<PathBuf>>,

	/// Template file extension; repeatable [default: html]
	#[arg(long = "template-ext", value_name = "EXT")]
	pub template_ext: Option<Vec<String>>,

	// ---- helper names ----
	/// Askama filter that takes a key as its input; repeatable [default: t, tn]
	#[arg(long = "filter", value_name = "NAME")]
	#[serde(alias = "filter")]
	pub filters: Option<Vec<String>>,

	/// Rust function that takes a key as its first argument; repeatable [default: loc, loc_with_args]
	#[arg(long = "function", value_name = "NAME")]
	#[serde(alias = "function")]
	pub functions: Option<Vec<String>>,

	// ---- checks ----
	/// Severity for keys that are defined but never used [default: warn]
	#[arg(long, value_enum, value_name = "LEVEL")]
	pub unused: Option<Severity>,

	/// Key-name glob never reported as unused; repeatable
	#[arg(long, value_name = "GLOB")]
	pub ignore_unused: Option<Vec<String>>,

	/// Treat `key.attribute` as a defined key [default: true]
	#[arg(long, num_args = 0..=1, default_missing_value = "true", value_name = "BOOL")]
	pub attributes: Option<bool>,

	/// Match keys inside `//` comment lines too [default: false]
	#[arg(long, num_args = 0..=1, default_missing_value = "true", value_name = "BOOL")]
	pub include_comments: Option<bool>,

	/// Fail when the locales directory is missing or empty [default: true]
	#[arg(long, num_args = 0..=1, default_missing_value = "true", value_name = "BOOL")]
	pub require_locales: Option<bool>,

	// ---- walking ----
	/// Path glob to skip, relative to PATH; repeatable [default: target/**]
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
	fn cli_and_toml_accept_the_same_names() {
		let cli = PartialConfig::parse_from([
			"flynt",
			"--locales-dir",
			"i18n",
			"--locale",
			"en",
			"--locale",
			"de",
			"--template-ext",
			"j2",
			"--filter",
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
	fn bare_bool_flag_means_true_and_the_negative_form_is_available() {
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
	fn an_empty_list_is_distinguishable_from_an_absent_one() {
		let absent: PartialConfig = toml::from_str("").expect("empty TOML is valid");
		assert!(absent.locales.is_none());

		let empty: PartialConfig = toml::from_str("locales = []").expect("an empty list is valid");
		assert_eq!(empty.locales, Some(Vec::new()));
	}

	#[test]
	fn the_singular_flag_names_are_accepted_in_the_config_file_too() {
		let plural: PartialConfig = toml::from_str(
			r#"
			locales = ["en"]
			filters = ["tr"]
			functions = ["translate"]
			"#,
		)
		.expect("the plural keys are valid");
		let singular: PartialConfig = toml::from_str(
			r#"
			locale = ["en"]
			filter = ["tr"]
			function = ["translate"]
			"#,
		)
		.expect("the singular aliases are valid");

		assert_eq!(plural, singular);
	}

	#[test]
	fn a_typo_in_the_config_file_is_an_error() {
		let err = toml::from_str::<PartialConfig>("locale-dir = \"i18n\"")
			.expect_err("deny_unknown_fields must reject a misspelled key");
		assert!(err.to_string().contains("locale-dir"), "{err}");
	}

	#[test]
	fn root_is_not_settable_from_the_config_file() {
		let err =
			toml::from_str::<PartialConfig>("root = \"..\"").expect_err("root must be CLI-only");
		assert!(err.to_string().contains("root"), "{err}");
	}

	#[test]
	fn the_cli_definition_is_internally_consistent() {
		use clap::CommandFactory;
		PartialConfig::command().debug_assert();
	}
}
