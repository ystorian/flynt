# .just/cargo.just

# Variables.
# Globaly required Rust tools.
cargo_tools := ['cargo-clean-all', 'cargo-deny', 'cargo-update', 'just', 'tomli', 'zizmor']
# Tools required by this project.
project_tools := []

# cargo-binstall.
need_binstall := if which("cargo-binstall") == [] { 'true' } else { 'false' }
binstall_options := ['--locked', '--no-confirm', '--only-signed']
native_target := if os() == 'windows' { arch() + '-pc-windows-msvc' } else if os() == 'macos' { arch() + '-apple-darwin' } else if os() == 'linux' { arch() + '-unknown-linux-gnu' } else { arch() }

# Install cargo tools
[env("GITHUB_TOKEN", github_token)]
[group('Cargo')]
install_cargo_tools:
	echo {{ CYAN }}Install cargo tools{{ NORMAL }}

	# Install cargo-binstall first.
	if {{ need_binstall }} ; then \
		echo {{ BLUE }}Install {{ GREEN }}cargo-binstall{{ NORMAL }} && \
		cargo install --locked --target {{ native_target }} cargo-binstall \
	;fi

	echo {{ CYAN }}Install the required cargo tools{{ NORMAL }}
	cargo-binstall \
		{{ join_list(binstall_options) }} \
		--targets {{ native_target }} \
		{{ join_list(cargo_tools) }} \
		{{ join_list(project_tools) }}

	just update_cargo_tools

	echo {{ done('') }}

# Update all
[group('Cargo')]
update_all:
	echo {{ BLUE }}Update all{{ NORMAL }}
	just update_rust
	just update_cargo_tools
	just bump_cargo
	just deny
	-just update_actions
	echo {{ done('') }}

# Update Rust
[group('Cargo')]
update_rust:
	echo {{ CYAN }}Update Rust{{ NORMAL }}
	rustup self update
	rustup update stable
	rustup component add rust-analyzer
	echo {{ done('') }}

# Update Cargo tools
[group('Cargo')]
update_cargo_tools:
	echo {{ CYAN }}Update cargo tools{{ NORMAL }}
	cargo install-update --all --git --locked --cooldown 7d
	echo {{ done('') }}

# Update Cargo.lock without bumping dependencies
[group('Cargo')]
lock_cargo:
	echo {{ CYAN }}Update {{ GREEN }}Cargo.lock{{ NORMAL }}
	cargo update --offline --workspace
	echo {{ done('') }}

# Bump all Cargo.lock dependencies
[group('Cargo')]
bump_cargo:
	echo {{ CYAN }}Bump all {{ YELLOW }}Cargo.lock{{ CYAN }} dependencies{{ NORMAL }}
	cargo update
	echo {{ done('') }}

# Update all dependencies and actions
[group('Cargo')]
bump_all_deps:
	echo {{ CYAN }}Update dependencies in {{ GREEN }}Cargo.toml{{ NORMAL }}
	just bump_cargo
	just deny
	-just update_actions
	echo {{ done('') }}

# Verify the crate is publishable with a dry run
[group('Cargo')]
[no-exit-message]
publish_check:
	echo {{ CYAN }}Check if this crate is publishable{{ NORMAL }}

	just check_tag {{ tag }}

	echo "{{ dim('Cargo.toml -> [package] -> publish') }}"
	if [ "$(tomli query --filepath Cargo.toml workspace.package.publish 2>/dev/null || tomli query --filepath Cargo.toml package.publish 2>/dev/null)" = "" ] ; then \
		echo "{{ error(['The', BOLD + 'publish', 'key is not set in', BOLD + 'Cargo.toml']) }}" && \
		exit 1 \
	;fi

	echo "{{ dim('publish = true') }}"
	if [ -n "$(tomli query --filepath Cargo.toml workspace.package.publish 2>/dev/null || tomli query --filepath Cargo.toml package.publish 2>/dev/null)" ] ; then \
		echo "{{ error(['The', BOLD + 'publish', 'key is not set to', BOLD + 'true']) }}" && \
		exit 1 \
	;fi

	echo {{ CYAN }}Run {{ YELLOW }}cargo publish{{ CYAN }} in {{ GREEN }}dry-run{{ NORMAL }}
	cargo publish --dry-run --all-features --workspace --locked --allow-dirty

	echo {{ done('') }}

# Publish the crate to crates.io
[group('Cargo')]
publish: publish_check
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
		echo "Cargo.lock is not in sync with Cargo.toml, run: just lock_cargo" >&2 && exit 1 \
	;fi

	# Check if Rust code is formatted correctly.
	if ! cargo fmt --all --check --quiet >/dev/null 2>&1; then \
		echo "Rust code is incorrectly formatted, run: just format" >&2 && exit 1 \
	;fi

# Check if this release can be tagged `v#.#.#`
[group('Cargo')]
[no-exit-message]
check_tag tag=tag:
	echo {{ CYAN }}Check if this release can be tagged {{ GREEN + tag + NORMAL }}

	# Tag name starts with `v`.
	if [ "{{ replace_regex(tag, '^v\d.*', '') }}" != "" ]; then \
		echo "{{ error(['Tag name must start with', GREEN + 'v', 'not', RED + replace_regex(tag, '\d+\.\d+\.\d+.*', '')]) }}" && \
		exit 1 \
	;fi

	# Tag has a semver format `v#.#.#`.
	if [ "{{ replace_regex(tag, 'v\d+\.\d+\.\d+', '') }}" != "" ]; then \
		echo "{{ error(['Tag version is not a', BOLD + 'semver', 'version like', GREEN + 'x.y.z']) }}" && \
		exit 1 \
	;fi

	# Tag version is equal or higher than version in `Cargo.toml`.
	if [ "{{ semver_matches(replace(tag, 'v', ''), '>=' + version) }}" != "true" ]; then \
		echo "{{ error(['Tag version must be equal or higher than', GREEN + version, 'not', RED + replace(tag, 'v', '')]) }}" && \
		exit 1 \
	;fi

	# Local branch is `main`.
	echo "{{ dim('- local branch name') }}"
	if [ "$(git branch --show-current)" != "{{ main_branch }}" ]; then \
		echo "{{ error(['Not on the', GREEN + main_branch, 'branch:', RED + shell('git branch --show-current')]) }}" && \
		exit 1 \
	;fi

	# Local branch is remote branch.
	echo "{{ dim('- remote branch name') }}"
	git fetch --quiet origin {{ main_branch }}
	if [ "$(git rev-parse {{ main_branch }})" != "$(git rev-parse origin/{{ main_branch }})" ]; then \
		echo "{{ error(['The local', BOLD + main_branch, 'branch differs from', BOLD + 'origin/' + main_branch]) }}" && \
		exit 1 \
	;fi

	# Local tag does not exist.
	if [ "$(git rev-parse --verify --quiet "refs/tags/{{ tag }}")" ]; then \
		echo "{{ error(['The tag', BOLD + tag, 'already exists in this repo']) }}" && \
		exit 1 \
	;fi

	# Remote tag does not exist.
	if [ "$(git ls-remote --tags origin '{{ tag }}')" ]; then \
		echo "{{ error(['The tag', BOLD + tag, 'already exists in GitHub']) }}" && \
		exit 1 \
	;fi

	# Working tree is clean (no uncommitted files).
	if [ -n "$(git status --porcelain)" ]; then \
		echo "{{ error('The working tree is dirty') + style('dim') + YELLOW }}" && \
		git status --porcelain && \
		exit 1 \
	;fi

# Push an annotated release tag `v#.#.#`
[group('Cargo')]
[no-exit-message]
push_tag tag=tag commit='':
	just check_tag {{ tag }}

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
	| map(. + {parts: (.rust_version | split(".") | map(tonumber))}) \
	| max_by(.parts) as $max \
	| map(select(.parts == $max.parts)) \
	| .[] \
	| "\(.name) v\(.version) msrv:\(.rust_version)" \
	' \
	| column -t
	echo {{ NORMAL }}
