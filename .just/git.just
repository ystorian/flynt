# @template/.just/git.just

# Variables.

# Repository owner (organization) and name.
# Read `owner/repo` from the git remote URL.
owner_repo := replace_regex(shell('git remote get-url origin 2>/dev/null || true'), '^.*github\.com[:/]|\.git$', '')
# This is to avoid a dependency on `gh` and consumming GraphQL API quota.

# Return the repository owner (organization).
owner := replace_regex(owner_repo, '/.*$', '')

# Return the repository name.
repo := replace_regex(owner_repo, '^.*/', '')

# pr_dev_files.
# Paths of the dev files to refresh.
dev_paths := ['.just', '.github', '.githooks', '.vscode', '.clippy.toml', '.dockerignore', '.editorconfig', '.gitattributes', '.gitignore', '.markdownlint.json', '.rustfmt.toml', '.typos.toml', 'deny.toml', 'droast.toml', 'justfile']

# Branch for the dev files PR.
dev_branch := 'refresh-dev-files'

# List of check run conclusions failing in CI.
run_failures := '["failure", "cancelled", "timed_out", "action_required", "startup_failure"]'

# Pre-commit hook.
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

## Recipes.

# Enable the git hooks in `.githooks`
[group('Git')]
[no-exit-message]
[script]
enable_githooks:
	echo {{ title(['Enable the git hooks in', BOLD + githooks_dir]) }}

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
	echo {{ title(['Check the', BOLD + 'pre-commit hook', 'installation']) }}

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
	echo {{ title(['Create a git hook pre-commit script', BOLD + precommit_file]) }}

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

# Commit the changed dev files to a new branch and create a PR
[group('Git')]
[no-exit-message]
[script]
pr_dev_files branch=dev_branch: check_command_gh
	echo {{ title(['Refresh the dev files in a', BOLD + 'pull request']) }}

	just lint_just;
	just check_typos;
	just check_actions --fast;
	just audit_actions --fast;

	# Local branch is `main`.
	current="$(git branch --show-current)";
	if [ "$current" != 'main' ]; then
		echo {{ error(['Not on the', GREEN + 'main', 'branch:', RED + '$current']) }};
		exit 1;
	fi

	# Local branch is remote branch.
	git fetch --quiet origin main
	if [ "$(git rev-parse main)" != "$(git rev-parse origin/main)" ]; then
		echo {{ error(['The local', BOLD + 'main', 'branch differs from', BOLD + 'origin/main']) }};
		exit 1;
	fi

	# Branch does not exist.
	if git show-ref --verify --quiet 'refs/heads/{{ branch }}' ||
			[ -n "$(git ls-remote --heads origin '{{ branch }}')" ]; then
		echo {{ error(['The branch already exists:', BOLD + branch]) }};
		exit 1;
	fi

	# Modified, deleted, and new dev files.
	files="$(git ls-files --modified --deleted --others --exclude-standard -- {{ dev_paths }} | sort -u)";
	if [ -z "$files" ]; then
		echo {{ warning('No changed dev files') }};
		exit 0;
	fi

	echo {{ info('Modified dev files:') }};
	echo "$files" | while IFS= read -r file; do
		echo {{ dim(GREEN + '$file') }};
	done

	git switch --quiet --create '{{ branch }}'
	echo "$files" | xargs git add --all --
	echo "$files" | xargs git commit --quiet --signoff --message 'Refresh dev files' --
	# Hide the GitHub "Create a pull request" hint.
	if ! push="$(git push --quiet --set-upstream origin '{{ branch }}' 2>&1)"; then
		echo "$push" >&2;
		exit 1;
	fi

	body="$(printf 'Refresh the dev files:\n\n%s\n' "$(echo "$files" | sed 's/^/- `/; s/$/`/')")";

	# Create the PR with the REST API.
	number="$(gh api --method POST 'repos/{{ owner }}/{{ repo }}/pulls' -f base=main -f head='{{ branch }}' \
		-f title='Refresh dev files' -f body="$body" --jq '.number')";
	echo {{ info(['Created PR', BOLD + '#$number']) }};

	# Assign the PR to the current user.
	pr="$(gh api --method POST "repos/{{ owner }}/{{ repo }}/issues/$number/assignees" \
		-f "assignees[]=$(gh api user --jq '.login')" --jq '.html_url')";
	echo {{ info('$pr') }};

	just merge_pr {{ branch }}

# Wait for CI, then squash and merge the PR for `branch`
[group('Git')]
[no-exit-message]
[script]
merge_pr branch=dev_branch: check_command_gh
	echo {{ title(['Merge the', BOLD + branch, 'pull request']) }}

	# Local branch is the dev branch.
	current="$(git branch --show-current)";
	if [ "$current" != "{{ branch }}" ]; then
		echo {{ error(['Not on the', GREEN + branch, 'branch:', RED + '$current']) }};
		exit 1;
	fi

	# Local branch is pushed.
	git fetch --quiet origin '{{ branch }}'
	sha="$(git rev-parse HEAD)";
	if [ "$sha" != "$(git rev-parse 'origin/{{ branch }}')" ]; then
		echo {{ error(['The local branch differs from', BOLD + 'origin/' + branch]) }};
		exit 1;
	fi

	# Wait for CI, refresh every 5 seconds.
	echo {{ info('Waiting for CI checks') }};
	tries=0;
	while true; do
		if ! status="$(gh api "repos/{{ owner }}/{{ repo }}/commits/$sha/check-runs" --jq '
				.check_runs | [length, (map(select(.status != "completed")) | length),
				(map(select(.conclusion as $c | {{ run_failures }} | index($c))) | length)]
				| map(tostring) | join(" ")' 2>&1)"; then
			printf '\n';
			echo {{ error('GitHub API error') }};
			echo "$status";
			echo {{ info(['Try again with', MAGENTA + 'just merge_pr ' + branch]) }};
			exit 1;
		fi;
		set -- $status;
		total="$1"; pending="$2"; failed="$3";

		if [ "$total" = '0' ]; then
			tries=$((tries + 1));
			if [ "$tries" -ge 24 ]; then
				echo {{ warning('No CI checks found after 2 minutes') }};
				exit 1;
			fi;
		elif [ "$pending" = '0' ]; then
			break;
		fi;

		printf '\r\033[2K%s' {{ note([GREEN + 'Checks:', BOLD + '$total', YELLOW + 'Pending:', '$pending', RED + 'Failed:', '$failed', '']) }};
		for i in {1..5}; do
			printf '⏵';
			sleep 1;
		done;
	done
	printf '\r\033[2K';

	gh api "repos/{{ owner }}/{{ repo }}/commits/$sha/check-runs" \
		--jq '.check_runs[] | "   \(.conclusion)\t\(.name)"'

	if [ "$failed" != '0' ]; then
		echo {{ error(['CI checks failed:', BOLD + '$failed']) }};
		exit 1;
	fi
	echo {{ info(GREEN + 'CI checks passed') }}

	# Ask before the merge.
	printf '%s' "{{ YELLOW + 'Squash and merge the PR? ' + CYAN + '[y/n] ' + NORMAL }}";
	read -r answer;
	if [ "$answer" != 'y' ]; then
		echo {{ warning(['PR not merged, still on', BOLD + branch]) }};
		exit 0;
	fi

	# Squash and merge the open PR.
	number="$(gh api 'repos/{{ owner }}/{{ repo }}/pulls?head={{ owner }}:{{ branch }}&state=open' --jq '.[0].number')";
	if [ -z "$number" ] || [ "$number" = 'null' ]; then
		echo {{ error(['No open PR for', BOLD + branch]) }};
		exit 1;
	fi
	# The REST API does not add the sign-off.
	signoff="Signed-off-by: $(git config user.name) <$(git config user.email)>";
	gh api --method PUT "repos/{{ owner }}/{{ repo }}/pulls/$number/merge" \
		-f merge_method=squash -f sha="$sha" \
		-f commit_title="Refresh dev files (#$number)" -f commit_message="$signoff" \
		--jq '.message'

	# Switch to main, then delete the branch.
	git switch --quiet main
	git pull --quiet --ff-only
	git branch --quiet -D '{{ branch }}'
	git push --quiet origin --delete '{{ branch }}' 2>/dev/null || true

	echo {{ info([GREEN + 'Merged', GREEN + BOLD + '#$number', 'https://github.com/' + owner + '/' + repo + '/pulls/$number']) }}
	echo {{ done(branch) }}

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

# Git hook pre-commit script for Typos
[group('Git')]
[no-exit-message]
[private]
[script]
precommit_typos:
	typos

# Check if this release can be tagged `v#.#.#`
[arg('allow_dirty', long='allow-dirty', flag, help='Allow dirty state with uncommitted changes')]
[group('Git')]
[no-exit-message]
[script]
check_tag tag allow_dirty:
	echo {{ title(['Check if this release can be tagged', BOLD + tag]) }}

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

	echo {{ title(['Push the tag', BOLD + tag]) }}

	git tag --sign {{ tag }} {{ commit }} --message "Release {{ tag }}"
	git push origin refs/tags/{{ tag }}

	echo {{ done('') }}

# Delete a `tag`
[confirm(YELLOW + '⚠️  Are you sure to delete the tag ' + RED + tag + YELLOW + ' here and in GitHub? ' + CYAN + '[y/n]')]
[group('Git')]
[no-exit-message]
[script]
delete_tag tag:
	echo {{ title(['Delete the tag', BG_RED + BOLD + ' ' + tag]) }}

	if [ "$(git tag --list '{{ tag }}')" ]; then
		git tag -d '{{ tag }}' 2>/dev/null;
		echo {{ info(['Deleted', tag, BOLD + 'from this repo']) }};
	else
		echo {{ warning(['Local tag not found:', BOLD + tag]) }};
	fi

	if [ "$(git ls-remote --tags origin '{{ tag }}')" ]; then
		git push origin --delete '{{ tag }}';
		echo {{ info(['Deleted', tag, BOLD + 'from GitHub']) }};
	else
		echo {{ warning(['Remote tag not found:', BOLD + tag]) }};
	fi

	echo {{ done('') }}
