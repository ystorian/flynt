# @template/justfile

# Tip:
# - Alias `j` to `just`: `just alias_just`

# These recipes use features that require a recent version of Just.
set minimum-version := '1.58.0'

# Set the default shell on Windows to `bash` (installed with Git).
[windows]
set shell := ['C:\Program Files\Git\bin\bash.exe', '-cu']

# Don't print comments.
set ignore-comments

# Be quiet.
set quiet

# Enable unstable features.
set unstable

# Values may be lists of strings instead of strings.
set lists

# Skip evaluating unused variables.
set lazy

# Use the `?` sigil to prefix a command.
# Prefixed commands with exit 1 will stop the current recipe, without stopping other recipes.
set guards

# Import common recipes.
import '.just/utils.just'

# Import recipes for this project.
import '.just/_main.just'

# List the recipes when called without parameters.
_:
	just --list
