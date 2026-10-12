# .just/_main.just

# Common recipes.
import 'git.just'
import 'github.just'
import 'utils.just'

# Language recipes.
import 'cargo.just'
import 'rust.just'

# Project recipes.
import 'ci.just'

# Project variables.

# Additional Cargo tools required by this project (see `.just/cargo.just`).
additional_cargo_tools := ['zizmor']

# Setup this project (tooling, hooks)
setup:
	just no_index

	# Rust.
	just update_rust
	just install_cargo_tools
	just clean_workspace

# Bump all dependencies
bump:
	just bump_cargo
	just update_actions
