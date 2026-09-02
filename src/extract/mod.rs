// src/extract/mod.rs

//! Finding translation keys in source files.

pub mod rust;
pub mod scan;
pub mod templates;

use anyhow::{Context, Result};
use regex::Regex;

use crate::model::KeyUsage;

/// What one extractor found.
#[derive(Debug, Default)]
pub struct Found {
	/// Every usage, sorted.
	pub usages: Vec<KeyUsage>,
	/// How many files were read.
	pub files: usize,
}

impl Found {
	/// Merges another result into this one.
	pub fn absorb(&mut self, other: Self) {
		self.usages.extend(other.usages);
		self.files += other.files;
	}
}

/// Builds an alternation of the configured helper names.
fn alternation(names: &[String]) -> Option<String> {
	if names.is_empty() {
		return None;
	}
	let mut sorted: Vec<&String> = names.iter().collect();
	sorted.sort_by_key(|n| (std::cmp::Reverse(n.len()), (*n).clone()));
	Some(
		sorted
			.iter()
			.map(|n| regex::escape(n))
			.collect::<Vec<_>>()
			.join("|"),
	)
}

/// Compiles a pattern.
fn compile(pattern: &str, what: &str) -> Result<Regex> {
	Regex::new(pattern)
		.with_context(|| format!("cannot build the {what} pattern from the configured names"))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn names(values: &[&str]) -> Vec<String> {
		values.iter().map(|v| (*v).to_owned()).collect()
	}

	#[test]
	fn no_names_no_pattern() {
		assert!(alternation(&[]).is_none());
	}

	#[test]
	fn longer_names_win() {
		assert_eq!(
			alternation(&names(&["loc", "loc_with_args"])).as_deref(),
			Some("loc_with_args|loc")
		);
	}

	#[test]
	fn equal_length_names_ordered() {
		assert_eq!(
			alternation(&names(&["tn", "xy", "ab"])).as_deref(),
			Some("ab|tn|xy")
		);
	}

	#[test]
	fn metacharacters_are_escaped() {
		let alts = alternation(&names(&["t.t"])).expect("one name yields a pattern");
		assert_eq!(alts, r"t\.t");
		let re = compile(&format!("^(?:{alts})$"), "test").expect("the pattern compiles");
		assert!(re.is_match("t.t"));
		assert!(!re.is_match("txt"));
	}
}
