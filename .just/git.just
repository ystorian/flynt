# .just/git.just

# Variables.
pre_commit_script := '''
	#!/usr/bin/env sh
	#
	# .githooks/pre-commit
	#
	# Install git pre-commit hooks in your local repo with `just install_git_hooks`.

	# .just/git.just
	just precommit_just
'''

# Install the git hooks from `.githooks`
[group('Git')]
[no-exit-message]
install_git_hooks:
	echo {{ CYAN }}Install git hooks from {{ GREEN }}.githooks{{ NORMAL }}
	if [ ! -d ".githooks" ]; then \
		echo "{{ error('Directory not found: ' + BOLD + '.githooks') }}" && \
		echo "{{ dim(['Create an initial pre-commit script with', NORMAL + MAGENTA + 'just create_precommit_script']) }}" && \
		exit 1 \
	;fi

	if [ ! -f ".githooks/pre-commit" ]; then \
		echo "{{ error('Script not found: ' + BOLD + '.githooks/pre-commit') }}" && \
		echo "{{ dim(['Create an initial pre-commit script with', NORMAL + MAGENTA + 'just create_precommit_script']) }}" && \
		exit 1 \
	;fi

	git config core.hooksPath .githooks
	echo "{{ dim(GREEN + 'Git hooks path set to ' + BOLD + GREEN + '.githooks') }}"
	echo {{ done('') }}

# Create an initial git pre-commit hook script.
[group('Git')]
[no-exit-message]
create_precommit_script:
	echo {{ CYAN }}Create an initial script {{ GREEN }}.githooks/pre-commit{{ NORMAL }}
	just create_dir .githooks
	if [ ! -f ".githooks/pre-commit" ]; then \
		just create_file .githooks/pre-commit && \
		just write_file .githooks/pre-commit '{{ pre_commit_script }}' \
	;else \
		echo "{{ dim(YELLOW + 'Pre-commit script already exist') }}" \
	;fi
	chmod +x .githooks/pre-commit && \
		echo "{{ dim(['Set the', BOLD + 'execute', 'bit on', BOLD + '.githooks/pre-commit']) }}"
	echo "{{ dim(['Install git hooks on your local repo with', NORMAL + MAGENTA + 'just install_git_hooks']) }}"

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
