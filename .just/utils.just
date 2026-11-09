# @template/.just/utils.just

### Variables.

# Architecture, using normalized output from `arch()`.
# - x86_64  -> x64
# - aarch64 -> arm64
arch_short := if arch() == "x86_64" { "x64" } else if arch() == "aarch64" { "arm64" } else { arch() }

# Rust and Cargo native targets per OS.
[linux]
native_target := arch() + '-unknown-linux-gnu'
[macos]
native_target := arch() + '-apple-darwin'
[windows]
native_target := arch() + '-pc-windows-msvc'

# Random temp file (per-recipe).
temp_file := env('TMPDIR') + '/just.' + file_stem(source_file()) + '.' + uuid() + '.tmp'
# macOS: /var/folders/by/<random>/T//just.<just file>.<uuid>.tmp

# Today's date (yyyy-mm-dd).
today := datetime('%Y-%m-%d')

### Functions

## Display functions

# Display the recipe heading.
# 🟦 [<message>]
title(message) := '"' + NORMAL + style(['cyan', 'reverse'], '🟦 ' + join_list(message, NORMAL + style(['cyan', 'reverse']) + ' ') + ' ') + '"'

# Display after a successful recipe.
# ✅ 2026-07-15 12:07:16.571 <recipe name> [<recipe arguments>]
done(args) := '"' + NORMAL + '✅ ' + style(['dim', 'green'], datetime('%F %T%.3f ')) + style('green', recipe_name()) + ' ' + style(['dim', 'green'], join_list(args)) + '"'

# Display an error message.
# Note: add `[no-exit-message]` on recipes for cleaner output.
# 🟥 <recipe name> Error: [<message>]
error(message) := '"' + NORMAL + '🟥 ' + style(['red', 'dim'], recipe_name()) + style(['red', 'bold'], ' Error: ') + style('yellow', join_list(message, NORMAL + YELLOW + ' ')) + '"'

# Display a warning message.
# ⚠️ Warning: [<message>]
warning(message) := '"' + NORMAL + '⚠️  ' + style(['yellow', 'dim', 'bold'], 'Warning: ') + style('yellow', join_list(message, NORMAL + YELLOW + ' ')) + '"'

# Display an info message.
# 🔷 [<message>]
info(message) := '"' + NORMAL + '🔷 ' + style('blue', join_list(message, NORMAL + BLUE + ' ')) + '"'

# Display an indented and dimmed message.
# __[<message>]
dim(message) := '"' + style(['cyan', 'dim'], '   ' + join_list(message, NORMAL + style(['cyan', 'dim']) + ' ')) + '"'

# Display in script.
echo(text) := 'echo ' + text

# Set the terminal title
#
# Prerequisite:
# - VS Code: Add `${sequence}` to `terminal.integrated.tabs.title`
#
# Usage:
# - To set the title to "Running": `{{ set_title('Running') }}`
# - To reset the title: `echo "{{ reset_title('') }}"`
# - or: `{{ set_title('') }}`
set_title(title) := "printf '" + '\033]0;%s\007' + "' " + '"' + title + '"'
reset_title := "\u{1b}]0;\u{7}"

## Strings.

# Base64 decode.
base64_decode(encoded) := `base64 -d <<< {{ encoded }}`

## Files

# Get a file size.
file_size(file) := if path_exists(file) { trim(shell('wc -c < "$1"', file)) } else { '? ' + file }

# Get a file date (yyyy-mm-dd).
file_date(file) := if path_exists(file) { shell('date -r "$1" +%F', file) } else { '? (' + file + ')' }

# Shell test: file was changed today.
# Returns shell code, to accept shell variables like '$file'.
is_today(file) := '[ "$(date -r "' + file + '" +%F)" = "' + today + '" ]'

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
		&& echo {{ dim('Created ' + GREEN + BOLD + file) }} \
		|| echo {{ error(['Could not create', BOLD + file]) }}; \
	fi

# Write `content` in an existing `file`
[group('Utils')]
[private]
[script]
write_file file content:
	if [ -f "{{ file }}" ]; then
		printf '%s\n' {{ quote(content) }} > {{ file }};
	fi

	if [ -f "{{ file }}" ]; then
		echo {{ dim(['Wrote', GREEN + "$(wc -c < " + file + " | tr -d ' ')" + ' bytes', 'in', GREEN + BOLD + file]) }};
	else
		echo {{ error(['Could not write into', BOLD + file]) }};
	fi

# Rename a `file`
[group('Utils')]
[no-exit-message]
[private]
[script]
rename_file old_file new_file:
	if [ ! -f "{{ old_file }}" ]; then
		echo {{ error(['Cannot rename', BOLD + old_file, ITALIC + '(file not found)']) }};
		exit 1;
	fi
	mv "{{ old_file }}" "{{ new_file }}" &&
		echo {{ dim(['Renamed', YELLOW + old_file, 'to', GREEN + BOLD + new_file]) }} ||
		echo {{ error(['Could not rename', BOLD + old_file, 'to', BOLD + new_file]) }}

# Move a `file` to a `directory`
[group('Utils')]
[no-exit-message]
[private]
[script]
move_file file directory:
	if [ ! -f "{{ file }}" ]; then
		echo {{ error(['Cannot move', BOLD + file, ITALIC + '(file not found)']) }};
		exit 1;
	fi

	just create_dir {{ directory }}

	mv "{{ file }}" "{{ directory + '/' + file_name(file) }}" &&
		echo {{ dim(['Moved', YELLOW + file, 'to', GREEN + directory + '/' + file_name(file)]) }} ||
		echo {{ error(['Could not move', BOLD + file, 'to', BOLD + directory]) }}

# Copy a `file` to a `directory`
[group('Utils')]
[no-exit-message]
[private]
[script]
copy_file file directory:
	if [ ! -f "{{ file }}" ]; then
		echo {{ error(['Cannot copy', BOLD + file, ITALIC + '(file not found)']) }};
		exit 1;
	fi

	just create_dir {{ directory }}

	cp "{{ file }}" "{{ directory }}" &&
		echo {{ dim(['Copied', YELLOW + file, 'to', GREEN + BOLD + directory]) }} ||
		echo {{ error(['Could not copy', BOLD + file, 'to', BOLD + directory]) }}

# Create a hardlink on `file` to `link`
[group('Utils')]
[no-exit-message]
[private]
[script]
file_link file link:
	if [ ! -f '{{ file }}' ]; then
		echo {{ error(['File not found:', BOLD + file]) }};
		exit 1;
	fi

	ln -f '{{ file }}' '{{ link }}' &&
		echo {{ dim(['Created hardlink on', BOLD + file, 'to', GREEN + BOLD + link]) }} ||
		echo {{ error(['Could not create hardlink', BOLD + link, 'to', BOLD + file]) }}

# Delete one or more `file`
[arg('silent', long='silent', flag, help='No output on success')]
[group('Utils')]
[private]
[script]
delete_file silent *file:
	for file in {{ quote(file) }}; do
		if [ -f "$file" ]; then
			rm -f "$file" 2>/dev/null \
				{{ if !silent { '&& echo ' + dim('Deleted ' + YELLOW + BOLD + "$file") } }} ||
				echo {{ error(['Could not delete', BOLD + "$file"]) }};
		fi;
	done

## Directories

# Create one or more directories
[group('Utils')]
[private]
[script]
create_dir *dir:
	for dir in {{ quote(dir) }}; do
		if [ ! -d "$dir" ]; then
			mkdir "$dir" &&
				echo {{ dim('Created directory ' + GREEN + BOLD + "$dir") }} ||
				echo {{ error(['Could not create directory', BOLD + "$dir"]) }};
		fi;
	done

# Delete one or more directories
[group('Utils')]
[private]
[script]
delete_dir *dir:
	for dir in {{ quote(dir) }}; do
		if [ -d "$dir" ]; then
			rm -rf "$dir" 2>/dev/null &&
				echo {{ dim('Deleted directory ' + YELLOW + BOLD + "$dir") }} ||
				echo {{ error(['Could not delete directory', BOLD + "$dir"]) }};
		fi;
	done

## Variables

# Test if one or more environment variables are declared
[group('Utils')]
[no-exit-message]
[private]
[script]
test_var *variable:
	for variable in {{ quote(variable) }}; do
		if [ -z "$(printenv $variable)" ]; then
			echo {{ error(['The environment variable', BOLD + "$variable", 'is not declared']) }};
			echo {{ warning('Variable names are case-sensitive') }};
			exit 1;
		fi;
	done

## Tools

# Check if a command is available
[arg('brew', long='brew', short='b', help='Homebrew formula')]
[arg('macos', long='macos', short='m', help='Installation command on macOS')]
[arg('linux', long='linux', short='l', help='Installation command on Linux')]
[arg('cargo', long='cargo', short='c', help='Installation command for Cargo tools')]
[arg('winget', long='winget', short='i', help='WinGet installation ID')]
[arg('windows', long='windows', short='w', help='Installation command on Windows')]
[group('Utils')]
[no-exit-message]
[private]
[script]
check_command command brew=[] cargo=[] macos=[] linux=[] windows=[] winget=[]:
	if ! command -v {{ command }} >/dev/null 2>&1; then
		echo {{ error(['The command', BOLD + command, 'is not available']) }};
		{{ if brew { echo(info(['To install it with HomeBrew, run:', MAGENTA + 'just brew' + brew])) } }}
		{{ if macos && os() == 'macos' { echo(info(['To install it on macOS, run:', MAGENTA + macos])) } }}
		{{ if linux && os() == 'linux' { echo(info(['To install it on Linux, run:', MAGENTA + linux])) } }}
		{{ if windows && os() == 'windows' { echo(info(['To install it on Windows, run:', MAGENTA + windows])) } }}
		{{ if winget { echo(info(['To install it on Windows, run:', MAGENTA + 'just winget ' + winget])) } }}
		{{ if cargo { echo(info(['To install it, run:', MAGENTA + 'just cargo_install ' + cargo])) } }}
		exit 1;
	fi

# Check if `bash` is available
[group('Utils')]
[no-exit-message]
[private]
check_command_bash:
	just check_command bash --winget='Git.Git'

# Check if `bun` is available
[group('Utils')]
[no-exit-message]
[private]
check_command_bun:
	just check_command bun --brew='oven-sh/bun/bun' --winget='Oven-sh.Bun'

# Lints

# Lint all Just recipes
[group('Utils')]
[no-exit-message]
[script]
lint_just:
	echo {{ title(['Lint', BOLD + 'all justfiles']) + style('dim') }}

	just --format --indentation '{{ "\t" }}' --justfile justfile

	for justfile in `ls .just`; do
		just --format  --indentation '{{ "\t" }}' --justfile .just/$justfile;
	done

	echo {{ done('') }}

# Check for lint issues in all Just recipes
[group('Utils')]
[no-exit-message]
[script]
check_just:
	echo {{ dim(['just', BOLD + just_version()]) }}

	error=''

	if ! just --format --check --indentation '{{ "\t" }}' >/dev/null 2>&1; then
		echo {{ error([BOLD + 'justfile', 'is not linted']) }};
		error='1';
	fi

	for justfile in `ls .just`; do
		if ! just --format --check --indentation '{{ "\t" }}' \
				--justfile .just/$justfile >/dev/null 2>&1; then
			echo {{ error([BOLD + ".just/$justfile", 'is not linted']) }};
			error='1';
		fi;
	done

	if [ -n "$error" ]; then
		echo {{ info(['Run', MAGENTA + 'just lint_just']) }};
		exit 1;
	else
		echo {{ done('') }};
	fi

# Check for lint issues in all Markdown files
[group('Utils')]
[no-exit-message]
check_markdown: check_command_bun
	echo {{ title(['Check all', BOLD + 'markdown', 'files']) }}

	bun x --bun markdownlint-cli2 "**/*.md" "#**/.terraform" "#**/data"

	echo {{ done('') }}

## Performances

# Disable content indexing on generated files, aka `Turbo Boost`
[group('Utils')]
[macos]
[no-exit-message]
[windows]
no_index dir='.':
	echo {{ title('Disable content indexing on generated files') }}

	# macOS.
	{{ if os() == 'macos' { echo(dim(['Disable', YELLOW + BOLD + 'Time Machine backups'])) } }}
	{{ if os() == 'macos' { echo(dim(['Disable', YELLOW + BOLD + 'Spotlight indexing'])) } }}

	# Windows.
	{{ if os() == 'windows' { echo(dim(['Disable', YELLOW + BOLD + 'Microsoft Defender'])) } }}
	{{ if os() == 'windows' { echo(dim([YELLOW + '  (requires administrator privileges)'])) } }}
	{{ if os() == 'macos' { echo(dim(['Disable', YELLOW + BOLD + 'content indexing'])) } }}

	if [ '{{ dir }}' != '.' ]; then \
		echo {{ info(['Directory:', BOLD + absolute_path(dir)]) }}; \
	fi

	if [ ! -d "{{ absolute_path(dir) }}" ]; then \
		echo {{ error(['Directory', BOLD + dir, 'not found']) }}; \
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
		echo {{ info(['Excluded', BOLD + dir, 'from Spotlight indexing']) }}; \
	fi

	# Disable Time Machine - enable on `.git`.
	if [[ "{{ absolute_path(dir) }}" =~ '/.git' ]]; then \
		if [[ "$(tmutil isexcluded "{{ absolute_path(dir) }}")" =~ '[Excluded]' ]]; then \
			tmutil removeexclusion "{{ absolute_path(dir) }}"; \
			echo {{ warning(['Included', BOLD + dir, 'in Time Machine backups']) }}; \
		fi; \
	else \
		if [[ "$(tmutil isexcluded "{{ absolute_path(dir) }}")" =~ '[Included]' ]]; then \
			tmutil addexclusion "{{ absolute_path(dir) }}"; \
			echo {{ info(['Excluded', BOLD + dir, 'from Time Machine backups']) }}; \
		fi; \
	fi

	echo {{ dim(GREEN + absolute_path(dir)) }}

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
			echo {{ error(['Administrator privileges required to disable Microsoft Defender on', BOLD + dir]) }}; \
			exit 1; \
		fi; \
		powershell \
			-NoProfile \
			-Command "& {Add-MpPreference -ExclusionPath '{{ absolute_path(dir) }}'" \
		&& echo {{ info(['Excluded', BOLD + dir, 'from Microsoft Defender']) }} \
		|| echo {{ error(['Could not exclude', BOLD + dir, 'from Microsoft Defender']) }}; \
	fi

# Alias `j` to `just`
[group('Utils')]
[private]
[unix]
alias_just:
	ln -s "{{ which('just') }}" "{{ parent_directory(which('just')) / 'j' }}"

# Alias `j` to `just`
[group('Utils')]
[private]
[windows]
alias_just:
	ln -s "{{ which('just') }}" "{{ parent_directory(which('just')) / 'j.exe' }}"

# Install with WinGet
[group('Utils')]
[private]
[windows]
winget id:
	winget install --exact --id {{ id }}

# Install with Homebrew
[group('Utils')]
[macos]
[private]
brew formula:
	brew install {{ formula }}

# Add relative path headers
[group('Utils')]
add_headers:
	echo {{ title('Add relative path headers') }}

	echo {{ info(['Rust:', BOLD + '*.rs']) }}
	just add_headers_ext --ext='rs' --prefix='//'

	echo {{ info(['TypeScript:', BOLD + '*.ts']) }}
	just add_headers_ext --ext='ts' --prefix='//'

	echo {{ info(['GitHub workflows:', BOLD + '*.yaml']) }}
	just add_headers_ext --ext='yaml' --prefix='#'

	echo {{ info(['Fluent translation resources:', BOLD + '*.ftl']) }}
	just add_headers_ext --ext='ftl' --prefix='###'

	echo {{ info(['CSS:', BOLD + '*.css']) }}
	just add_headers_ext --ext='css' --prefix='/*' --suffix='*/'

	echo {{ done('') }}

# Add relative path headers
[arg('ext', long='ext', help='File extension')]
[arg('prefix', long='prefix', help='Comment prefix')]
[arg('suffix', long='suffix', help='Comment suffix')]
[group('Utils')]
[private]
[script]
add_headers_ext ext prefix suffix=[]:
	suffix='{{ if suffix { ' ' + suffix } else { '' } }}'
	files=$(find . \( -name target -o -name .git -o -name dist -o -name node_modules \) -prune -o -type f -name '*.{{ ext }}' -print | sed 's|^\./||' | sort)
	for file in $files; do
		[ -f "$file" ] || continue
		header=$(head -n 1 "$file")
		[ "$header" = "{{ prefix }} $file$suffix" ] && printf '\r\033[2K\033[2;32m   %s' "$file" && continue
		tmp=$(mktemp)
		case "$header" in
			"{{ prefix }} "*.{{ ext }}"$suffix") { echo "{{ prefix }} $file$suffix"; tail -n +2 "$file"; } > "$tmp" ;;
			*) { printf '{{ prefix }} %s%s\n\n' "$file" "$suffix"; cat "$file"; } > "$tmp" ;;
		esac
		cat "$tmp" > "$file" && rm "$tmp"
		printf '\r\033[2K\033[2;33m   %s\n' "$file"
	done
	printf '\r\033[2K'

# Check for typos
[group('Utils')]
[no-exit-message]
check_typos: check_command_typos
	echo {{ title('Check for typos') }}

	if [ ! -f ".typos.toml" ]; then \
		echo {{ error(['Configuration file not found:', BOLD + '.typos.toml']) }}; \
		exit 1; \
	fi

	typos

	echo {{ done('') }}

# Check if `typos` is available
[group('Utils')]
[private]
check_command_typos:
	just check_command typos -c='typos-cli'

# Install cargo tool
[env("GITHUB_TOKEN", github_token)]
[group('Utils')]
[no-exit-message]
cargo_install tool:
	echo {{ title(['Install cargo tool', tool, 'with', BOLD + 'cargo-binstall']) }}

	cargo-binstall --log-level warn --locked --no-confirm --disable-telemetry \
		--targets {{ native_target }} {{ tool }} \
	&& echo {{ done('') }} \
	|| echo {{ error(['Could not install', BOLD + tool]) }}
