# <img src="https://raw.githubusercontent.com/ystorian/flynt/main/flynt.svg" width="48" align="absmiddle" alt="Flynt logo"> Flynt

**Fluent linter for Askama templates**

![Build Status](https://github.com/ystorian/flynt/actions/workflows/ci-rust.yaml/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/flynt.svg)](https://crates.io/crates/flynt)
[![docs.rs](https://docs.rs/flynt/badge.svg)](https://docs.rs/flynt)

Flynt collects the [Fluent](https://projectfluent.org) translation keys used in Rust code and
[Askama](https://askama.readthedocs.io) templates. Then it compares them against the Fluent `.ftl` files in the
`locales` directory to report issues.

## Install


Flynt [releases](https://github.com/ystorian/flynt/releases) are signed and can be installed
with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```shell
cargo binstall --only-signed flynt
```

## Quick start

```shell
$ flynt

✓ All translation keys validated

Summary:
  136 keys used
  136 keys defined
  2 locales checked: en, fr
```


## Checks
Flynt reports every syntax problem in one pass.

1. Coverage: A key used in code or in a template is defined in every locale.
2. Consistency: Every locale defines the same set of keys.
3. Duplicated keys: No locale defines the same key twice.
4. Unused keys: Every defined key is used. Unused keys are a warning.

### Syntax
Every `.ftl` file parses.

## How it works
Flynt finds the relevant files in this order:

1. **Cargo**
   - Read `Cargo.toml` to get the member crates.
   - Scan the `src` directory of every entry in `[workspace] members`.
   - Or scan the crate's own `src` for project without `[workspace]`.
2. **Templates**
   - Scan the `templates` directory next to each of those crates.
3. **Locales**
   - Scan every subdirectory of `locales` and search recursively for `.ftl` files.

> The layout is `locales/<locale>/**/*.ftl`.

## Ignored files and text

- Default members: The crate's `[workspace]`.`default-members` is ignored.
- Comments: Lines that begin with `//` are ignored.


## What it looks for

By default, these four helpers: `t` and `tn` Askama filters, and `loc` and `loc_with_args` Rust functions.

### Askama filters

```jinja
{{ "tpl-title" | t(&lang) }}
{{ "mail-count" | tn(&lang, "count", count) }}
```

> [!TIP] Askama filters override
> Use `--filters` if the project uses different filter names.

### Rust functions

```rust
loc("err-not-found", &lang)
loc_with_args("tpl-elapsed-weeks", &lang, &args)
```

> [!TIP] Rust functions override
> Use `--functions` if the project uses different function names.

## Options

| `flynt` flags                 | `.flynt.toml` keys | Default values                           |
| ----------------------------- | ------------------ | ---------------------------------------- |
| `[PATH]`                      |                    | `.`                                      |
| `--config <FILE>`             |                    | `.flynt.toml`                            |
| `--no-config`                 |                    | `false`                                  |
| `--manifest-path <FILE>`      | `manifest-path`    | `Cargo.toml`                             |
| `--locales-dir <DIR>`         | `locales-dir`      | `locales`                                |
| `--locales <LOCALE>`          | `locales`          | all subdirectories in `locales`          |
| `--reference-locale <LOCALE>` | `reference-locale` | `en`                                     |
| `--src <DIR>`                 | `src`              | `<member>/src` for each workspace member |
| `--add-src <DIR>`             | `add-src`          | _none_                                   |
| `--templates <DIR>`           | `templates`        | `<member>/templates` where present       |
| `--add-templates <DIR>`       | `add-templates`    | _none_                                   |
| `--template-ext <EXT>`        | `template-ext`     | `html`                                   |
| `--filters <NAME>`            | `filters`          | `t`, `tn`                                |
| `--functions <NAME>`          | `functions`        | `loc`, `loc_with_args`                   |
| `--unused <LEVEL>`            | `unused`           | `warn` (`error`, `warn`, `allow`)        |
| `--ignore-unused <GLOB>`      | `ignore-unused`    | _none_                                   |
| `--attributes[=BOOL]`         | `attributes`       | `true`                                   |
| `--exclude <GLOB>`            | `exclude`          | `target/**`                              |
| `--follow-links[=BOOL]`       | `follow-links`     | `true`                                   |
| `--format <FORMAT>`           | `format`           | `text` (`text`, `json`)                  |
| `--color <WHEN>`              | `color`            | `auto` (`auto`, `always`, `never`)       |
| `-q, --quiet[=BOOL]`          | `quiet`            | `false`                                  |


## Configuration file

Create a `.flynt.toml` configuration file at the root of the project. The keys match the flag names.

It can set anything you can pass on the command line:

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
1. command line parameters
2. configuration keys in `.flynt.toml`
3. default settings


## JSON output

Use `--format json` to generate a report for a CI step or a dashboard:
- paths are relative to the linted directory
- lists are sorted

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

`schema_version` is bumped on breaking changes.

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

`flynt::check` does not produce any output, use `flynt::report::render` to generate the report.

## Exit codes

| Code | Meaning                                                     |
| ---- | ----------------------------------------------------------- |
| `0`  | Clean or findings with **warning** only                     |
| `1`  | Findings with **errors**                                    |
| `2`  | Internal errors (bad path, unreadable manifest, no locales) |


## Limitations

- Keys assembled at runtime like `format!("err-{code}")` cannot be seen. Cover them with
  `--ignore-unused`.
- Only `//` line comments are skipped.
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
