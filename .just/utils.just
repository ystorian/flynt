# @template/.just/utils.just

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

# Display a warning message.
# ⚠️ Warning: [<message>]
warning(message) := f'{{ style(['yellow', 'dim', 'bold'], '⚠️  Warning: ') + style('yellow', join_list(message, ' ' + style('yellow'))) }}'

# Display an info message.
# 🅘 [<message>]
style_info := ['blue', 'dim']
info(message) := f"{{ style(style_info, '🅘  ' + join_list(message, NORMAL + ' ' + style(style_info))) }}"

# Display an indented and dimmed message.
# __[<message>]
dim(message) := f'{{ style('dim') + CYAN + '  ' + join_list(message, ' ' + NORMAL + style('dim')) + NORMAL }}'

# Print out.
echo(text) := 'echo "' + text + '"'

display:
	echo "{{ dim(['This is', 'dim']) }}"
	echo "{{ info(['This is', 'info']) }}"
	echo "{{ warning(['This is', 'warning']) }}"
	echo "{{ error(['This is', 'error']) }}"

# Set the terminal title
#
# Usage:
# - VS Code: Add `${sequence}` to `terminal.integrated.tabs.title`
#
# Must be used within quotes, for example:
# - To set the title to "Running": `echo "{{ set_title('Running') }}"`
# - To reset the title: `echo "{{ set_title('') }}"`
set_title(title) := "\u{1b}]0;" + title + "\u{7}"

## Strings.

# Base64 decode.
base64_decode(encoded) := `base64 -d <<< {{ encoded }}`

## Files

# Get a file size.
file_size(file) := if path_exists(file) { trim(shell('wc -c < "$1"', file)) } else { '? ' + file }

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
		touch "{{ file }}" \
		&& echo "{{ dim('Created ' + GREEN + BOLD + file) }}" \
		|| echo "{{ error(['Could not create', BOLD + file]) }}"; \
	fi

# Write `content` in an existing `file`
[group('Utils')]
[private]
write_file file content:
	if [ -f "{{ file }}" ]; then \
		printf '%s\n' {{ quote(content) }} > {{ file }}; \
	fi
	if [ -f "{{ file }}" ]; then \
		echo "{{ dim(['Wrote', GREEN + BOLD + file_size(file) + ' bytes', 'in', GREEN + BOLD + file]) }}"; \
	else \
		echo "{{ dim(['Could not write into', BOLD + file]) }}"; \
	fi

# Rename a `file`
[group('Utils')]
[no-exit-message]
[private]
rename_file old_file new_file:
	if [ ! -f "{{ old_file }}" ]; then \
		echo "{{ error(['Cannot rename', BOLD + old_file, ITALIC + '(file not found)']) }}"; \
		exit 1; \
	fi
	mv "{{ old_file }}" "{{ new_file }}" \
		&& echo "{{ dim(['Renamed', YELLOW + old_file, 'to', GREEN + BOLD + new_file]) }}" \
		|| echo "{{ error(['Could not rename', BOLD + old_file, 'to', BOLD + new_file]) }}"

# Move a `file` to a `directory`
[group('Utils')]
[no-exit-message]
[private]
move_file file directory:
	if [ ! -f "{{ file }}" ]; then \
		echo "{{ error(['Cannot move', BOLD + file, ITALIC + '(file not found)']) }}"; \
		exit 1; \
	fi
	just create_dir {{ directory }}
	mv "{{ file }}" "{{ directory + '/' + file_name(file) }}" \
		&& echo "{{ dim(['Moved', YELLOW + file, 'to', GREEN + BOLD + directory + '/' + file_name(file)]) }}" \
		|| echo "{{ error(['Could not move', BOLD + file, 'to', BOLD + directory + '/' + file_name(file)]) }}"

# Copy a `file` to a `directory`
[group('Utils')]
[no-exit-message]
[private]
copy_file file directory:
	if [ ! -f "{{ file }}" ]; then \
		echo "{{ error(['Cannot copy', BOLD + file, ITALIC + '(file not found)']) }}"; \
		exit 1; \
	fi
	just create_dir {{ directory }}
	cp "{{ file }}" "{{ directory }}" \
		&& echo "{{ dim(['Copied', YELLOW + file, 'to', GREEN + BOLD + directory]) }}" \
		|| echo "{{ error(['Could not copy', BOLD + file, 'to', BOLD + directory]) }}"

# Create a hardlink on `file` to `link`
[group('Utils')]
[no-exit-message]
[private]
file_link file link:
	if [ ! -f '{{ file }}' ]; then \
		echo "{{ error(['File not found:', BOLD + file]) }}"; \
		exit 1; \
	fi

	ln -fh '{{ file }}' '{{ link }}' \
		&& echo "{{ dim(['Created hardlink on', BOLD + file, 'to', GREEN + BOLD + link]) }}" \
		|| echo "{{ error(['Could not create hardlink', BOLD + link, 'to', BOLD + file]) }}"

# Delete one or more `file`
[group('Utils')]
[private]
delete_file *file:
	for file in {{ quote(file) }}; do \
		if [ -f "$file" ]; then \
			rm -f "$file" 2>/dev/null  \
			&& echo "{{ dim('Deleted ' + YELLOW + BOLD + "$file") }}" \
			|| echo "{{ error(['Could not delete', BOLD + "$file"]) }}"; \
		fi; \
	done

## Directories

# Create one or more directories
[group('Utils')]
[private]
create_dir *dir:
	for dir in {{ quote(dir) }}; do \
		if [ ! -d "$dir" ]; then \
			mkdir "$dir" \
			&& echo "{{ dim('Created directory ' + GREEN + BOLD + "$dir") }}" \
			|| echo "{{ error(['Could not create directory', BOLD + "$dir"]) }}";\
		fi; \
	done

# Delete one or more directories
[group('Utils')]
[private]
delete_dir *dir:
	for dir in {{ quote(dir) }}; do \
		if [ -d "$dir" ]; then \
			rm -rf "$dir" 2>/dev/null \
			&& echo "{{ dim('Deleted directory ' + YELLOW + BOLD + "$dir") }}" \
			|| echo "{{ error(['Could not delete directory', BOLD + "$dir"]) }}"; \
		fi; \
	done

## Variables

# Test if one or more environment variables are declared
[group('Utils')]
[no-exit-message]
[private]
test_var *variable:
	for variable in {{ quote(variable) }}; do \
		if [ -z "$(printenv $variable)" ]; then \
			echo "{{ error(['The environment variable', BOLD + "$variable", 'is not declared']) }}"; \
			echo "{{ warning('Variable names are case-sensitive') }}"; \
			exit 1; \
		fi; \
	done

## Tools

# Check if a command is available
[group('Utils')]
[no-exit-message]
[private]
check_command command macos_text='' windows_text='':
	if ! command -v {{ command }} >/dev/null 2>&1; then \
		echo "{{ error(['The command', BOLD + command, 'is not available']) }}"; \
		echo "{{ if os() == 'macos' { dim(['Install', YELLOW + BOLD + command, 'with', GREEN + BOLD + macos_text]) } }}"; \
		echo "{{ if os() == 'windows' { dim(['Install', YELLOW + BOLD + command, 'with', GREEN + BOLD + windows_text]) } }}"; \
		exit 1; \
	fi

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
	echo "{{ dim(['just', BOLD + just_version()]) }}"
	if ! just --format --check --indentation '{{ "\t" }}' >/dev/null 2>&1; then \
		echo "{{ error([BOLD + 'justfile', 'is not linted, run', MAGENTA + 'just lint_just']) }}"; \
	fi

	for justfile in `ls .just`; do \
		if ! just --format --check --indentation '{{ "\t" }}' \
			--justfile .just/$justfile >/dev/null 2>&1; then \
			echo "{{ error([BOLD + '.just/$justfile', 'is not linted, run', MAGENTA + 'just lint_just']) }}"; \
		fi; \
	done
	echo {{ done('') }}

# Check for lint issues in all Markdown files
[group('Utils')]
check_markdown:
	echo {{ CYAN }}Check all {{ BOLD }}markdown{{ NORMAL + CYAN }} files{{ NORMAL }}

	BUN_TMPDIR="$TMPDIR" \
	BUN_INSTALL_CACHE_DIR="$TMPDIR/bun-cache" \
	bunx --bun markdownlint-cli2 "**/*.md" "#**/.terraform"
	echo {{ done('') }}

## Performances

# Disable content indexing on generated files, aka `Turbo Boost`
[group('Utils')]
[macos]
[no-exit-message]
[windows]
no_index dir='.':
	echo {{ CYAN }}Disable content indexing on generated files{{ NORMAL }}

	# macOS.
	echo "{{ if os() == 'macos' { dim(['- Disable', YELLOW + BOLD + 'Time Machine backups']) } }}"
	echo "{{ if os() == 'macos' { dim(['- Disable', YELLOW + BOLD + 'Spotlight indexing']) } }}"

	# Windows.
	echo "{{ if os() == 'windows' { dim(['- Disable', YELLOW + BOLD + 'Microsoft Defender']) } }}"
	echo "{{ if os() == 'windows' { dim([YELLOW + '  (requires administrator privileges)']) } }}"
	echo "{{ if os() == 'macos' { dim(['- Disable', YELLOW + BOLD + 'content indexing']) } }}"

	if [ '{{ dir }}' != '.' ]; then \
		echo "{{ BLUE }}Directory: {{ BOLD + absolute_path(dir) + NORMAL }}"; \
	fi

	if [ ! -d "{{ absolute_path(dir) }}" ]; then \
		echo "{{ error(['Directory', BOLD + dir, 'not found']) }}"; \
		exit 1; \
	fi

	## Global.

	# Rust.

	# Global Cargo `git`, assuming it contains `CACHEDIR.TAG`.
	just no_index_dir "{{ env('CARGO_HOME') + '/git' }}"

	# Global Cargo `registry`, assuming it contains `CACHEDIR.TAG`.
	just no_index_dir "{{ env('CARGO_HOME') + '/registry' }}"

	# Global Rustup directory.
	just no_index_dir "{{ env('RUSTUP_HOME') }}"

	## Local.

	# Git.
	just no_index_dir "{{ absolute_path(dir + '/' + '.git') }}"

	# Rust.
	just no_index_dir "{{ absolute_path(dir + '/' + 'dist') }}"
	just no_index_dir "{{ absolute_path(dir + '/' + 'target') }}"
	just no_index_dir "{{ absolute_path(dir + '/' + 'data') }}"

	# Javascript.
	just no_index_dir "{{ absolute_path(dir + '/' + 'node_modules') }}"

	# PHP.
	just no_index_dir "{{ absolute_path(dir + '/' + 'vendor') }}"

	echo {{ done('') }}

# Disable Spotlight indexing and Time Machine backups on a directory
[group('Utils')]
[macos]
[no-exit-message]
[private]
no_index_dir dir:
	# Notes:
	# - `.metadata_never_index` works only on Volumes, delete them.
	# - `com.apple.metadata:kMDItemSupportFileType` is undocumented.

	?[ -d "{{ absolute_path(dir) }}" ]

	# Disable Spotlight indexing.
	just delete_file "{{ dir + '/.metadata_never_index' }}"

	if ! [[ "$(xattr -p com.apple.metadata:kMDItemSupportFileType \
			"{{ absolute_path(dir) }}" 2>/dev/null)" =~ 'MDSystemFile' ]]; then \
		xattr -w com.apple.metadata:kMDItemSupportFileType MDSystemFile \
			"{{ absolute_path(dir) }}"; \
		echo "{{ warning(['Excluded', BOLD + dir, 'from Spotlight indexing']) }}"; \
	fi

	# Disable Time Machine - enable on `.git`.
	if [[ "{{ absolute_path(dir) }}" =~ '/.git' ]]; then \
		if [[ "$(tmutil isexcluded "{{ absolute_path(dir) }}")" =~ '[Excluded]' ]]; then \
			tmutil removeexclusion "{{ absolute_path(dir) }}"; \
			echo "{{ warning(['Included', BOLD + dir, 'in Time Machine backups']) }}"; \
		fi; \
	else \
		if [[ "$(tmutil isexcluded "{{ absolute_path(dir) }}")" =~ '[Included]' ]]; then \
			tmutil addexclusion "{{ absolute_path(dir) }}"; \
			echo "{{ warning(['Excluded', BOLD + dir, 'from Time Machine backups']) }}"; \
		fi; \
	fi
	echo "{{ dim(GREEN + absolute_path(dir)) }}"

# Disable content indexing and Microsoft Defender on a directory
[group('Utils')]
[no-exit-message]
[private]
[windows]
no_index_dir dir:
	# Notes:
	# - Command flags on Windows require double slash since we are running in bash.
	# - Administrator privileges are required to disable Microsoft Defender on directories.

	?[ -d "{{ absolute_path(dir) }}" ]

	# Disable content indexing.
	attrib +i "{{ absolute_path(dir) }}" //s //d

	# Disable Microsoft Defender.
	if ! [ "$(powershell -NoProfile -Command "& {(Get-MpPreference).ExclusionPath -contains '{{ absolute_path(dir) }}'}")" == 'False' ]; then \
		if ! net session >/dev/null 2>&1; then \
			echo "{{ error(['Administrator privileges required to disable Microsoft Defender on', BOLD + dir]) }}"; \
			exit 1; \
		fi; \
		powershell \
			-NoProfile \
			-Command "& {Add-MpPreference -ExclusionPath '{{ absolute_path(dir) }}'" \
		&& echo "{{ warning(['Excluded', BOLD + dir, 'from Microsoft Defender']) }}" \
		|| echo "{{ error(['Could not exclude', BOLD + dir, 'from Microsoft Defender']) }}"; \
	fi

# Alias `j` to `just`
[private]
[unix]
alias_just:
	ln -s "{{ which('just') }}" "{{ parent_directory(which('just')) / 'j' }}"

# Alias `j` to `just`
[private]
[windows]
alias_just:
	ln -s "{{ which('just') }}" "{{ parent_directory(which('just')) / 'j.exe' }}"
