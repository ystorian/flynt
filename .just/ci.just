# .just/ci.just

# Run the CI tests like in `.github/workflows/ci-rust.yaml`
[group('CI')]
ci:
	echo {{ title(['Run all', BOLD + 'CI', 'checks']) }}

	# Preparation.
	just lint_just
	just lock_cargo
	just format
	just fetch

	# Check.
	just check_typos
	just check_rust
	just check_deny
	just check_actions --fast
	just audit_actions --fast

	# Test.
	just test
	just clippy --strict

	echo {{ done('') }}
