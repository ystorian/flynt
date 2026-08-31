# justfile

# Set the minimum Just version.
set minimum-version := '1.58.0'

# Set the default shell on Windows to `bash` (installed with Git).
set windows-shell := ['C:\Program Files\Git\bin\bash.exe', '-cu']

# Don't print comments.
set ignore-comments

# Allow variable redefinition.
set allow-duplicate-variables

# Be quiet.
set quiet

# Enable unstable features.
set unstable

# Values may be lists of strings instead of strings.
set lists

# Skip evaluating unused variables.
set lazy

# Import common recipes
import? '.just/ci.just'
import? '.just/git.just'
import? '.just/github.just'
import? '.just/utils.just'

# List the commands when called without parameters.
_:
	# List recipes.
	just --list

## Project specifics.

# Project variables.

# Project recipes.
import '.just/flynt.just'

# Project language recipes.
import '.just/cargo.just'
import '.just/rust.just'
