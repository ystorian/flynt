// src/main.rs

//! The `flynt` command-line interface.

use std::io::{self, IsTerminal as _, Write as _};
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser as _;
use flynt::config::{self, PartialConfig};
use flynt::report;

/// Exit code for an internal problem.
const TOOL_ERROR: u8 = 2;

fn main() -> ExitCode {
	match run() {
		Ok(code) => ExitCode::from(code),
		Err(error) => {
			eprintln!("flynt: error: {error:#}");
			ExitCode::from(TOOL_ERROR)
		}
	}
}

/// Runs the checks and prints the report.
fn run() -> Result<u8> {
	let cli = PartialConfig::parse();
	let config = config::load(&cli)?;
	let report = flynt::check(&config)?;

	let mut out = io::stdout().lock();
	let color = report::color_enabled(config.color, out.is_terminal());
	report::render(&report, &config, color, &mut out)?;
	out.flush()?;

	Ok(report.exit_code())
}
