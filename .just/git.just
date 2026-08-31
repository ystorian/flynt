# .just/git.just

# Variables.
git_hooks_dir := '.githooks'
pre_commit_file := git_hooks_dir + '/pre-commit'
pre_commit_script := f'''
	#!/usr/bin/env sh
	#
	# {{ pre_commit_file }}
	#
	# Install git pre-commit hooks in your local repo with `just install_git_hooks`.

	# .just/git.just
	just precommit_just
'''

# Install the git hooks in `.githooks`
[group('Git')]
[no-exit-message]
install_git_hooks:
	echo {{ CYAN }}Install git hooks in the {{ GREEN + git_hooks_dir + CYAN }} directory{{ NORMAL }}
	if [ ! -d "{{ git_hooks_dir }}" ]; then \
		echo "{{ error(['Directory not found:', BOLD + git_hooks_dir]) }}" && \
		echo "{{ dim(['Create an initial pre-commit script with', NORMAL + MAGENTA + 'just create_precommit_script']) }}" && \
		exit 1 \
	;fi

	if [ ! -f "{{ pre_commit_file }}" ]; then \
		echo "{{ error('Script not found: ' + BOLD + pre_commit_file) }}" && \
		echo "{{ dim(['Create an initial pre-commit script with', NORMAL + MAGENTA + 'just create_precommit_script']) }}" && \
		exit 1 \
	;fi

	git config set core.hooksPath {{ git_hooks_dir }}

	echo "{{ dim(GREEN + 'Git hooks path set to ' + BOLD + GREEN + git_hooks_dir) }}"
	echo {{ done('') }}

# Check if git hooks are installed
[group('Git')]
[no-exit-message]
check_git_hooks:
	echo {{ CYAN }}Check {{ GREEN }}git hooks{{ NORMAL }}
	if [ ! -d "{{ git_hooks_dir }}" ]; then \
		echo "{{ warning(['Directory not found:', RED + BOLD + git_hooks_dir]) }}" \
	;else \
		echo "{{ dim(['Directory exists:', GREEN + git_hooks_dir]) }}" \
	;fi

	if [ ! -f "{{ pre_commit_file }}" ]; then \
		echo "{{ warning(['Pre-commit script not found:', RED + BOLD + pre_commit_file]) }}" \
	;else \
		echo "{{ dim(['Pre-commit script:', GREEN + pre_commit_file]) }}" \
	;fi

	hooks_path="$(git config get core.hooksPath || true)"; \
	if [ -z "$hooks_path" ]; then \
		echo "{{ warning(['Local git hook config not set']) }}" \
	;elif [ "$hooks_path" != "{{ git_hooks_dir }}" ]; then \
		echo "{{ warning(['Local git hook path not set to', RED + BOLD + git_hooks_dir]) }}" && \
		echo "{{ dim(['Configured git hook path:', YELLOW + BOLD + '$hooks_path']) }}" \
	;else \
		echo "{{ dim(['Local git hook config set to', GREEN + git_hooks_dir]) }}" \
	;fi

	echo {{ CYAN + ITALIC }}Create a pre-commit script with: \
		{{ MAGENTA }}just create_precommit_script{{ NORMAL }}
	echo {{ CYAN + ITALIC }}Enable the pre-commit hook with: \
		{{ MAGENTA }}just install_git_hooks{{ NORMAL }}

# Create an initial git pre-commit hook script.
[group('Git')]
[no-exit-message]
create_precommit_script:
	echo {{ CYAN }}Create an initial script {{ YELLOW + pre_commit_file + NORMAL }}
	just create_dir {{ git_hooks_dir }}
	if [ ! -f "{{ pre_commit_file }}" ]; then \
		just create_file {{ pre_commit_file }} && \
		just write_file {{ pre_commit_file }} '{{ pre_commit_script }}' \
	;else \
		echo "{{ dim(YELLOW + 'Pre-commit script already exist') }}" \
	;fi
	chmod +x {{ pre_commit_file }} && \
		echo "{{ dim(['Set the', BOLD + 'execute', 'bit on', BOLD + pre_commit_file]) }}"

	echo {{ GREEN }}Pre-commit script ready{{ NORMAL }}
	echo "{{ warning(['Enable git hooks with', NORMAL + MAGENTA + 'just install_git_hooks']) }}"

# Git pre-commit hook for Just
[group('Git')]
[no-exit-message]
[private]
precommit_just:
	if ! just --format --check --indentation '{{ "\t" }}' >/dev/null 2>&1; then \
		echo "justfile is not linted, run: just lint_just" >&2 && exit 1 \
	;fi
	for justfile in `ls .just`; do \
		if ! just --format --check --indentation '{{ "\t" }}' \
			--justfile .just/$justfile >/dev/null 2>&1; then \
			echo ".just/$justfile is not linted, run: just lint_just" >&2 && exit 1 \
		;fi \
	;done
