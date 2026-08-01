# CHANGELOG

All significant changes to this software be documented in this file.

## Unreleased

### Breaking Changes

* Rename connection management from `scopeql config` to `scopeql connection` with subcommands `list`, `default`, `add`, and `remove`.
* Stop falling back to an implicit local default connection when no config file or complete config environment exists.

### New Features

* Prompt for connection setup before entering the REPL when no config file or config environment variables exist.
* Allow deleting the last configured connection.
* List REPL connections when `/connect` is run without a connection name.

### Bug Fixes

* Allow the ScopeQL CLI to compile on stable Rust.
* Reject duplicate connection names in `scopeql connection add`.
* Show the REPL command list after an unknown REPL command.

## v0.6.0 (2026-06-03)

### Breaking Changes

* Remove `--header` options and the REPL `/headers` command.
* REPL command now starts with slash (`/`) instead of backslash (`\`), e.g. `\format` becomes `/format`.
* Drop `scopeql gen config` command since connection management is now handled by `scopeql config` subcommands.
* Connection specs now requires an "auth" field, like:

```toml
[connections.local]
endpoint = "http://127.0.0.1:6543"
auth = "direct"

[connections.cloud]
endpoint = "https://c065.aws.scopedb.cloud"
auth = "api_key"
api_key = "sk_..."
```

### New Features

* Allow custom HTTP headers to be configured through connection configuration or `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_HEADERS`.
* Support new command `scopeql config` with subcommands `set-connection`, `delete-connection`, `get-connections`, and `use-connection` to manage connection specs.
* Support REPL command `/connections` to list available connections and the current connection.

## v0.5.1 (2026-04-15)

### New Features

* Allow `run` and `load` command to specify `--header "<key>: <value>"` for custom HTTP headers.
* Add REPL command `\headers [set|unset|unsetall]` to manage custom HTTP headers in REPL.

## v0.5.0 (2026-04-14)

### Breaking Changes

* Disallow changing connection in REPL (`\connect` is dropped). To connect to another ScopeDB instance, start a new REPL session.

### Improvements

* `scopeql run` supports multiple top-level statements again. When you need machine-readable output across multiple statements `--format jsonl` or `--format csv`.
* `--format` now belongs to `scopeql run`; the REPL starts in `table` mode and can switch formats with `\format`.
* Connections can now configure `api_key`, which is sent as an `Authorization: Bearer <key>` header.

## v0.4.3 (2026-02-13)

### Bug Fixes

* Install CA certificates for HTTPS support in Docker Image.

## v0.4.2 (2026-02-13)

This release bumps the minimal supported ScopeDB version to v0.2.0, which drops support for `CREATE EQUALITY INDEX` and `CREATE OBJECT INDEX`. Both of them are now merged into the new `CREATE POINT INDEX` clause.

### Breaking Changes

* No longer recognize `EQUALITY` token since `CREATE EQUALITY INDEX` clause has been merged into `CREATE POINT INDEX`.

### New Features

* Recognize `POINT` token for `CREATE POINT INDEX` clause.

## v0.3.2 (2026-02-08)

### Improvements

* Recognize `PARTITION` token for `PARTITION BY` clause.
* Better error message for unrecognized tokens.

## v0.3.1 (2026-01-31)

Functionality identical to v0.3.0, released the first binary after repo migration.

## v0.3.0 (2025-12-22)

### Breaking Changes

* `scopeql -c < script.sql` no longer supported, use `scopeql run -f script.sql` instead.
* `scopeql -c -` no longer supported.

### New Features

* Support `scopeql run <statement>` to run statement directly.
* Support `scopeql run -f <filename>` to run script from file.
* Support new HTTP request `DataType::UInt` as `uint`.
* Recognize `DATABASES` and `SCHEMAS` tokens to support related SHOW statements.

## v0.2.2 (2025-12-08)

### New Features

* Support `scopeql -c < script.sql` to run script from file.
* Support `scopeql load -f <file> -t <transform>` command to load data from file.
* Support `ANALYZE` keyword so that `EXPLAIN ANALYZE <query>` works.

## v0.2.1 (2025-10-30)

### New Features

* Recognize `VACUUM` token to support `VACUUM` command.

### Improvements

* Repl now pretty-prints semi-structure data.

## v0.2.0 (2025-10-21)

### Breaking Changes

* No longer support `-e` option for specifying the entrypoint. Use config files instead.

### New Features

* Support load config from file:
  * Specify config file with `--config-file` option.
  * If not specified, trying to look up from:
    * `$HOME/.scopeql/config.toml`
    * `$HOME/.config/scopeql/config.toml`
    * `${CONFIG_DIR:-$XDG_CONFIG_HOME}/scopeql/config.toml`; see [this page](https://docs.rs/dirs/6.0.0/dirs/fn.config_dir.html) for more details about `config_dir`.
  * Otherwise, fallback to default config.

## v0.1.1 (2025-08-21)

### Developments

* Fix the release workflow to properly build AMD64 image.

## v0.1.0 (2025-08-21)

* Initial release.
