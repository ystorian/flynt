# .just/ci.just

# Run the CI tests like in `.github/workflows/ci-rust.yaml`
[env("CARGO_INCREMENTAL", "0")]
[env("CARGO_PROFILE_TEST_DEBUG", "0")]
[env("CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_RUSTFLAGS", "")]
[env("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS", "")]
[group('CI')]
ci:
	echo {{ CYAN }}Run all {{ GREEN }}CI{{ CYAN }} checks{{ NORMAL }}

	# Preparation.
	just lint_just
	just lock_cargo
	just format
	just check_actions
	just audit_actions
	just fetch

	# CI tests.
	just deny
	just test ci
	just clippy ci

	echo {{ done('') }}
