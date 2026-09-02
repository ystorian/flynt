// src/check/consistency.rs

//! Consistency: locales must share keys.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{InconsistentKey, LocaleKeys};

/// Finds keys some locales miss.
#[must_use]
pub fn inconsistent(locales: &BTreeMap<String, LocaleKeys>) -> Vec<InconsistentKey> {
	// A single locale cannot disagree.
	if locales.len() < 2 {
		return Vec::new();
	}

	let all: BTreeSet<&String> = locales
		.values()
		.flat_map(|locale| locale.keys.keys())
		.collect();

	let mut findings = Vec::new();
	for key in all {
		let (present_in, missing_in): (Vec<String>, Vec<String>) = locales
			.iter()
			.map(|(name, locale)| (name.clone(), locale.keys.contains_key(key)))
			.fold(
				(Vec::new(), Vec::new()),
				|(mut present, mut absent), (name, has)| {
					if has {
						present.push(name);
					} else {
						absent.push(name);
					}
					(present, absent)
				},
			);

		if missing_in.is_empty() {
			continue;
		}

		findings.push(InconsistentKey {
			key: key.clone(),
			present_in,
			missing_in,
		});
	}

	findings
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::check::tests::locale;

	#[test]
	fn matching_locales_report_nothing() {
		let locales = BTreeMap::from([
			("en".to_owned(), locale("en", &["a", "b"])),
			("fr".to_owned(), locale("fr", &["b", "a"])),
		]);
		assert!(inconsistent(&locales).is_empty());
	}

	#[test]
	fn finding_both_sides() {
		let locales = BTreeMap::from([
			("en".to_owned(), locale("en", &["a", "only-en"])),
			("fr".to_owned(), locale("fr", &["a"])),
		]);
		let found = inconsistent(&locales);
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].key, "only-en");
		assert_eq!(found[0].present_in, vec!["en".to_owned()]);
		assert_eq!(found[0].missing_in, vec!["fr".to_owned()]);
	}

	#[test]
	fn finding_inconsistent() {
		let locales = BTreeMap::from([
			("en".to_owned(), locale("en", &["shared", "only-en"])),
			("fr".to_owned(), locale("fr", &["shared", "only-fr"])),
		]);
		let found = inconsistent(&locales);
		let keys: Vec<&str> = found.iter().map(|f| f.key.as_str()).collect();
		assert_eq!(keys, vec!["only-en", "only-fr"]);
	}

	#[test]
	fn multiple_inconsistent() {
		let locales = BTreeMap::from([
			("de".to_owned(), locale("de", &[])),
			("en".to_owned(), locale("en", &["k"])),
			("fr".to_owned(), locale("fr", &["k"])),
		]);
		let found = inconsistent(&locales);
		assert_eq!(found[0].present_in, vec!["en".to_owned(), "fr".to_owned()]);
		assert_eq!(found[0].missing_in, vec!["de".to_owned()]);
	}

	#[test]
	fn findings_sorted_by_key() {
		let locales = BTreeMap::from([
			("en".to_owned(), locale("en", &["zebra", "apple", "mango"])),
			("fr".to_owned(), locale("fr", &[])),
		]);
		let keys: Vec<String> = inconsistent(&locales).into_iter().map(|f| f.key).collect();
		assert_eq!(keys, vec!["apple", "mango", "zebra"]);
	}

	#[test]
	fn single_locale_consistent() {
		let locales = BTreeMap::from([("en".to_owned(), locale("en", &["a", "b"]))]);
		assert!(inconsistent(&locales).is_empty());
	}
}
