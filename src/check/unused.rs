// src/check/unused.rs

//! Unused keys.

use std::collections::BTreeSet;

use anyhow::{Context, Result};

use crate::model::{LocaleKeys, UnusedKey};

/// Finds unused keys defined by the reference locale.
///
/// Only the reference locale is examined.
///
/// # Errors
///
/// Returns an error when an `ignore_unused` entry is not a valid glob.
pub fn unused(
	reference: &LocaleKeys,
	used: &BTreeSet<String>,
	ignore: &[String],
) -> Result<Vec<UnusedKey>> {
	let ignored = ignore
		.iter()
		.map(|p| {
			glob::Pattern::new(p)
				.with_context(|| format!("--ignore-unused {p:?} is not a valid glob"))
		})
		.collect::<Result<Vec<_>>>()?;

	let mut findings = Vec::new();
	for (key, definitions) in &reference.keys {
		if used.contains(key) || reference.terms.contains(key) {
			continue;
		}
		if ignored.iter().any(|p| p.matches(key)) {
			continue;
		}
		// Report the first definition.
		let Some(definition) = definitions.iter().min() else {
			continue;
		};
		findings.push(UnusedKey {
			key: key.clone(),
			locale: reference.locale.clone(),
			definition: definition.clone(),
		});
	}

	findings.sort_by(|a, b| a.key.cmp(&b.key));
	Ok(findings)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::check::tests::{locale, locale_with_terms};

	fn used(keys: &[&str]) -> BTreeSet<String> {
		keys.iter().map(|k| (*k).to_owned()).collect()
	}

	fn keys_of(found: &[UnusedKey]) -> Vec<&str> {
		found.iter().map(|f| f.key.as_str()).collect()
	}

	#[test]
	fn a_used_key_is_not_reported() {
		let reference = locale("en", &["a", "b"]);
		let found = unused(&reference, &used(&["a", "b"]), &[]).expect("no globs to compile");
		assert!(found.is_empty());
	}

	#[test]
	fn an_unused_key_is_reported_with_its_definition() {
		let reference = locale("en", &["a", "stale"]);
		let found = unused(&reference, &used(&["a"]), &[]).expect("no globs to compile");
		assert_eq!(keys_of(&found), vec!["stale"]);
		assert_eq!(found[0].locale, "en");
		assert_eq!(found[0].definition.at.line, 1);
	}

	#[test]
	fn terms_are_never_reported() {
		// Nothing outside the `.ftl` files can reference a term.
		let reference = locale_with_terms("en", &["a", "-brand"], &["-brand"]);
		let found = unused(&reference, &used(&["a"]), &[]).expect("no globs to compile");
		assert!(found.is_empty());
	}

	#[test]
	fn an_ignore_glob_silences_a_key() {
		let reference = locale("en", &["err-404", "err-500", "stale"]);
		let found =
			unused(&reference, &used(&[]), &["err-*".to_owned()]).expect("the glob compiles");
		assert_eq!(keys_of(&found), vec!["stale"]);
	}

	#[test]
	fn several_ignore_globs_all_apply() {
		let reference = locale("en", &["err-404", "mail-hi", "stale"]);
		let found = unused(
			&reference,
			&used(&[]),
			&["err-*".to_owned(), "mail-*".to_owned()],
		)
		.expect("the globs compile");
		assert_eq!(keys_of(&found), vec!["stale"]);
	}

	#[test]
	fn an_invalid_ignore_glob_is_an_error() {
		let reference = locale("en", &["a"]);
		let err = unused(&reference, &used(&[]), &["[".to_owned()])
			.expect_err("an unclosed class is not a valid glob");
		assert!(format!("{err:#}").contains("--ignore-unused"));
	}

	#[test]
	fn findings_are_sorted_by_key() {
		let reference = locale("en", &["zebra", "apple", "mango"]);
		let found = unused(&reference, &used(&[]), &[]).expect("no globs to compile");
		assert_eq!(keys_of(&found), vec!["apple", "mango", "zebra"]);
	}
}
