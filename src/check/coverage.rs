// src/check/coverage.rs

//! Coverage: used keys must be defined.

use std::collections::BTreeMap;

use crate::model::{KeyUsage, LocaleKeys, MissingKey};

/// Finds used keys missing from a locale.
#[must_use]
pub fn missing(used: &[KeyUsage], locales: &BTreeMap<String, LocaleKeys>) -> Vec<MissingKey> {
	let mut grouped: BTreeMap<&str, Vec<KeyUsage>> = BTreeMap::new();
	for usage in used {
		grouped
			.entry(usage.key.as_str())
			.or_default()
			.push(usage.clone());
	}

	let mut findings = Vec::new();
	for (key, mut usages) in grouped {
		let missing_in: Vec<String> = locales
			.iter()
			.filter(|(_, locale)| !locale.keys.contains_key(key))
			.map(|(name, _)| name.clone())
			.collect();

		if missing_in.is_empty() {
			continue;
		}

		usages.sort();
		findings.push(MissingKey {
			key: key.to_owned(),
			usages,
			missing_in,
		});
	}

	findings
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::check::tests::{locale, usage};

	#[test]
	fn key_everywhere_unreported() {
		let locales = BTreeMap::from([
			("en".to_owned(), locale("en", &["a", "b"])),
			("fr".to_owned(), locale("fr", &["a", "b"])),
		]);
		assert!(missing(&[usage("a"), usage("b")], &locales).is_empty());
	}

	#[test]
	fn missing_key_names_locale() {
		let locales = BTreeMap::from([
			("en".to_owned(), locale("en", &["a"])),
			("fr".to_owned(), locale("fr", &[])),
		]);
		let found = missing(&[usage("a")], &locales);
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].key, "a");
		assert_eq!(found[0].missing_in, vec!["fr".to_owned()]);
	}

	#[test]
	fn missing_key_locales_order() {
		let locales = BTreeMap::from([
			("fr".to_owned(), locale("fr", &[])),
			("en".to_owned(), locale("en", &[])),
			("de".to_owned(), locale("de", &[])),
		]);
		let found = missing(&[usage("ghost")], &locales);
		assert_eq!(
			found[0].missing_in,
			vec!["de".to_owned(), "en".to_owned(), "fr".to_owned()]
		);
	}

	#[test]
	fn repeated_usages_grouped() {
		let locales = BTreeMap::from([("en".to_owned(), locale("en", &[]))]);
		let found = missing(&[usage("a"), usage("a"), usage("a")], &locales);
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].usages.len(), 3);
	}

	#[test]
	fn findings_sorted_by_key() {
		let locales = BTreeMap::from([("en".to_owned(), locale("en", &[]))]);
		let found = missing(&[usage("zebra"), usage("apple"), usage("mango")], &locales);
		let keys: Vec<&str> = found.iter().map(|f| f.key.as_str()).collect();
		assert_eq!(keys, vec!["apple", "mango", "zebra"]);
	}

	#[test]
	fn no_locales_no_missing() {
		assert!(missing(&[usage("a")], &BTreeMap::new()).is_empty());
	}
}
