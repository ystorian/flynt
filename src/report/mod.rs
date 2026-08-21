// src/report/mod.rs

//! Rendering a [`Report`].
//!
//! Nothing here writes to `stdout` directly. Every renderer takes an [`std::io::Write`] instead.
//! Tests can then assert on the output.

pub mod json;
pub mod text;

use std::io::Write;

use crate::config::{ColorChoice, Config, OutputFormat};
use crate::model::Report;

/// Renders the report in the configured format.
///
/// # Errors
///
/// Propagates write failures from `out`. Also propagates serialization failures from the JSON
/// renderer.
pub fn render(
	report: &Report,
	config: &Config,
	color: bool,
	out: &mut impl Write,
) -> std::io::Result<()> {
	match config.format {
		OutputFormat::Text => text::render(report, config, color, out),
		OutputFormat::Json => json::render(report, out),
	}
}

/// Decides whether to emit color.
///
/// `Auto` also honors `NO_COLOR`.
#[must_use]
pub fn color_enabled(choice: ColorChoice, is_terminal: bool) -> bool {
	match choice {
		ColorChoice::Always => true,
		ColorChoice::Never => false,
		ColorChoice::Auto => is_terminal && std::env::var_os("NO_COLOR").is_none(),
	}
}

/// ANSI escapes, switched off in one place.
pub(crate) struct Paint {
	on: bool,
}

impl Paint {
	pub(crate) fn new(on: bool) -> Self {
		Self { on }
	}

	fn wrap(&self, code: &str, text: &str) -> String {
		if self.on {
			format!("\x1b[{code}m{text}\x1b[0m")
		} else {
			text.to_owned()
		}
	}

	pub(crate) fn red(&self, text: &str) -> String {
		self.wrap("31", text)
	}

	pub(crate) fn green(&self, text: &str) -> String {
		self.wrap("32", text)
	}

	pub(crate) fn yellow(&self, text: &str) -> String {
		self.wrap("33", text)
	}

	pub(crate) fn bold(&self, text: &str) -> String {
		self.wrap("1", text)
	}

	pub(crate) fn dim(&self, text: &str) -> String {
		self.wrap("2", text)
	}

	/// Wraps `text` as underlined cyan, marking it as a source location.
	pub(crate) fn link(&self, text: &str) -> String {
		if self.on {
			format!("\x1b[4;36m{text}\x1b[0m")
		} else {
			text.to_owned()
		}
	}

	/// Wraps `prefix` and `suffix` in green, with `bold_part` bold in between.
	///
	/// A plain nested wrap would not work: the bold segment's reset code would also clear the
	/// surrounding green.
	pub(crate) fn green_bold(&self, prefix: &str, bold_part: &str, suffix: &str) -> String {
		if self.on {
			format!("\x1b[32m{prefix}\x1b[1m{bold_part}\x1b[0m\x1b[32m{suffix}\x1b[0m")
		} else {
			format!("{prefix}{bold_part}{suffix}")
		}
	}
}

/// `"1 key"` but `"2 keys"`.
pub(crate) fn plural(count: usize, singular: &str) -> String {
	if count == 1 {
		format!("{count} {singular}")
	} else {
		format!("{count} {singular}s")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn always_and_never_ignore_the_terminal() {
		assert!(color_enabled(ColorChoice::Always, false));
		assert!(!color_enabled(ColorChoice::Never, true));
	}

	#[test]
	fn auto_needs_a_terminal() {
		// NO_COLOR is not set in the test environment. A terminal decides instead.
		assert!(!color_enabled(ColorChoice::Auto, false));
	}

	#[test]
	fn paint_is_a_no_op_when_off() {
		let plain = Paint::new(false);
		assert_eq!(plain.red("x"), "x");
		assert_eq!(plain.bold("x"), "x");
	}

	#[test]
	fn paint_wraps_when_on() {
		let colored = Paint::new(true);
		assert_eq!(colored.red("x"), "\x1b[31mx\x1b[0m");
	}

	#[test]
	fn counts_are_pluralized() {
		assert_eq!(plural(0, "key"), "0 keys");
		assert_eq!(plural(1, "key"), "1 key");
		assert_eq!(plural(2, "key"), "2 keys");
	}
}
