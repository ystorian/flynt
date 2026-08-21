// src/parse/fluent.rs

//! Reading the keys defined by Fluent (`.ftl`) files.
//!
//! Parsing happens in two passes over each file.
//!
//! The first pass records the line of every identifier that opens an entry. `fluent_syntax`'s AST
//! carries no spans, and this line scan is the only way to point a report at a definition.
//!
//! The second pass parses the file properly. It keeps only the identifiers the real parser
//! confirms. This lets the line scan use a simple pattern.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use fluent_syntax::ast::Entry;
use fluent_syntax::parser::parse;
use regex::Regex;

use crate::config::Config;
use crate::extract::scan::{LineIndex, Walker, read};
use crate::model::{KeyDefinition, LocaleKeys, Location, ParseError};

/// A message or term opening a line: `key = ...` or `-term = ...`.
static ENTRY_LINE: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r"^(-?[A-Za-z][A-Za-z0-9_-]*)[ \t]*=")
		.expect("the entry-line pattern is a valid regex")
});

/// An attribute line, which is indented: `    .attr = ...`.
static ATTRIBUTE_LINE: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r"^[ \t]+\.([A-Za-z][A-Za-z0-9_-]*)[ \t]*=")
		.expect("the attribute-line pattern is a valid regex")
});

/// What parsing every locale produced.
#[derive(Debug, Default)]
pub struct Parsed {
	/// One entry per locale, keyed by locale name.
	pub locales: BTreeMap<String, LocaleKeys>,
	/// Every Fluent syntax error found, sorted.
	pub errors: Vec<ParseError>,
	/// How many `.ftl` files were read.
	pub files: usize,
	/// How many `.ftl` files each locale contributed.
	pub files_per_locale: BTreeMap<String, usize>,
}

/// Parses every configured locale.
///
/// # Errors
///
/// Returns an error when a locale directory cannot be walked or a file cannot be read as `UTF-8`.
/// Fluent syntax errors are collected into `Parsed::errors`.
pub fn parse_locales(config: &Config, walker: &Walker) -> Result<Parsed> {
	let mut parsed = Parsed::default();

	for locale in &config.locales {
		let dir = config.locales_dir.join(locale);
		let mut keys = LocaleKeys {
			locale: locale.clone(),
			..LocaleKeys::default()
		};

		let files = walker.files(&dir, is_fluent)?;
		parsed.files_per_locale.insert(locale.clone(), files.len());

		for path in files {
			let content = read(&path)?;
			let relative = config.relative(&path);
			let file = parse_file(&content, &relative, config.attributes);
			parsed.files += 1;
			parsed.errors.extend(file.errors);
			for (key, definition) in file.definitions {
				keys.keys.entry(key).or_default().push(definition);
			}
			keys.terms.extend(file.terms);
		}

		parsed.locales.insert(locale.clone(), keys);
	}

	parsed.errors.sort_by(|a, b| a.at.cmp(&b.at));
	Ok(parsed)
}

/// Whether a path is a Fluent file.
fn is_fluent(path: &Path) -> bool {
	path.extension().and_then(|e| e.to_str()) == Some("ftl")
}

/// One file's contribution.
struct File {
	/// Every definition, in file order, to preserve duplicates.
	definitions: Vec<(String, KeyDefinition)>,
	/// Fluent terms.
	terms: BTreeSet<String>,
	/// Syntax errors found in this file.
	errors: Vec<ParseError>,
}

/// Collects the keys defined by one file's contents.
fn parse_file(content: &str, path: &Path, attributes: bool) -> File {
	let index = LineIndex::new(content);
	let mut lines = line_numbers(content);
	let mut definitions = Vec::new();
	let mut terms = BTreeSet::new();

	// A recovered resource is usable as a clean one.
	let (resource, errors) = match parse(content) {
		Ok(resource) => (resource, Vec::new()),
		Err((resource, errors)) => (resource, errors),
	};

	for entry in resource.body {
		let (id, attrs, is_term) = match entry {
			Entry::Message(m) => (
				m.id.name.to_owned(),
				m.attributes
					.iter()
					.map(|a| a.id.name.to_owned())
					.collect::<Vec<_>>(),
				false,
			),
			Entry::Term(t) => (
				// A term is referenced with its leading dash. That dash is part of its key.
				format!("-{}", t.id.name),
				t.attributes
					.iter()
					.map(|a| a.id.name.to_owned())
					.collect::<Vec<_>>(),
				true,
			),
			Entry::Comment(_)
			| Entry::GroupComment(_)
			| Entry::ResourceComment(_)
			| Entry::Junk { .. } => continue,
		};

		definitions.push((id.clone(), definition(path, take_line(&mut lines, &id))));
		if is_term {
			terms.insert(id.clone());
		}

		if attributes {
			for attr in attrs {
				let key = format!("{id}.{attr}");
				let line = take_line(&mut lines, &key);
				definitions.push((key, definition(path, line)));
			}
		}
	}

	File {
		definitions,
		terms,
		errors: errors
			.into_iter()
			.map(|e| {
				let (line, column) = index.locate(e.pos.start);
				ParseError {
					at: Location {
						file: path.to_path_buf(),
						line,
						column,
					},
					message: e.kind.to_string(),
				}
			})
			.collect(),
	}
}

fn definition(path: &Path, line: usize) -> KeyDefinition {
	KeyDefinition {
		at: Location {
			file: path.to_path_buf(),
			line,
			column: 1,
		},
	}
}

/// Consumes the next recorded line for `key`.
fn take_line(lines: &mut HashMap<String, VecDeque<usize>>, key: &str) -> usize {
	lines
		.get_mut(key)
		.and_then(VecDeque::pop_front)
		.unwrap_or(1)
}

/// Records the line of every identifier that opens an entry or an attribute.
fn line_numbers(content: &str) -> HashMap<String, VecDeque<usize>> {
	let mut lines: HashMap<String, VecDeque<usize>> = HashMap::new();
	let mut current: Option<String> = None;

	for (number, line) in content.lines().enumerate() {
		let number = number + 1;

		if let Some(captures) = ENTRY_LINE.captures(line) {
			let id = captures[1].to_owned();
			lines.entry(id.clone()).or_default().push_back(number);
			current = Some(id);
		} else if let Some(captures) = ATTRIBUTE_LINE.captures(line) {
			// An attribute belongs to whichever entry opened last.
			if let Some(parent) = &current {
				let key = format!("{parent}.{}", &captures[1]);
				lines.entry(key).or_default().push_back(number);
			}
		}
	}

	lines
}

#[cfg(test)]
mod tests {
	use super::*;

	fn keys_of(content: &str, attributes: bool) -> Vec<(String, usize)> {
		parse_file(content, Path::new("locales/en/a.ftl"), attributes)
			.definitions
			.into_iter()
			.map(|(k, d)| (k, d.at.line))
			.collect()
	}

	#[test]
	fn messages_are_collected_with_their_lines() {
		let content = "# a comment\nfirst = One\nsecond = Two\n";
		assert_eq!(
			keys_of(content, true),
			vec![("first".to_owned(), 2), ("second".to_owned(), 3)]
		);
	}

	#[test]
	fn a_continuation_line_containing_an_equals_sign_is_not_a_definition() {
		let content = "key = Some text\n  more text with a = sign\nother = Two\n";
		assert_eq!(
			keys_of(content, true),
			vec![("key".to_owned(), 1), ("other".to_owned(), 3)]
		);
	}

	#[test]
	fn a_key_defined_twice_reports_two_different_lines() {
		let content = "dup = One\nother = Two\ndup = Three\n";
		assert_eq!(
			keys_of(content, true),
			vec![
				("dup".to_owned(), 1),
				("other".to_owned(), 2),
				("dup".to_owned(), 3),
			]
		);
	}

	#[test]
	fn a_term_keeps_its_leading_dash() {
		let file = parse_file("-brand = Ystorian\n", Path::new("a.ftl"), true);
		assert_eq!(file.definitions[0].0, "-brand");
		assert!(file.terms.contains("-brand"));
	}

	#[test]
	fn attributes_are_collected_when_enabled() {
		// Fluent indents with spaces only. A tab here would not parse.
		let content = "input = Label\n    .placeholder = Type here\n    .title = A title\n";
		assert_eq!(
			keys_of(content, true),
			vec![
				("input".to_owned(), 1),
				("input.placeholder".to_owned(), 2),
				("input.title".to_owned(), 3),
			]
		);
	}

	#[test]
	fn attributes_are_skipped_when_disabled() {
		let content = "input = Label\n    .placeholder = Type here\n";
		assert_eq!(keys_of(content, false), vec![("input".to_owned(), 1)]);
	}

	#[test]
	fn comments_are_not_definitions() {
		let content = "### resource\n## group\n# standalone\nreal = Yes\n";
		assert_eq!(keys_of(content, true), vec![("real".to_owned(), 4)]);
	}

	#[test]
	fn a_broken_file_still_yields_its_good_entries_and_reports_the_error() {
		let content = "good = Fine\ng@Rb@ge = #2y ds\nalso-good = Fine\n";
		let file = parse_file(content, Path::new("locales/en/broken.ftl"), true);

		let keys: Vec<String> = file.definitions.into_iter().map(|(k, _)| k).collect();
		assert!(keys.contains(&"good".to_owned()), "{keys:?}");
		assert!(!file.errors.is_empty(), "the junk entry must be reported");

		let error = &file.errors[0];
		assert_eq!(error.at.file, Path::new("locales/en/broken.ftl"));
		assert_eq!(error.at.line, 2);
		// The real parser message, not just a count of errors.
		assert!(!error.message.is_empty());
		assert!(
			error.message.contains("Expected") || error.message.contains("expected"),
			"unexpected message: {}",
			error.message
		);
	}

	#[test]
	fn a_clean_file_reports_no_errors() {
		let file = parse_file("a = One\nb = Two\n", Path::new("a.ftl"), true);
		assert!(file.errors.is_empty());
	}

	#[test]
	fn only_ftl_files_are_accepted() {
		assert!(is_fluent(Path::new("locales/en/a.ftl")));
		assert!(!is_fluent(Path::new("locales/en/a.txt")));
		assert!(!is_fluent(Path::new("locales/en/README.md")));
	}
}
