# @template/.just/cargo.just

# Variables.

# Windows.
llvm_var := 'llvmArm64'
llvm_path := '%ProgramFiles(x86)%\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\ARM64\bin'
windows_kits := '%ProgramFiles(x86)%\Windows Kits\10\bin\10.0.28000.0\arm64'

# Cargo tools required for all Rust projects.
required_cargo_tools := ['cargo-clean-all', 'cargo-deny', 'cargo-update', 'just', 'tomli']
# Note:
# - Additional Cargo tools required by this project are defined in `.just/_main.just`.
# - In `_main.just` set the variable `additional_cargo_tools := []` for none.

# cargo-binstall.
binstall_options := ['--locked', '--no-confirm', '--disable-telemetry']

# Native target per OS.
[linux]
native_target := arch() + '-unknown-linux-gnu'
[macos]
native_target := arch() + '-apple-darwin'
[windows]
native_target := arch() + '-pc-windows-msvc'

# API token for `crates.io`.
#
# Note: use `cargo login` to generate.
crates_io_token := trim(replace(shell('tomli query \
	--filepath ' + env('CARGO_HOME') + '/credentials.toml \
	registry.token \
	2>/dev/null \
	|| echo ""'), '"', ''))

# User-Agent for `crates.io`, required for API requests.
crates_io_user_agent := 'cargo-just (+https://github.com/' + owner + '/' + repo + ')'

# Return true if publish is true in Cargo.toml (supports workspace).
cargo_is_publishable := if trim(shell('tomli query --filepath Cargo.toml workspace.package.publish 2>/dev/null || tomli query --filepath Cargo.toml package.publish 2>/dev/null || echo false')) == 'true' { 'true' } else { 'false' }

# Return true if package is set using `include` in Cargo.toml (supports workspace).
cargo_has_include := if trim(shell('tomli query --filepath Cargo.toml workspace.package.include 2>/dev/null || tomli query --filepath Cargo.toml package.include 2>/dev/null || echo false')) =~ 'Cargo' { 'true' } else { 'false' }

# Install cargo tools
[env("GITHUB_TOKEN", github_token)]
[group('Cargo')]
install_cargo_tools:
	echo {{ CYAN }}Install cargo tools with {{ BOLD }}cargo-binstall{{ NORMAL }}

	# Install cargo-binstall first.
	if ! command -v cargo-binstall >/dev/null 2>&1; then \
		echo {{ BLUE }}Install {{ GREEN }}cargo-binstall{{ NORMAL }}; \
		cargo install --locked --target {{ native_target }} cargo-binstall; \
	fi

	echo {{ dim(['Required:', GREEN + required_cargo_tools]) }}
	echo {{ dim(['Additional:', YELLOW + additional_cargo_tools]) }}

	cargo-binstall \
		{{ join_list(binstall_options) }} \
		--targets {{ native_target }} \
		--log-level warn \
		{{ join_list(required_cargo_tools) }} \
		{{ join_list(additional_cargo_tools) }}

	echo {{ done('') }}

# Update Rust
[group('Cargo')]
update_rust:
	echo {{ CYAN }}Update Rust{{ NORMAL }}

	echo {{ dim('self update') }}
	rustup --quiet self update >/dev/null

	echo {{ dim('update') }}
	rustup --quiet update stable  >/dev/null

	echo {{ dim('rust-analyzer') }}
	rustup --quiet component add rust-analyzer

	echo {{ info(shell('rustc --version')) }}

	echo {{ done('') }}

# Update Cargo tools
[group('Cargo')]
update_cargo_tools: install_cargo_tools
	echo {{ CYAN }}Update cargo tools{{ NORMAL }}
	echo {{ dim(['Cooldown:', GREEN + BOLD + '7', GREEN + 'days']) }}

	cargo install-update --all --git --locked --cooldown 7d --quiet

	echo {{ done('') }}

# Update Cargo.lock without bumping dependencies
[group('Cargo')]
lock_cargo:
	echo {{ CYAN }}Update {{ GREEN }}Cargo.lock{{ NORMAL }}

	cargo update --offline --workspace --verbose

	echo {{ done('') }}

# Bump all Cargo.lock dependencies
[group('Cargo')]
bump_cargo:
	echo {{ CYAN }}Bump all dependencies in {{ YELLOW }}Cargo.lock{{ NORMAL }}

	cargo update --verbose

	just deny

	echo {{ done('') }}

# Check the crate for publication in crates.io
[group('Cargo')]
[no-exit-message]
publish_check:
	echo {{ CYAN }}Check this crate{{ NORMAL }}

	echo {{ dim('Cargo.toml: package.publish = true') }}
	if [ "{{ cargo_is_publishable }}" = "false" ]; then \
		echo {{ error([BOLD + 'package.publish = true', 'is not set in', BOLD + 'Cargo.toml']) }}; \
		exit 1; \
	fi

	echo {{ dim('Cargo.toml: package.include = [...]') }}
	if [ "{{ cargo_has_include }}" = "false" ]; then \
		echo {{ error([BOLD + 'package.include', 'is not set in', BOLD + 'Cargo.toml']) }}; \
		exit 1; \
	fi

	echo {{ CYAN }}List {{ YELLOW }}non-standard{{ CYAN }} files that will be included{{ RED }}
	-cargo package --list --allow-dirty \
	| grep -vE '^(Cargo\.toml(\.orig)?|Cargo\.lock|\.cargo_vcs_info\.json|src/.*|LICENSE.*|README.*|[a-z]*\.svg)$'

	echo {{ CYAN }}Run {{ YELLOW }}cargo publish{{ CYAN }} in {{ GREEN }}dry-run{{ NORMAL }}

	cargo publish --dry-run --all-features --workspace --locked --allow-dirty

# Publish the crate to crates.io
[group('Cargo')]
publish: publish_check
	echo {{ CYAN }}Publish to {{ GREEN }}crates.io{{ NORMAL }}

	just check_tag {{ tag }}
	cargo publish

	echo {{ done('') }}

# Git pre-commit hook for Cargo
[group('Cargo')]
[no-exit-message]
[private]
precommit_cargo:
	# Check if `Cargo.toml` and `Cargo.lock` are in sync.
	if ! cargo metadata --frozen --format-version 1 --quiet >/dev/null 2>&1; then \
		echo "Cargo.lock is not in sync with Cargo.toml, run: just lock_cargo" >&2 && exit 1; \
	fi

	# Check if Rust code is formatted correctly.
	if ! cargo fmt --all --check --quiet >/dev/null 2>&1; then \
		echo "Rust code is incorrectly formatted, run: just format" >&2 && exit 1; \
	fi

# Return the MSRV for this repo and its crates
[group('Cargo')]
check_msrv:
	echo {{ CYAN }}Get the {{ YELLOW }}Minimum Supported Rust Version{{ NORMAL }}

	echo {{ YELLOW }}MRSV:{{ GREEN + BOLD }} \
		`cargo metadata --frozen --format-version 1 | jq -r '\
			[.packages[].rust_version | select(. != null)] \
			| map(split(".") | map(tonumber)) \
			| max \
			| map(tostring) \
			| join(".")'`

	echo {{ NORMAL + CYAN }}Required by these crates:{{ YELLOW }}
	cargo metadata --frozen --format-version 1 | jq -r '\
		[.packages[] | select(.rust_version != null) | {name, version, rust_version}] \
		| map(. + {parts: (.rust_version | split(".") \
		| map(tonumber))}) \
		| max_by(.parts) as $max \
		| map(select(.parts == $max.parts)) \
		| .[] \
		| "\(.name) v\(.version) msrv:\(.rust_version)" \
		' \
	| column -t

	echo {{ NORMAL }}

# Create a minisign key pair
[group('Cargo')]
[no-exit-message]
create_minisign_keys:
	echo {{ CYAN }}Create a {{ YELLOW }}Minisign{{ NORMAL + CYAN }} key pair{{ NORMAL }}

	minisign -G -p signing.pub -s signing.key

	echo {{ done('') }}

# Check if `curl` is available
[group('Cargo')]
[private]
check_command_curl:
	just check_command curl 'brew install curl' 'winget install --id cURL.cURL'

# Check the security settings for `crate`
[group('Cargo')]
[no-exit-message]
check_crate_security crate=repo:
	# API: https://crates.io/api/openapi.json
	echo {{ CYAN }}Check {{ BOLD }}crates.io{{ NORMAL + CYAN }} security settings for \
		{{ YELLOW + crate + NORMAL }}

	just check_command_curl
	just check_command_jq

	crate_json=$(curl \
		--silent \
		--fail \
		--user-agent "{{ crates_io_user_agent }}" \
		"https://crates.io/api/v1/crates/{{ crate }}") || { \
		echo {{ error(['Crate is not published on', BOLD + 'crates.io']) }}; \
		exit 1;} \
	&& if [ "$(printf '%s' "$crate_json" | jq -r '.crate.trustpub_only')" = "true" ]; then \
		echo {{ info(['Trusted publishing is', GREEN + 'enabled']) }}; \
	else \
		echo {{ error('Trusted publishing is not enabled') }}; \
		exit 1; \
	fi

	if [ "{{ crates_io_token }}" = "" ]; then \
		echo {{ error(['Not logged in', BOLD + 'crates.io', 'registry']) }}; \
		echo {{ info(['Run', MAGENTA + 'cargo login']) }}; \
		exit 1; \
	fi

	curl \
		--silent \
		--fail \
		--user-agent "{{ crates_io_user_agent }}" \
		--header "Authorization: {{ crates_io_token }}" \
		"https://crates.io/api/v1/trusted_publishing/github_configs?crate={{ crate }}" \
	| jq -r '.github_configs[] \
		| "Repository:  \(.repository_owner)/\(.repository_name)\nWorkflow:    \(.workflow_filename)\nEnvironment: \(.environment // "any")\n"'

# Show the crates depending on `crate`
[group('Cargo')]
[no-exit-message]
[script]
deps crate:
	# Note: runs once per version when `crate` has multiple versions.
	tree="cargo tree --target all --all-features --workspace"
	case "{{ crate }}" in
		*@*) $tree --invert "{{ crate }}"; exit ;;
	esac
	# List each version of the crate once.
	versions=$($tree --prefix none --format '{p}' --quiet \
		| grep "^{{ crate }} v" | cut -d ' ' -f 2 | sort -u)
	if [ -z "$versions" ]; then
		echo {{ error([BOLD + crate, 'not found in the dependency tree']) }}
		exit 1
	fi
	for version in $versions; do
		$tree --invert "{{ crate }}@${version#v}"
		echo
	done

# Show crates with multiple versions
[group('Cargo')]
[no-exit-message]
deps_dupe:
	echo {{ CYAN }}Show duplicate crates with multiple versions{{ NORMAL }}
	cargo tree --target all --all-features --workspace --duplicates --quiet

# Check for Visual Studio Build Tools
[group('Cargo')]
[no-exit-message]
[script]
[windows]
check_build_tools: check_command_bash
	echo {{ CYAN }}Check for Visual Studio {{ BOLD }}Build Tools{{ NORMAL }}

	error='0';

	just test_var {{ llvm_var }}
	echo {{ dim(['LLVM environment variable:', BOLD + llvm_var, '=', YELLOW + env(llvm_var)]) }}

	if [ -d "${{ llvm_var }}" ]; then
		echo {{ dim(['LLVM directory:', GREEN + env(llvm_var)]) }}
	else
		echo {{ error(['LLVM directory not found:', BOLD + env(llvm_var)]) }};
		error='1';
	fi

	if [ -f "{{ env(llvm_var) }}/lld-link.exe" ]; then
		echo {{ dim(['lld-link version:', GREEN + shell('"' + env(llvm_var) + '/lld-link.exe" --version')]) }};
	else
		echo {{ error(['lld-link.exe not found in:', RED + env(llvm_var)]) }};
		error='1';
	fi

	signtool=$(command -v signtool || true);
	if [ -f "$signtool" ]; then
		echo {{ dim(['signtool:', which('signtool')]) }};
	else
		echo {{ error(['Executable not found:', RED + 'signtool.exe']) }};
		echo {{ info(['PATH should contain', MAGENTA + windows_kits]) }};
		error='1';
	fi

	if [[ "{{ env('PATH') }}" =~ "{{ env(llvm_var) }}" ]]; then
		echo {{ dim(['PATH contains', GREEN + env(llvm_var)]) }};
	else
		echo {{ error(['PATH does not contain', BLUE + env(llvm_var)]) }};
		error='1';
	fi

	exit $error
