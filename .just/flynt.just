# .just/flynt.just

# Scratch copy of the fixtures used by `demo`.
demo_dir := 'target/demo'

# Lint some test fixtures with failing examples
[group('Flynt')]
flynt_demo:
	echo {{ CYAN }}Run {{ BOLD }}cargo test{{ NORMAL + CYAN }} on {{ YELLOW }}demo fixtures{{ NORMAL }}
	just create_dir "{{ demo_dir }}"
	cp -R tests/fixtures/clean "{{ demo_dir }}/clean"
	cp -R tests/fixtures/missing_key "{{ demo_dir }}/missing_key"
	find "{{ demo_dir }}" -name _Cargo.toml -execdir mv _Cargo.toml Cargo.toml \;
	find "{{ demo_dir }}" -name _flynt.toml -execdir mv _flynt.toml .flynt.toml \;
	find "{{ demo_dir }}" -name _gitkeep -delete

	echo {{ CYAN + BOLD }}Demo: {{ NORMAL + GREEN }}Clean tree{{ NORMAL }}
	cargo run --quiet -- {{ demo_dir }}/clean
	echo ""

	echo {{ CYAN + BOLD }}Demo: {{ NORMAL + YELLOW }}Tree with findings{{ NORMAL }}
	-cargo run --quiet -- {{ demo_dir }}/missing_key
	echo ""

	just delete_dir "{{ demo_dir }}"
	echo {{ done('') }}
