// src/check/duplicates.rs

//! Duplicates: a key must be defined once per locale.

use std::collections::BTreeMap;

use crate::model::{DuplicateKey, LocaleKeys};

/// Finds keys defined more than once within a single locale.
#[must_use]
pub fn duplicates(locales: &BTreeMap<String, LocaleKeys>) -> Vec<DuplicateKey> {
	let mut findings = Vec::new();

	for (name, locale) in locales {
		for (key, definitions) in &locale.keys {
			if definitions.len() < 2 {
				continue;
			}
			let mut definitions = definitions.clone();
			definitions.sort();
			findings.push(DuplicateKey {
				key: key.clone(),
				locale: name.clone(),
				definitions,
			});
		}
	}

	findings.sort_by(|a, b| (&a.key, &a.locale).cmp(&(&b.key, &b.locale)));
	findings
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::check::tests::{locale, locale_with};

	#[test]
	fn a_key_defined_once_is_not_reported() {
		let locales = BTreeMap::from([("en".to_owned(), locale("en", &["a", "b"]))]);
		assert!(duplicates(&locales).is_empty());
	}

	#[test]
	fn two_definitions_in_different_files_are_reported() {
		let locales = BTreeMap::from([(
			"en".to_owned(),
			locale_with("en", &[("dup", &[("a.ftl", 3), ("b.ftl", 9)])]),
		)]);
		let found = duplicates(&locales);
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].key, "dup");
		assert_eq!(found[0].locale, "en");
		assert_eq!(found[0].definitions.len(), 2);
	}

	#[test]
	fn two_definitions_in_the_same_file_point_at_different_lines() {
		let locales = BTreeMap::from([(
			"en".to_owned(),
			locale_with("en", &[("dup", &[("a.ftl", 1), ("a.ftl", 7)])]),
		)]);
		let found = duplicates(&locales);
		let lines: Vec<usize> = found[0].definitions.iter().map(|d| d.at.line).collect();
		assert_eq!(lines, vec![1, 7]);
	}

	#[test]
	fn each_locale_is_reported_separately() {
		let locales = BTreeMap::from([
			(
				"en".to_owned(),
				locale_with("en", &[("dup", &[("a.ftl", 1), ("b.ftl", 1)])]),
			),
			(
				"fr".to_owned(),
				locale_with("fr", &[("dup", &[("a.ftl", 1), ("b.ftl", 1)])]),
			),
		]);
		let found = duplicates(&locales);
		assert_eq!(found.len(), 2);
		assert_eq!(found[0].locale, "en");
		assert_eq!(found[1].locale, "fr");
	}

	#[test]
	fn findings_are_sorted_by_key_then_locale() {
		let locales = BTreeMap::from([
			(
				"fr".to_owned(),
				locale_with("fr", &[("zebra", &[("a.ftl", 1), ("b.ftl", 1)])]),
			),
			(
				"en".to_owned(),
				locale_with(
					"en",
					&[
						("zebra", &[("a.ftl", 1), ("b.ftl", 1)]),
						("apple", &[("a.ftl", 2), ("b.ftl", 2)]),
					],
				),
			),
		]);
		let found = duplicates(&locales);
		let pairs: Vec<(&str, &str)> = found
			.iter()
			.map(|f| (f.key.as_str(), f.locale.as_str()))
			.collect();
		assert_eq!(
			pairs,
			vec![("apple", "en"), ("zebra", "en"), ("zebra", "fr")]
		);
	}
}
