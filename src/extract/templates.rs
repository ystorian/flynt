// src/extract/templates.rs

//! Extracting translation keys from Askama templates.
//!
//! Matches a string literal piped into one of the configured filters, such as
//! `{{ "key" | t(&lang) }}` or `{{ "key" | tn(&lang, "param", value) }}`.

use std::path::Path;

use anyhow::Result;
use regex::Regex;

use super::scan::{LineIndex, Walker};
use super::{Found, alternation, compile};
use crate::config::Config;
use crate::model::{KeyUsage, Location, UsageType};

/// Scans every configured template directory.
///
/// # Errors
///
/// Returns an error when a directory cannot be walked, a file cannot be read as
/// UTF-8, or the configured filter names do not form a valid pattern.
pub fn keys(config: &Config, walker: &Walker) -> Result<Found> {
	let Some(pattern) = pattern(&config.filters)? else {
		return Ok(Found::default());
	};

	let mut found = Found::default();
	for dir in &config.templates {
		for path in walker.files(dir, |p| is_template(p, config))? {
			let content = super::scan::read(&path)?;
			let relative = config.relative(&path);
			found.usages.extend(keys_in(&pattern, &content, &relative));
			found.files += 1;
		}
	}

	found.usages.sort();
	Ok(found)
}

/// Whether a path carries one of the configured template extensions.
fn is_template(path: &Path, config: &Config) -> bool {
	path.extension()
		.and_then(|e| e.to_str())
		.is_some_and(|e| config.is_template_ext(e))
}

/// Builds the filter pattern from the configured filter names.
///
/// The opening `{{` is deliberately not required. Anchoring on it would miss a key used inside a
/// `{% ... %}` block.
fn pattern(filters: &[String]) -> Result<Option<Regex>> {
	let Some(alts) = alternation(filters) else {
		return Ok(None);
	};
	let regex = compile(
		&format!(r#""([^"]+)"\s*\|\s*(?:{alts})\s*\("#),
		"template filter",
	)?;
	Ok(Some(regex))
}

/// Finds every key in one template's contents.
fn keys_in(pattern: &Regex, content: &str, path: &Path) -> Vec<KeyUsage> {
	let index = LineIndex::new(content);
	let mut usages = Vec::new();

	for captures in pattern.captures_iter(content) {
		let Some(key) = captures.get(1) else { continue };
		let (line, column) = index.locate(key.start());
		usages.push(KeyUsage {
			key: key.as_str().to_owned(),
			at: Location {
				file: path.to_path_buf(),
				line,
				column,
			},
			kind: UsageType::Template,
		});
	}

	usages
}

#[cfg(test)]
mod tests {
	use super::*;

	fn names(values: &[&str]) -> Vec<String> {
		values.iter().map(|v| (*v).to_owned()).collect()
	}

	fn extract(content: &str, filters: &[&str]) -> Vec<KeyUsage> {
		let pattern = pattern(&names(filters))
			.expect("the pattern compiles")
			.expect("there is at least one name");
		keys_in(&pattern, content, Path::new("templates/home.html"))
	}

	fn keys_of(content: &str) -> Vec<String> {
		extract(content, &["t", "tn"])
			.into_iter()
			.map(|u| u.key)
			.collect()
	}

	#[test]
	fn both_default_filters_are_found() {
		let content = r#"
			<h1>{{ "tpl-title" | t(&lang) }}</h1>
			<p>{{ "mail-count" | tn(&lang, "count", count) }}</p>
		"#;
		assert_eq!(keys_of(content), vec!["tpl-title", "mail-count"]);
	}

	#[test]
	fn the_longer_filter_name_is_preferred() {
		// `tn` must not be truncated to `t` by the alternation order.
		let usages = extract(r#"{{ "k" | tn(&lang, "n", v) }}"#, &["t", "tn"]);
		assert_eq!(usages.len(), 1);
		assert_eq!(usages[0].key, "k");
	}

	#[test]
	fn a_filter_split_across_lines_is_found() {
		let content = "{{ \"wrapped\"\n\t| t(&lang) }}";
		let usages = extract(content, &["t"]);
		assert_eq!(usages.len(), 1);
		assert_eq!(usages[0].key, "wrapped");
		assert_eq!(usages[0].at.line, 1);
	}

	#[test]
	fn a_key_inside_a_block_tag_is_found() {
		// Not anchoring on `{{`.
		let content = r#"{% block title %}{{ "tpl-title" | t(&lang) }}{% endblock %}"#;
		assert_eq!(keys_of(content), vec!["tpl-title"]);
	}

	#[test]
	fn a_chained_filter_still_yields_the_key() {
		assert_eq!(keys_of(r#"{{ "k" | t(&lang) | upper }}"#), vec!["k"]);
	}

	#[test]
	fn a_plain_string_without_the_filter_is_not_a_key() {
		let content = r#"<a href="/home" class="button">{{ title }}</a>"#;
		assert!(keys_of(content).is_empty());
	}

	#[test]
	fn custom_filter_names_replace_the_defaults() {
		let content = r#"{{ "mine" | x(&lang) }} {{ "theirs" | t(&lang) }}"#;
		let keys: Vec<String> = extract(content, &["x"])
			.into_iter()
			.map(|u| u.key)
			.collect();
		assert_eq!(keys, vec!["mine"]);
	}

	#[test]
	fn the_location_points_at_the_key_itself() {
		let content = "<html>\n<h1>{{ \"the-key\" | t(&lang) }}</h1>\n";
		let usages = extract(content, &["t"]);
		assert_eq!(usages[0].at.line, 2);
		// The position points at the key text without the quotes around it.
		assert_eq!(usages[0].at.column, 9);
		assert_eq!(usages[0].kind, UsageType::Template);
	}

	#[test]
	fn whitespace_around_the_pipe_is_tolerated() {
		assert_eq!(keys_of(r#"{{"k"|t(&lang)}}"#), vec!["k"]);
		assert_eq!(keys_of("{{ \"k\"  |  t (&lang) }}"), vec!["k"]);
	}

	#[test]
	fn no_configured_filters_means_no_pattern() {
		assert!(
			pattern(&[])
				.expect("an empty list is not an error")
				.is_none()
		);
	}
}
