# @template/.just/cargo.just

# Variables.

# Cargo tools required for all Rust projects.
required_cargo_tools := ['cargo-clean-all', 'cargo-deny', 'cargo-update', 'just', 'tomli']
# Note:
# - Additional Cargo tools required by this project are defined in `.just/_main.just`.
# - In `_main.just` set the variable `additional_cargo_tools := []` for none.

# cargo-binstall.
binstall_options := ['--locked', '--no-confirm', '--only-signed', '--disable-telemetry']

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

	rustup --quiet self update >/dev/null
	rustup --quiet update stable  >/dev/null
	rustup --quiet component add rust-analyzer
	rustc --version

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

	echo "{{ dim('Cargo.toml: package.publish = true') }}"
	if [ "{{ cargo_is_publishable }}" = "false" ]; then \
		echo "{{ error([BOLD + 'package.publish = true', 'is not set in', BOLD + 'Cargo.toml']) }}"; \
		exit 1; \
	fi

	echo "{{ dim('Cargo.toml: package.include = [...]') }}"
	if [ "{{ cargo_has_include }}" = "false" ]; then \
		echo "{{ error([BOLD + 'package.include', 'is not set in', BOLD + 'Cargo.toml']) }}"; \
		exit 1; \
	fi

	echo {{ CYAN }}List {{ YELLOW }}non-standard{{ CYAN }} files that will be included{{ RED }}
	-cargo package --list --allow-dirty \
	| grep -vE '^(Cargo\.toml(\.orig)?|Cargo\.lock|\.cargo_vcs_info\.json|src/.*|LICENSE.*|README.*|[a-z]*\.svg)$'

	echo {{ CYAN }}Run {{ YELLOW }}cargo publish{{ CYAN }} in {{ GREEN }}dry-run{{ NORMAL }}

	cargo publish --dry-run --all-features --workspace --locked --allow-dirty

# Publish the crate to crates.io
[group('Cargo')]
publish: publish_check check_tag
	echo {{ CYAN }}Publish to {{ GREEN }}crates.io{{ NORMAL }}

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

# Check if this release can be tagged `v#.#.#`
[arg('allow_dirty', long='allow-dirty', value='true')]
[group('Cargo')]
[no-exit-message]
check_tag tag=tag allow_dirty='false':
	echo {{ CYAN }}Check if this release can be tagged {{ GREEN + tag + NORMAL }}

	# Tag name starts with `v`.
	if [ "{{ replace_regex(tag, '^v\d.*', '') }}" != "" ]; then \
		echo "{{ error(['Tag name must start with', GREEN + 'v', 'not', RED + replace_regex(tag, '\d+\.\d+\.\d+.*', '')]) }}"; \
		exit 1; \
	fi

	# Tag has a semver format `v#.#.#`.
	if [ "{{ replace_regex(tag, 'v\d+\.\d+\.\d+', '') }}" != "" ]; then \
		echo "{{ error(['Tag version is not a', BOLD + 'semver', 'version like', GREEN + 'x.y.z']) }}"; \
		exit 1; \
	fi

	# Tag version is equal or higher than version in `Cargo.toml`.
	if [ "{{ semver_matches(replace(tag, 'v', ''), '>=' + version) }}" != "true" ]; then \
		echo "{{ error(['Tag version must be equal or higher than', GREEN + version, 'not', RED + replace(tag, 'v', '')]) }}"; \
		exit 1; \
	fi

	# Local branch is `main`.
	if [ "$(git branch --show-current)" != "{{ main_branch }}" ]; then \
		echo "{{ error(['Not on the', GREEN + main_branch, 'branch:', RED + shell('git branch --show-current')]) }}"; \
		exit 1; \
	fi

	# Local branch is remote branch.
	git fetch --quiet origin {{ main_branch }}
	if [ "$(git rev-parse {{ main_branch }})" \
		!= "$(git rev-parse origin/{{ main_branch }})" ]; then \
		echo "{{ error(['The local', BOLD + main_branch, 'branch differs from', BOLD + 'origin/' + main_branch]) }}"; \
		exit 1; \
	fi

	# Local tag does not exist.
	if [ "$(git rev-parse --verify --quiet "refs/tags/{{ tag }}")" ]; then \
		echo "{{ error(['The tag', BOLD + tag, 'already exists in this repo']) }}"; \
		exit 1; \
	fi

	# Remote tag does not exist.
	if [ "$(git ls-remote --tags origin '{{ tag }}')" ]; then \
		echo "{{ error(['The tag', BOLD + tag, 'already exists in GitHub']) }}"; \
		exit 1; \
	fi

	# Working tree is clean (no uncommitted files).
	if [ -n "$(git status --porcelain)" ]; then \
		if [ "{{ allow_dirty }}" = "false" ]; then \
			echo "{{ error('The working tree is dirty') + style('dim') + YELLOW }}"; \
			git status --porcelain; \
			exit 1; \
		else \
			echo "{{ warning(['The working tree is dirty -', BOLD + 'bypassed by', BOLD + ITALIC + '--allow-dirty']) + style('dim') + YELLOW }}"; \
			git status --porcelain; \
		fi; \
	fi

	echo "{{ NORMAL + GREEN }}Ready to tag for release: {{ BOLD + repo }} {{ tag + NORMAL }}"

# Push an annotated release tag `v#.#.#`
[arg('allow_dirty', long='allow-dirty', value='--allow-dirty')]
[group('Cargo')]
[no-exit-message]
push_tag tag=tag commit='' allow_dirty='':
	just check_tag {{ tag }} {{ allow_dirty }}

	echo {{ CYAN }}Push the tag {{ GREEN + tag + NORMAL }}

	git tag --sign {{ tag }} {{ commit }} --message "Release {{ tag }}"
	git push origin refs/tags/{{ tag }}

	echo {{ done('') }}

# Delete a `tag`
[confirm(YELLOW + '⚠️  Are you sure to delete the tag ' + RED + tag + YELLOW + ' here and in GitHub? ' + CYAN)]
[group('Cargo')]
[no-exit-message]
delete_tag tag:
	echo {{ CYAN }}Delete the tag {{ RED + tag + CYAN }} in this repo{{ NORMAL }}

	-git tag -d '{{ tag }}'

	echo {{ CYAN }}Delete the tag {{ RED + tag + CYAN }} in GitHub{{ NORMAL }}
	-git push origin --delete '{{ tag }}'

	echo {{ done('') }}

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
		echo "{{ error(['Crate is not published on', BOLD + 'crates.io']) }}"; \
		exit 1;} \
	&& if [ "$(printf '%s' "$crate_json" | jq -r '.crate.trustpub_only')" = "true" ]; then \
		echo "{{ info(['Trusted publishing is', GREEN + 'enabled']) }}"; \
	else \
		echo "{{ error('Trusted publishing is not enabled') }}"; \
		exit 1; \
	fi

	if [ "{{ crates_io_token }}" = "" ]; then \
		echo "{{ error(['Not logged in', BOLD + 'crates.io', 'registry']) }}"; \
		echo "{{ info(['Run', MAGENTA + 'cargo login']) }}"; \
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
deps crate:
	cargo tree --target all --all-features --workspace --invert {{ crate }}

# Show crates with multiple versions
[group('Cargo')]
[no-exit-message]
deps_dupe:
	echo {{ CYAN }}Show duplicate crates with multiple versions{{ NORMAL }}
	cargo tree --target all --all-features --workspace --duplicates --quiet
