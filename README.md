# <img src="https://raw.githubusercontent.com/ystorian/flynt/main/flynt.svg" width="32" align="absbottom" alt="Flynt logo"> Flynt

**Linter for Fluent translation keys in Rust projects using Askama templates**

![Build Status](https://github.com/ystorian/flynt/actions/workflows/ci-rust.yaml/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/flynt.svg)](https://crates.io/crates/flynt)
[![docs.rs](https://docs.rs/flynt/badge.svg)](https://docs.rs/flynt)

Flynt collects the [Fluent](https://projectfluent.org) translation keys used in your Rust code and
[Askama](https://askama.readthedocs.io) templates. Then it compares them against your `.ftl` files
to report issues.

Flynt does not need to be configured by default. It reads the crates to scan from your `Cargo.toml`
and reads the locales from the subdirectories in the `locales` directory.

## Install

```shell
cargo install flynt
```

## Quick start

```shell
cd my-project
flynt
```

```text
✓ All translation keys validated

Summary:
  136 keys used
  136 keys defined
  2 locales checked: en, fr
```

Flynt takes the directory to lint as its only argument. It does not care where you run it from:

```shell
flynt /path/to/my-project
```

## Checks
Flynt reports every syntax problem in one pass.

### Coverage
A key used in code or in a template is defined in every locale.

### Consistency
Every locale defines the same set of keys.

### Duplicates
No locale defines the same key twice.

### Unused
Every defined key is used somewhere. Unused keys are a warning.

### Syntax
Every `.ftl` file parses.

## How it works
Flynt finds the relevant files in this order:

### 1. `Cargo.toml`
Read `Cargo.toml` to get the member crates, then scan the `src` directory of every entry in
`[workspace] members`, with globs such as `crates/*` expanded, and `[workspace] exclude` respected.

For project without `[workspace]`, it scans the crate's own `src`.

### 2. Templates
Scan the `templates` directory next to each of those crates, where one exists.

### 3. Locales
Scan every subdirectory of `locales`, each searched recursively for `.ftl` files.

The layout is `locales/<locale>/**/*.ftl`.

## Ignored files and text

### `default-members`
The crate's `[workspace] default-members` is ignored.

### Comments
Lines that begin with `//` are not scanned.

A crate can document its own helper in a doc comment without adding a phantom key.

Use `--include-comments` to scan comment lines anyway.


## What it looks for

By default, these four helpers:

```jinja
{{ "tpl-title" | t(&lang) }}
{{ "mail-count" | tn(&lang, "count", count) }}
```

```rust
loc("err-not-found", &lang)
loc_with_args("tpl-elapsed-weeks", &lang, &args)
```

Use `--filter` and `--function` if your project names them differently. Both accept the key as a
string literal in the position shown above. Both may be split across lines.

## Options

| Flag                          | `.flynt.toml` key     | Default                                  |
| ----------------------------- | --------------------- | ---------------------------------------- |
| `[PATH]`                      | _(command line only)_ | `.`                                      |
| `--config <FILE>`             | _(command line only)_ | `.flynt.toml`                            |
| `--no-config`                 | _(command line only)_ | `false`                                  |
| `--manifest-path <FILE>`      | `manifest-path`       | `Cargo.toml`                             |
| `--locales-dir <DIR>`         | `locales-dir`         | `locales`                                |
| `--locale <LOCALE>`           | `locales`             | all subdirectory in `locales`            |
| `--reference-locale <LOCALE>` | `reference-locale`    | `en`, else the first sorted              |
| `--src <DIR>`                 | `src`                 | `<member>/src` for each workspace member |
| `--add-src <DIR>`             | `add-src`             | none                                     |
| `--templates <DIR>`           | `templates`           | `<member>/templates` where present       |
| `--add-templates <DIR>`       | `add-templates`       | none                                     |
| `--template-ext <EXT>`        | `template-ext`        | `html`                                   |
| `--filter <NAME>`             | `filters`             | `t`, `tn`                                |
| `--function <NAME>`           | `functions`           | `loc`, `loc_with_args`                   |
| `--unused <LEVEL>`            | `unused`              | `warn` (`error`, `warn`, `allow`)        |
| `--ignore-unused <GLOB>`      | `ignore-unused`       | none                                     |
| `--attributes[=BOOL]`         | `attributes`          | `true`                                   |
| `--include-comments[=BOOL]`   | `include-comments`    | `false`                                  |
| `--require-locales[=BOOL]`    | `require-locales`     | `true`                                   |
| `--exclude <GLOB>`            | `exclude`             | `target/**`                              |
| `--follow-links[=BOOL]`       | `follow-links`        | `true`                                   |
| `--format <FORMAT>`           | `format`              | `text` (`text`, `json`)                  |
| `--color <WHEN>`              | `color`               | `auto` (`auto`, `always`, `never`)       |
| `-q, --quiet[=BOOL]`          | `quiet`               | `false`                                  |


> `--locale`, `--filter`, and `--function` read better in the singular when repeated on the command
> line. The configuration file accepts either spelling for each.

## Configuration file

Drop a `.flynt.toml` file at the root of the project. Its keys match the long flag names exactly.

You can write down anything you can pass on the command line:

```toml
# .flynt.toml
locales-dir = "i18n"
locales = ["en", "fr", "de"]

# This project uses different names for its helper functions.
filters = ["tr", "trn"]
functions = ["translate"]

# These keys are assembled at runtime. They only look unused.
unused = "error"
ignore-unused = ["err-http-*", "dashboard-metric-*"]
```

### Order
1. The command line wins over `.flynt.toml`.
2. `.flynt.toml` wins over what was inferred.
3. Anything left unspecified falls back to the built-in default.


## JSON output

`--format json` emits the whole report for a CI step or a dashboard. Paths are relative to the
linted directory. Every list is sorted to keep the output stable between runs and easier to diff.

```json
{
  "schema_version": 1,
  "summary": {
    "used": 136,
    "defined": 136,
    "defined_per_locale": { "en": 136, "fr": 136 },
    "locales": ["en", "fr"],
    "reference_locale": "en",
    "files_scanned": 199
  },
  "missing_keys": [
    {
      "key": "tpl-orphan",
      "usages": [
        {
          "key": "tpl-orphan",
          "at": { "file": "types/templates/home.html", "line": 42, "column": 9 },
          "kind": "template"
        }
      ],
      "missing_in": ["fr"]
    }
  ],
  "inconsistent_keys": [{ "key": "only-en", "present_in": ["en"], "missing_in": ["fr"] }],
  "duplicate_keys": [
    {
      "key": "dup",
      "locale": "en",
      "definitions": [{ "at": { "file": "locales/en/a.ftl", "line": 3, "column": 1 } }]
    }
  ],
  "unused_keys": [
    {
      "key": "stale",
      "locale": "en",
      "definition": { "at": { "file": "locales/en/app.ftl", "line": 7, "column": 1 } }
    }
  ],
  "parse_errors": [
    {
      "at": { "file": "locales/en/broken.ftl", "line": 12, "column": 7 },
      "message": "Expected a token starting with \"=\""
    }
  ]
}
```

`schema_version` is bumped on any breaking change.

## Use as a library

```rust
use flynt::config::{self, PartialConfig};

let cli = PartialConfig {
    root: Some("../my-project".into()),
    ..PartialConfig::default()
};
let config = config::load(&cli)?;
let report = flynt::check(&config)?;

for finding in &report.missing_keys {
    println!("{} is missing in {}", finding.key, finding.missing_in.join(", "));
}
# Ok::<(), anyhow::Error>(())
```

`flynt::check` produces no output of its own. Render it with `flynt::report::render`, or act on the
findings directly.

## Exit codes

| Code | Meaning                                                                    |
| ---- | -------------------------------------------------------------------------- |
| `0`  | Clean, or warnings only                                                    |
| `1`  | Findings at error severity                                                 |
| `2`  | Flynt could not do its job: a bad path, an unreadable manifest, no locales |


## Limitations

- Keys assembled at runtime like `format!("err-{code}")` cannot be seen. Cover them with
  `--ignore-unused`.
- Only whole-line `//` comments are skipped. Flynt still scans `/* ... */` blocks.
- A crate whose `[lib] path` points outside `src` is not found by inference. Add it with
  `--add-src`.
- Fluent terms (`-brand = ...`) count for the duplicate check. They are never reported as unused.
  Nothing outside the `.ftl` files can reference a term.

## Requirements

Rust 1.85 or later, edition 2024.

## License

Licensed under either of

- [Apache License](LICENSE-APACHE), Version 2.0
- [MIT License](LICENSE-MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
