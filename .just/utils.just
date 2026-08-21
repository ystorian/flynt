# .just/utils.just

### Variables.

# The default `main` branch for all repos.
main_branch := 'main'

# Architecture, using normalized output from `arch()`.
# - x86_64  -> x64
# - aarch64 -> arm64
arch_short := if arch() == "x86_64" { "x64" } else if arch() == "aarch64" { "arm64" } else { arch() }

# Random temp file.
temp_file := env('TMPDIR') + '/just.' + file_stem(source_file()) + '.' + uuid() + '.tmp'
# macOS: /var/folders/by/<random>/T//just.<just file>.<uuid>.tmp

### Functions

## Display functions

# Display after a successful recipe.
# ✅ 2026-07-15 12:07:16.571 <recipe name> [<recipe arguments>]
done(args) := f'{{ GREEN }}✅ {{ style('dim') + datetime('%F %T%.3f ') + NORMAL + GREEN + BOLD + recipe_name() + NORMAL + GREEN + ITALIC }} {{ join_list(args) + NORMAL + "\n" }}'

# Display an error message.
# Note: add `[no-exit-message]` on recipes for cleaner output.
# ❌ <recipe name> Error: [<message>]
error(message) := f'{{ RED }}❌ {{ style('dim') + recipe_name() + NORMAL + RED + BOLD + ' Error: ' + NORMAL + YELLOW + join_list(message, ' ' + NORMAL + YELLOW) + NORMAL + "\n" }}'

# Display an warning message.
# ⚠️ Warning: [<message>]
warning(message) := f'{{ YELLOW }}⚠️ {{ style('dim') + BOLD + ' Warning: ' + NORMAL + YELLOW + join_list(message, ' ' + NORMAL + YELLOW) + NORMAL }}'

# Display a dimmed and indented message.
# __[<message>]
dim(message) := f'{{ style('dim') + CYAN + '  ' + join_list(message, ' ' + NORMAL + style('dim')) + NORMAL }}'

# Set the terminal title
#
# Usage:
# - VS Code: Add `${sequence}` to `terminal.integrated.tabs.description`
# - Must be used within quotes, for example:
# - To set the title to "Running": `echo "{{ set_title('Running') }}"`
# - To reset the title: `echo "{{ set_title('') }}"`
set_title(title) := f'\033]0;{{ title }}\07'

## Files

# Get a file size.
file_size(file) := trim(shell('wc -c < "$1"', file))

## Git

# Get the short commit hash.
commit() := `git rev-parse --short HEAD`

# Get the branch name.
branch() := `git branch --show-current`

### Recipes

## Files

# Create an empty `file`
[group('Utils')]
[private]
create_file file:
	if [ ! -f "{{ file }}" ]; then \
		touch "{{ file }}" && \
		echo "{{ dim('Created ' + GREEN + BOLD + file) }}" || \
		echo "{{ error(['Could not create', BOLD + file]) }}" \
	;fi

# Write `content` in an existing `file`
[group('Utils')]
[private]
write_file file content:
	if [ -f "{{ file }}" ]; then \
		printf '%s\n' {{ quote(content) }} > {{ file }} \
	;fi
	if [ -f "{{ file }}" ]; then \
		echo "{{ dim(['Wrote', GREEN + BOLD + file_size(file) + ' bytes', 'in', GREEN + BOLD + file]) }}" \
	;else \
		echo "{{ dim(['Could not write into', BOLD + file]) }}" \
	;fi

# Rename a `file`
[group('Utils')]
[no-exit-message]
[private]
rename_file old_file new_file:
	if [ ! -f "{{ old_file }}" ]; then \
		echo "{{ error(['Cannot rename', BOLD + old_file, ITALIC + '(file not found)']) }}" && \
		exit 1 \
	;fi
	mv "{{ old_file }}" "{{ new_file }}" && \
		echo "{{ dim(['Renamed', YELLOW + old_file, 'to', GREEN + BOLD + new_file]) }}" || \
		echo "{{ error(['Could not rename', BOLD + old_file, 'to', BOLD + new_file]) }}"

# Move a `file` to a `directory`
[group('Utils')]
[no-exit-message]
[private]
move_file file directory:
	if [ ! -f "{{ file }}" ]; then \
		echo "{{ error(['Cannot move', BOLD + file, ITALIC + '(file not found)']) }}" && \
		exit 1 \
	;fi
	just create_dir {{ directory }}
	mv "{{ file }}" "{{ directory + '/' + file_name(file) }}" && \
		echo "{{ dim(['Moved', YELLOW + file, 'to', GREEN + BOLD + directory + '/' + file_name(file)]) }}" || \
		echo "{{ error(['Could not move', BOLD + file, 'to', BOLD + directory + '/' + file_name(file)]) }}"

# Copy a `file` to a `directory`
[group('Utils')]
[no-exit-message]
[private]
copy_file file directory:
	if [ ! -f "{{ file }}" ]; then \
		echo "{{ error(['Cannot copy', BOLD + file, ITALIC + '(file not found)']) }}" && \
		exit 1 \
	;fi
	just create_dir {{ directory }}
	cp "{{ file }}" "{{ directory }}" && \
		echo "{{ dim(['Copied', YELLOW + file, 'to', GREEN + BOLD + directory]) }}" || \
		echo "{{ error(['Could not copy', BOLD + file, 'to', BOLD + directory]) }}"

# Create a hardlink `link` to a `file`
[group('Utils')]
[no-exit-message]
[private]
link_file file link:
	if [ ! -f "{{ file }}" ]; then \
		echo "{{ error(['Cannot link', BOLD + file, ITALIC + '(file not found)']) }}" && \
		exit 1 \
	;fi
	ln -f {{ file }} {{ link }} && \
		echo "{{ dim(['Created hardlink on', YELLOW + file, 'to', GREEN + BOLD + link]) }}" || \
		echo "{{ error(['Could not create hardlink', BOLD + link, 'to', BOLD + file]) }}"

# Delete a `file`
[group('Utils')]
[private]
delete_file file:
	if [ -f "{{ file }}" ]; then \
		rm -f "{{ file }}" 2>/dev/null && \
			echo "{{ dim('Deleted ' + YELLOW + BOLD + file) }}" || \
			echo "{{ error(['Could not delete', BOLD + file]) }}" \
	;fi

## Directories

# Create a `directory`
[group('Utils')]
[private]
create_dir directory:
	if [ ! -d "{{ directory }}" ]; then \
		mkdir "{{ directory }}" && \
			echo "{{ dim('Created directory ' + GREEN + BOLD + directory) }}" || \
			echo "{{ error(['Could not create directory', BOLD + directory]) }}" \
	;fi

# Delete a `directory`
[group('Utils')]
[private]
delete_dir directory:
	if [ -d "{{ directory }}" ]; then \
		rm -rf "{{ directory }}" 2>/dev/null && \
			echo "{{ dim('Deleted directory ' + YELLOW + BOLD + directory) }}" || \
			echo "{{ error(['Could not delete directory', BOLD + directory]) }}" \
	;fi

## Variables

# Test if an environment `variable` is set
[group('Utils')]
[no-exit-message]
[private]
test_var variable:
	if [ -z "$(printenv {{ variable }})" ]; then \
		echo "{{ error(['The environment variable', BOLD + variable, 'is not declared', ITALIC + '(variable names are case-sensitive)']) }}" && \
		exit 1 \
	;fi

## Tools

# Check if a command is available
[group('Utils')]
[no-exit-message]
[private]
check_command command macos_text='' windows_text='':
	if ! command -v {{ command }} >/dev/null 2>&1; then \
		echo "{{ error(['The command', BOLD + command, 'is not available']) }}" && \
		echo "{{ if os() == 'macos' { dim(['Install', YELLOW + BOLD + command, 'with', GREEN + BOLD + macos_text]) } }}" && \
		echo "{{ if os() == 'windows' { dim(['Install', YELLOW + BOLD + command, 'with', GREEN + BOLD + windows_text]) } }}" && \
		exit 1 \
	;fi

# Lints

# Lint all Just recipes
[group('Utils')]
[no-exit-message]
lint_just:
	echo {{ CYAN }}Lint {{ BOLD }}all justfiles{{ NORMAL + YELLOW + style('dim') }}

	just --format --indentation '{{ "\t" }}' --justfile justfile

	for justfile in `ls .just`; do \
		just --format  --indentation '{{ "\t" }}' --justfile .just/$justfile \
	;done
	echo {{ done('') }}

# Check for lint issues in all Just recipes
[group('Utils')]
[no-exit-message]
check_just:
	if ! just --format --check --indentation '{{ "\t" }}' >/dev/null 2>&1; then \
		echo "{{ error([BOLD + 'justfile', 'is not linted, run', MAGENTA + 'just lint_just']) }}" \
	;fi

	for justfile in `ls .just`; do \
		if ! just --format --check --indentation '{{ "\t" }}' \
			--justfile .just/$justfile >/dev/null 2>&1; then \
			echo "{{ error([BOLD + '.just/$justfile', 'is not linted, run', MAGENTA + 'just lint_just']) }}" \
		;fi \
	;done
	echo {{ done('') }}

# Check for lint issues in all Markdown files
[group('Utils')]
check_markdown:
	echo {{ CYAN }}Check all {{ BOLD }}markdown{{ NORMAL + CYAN }} files{{ NORMAL }}
	bunx --bun markdownlint-cli2  "**/*.md"
	echo {{ done('') }}

## Performances

# Disable Spotlight indexing and Time Machine backups on generated files
[group('Utils')]
[macos]
no_index:
	echo {{ CYAN }}Disable {{ YELLOW }}Spotlight{{ CYAN }} indexing and \
	{{ YELLOW }}Time Machine{{ CYAN }} backups on generated files{{ NORMAL }}

	# Dotted directories like ".git" are ignored by SpotLight, but we also exclude them from Time
	# Machine backups.

	echo {{ dim('Git') }}
	just no_index_dir .git

	echo {{ dim('Rust') }}
	# Global Cargo `git`, assuming it contains `CACHEDIR.TAG`.
	just no_index_dir "{{ env('CARGO_HOME') + '/git' }}"

	# Global Cargo `registry`, assuming it contains `CACHEDIR.TAG`.
	just no_index_dir "{{ env('CARGO_HOME') + '/registry' }}"

	# Global Rustup directory.
	just no_index_dir "{{ env('RUSTUP_HOME') }}"

	just no_index_dir dist
	just no_index_dir target
	just no_index_dir data

	echo {{ dim('Javascript') }}
	just no_index_dir node_modules

	echo {{ dim('PHP') }}
	just no_index_dir vendor

# Disable Spotlight indexing and Time Machine backups on a directory
[group('Utils')]
[macos]
[no-exit-message]
[private]
no_index_dir dir:
	# Notes:
	# - `.metadata_never_index` works only on Volumes, delete them.
	# - `com.apple.metadata:kMDItemSupportFileType` is undocumented.
	if [ ! -d "{{ absolute_path(dir) }}" ]; then \
		echo "{{ dim([YELLOW + 'Skipping non-existing directory', YELLOW + BOLD + dir]) }}" \
	;fi

	just delete_file "{{ dir + '/.metadata_never_index' }}"

	[ -d "{{ absolute_path(dir) }}" ] || exit 0; \
	case "$(xattr \
		-p com.apple.metadata:kMDItemSupportFileType \
		"{{ absolute_path(dir) }}" 2>/dev/null)" in \
		*MDSystemFile*) \
			echo "{{ dim(['Already excluded', BOLD + dir, 'from Spotlight indexing']) }}" \
		;; \
		*) \
			xattr \
			-w com.apple.metadata:kMDItemSupportFileType \
			MDSystemFile \
			"{{ absolute_path(dir) }}" && \
			echo "{{ dim(['Excluded', GREEN + BOLD + dir, 'from Spotlight indexing']) }}" \
		;; \
	esac

	[ -d "{{ absolute_path(dir) }}" ] || exit 0; \
	case "$(tmutil \
		isexcluded \
		"{{ absolute_path(dir) }}")" in \
		*'[Excluded]'*) \
			echo "{{ dim(['Already excluded', BOLD + dir, 'from Time Machine']) }}" \
		;; \
		*) \
			tmutil \
			addexclusion \
			"{{ absolute_path(dir) }}" && \
			echo "{{ dim(['Excluded', GREEN + BOLD + dir, 'from Time Machine']) }}" \
		;; \
	esac
