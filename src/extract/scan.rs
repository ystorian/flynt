// src/extract/scan.rs

//! Shared file walking and byte-offset-to-line mapping.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::config::Config;

/// Maps byte offsets in a file to one-based line and column numbers.
///
/// Matching runs over the whole file to find calls or filters split across lines.
pub struct LineIndex<'a> {
	content: &'a str,
	/// Byte offset of the start of each line.
	starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
	/// Indexes the line starts of `content`.
	pub fn new(content: &'a str) -> Self {
		let mut starts = vec![0];
		starts.extend(
			content
				.char_indices()
				.filter(|(_, c)| *c == '\n')
				.map(|(i, _)| i + 1),
		);
		Self { content, starts }
	}

	/// One-based line number containing `offset`.
	fn line_of(&self, offset: usize) -> usize {
		// `partition_point` gives the count of starts at or before the offset.
		self.starts.partition_point(|&start| start <= offset).max(1)
	}

	/// One-based line and column of `offset`.
	pub fn locate(&self, offset: usize) -> (usize, usize) {
		let line = self.line_of(offset);
		let start = self.starts[line - 1];
		let offset = offset.min(self.content.len());
		let column = self.content[start..offset].chars().count() + 1;
		(line, column)
	}

	/// Whether the line containing `offset` is a `//` comment.
	pub fn is_comment_line(&self, offset: usize) -> bool {
		let line = self.line_of(offset);
		let start = self.starts[line - 1];
		let end = self.starts.get(line).copied().unwrap_or(self.content.len());
		self.content[start..end].trim_start().starts_with("//")
	}
}

/// Walks directories, skipping hidden entries and excluded paths.
pub struct Walker {
	root: PathBuf,
	exclude: Vec<glob::Pattern>,
	follow_links: bool,
}

impl Walker {
	/// Compiles the exclude globs from the configuration.
	///
	/// # Errors
	///
	/// Returns an error when an exclude entry is not a valid glob.
	pub fn new(config: &Config) -> Result<Self> {
		let exclude = config
			.exclude
			.iter()
			.map(|p| {
				glob::Pattern::new(p)
					.with_context(|| format!("--exclude {p:?} is not a valid glob"))
			})
			.collect::<Result<Vec<_>>>()?;
		Ok(Self {
			root: config.root.clone(),
			exclude,
			follow_links: config.follow_links,
		})
	}

	/// Lists the files under `dir` that `accept` approves, sorted.
	///
	/// A directory that does not exist yields nothing.
	///
	/// # Errors
	///
	/// Returns an error when a directory that does exist cannot be read.
	pub fn files(&self, dir: &Path, accept: impl Fn(&Path) -> bool) -> Result<Vec<PathBuf>> {
		if !dir.is_dir() {
			return Ok(Vec::new());
		}

		let mut found = Vec::new();
		let walk = WalkDir::new(dir)
			.follow_links(self.follow_links)
			.sort_by_file_name()
			.into_iter()
			.filter_entry(|e| !self.is_skipped(e.path(), e.depth()));

		for entry in walk {
			let entry =
				entry.with_context(|| format!("cannot read the directory: {}", dir.display()))?;
			if entry.file_type().is_file() && accept(entry.path()) {
				found.push(entry.path().to_path_buf());
			}
		}

		found.sort();
		Ok(found)
	}

	/// Whether an entry should not be descended into or read.
	fn is_skipped(&self, path: &Path, depth: usize) -> bool {
		// Never skip the directory the walk started from. Even if its own name
		// would match, the caller asked for it explicitly.
		if depth == 0 {
			return false;
		}

		let name = path.file_name().unwrap_or_default().to_string_lossy();
		if name.starts_with('.') {
			return true;
		}

		let relative = path.strip_prefix(&self.root).unwrap_or(path);
		self.exclude.iter().any(|p| p.matches_path(relative))
	}
}

/// Reads a file as UTF-8, naming it on failure.
///
/// # Errors
///
/// Returns an error when the file cannot be read or is not UTF-8.
pub fn read(path: &Path) -> Result<String> {
	std::fs::read_to_string(path)
		.with_context(|| format!("cannot read the file: {}", path.display()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_first_character_is_line_one_column_one() {
		let index = LineIndex::new("abc\ndef");
		assert_eq!(index.locate(0), (1, 1));
	}

	#[test]
	fn offsets_map_to_the_right_line_and_column() {
		let content = "one\ntwo\nthree\n";
		let index = LineIndex::new(content);
		assert_eq!(index.locate(0), (1, 1));
		assert_eq!(index.locate(2), (1, 3));
		// The newline itself still belongs to the line it terminates.
		assert_eq!(index.locate(3), (1, 4));
		assert_eq!(index.locate(4), (2, 1));
		assert_eq!(index.locate(8), (3, 1));
		assert_eq!(index.locate(12), (3, 5));
	}

	#[test]
	fn columns_count_characters_not_bytes() {
		// Each accented character is two bytes. A byte column would drift.
		let content = "éé\"key\"";
		let index = LineIndex::new(content);
		let offset = content.find('"').expect("the quote is there");
		assert_eq!(index.locate(offset), (1, 3));
	}

	#[test]
	fn a_four_byte_character_counts_as_one_column() {
		// An emoji is four bytes and one `char`.
		let content = "👋👋\"key\"";
		let index = LineIndex::new(content);
		let offset = content.find('"').expect("the quote is there");
		assert_eq!(offset, 8);
		assert_eq!(index.locate(offset), (1, 3));
	}

	#[test]
	fn an_offset_past_the_end_does_not_panic() {
		let index = LineIndex::new("abc");
		assert_eq!(index.locate(99), (1, 4));
	}

	#[test]
	fn a_line_with_no_trailing_newline_is_still_indexed() {
		let index = LineIndex::new("a\nb");
		assert_eq!(index.locate(2), (2, 1));
	}

	#[test]
	fn comment_lines_are_recognized_in_every_form() {
		let content = "let a = 1;\n// plain\n\t/// doc\n  //! inner\ncode();\n";
		let index = LineIndex::new(content);
		let at = |needle: &str| content.find(needle).expect("the marker is there");

		assert!(!index.is_comment_line(at("let a")));
		assert!(index.is_comment_line(at("plain")));
		assert!(index.is_comment_line(at("doc")));
		assert!(index.is_comment_line(at("inner")));
		assert!(!index.is_comment_line(at("code()")));
	}

	#[test]
	fn a_trailing_comment_does_not_make_the_line_a_comment() {
		let content = "let key = 1; // see loc(\"x\")\n";
		let index = LineIndex::new(content);
		assert!(!index.is_comment_line(0));
	}
}
