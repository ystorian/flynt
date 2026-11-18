# @template/.just/rust.just

# Get the app version from `Cargo.toml`, with or without workspace.
version := trim(replace(`tomli query --filepath Cargo.toml workspace.package.version 2>/dev/null ||
	tomli query --filepath Cargo.toml package.version`, '"', ''))

# Release tag `v#.#.#`.
tag := 'v' + version

# Display "in quiet mode" for non-release profiles.
display_profile(profile) := if profile != 'release' { echo(info(['Profile:', BOLD + profile, 'in quiet mode'])) + ';' + set_title('🦀 ' + recipe_name()) } else { set_title('🦀 ' + recipe_name()) }

# Clippy deny list for `strict` mode.
clippy_deny_list := ['doc_paragraphs_missing_punctuation', 'min_ident_chars', 'missing_docs_in_private_items', 'redundant_test_prefix']

# Clippy warn list for `strict` mode.
clippy_warn_list := ['absolute_paths', 'allow_attributes', 'allow_attributes_without_reason', 'float_cmp_const', 'needless_raw_strings', 'str_to_string', 'string_slice', 'separated_literal_suffix', 'unused_trait_names']

# Clippy allow list.
clippy_allow_list := ['arbitrary_source_item_ordering', 'assertions_on_result_states', 'default_numeric_fallback', 'float_arithmetic', 'needless_return', 'print_stdout', 'question_mark_used', 'separated_literal_suffix', 'single_call_fn']

# Host target tuple.
host_tuple := shell('rustc --print host-tuple')

# Check the Visual Studio build tools environment variables
[no-exit-message]
[private]
[script]
[windows]
check_env:
	if ! [ -z "$(printenv VCINSTALLDIR)" ]; then
		echo {{ error(['The environment variable', BOLD + 'VCINSTALLDIR', 'is set']) }};
		exit 1;
	fi

# Check the Visual Studio build tools environment variables
[private]
[unix]
check_env:
	exit 0

# Benchmark
[group('Rust')]
bench profile='dev': check_env
	echo {{ title(['Run', BOLD + 'cargo bench']) }}
	{{ display_profile(profile) }}

	cargo bench --all-features --workspace \
		{{ if profile != 'release' { '--profile ' + profile + ' --quiet' } }}

	echo {{ done([profile, reset_title]) }}

# Check for errors
[group('Rust')]
check profile='dev': check_env
	echo {{ title(['Run', BOLD + 'cargo check']) }}
	{{ display_profile(profile) }}

	cargo check --workspace --all-features --future-incompat-report \
		{{ if profile != 'release' { '--profile ' + profile + ' --quiet' } else { '--release' } }}

	echo {{ done([profile, reset_title]) }}

# Check dependencies and licenses
[group('Rust')]
check_deny: check_command_deny
	echo {{ title(['Check dependencies and licenses with', BOLD + 'cargo-deny']) }}

	# Delete the cache if corrupted.
	cache="${TMPDIR:-${TEMP:-/tmp}}/cargo-deny/std-replacement-data"; \
	if [ -d "$cache" ] && { [ ! -f "$cache/.git/HEAD" ] || [ ! -f "$cache/all.bin" ]; }; then \
		just delete_dir "$cache"; \
	fi

	cargo-deny --all-features --workspace --log-level error check

	echo {{ done('') }}

# Format Rust code
[group('Rust')]
format:
	echo {{ title(['Run', BOLD + 'cargo fmt']) }}

	cargo fmt --all

	echo {{ done('') }}
alias fmt := format

# Check Rust code formatting
[group('Rust')]
check_rust:
	echo {{ title(['Check Rust code formatting with', BOLD + 'cargo fmt']) }}

	echo {{ info(shell('cargo fmt --version')) }}
	cargo fmt --all --check

	echo {{ done('') }}

# Clippy
[arg('fix', long='fix', flag, help='Fix easy Clippy lints')]
[arg('links', long='links', flag, help='Show Clippy docs links')]
[arg('dupes', long='dupes', flag, help='Show warnings on multiple dependency versions')]
[arg('strict', long='strict', flag, help='Enable strict Ystorian lints')]
[group('Rust')]
[no-exit-message]
clippy links strict dupes fix profile='dev': check_env
	echo {{ title(['Run', BOLD + 'cargo clippy']) }}
	{{ set_title('🦀 ' + recipe_name()) }}
	{{ if !links { echo(dim(['Clippy docs links are hidden by default, set', BOLD + '--links', 'to show them'])) } }}
	{{ if dupes { echo(warning([BOLD + 'dupes', 'will be flagged'])) } }}
	{{ if strict { echo(warning([BOLD + 'strict', 'mode enabled with Ystorian lints'])) } }}
	{{ if fix { echo(warning([BOLD + 'fix', 'mode enabled'])) } }}

	# The `ci` profile runs offline.

	{{ if !links && !strict { 'env CLIPPY_DISABLE_DOCS_LINKS=true' } }} \
	cargo clippy \
		{{ if profile != 'release' { '--profile ' + profile + ' --quiet' } else { '--release' } }} \
		{{ if fix { '--fix --allow-dirty' } }} \
		{{ if profile == 'ci' { '--frozen' } }} \
		--all-targets --all-features --all -- \
		--warn clippy::pedantic \
		--warn clippy::cargo \
		--allow clippy::multiple_crate_versions \
		--deny clippy::all \
		{{ if dupes { '--warn clippy::multiple_crate_versions' } }} \
		{{ if strict { prepend('--warn clippy::', clippy_warn_list) } }} \
		{{ if strict { prepend('--deny clippy::', clippy_deny_list) } }} \
		{{ prepend('--allow clippy::', clippy_allow_list) }} \
		--deny warnings

	echo {{ done([profile, reset_title]) }}

# Clippy with only `lint`
[env('CARGO_TERM_QUIET', 'true')]
[group('Rust')]
clippy_lint lint: check_env
	echo {{ title(['Run', BOLD + 'cargo clippy']) }}
	echo {{ info(['Lint:', BOLD + lint]) }}

	cargo clippy --all-targets --all-features --all -- --warn clippy::{{ lint }}

	echo {{ done(lint) }}

# Fix easy Clippy lints (requires no uncommitted changes)
[group('Rust')]
clippy_fix: check_env
	echo {{ title(['Fix', BOLD + 'easy', 'Clippy lints']) }}

	just clippy --fix

	echo {{ done('') }}

# Run unit and integration tests
[group('Rust')]
[no-exit-message]
[script]
test profile='dev': check_env
	echo {{ title(['Run', BOLD + 'cargo test']) }}
	{{ display_profile(profile) }}

	# Notes:
	# - `kache test-runner` version 1.00 errors with "no LC_RPATH's found".
	# - We bypass `kache` on macOS with `env -u CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER`.
	# - The `ci` profile runs once, offline and locked (`--frozen`).

	cargo_test='cargo test --all-features --workspace --future-incompat-report';

	# Bypass `kache` on macOS.
	env -u CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER >/dev/null;

	if [ "{{ profile }}" = "ci" ]; then
		$cargo_test --frozen --profile ci;
	elif [ "{{ profile }}" != "release" ]; then
		if ! $cargo_test --quiet --profile {{ profile }} >/dev/null 2>&1; then
			echo {{ warning('Tests failed, running again with verbose output') }};
			$cargo_test --profile {{ profile }};
		fi;
	else
		$cargo_test --release;
	fi

	echo {{ done([profile, reset_title]) }}

# Clean the workspace
[group('Rust')]
clean_workspace:
	echo {{ title(['Clean the', BOLD + 'workspace']) }}

	cargo-clean-all --keep-empty-target --keep-executable-same-dir --yes .

	echo {{ done('') }}

# Delete the workspace artifacts of `profile`, keep the dependencies
[group('Rust')]
[script]
prune_target profile='dev': check_command_jq
	echo {{ title(['Prune the workspace artifacts in', BOLD + 'target/' + profile]) }}
	# Used before the CI saves the Cargo cache.

	# Cargo names the folder `debug` for `dev` and `test`, `release` for `bench`.
	case "{{ profile }}" in
		dev | test) dir="target/debug" ;;
		bench) dir="target/release" ;;
		*) dir="target/{{ profile }}" ;;
	esac

	if [ ! -d "$dir" ]; then
		echo {{ warning('The target is empty for this profile, nothing to prune') }};
		exit 0;
	fi

	echo {{ note(['Before pruning:', YELLOW + '$(du -sh "target")']) }};

	# `cargo clean -p` supports all the `build-dir` layouts.
	# Remove the `\r` that `jq` adds on Windows.
	packages="$(cargo metadata --frozen --no-deps --format-version 1 \
		| jq -r '.packages[].name | "-p " + .' | tr -d '\r')"
	cargo clean --frozen --quiet --profile {{ profile }} $packages
	rm -rf "$dir/incremental" "$dir/examples"
	echo {{ note(['After pruning: ', GREEN + '$(du -sh "target")']) }};

	echo {{ done(profile) }}

# Build and install on this machine
[group('Rust')]
install: check_env
	echo {{ title(['Run', BOLD + 'cargo install']) }}
	{{ display_profile('release') }}

	cargo install --path .

	echo {{ done([reset_title]) }}

# Download the crates in `Cargo.toml`
[group('Rust')]
fetch:
	echo {{ title(['Run', BOLD + 'cargo fetch', 'for', BOLD + host_tuple]) }}

	cargo fetch --locked --verbose --target "{{ host_tuple }}"

	echo {{ done('') }}

# Check if `cargo-deny` is available
[group('Utils')]
[no-exit-message]
[private]
check_command_deny:
	just check_command cargo-deny -c='cargo-deny'
