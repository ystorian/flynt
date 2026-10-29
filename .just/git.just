# @template/.just/git.just

# Variables.
githooks_dir := '.githooks'
precommit_file := githooks_dir + '/pre-commit'
precommit_script := f'''
	#!/usr/bin/env sh
	#
	# {{ precommit_file }}
	#
	# Enable this pre-commit script with `just enable_githooks`.

	# .just/git.just
	just precommit_just
'''

# Enable the git hooks in `.githooks`
[group('Git')]
[no-exit-message]
[script]
enable_githooks:
	echo {{ CYAN }}Enable the git hooks in {{ GREEN + githooks_dir + NORMAL }}

	not_found='';

	if [ ! -d "{{ githooks_dir }}" ]; then
		echo {{ error(['Directory not found:', BOLD + githooks_dir]) }};
		not_found='1';
	fi

	if [ ! -f "{{ precommit_file }}" ]; then
		echo {{ error('Script not found: ' + BOLD + precommit_file) }};
		not_found='1';
	fi

	if [ -n "$not_found" ]; then
		echo {{ info(['Create a pre-commit script with', MAGENTA + 'just create_precommit']) }};
		exit 1;
	fi

	if [ "$(git config get core.hooksPath)" != "{{ githooks_dir }}" ]; then
		git config set core.hooksPath {{ githooks_dir }};
		echo {{ info(['Git hooks path set to', GREEN + BOLD + githooks_dir]) }};
	fi

	echo {{ done('') }}

# Check if the git pre-commit hook script is installed
[group('Git')]
[no-exit-message]
[script]
check_precommit:
	echo {{ CYAN }}Check {{ GREEN }}pre-commit hook{{ NORMAL }}

	installed='';
	enabled='';

	if [ ! -d "{{ githooks_dir }}" ]; then
		echo {{ warning(['Directory not found:', RED + BOLD + githooks_dir]) }};
	elif [ ! -f "{{ precommit_file }}" ]; then
		echo {{ warning(['Pre-commit script not found:', RED + BOLD + precommit_file]) }};
	elif [ ! -x "{{ precommit_file }}" ]; then
		echo {{ warning(['Pre-commit script not executable:', BOLD + precommit_file]) }};
	else
		echo {{ dim(['Pre-commit script:', GREEN + precommit_file]) }};
		installed='1';
	fi

	hooks_path="$(git config get core.hooksPath || true)";
	if [ -z "$hooks_path" ]; then
		echo {{ warning(['Local git hooks config not set']) }};
	elif [ "$hooks_path" != "{{ githooks_dir }}" ]; then
		echo {{ warning(['Local git hook path not set to', BOLD + githooks_dir]) }};
		echo {{ info(['Configured git hooks path:', BOLD + '$hooks_path']) }};
	else
		echo {{ dim(['Git hooks:', GREEN + 'enabled']) }};
		enabled='1';
	fi

	if [ ! -n "$installed" ]; then
		echo {{ info(['Create a pre-commit script with:', MAGENTA + 'just create_precommit']) }};
		exit 1;
	fi

	if [ ! -n "$enabled" ]; then
		echo {{ info(['Enable git hooks with:', MAGENTA + 'just enable_githooks']) }};
		exit 1;
	fi

	echo {{ done('') }}

# Create a git hook pre-commit script.
[arg('enable', long='enable', flag, help='Enable the git hooks')]
[group('Git')]
[no-exit-message]
[script]
create_precommit enable:
	echo {{ CYAN }}Create a git hook pre-commit script {{ YELLOW + precommit_file + NORMAL }}

	just create_dir {{ githooks_dir }}

	if [ ! -f "{{ precommit_file }}" ]; then
		just create_file {{ precommit_file }};
		just write_file {{ precommit_file }} '{{ precommit_script }}';
	else
		echo {{ dim(YELLOW + 'Pre-commit script already exist') }};
	fi

	chmod +x {{ precommit_file }} &&
		echo {{ dim(['Set the', BOLD + 'execute', 'bit on', BOLD + precommit_file]) }}

	{{ if enable { 'just enable_githooks && echo ' + done('') } }}
	{{ if !enable { 'echo ' + info(['Enable with', MAGENTA + 'just enable_githooks']) } }}

# Git hook pre-commit script for Just
[group('Git')]
[no-exit-message]
[private]
[script]
precommit_just:
	if ! just --fmt --quiet --check --indentation '{{ "\t" }}'; then
		echo "justfile is not linted, run: just lint_just" >&2;
		exit 1;
	fi
	for justfile in `ls .just`; do
		if ! just --fmt --quiet --check --indentation '{{ "\t" }}' -f .just/$justfile; then
			echo ".just/$justfile is not linted, run: just lint_just" >&2;
			exit 1;
		fi;
	done

# Check if this release can be tagged `v#.#.#`
[arg('allow_dirty', long='allow-dirty', flag, help='Allow dirty state with uncommitted changes')]
[group('Git')]
[no-exit-message]
[script]
check_tag tag allow_dirty:
	echo {{ CYAN }}Check if this release can be tagged {{ GREEN + tag + NORMAL }}

	# Tag name starts with `v`.
	if [ "{{ replace_regex(tag, '^v\d.*', '') }}" != "" ]; then
		echo {{ error(['Tag name must start with', GREEN + BOLD + 'v']) }};
		exit 1;
	fi

	# Tag has a semver format `v#.#.#`.
	if [ "{{ replace_regex(tag, 'v\d+\.\d+\.\d+', '') }}" != "" ]; then
		echo {{ error(['Tag version is not a', BOLD + 'semver', 'version like', GREEN + 'x.y.z']) }};
		exit 1;
	fi

	# Tag version is equal or higher than the version in `Cargo.toml`.
	if [ -f 'Cargo.toml' ]; then
		cargo_version="$(tomli query --filepath Cargo.toml workspace.package.version 2>/dev/null || tomli query --filepath Cargo.toml package.version)";
		cargo_version="$(echo "$cargo_version" | tr -d '"' | xargs)";
		lowest="$(printf '%s\n%s\n' "$cargo_version" "{{ replace(tag, 'v', '') }}" | sort -V | head -n 1)";
		if [ "$lowest" != "$cargo_version" ]; then
			echo {{ error(['Tag version', RED + tag, 'must be equal or higher than', GREEN + "$cargo_version"]) }};
			exit 1;
		fi;
	fi

	# Local branch is `main`.
	if [ "$(git branch --show-current)" != "main" ]; then
		echo {{ error(['Not on the', GREEN + 'main', 'branch:', RED + shell('git branch --show-current')]) }};
		exit 1;
	fi

	# Local branch is remote branch.
	git fetch --quiet origin main
	if [ "$(git rev-parse main)" != "$(git rev-parse origin/main)" ]; then
		echo {{ error(['The local', BOLD + 'main', 'branch differs from', BOLD + 'origin/main']) }};
		exit 1;
	fi

	# Local tag does not exist.
	if [ "$(git rev-parse --verify --quiet "refs/tags/{{ tag }}")" ]; then
		echo {{ error(['The tag', BOLD + tag, 'already exists in this repo']) }};
		exit 1;
	fi

	# Remote tag does not exist.
	if [ "$(git ls-remote --tags origin '{{ tag }}')" ]; then
		echo {{ error(['The tag', BOLD + tag, 'already exists in GitHub']) }};
		exit 1;
	fi

	# Tag version is higher than the latest remote tag.
	latest_tag="$(git ls-remote --tags --refs origin 'v*' | sed 's|.*refs/tags/||' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -n 1)";
	if [ -n "$latest_tag" ]; then
		highest="$(printf '%s\n%s\n' "$latest_tag" '{{ tag }}' | sort -V | tail -n 1)";
		if [ "$highest" != '{{ tag }}' ]; then
			echo {{ error(['Tag version', RED + tag, 'must be higher than', GREEN + "$latest_tag"]) }};
			exit 1;
		fi;
	fi

	# Working tree is clean (no uncommitted files).
	if [ -n "$(git status --porcelain)" ]; then
		if [ '{{ allow_dirty }}' = '' ]; then
			echo {{ error('The working tree is dirty') + style('dim') + YELLOW }};
			git status --porcelain;
			exit 1;
		else
			echo {{ warning(['The working tree is dirty,', BOLD + 'bypassed by', MAGENTA + '--allow-dirty']) + style('dim') + YELLOW }};
			git status --porcelain;
			echo {{ info([YELLOW + 'Ready to tag for release:', repo, GREEN + BOLD + tag, ITALIC + '(tree is dirty)']) }}
			exit 0;
		fi;
	fi

	echo {{ info([GREEN + 'Ready to tag for release:', repo, GREEN + BOLD + tag]) }}

# Push an annotated release tag `v#.#.#`
[arg('allow_dirty', long='allow-dirty', flag, help='Allow dirty state with uncommitted changes')]
[group('Git')]
[no-exit-message]
push_tag allow_dirty tag commit='':
	just check_tag {{ tag }} {{ if allow_dirty { '--allow-dirty' } }}

	echo {{ CYAN }}Push the tag {{ GREEN + tag + NORMAL }}

	git tag --sign {{ tag }} {{ commit }} --message "Release {{ tag }}"
	git push origin refs/tags/{{ tag }}

	echo {{ done('') }}

# Delete a `tag`
[confirm(YELLOW + '⚠️  Are you sure to delete the tag ' + RED + tag + YELLOW + ' here and in GitHub? ' + CYAN)]
[group('Git')]
[no-exit-message]
delete_tag tag:
	echo {{ CYAN }}Delete the tag {{ RED + tag + CYAN }} in this repo{{ NORMAL }}

	-git tag -d '{{ tag }}'

	echo {{ CYAN }}Delete the tag {{ RED + tag + CYAN }} in GitHub{{ NORMAL }}
	-git push origin --delete '{{ tag }}'

	echo {{ done('') }}
