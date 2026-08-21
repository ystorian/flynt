# .just/rust.just

# Get the app version from `Cargo.toml`, with or without workspace.
# Requires `tomli`, installed by `just install_cargo_tools`.
version := trim(replace(`tomli query --filepath Cargo.toml workspace.package.version 2>/dev/null ||
	tomli query --filepath Cargo.toml package.version`, '"', ''))
# Release tag `v#.#.#`.
tag := 'v' + version

# Benchmark
[env("CARGO_TERM_QUIET", "true")]
[group('Rust')]
bench:
	echo {{ CYAN }}Run {{ BOLD }}cargo bench{{ NORMAL + CYAN + style('dim') }} in quiet mode{{ NORMAL }}
	cargo bench --all-features --workspace
	echo {{ done('') }}

# Check for errors
[env("CARGO_TERM_QUIET", "true")]
[group('Rust')]
check:
	echo {{ CYAN }}Run {{ BOLD }}cargo check{{ NORMAL + CYAN + style('dim') }} in quiet mode{{ NORMAL }}
	cargo check --workspace --all-features
	echo {{ done('') }}

# Check dependencies and licenses
[group('Rust')]
deny:
	echo {{ CYAN }}Run {{ BOLD }}cargo-deny{{ NORMAL }}
	cargo-deny --all-features --workspace --log-level error check
	echo {{ done('') }}

# Format Rust code
[group('Rust')]
format:
	echo {{ CYAN }}Run {{ BOLD }}cargo fmt{{ NORMAL }}
	cargo fmt --all
	echo {{ done('') }}
alias fmt := format

# Clippy
[env("CARGO_TERM_QUIET", "true")]
[group('Rust')]
clippy:
	echo {{ CYAN }}Run {{ BOLD }}cargo clippy {{ YELLOW + ITALIC }}pedantic{{ NORMAL + CYAN + style('dim') }} in quiet mode{{ NORMAL }}
	cargo clippy --all-targets --all-features --all -- --deny clippy::all --warn clippy::pedantic
	echo {{ done('') }}
alias lint := clippy

# Clippy with only `lint`
[env("CARGO_TERM_QUIET", "true")]
[group('Rust')]
clippy_lint lint:
	echo {{ CYAN }}Run {{ BOLD }}cargo clippy{{ NORMAL + CYAN }} for the {{ YELLOW + lint + CYAN }} lint{{ NORMAL }}
	cargo clippy --all-targets --all-features --all -- --warn clippy::{{ lint }}
	echo {{ done(lint) }}

# Fix easy Clippy lints (requires no uncommitted changes)
[env("CARGO_TERM_QUIET", "true")]
[group('Rust')]
clippy_fix:
	echo {{ CYAN }}Fix {{ GREEN }}easy{{ CYAN }} Clippy lints{{ NORMAL }}
	echo ⚠️ {{ YELLOW }}Commit all your changes first{{ NORMAL }}
	cargo clippy --fix
	echo {{ done('') }}

# Run unit and integration tests
[group('Rust')]
[no-exit-message]
test:
	echo {{ CYAN }}Run {{ BOLD }}cargo test{{ NORMAL + CYAN + style('dim') }} in quiet mode{{ NORMAL }}
	if ! cargo test --all-features --workspace --future-incompat-report --quiet >/dev/null 2>&1; then \
		cargo test --all-features --workspace --future-incompat-report \
	;fi
	echo {{ done('') }}

# Clean the workspace
[group('Rust')]
clean_workspace:
	echo {{ CYAN }}Clean the workspace{{ NORMAL }}
	cargo-clean-all --yes .
	just deny
	echo {{ done('') }}

# Build and install on this machine
[env("CARGO_TERM_QUIET", "true")]
[group('Rust')]
install:
	echo {{ CYAN }}Run {{ BOLD }}cargo install{{ NORMAL + CYAN + style('dim') }} in quiet mode{{ NORMAL }}
	cargo install --path .
	echo {{ done('') }}

# Download the crates in `Cargo.toml`
[env("CARGO_TERM_QUIET", "true")]
[group('Rust')]
fetch:
	echo {{ CYAN }}Run {{ BOLD }}cargo fetch{{ NORMAL + CYAN + style('dim') }} in quiet mode{{ NORMAL }}
	cargo fetch --locked
	echo {{ done('') }}
