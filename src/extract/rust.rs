// src/extract/rust.rs

//! Extracting translation keys from Rust files.

use std::path::Path;

use anyhow::Result;
use regex::Regex;

use super::scan::{LineIndex, Walker};
use super::{Found, alternation, compile};
use crate::config::Config;
use crate::model::{KeyUsage, Location, UsageType};

/// Scans every configured Rust source directory.
///
/// # Errors
///
/// - Directory cannot be walked.
/// - File is not valid UTF-8.
/// - Function names form an invalid pattern.
pub fn keys(config: &Config, walker: &Walker) -> Result<Found> {
	let Some(pattern) = pattern(&config.functions)? else {
		return Ok(Found::default());
	};

	let mut found = Found::default();
	for dir in &config.src {
		for path in walker.files(dir, is_rust)? {
			let content = super::scan::read(&path)?;
			let relative = config.relative(&path);
			found.usages.extend(keys_in(&pattern, &content, &relative));
			found.files += 1;
		}
	}

	found.usages.sort();
	Ok(found)
}

/// Whether a path is a Rust file.
fn is_rust(path: &Path) -> bool {
	path.extension().and_then(|e| e.to_str()) == Some("rs")
}

/// Builds the call pattern from function names.
fn pattern(functions: &[String]) -> Result<Option<Regex>> {
	let Some(alts) = alternation(functions) else {
		return Ok(None);
	};
	let regex = compile(
		&format!(r#"\b(?:{alts})\s*\(\s*"([^"]+)""#),
		"Rust function",
	)?;
	Ok(Some(regex))
}

/// Finds every key in one file's contents.
fn keys_in(pattern: &Regex, content: &str, path: &Path) -> Vec<KeyUsage> {
	let index = LineIndex::new(content);
	let mut usages = Vec::new();

	for captures in pattern.captures_iter(content) {
		let Some(key) = captures.get(1) else { continue };
		if index.is_comment_line(key.start()) {
			continue;
		}
		let (line, column) = index.locate(key.start());
		usages.push(KeyUsage {
			key: key.as_str().to_owned(),
			at: Location {
				file: path.to_path_buf(),
				line,
				column,
			},
			kind: UsageType::Rust,
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

	fn extract(content: &str, functions: &[&str]) -> Vec<KeyUsage> {
		let pattern = pattern(&names(functions))
			.expect("the pattern compiles")
			.expect("there is at least one name");
		keys_in(&pattern, content, Path::new("src/lib.rs"))
	}

	fn keys_of(content: &str) -> Vec<String> {
		extract(content, &["loc", "loc_with_args"])
			.into_iter()
			.map(|u| u.key)
			.collect()
	}

	#[test]
	fn both_defaults_found() {
		let content = r#"
			let a = loc("first", &lang);
			let b = loc_with_args("second", &lang, &args);
		"#;
		assert_eq!(keys_of(content), vec!["first", "second"]);
	}

	#[test]
	fn alloc_not_mistaken_loc() {
		let content = r#"
			let a = alloc("not-a-key");
			let b = realloc("also-not");
			let c = loc("real-key", &lang);
		"#;
		assert_eq!(keys_of(content), vec!["real-key"]);
	}

	#[test]
	fn method_call_still_found() {
		let content = r#"let a = self.loc("k", &lang);"#;
		assert_eq!(keys_of(content), vec!["k"]);
	}

	#[test]
	fn comments_are_skipped() {
		let content = r#"
			/// - `loc("key", &lang)`
			/// - `loc_with_args("key", &lang, &args)`
			//! loc("inner-doc")
			// loc("plain")
			let real = loc("actual", &lang);
		"#;
		assert_eq!(keys_of(content), vec!["actual"]);
	}

	#[test]
	fn call_split_across_lines() {
		let content = "let a = loc(\n\t\"wrapped\",\n\t&lang,\n);";
		let usages = extract(content, &["loc"]);
		assert_eq!(usages.len(), 1);
		assert_eq!(usages[0].key, "wrapped");
		assert_eq!(usages[0].at.line, 2);
	}

	#[test]
	fn custom_names_replace_defaults() {
		let content = r#"translate("mine", &lang); loc("theirs", &lang);"#;
		let keys: Vec<String> = extract(content, &["translate"])
			.into_iter()
			.map(|u| u.key)
			.collect();
		assert_eq!(keys, vec!["mine"]);
	}

	#[test]
	fn location_points_at_key() {
		let content = "fn a() {\n\tloc(\"the-key\", &lang);\n}";
		let usages = extract(content, &["loc"]);
		assert_eq!(usages[0].at.line, 2);
		// The key starts at column 7.
		assert_eq!(usages[0].at.column, 7);
		assert_eq!(usages[0].kind, UsageType::Rust);
		assert_eq!(usages[0].at.file, Path::new("src/lib.rs"));
	}

	#[test]
	fn whitespace_inside_call_ok() {
		let content = "loc  (  \"spaced\" , &lang);";
		assert_eq!(keys_of(content), vec!["spaced"]);
	}

	#[test]
	fn emoji_key_captured_whole() {
		// A four-byte character must survive intact.
		let content = "let a = loc(\"greeting 👋\", &lang);";
		let usages = extract(content, &["loc"]);
		assert_eq!(usages.len(), 1);
		assert_eq!(usages[0].key, "greeting 👋");
		assert_eq!(usages[0].at.column, 14);
	}

	#[test]
	fn empty_key_rejected() {
		assert!(keys_of(r#"loc("", &lang);"#).is_empty());
	}

	#[test]
	fn no_functions_no_pattern() {
		assert!(
			pattern(&[])
				.expect("an empty list is not an error")
				.is_none()
		);
	}

	#[test]
	fn only_rust_files() {
		assert!(is_rust(Path::new("a/b.rs")));
		assert!(!is_rust(Path::new("a/b.html")));
		assert!(!is_rust(Path::new("a/b")));
	}
}
