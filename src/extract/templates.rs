// src/extract/templates.rs

//! Extracting translation keys from Askama templates.

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
/// - Directory cannot be walked.
/// - File is not valid UTF-8.
/// - Filter names form an invalid pattern.
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

/// Whether a path has a template extension.
fn is_template(path: &Path, config: &Config) -> bool {
	path.extension()
		.and_then(|e| e.to_str())
		.is_some_and(|e| config.is_template_ext(e))
}

/// Builds the filter pattern from filter names.
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
	fn both_defaults_found() {
		let content = r#"
			<h1>{{ "tpl-title" | t(&lang) }}</h1>
			<p>{{ "mail-count" | tn(&lang, "count", count) }}</p>
		"#;
		assert_eq!(keys_of(content), vec!["tpl-title", "mail-count"]);
	}

	#[test]
	fn longer_filter_preferred() {
		// `tn` must not truncate to `t`.
		let usages = extract(r#"{{ "k" | tn(&lang, "n", v) }}"#, &["t", "tn"]);
		assert_eq!(usages.len(), 1);
		assert_eq!(usages[0].key, "k");
	}

	#[test]
	fn filter_split_across_lines() {
		let content = "{{ \"wrapped\"\n\t| t(&lang) }}";
		let usages = extract(content, &["t"]);
		assert_eq!(usages.len(), 1);
		assert_eq!(usages[0].key, "wrapped");
		assert_eq!(usages[0].at.line, 1);
	}

	#[test]
	fn key_inside_block_tag() {
		// Not anchoring on `{{`.
		let content = r#"{% block title %}{{ "tpl-title" | t(&lang) }}{% endblock %}"#;
		assert_eq!(keys_of(content), vec!["tpl-title"]);
	}

	#[test]
	fn chained_filter_yields_key() {
		assert_eq!(keys_of(r#"{{ "k" | t(&lang) | upper }}"#), vec!["k"]);
	}

	#[test]
	fn plain_string_not_key() {
		let content = r#"<a href="/home" class="button">{{ title }}</a>"#;
		assert!(keys_of(content).is_empty());
	}

	#[test]
	fn custom_names_replace_defaults() {
		let content = r#"{{ "mine" | x(&lang) }} {{ "theirs" | t(&lang) }}"#;
		let keys: Vec<String> = extract(content, &["x"])
			.into_iter()
			.map(|u| u.key)
			.collect();
		assert_eq!(keys, vec!["mine"]);
	}

	#[test]
	fn location_points_at_key() {
		let content = "<html>\n<h1>{{ \"the-key\" | t(&lang) }}</h1>\n";
		let usages = extract(content, &["t"]);
		assert_eq!(usages[0].at.line, 2);
		// Points at the key without quotes.
		assert_eq!(usages[0].at.column, 9);
		assert_eq!(usages[0].kind, UsageType::Template);
	}

	#[test]
	fn whitespace_around_pipe_ok() {
		assert_eq!(keys_of(r#"{{"k"|t(&lang)}}"#), vec!["k"]);
		assert_eq!(keys_of("{{ \"k\"  |  t (&lang) }}"), vec!["k"]);
	}

	#[test]
	fn no_filters_no_pattern() {
		assert!(
			pattern(&[])
				.expect("an empty list is not an error")
				.is_none()
		);
	}
}
