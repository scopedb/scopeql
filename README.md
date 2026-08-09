# ScopeQL: ScopeDB Command Line Interface

[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[license-badge]: https://img.shields.io/crates/l/scopeql
[license-url]: LICENSE
[actions-badge]: https://github.com/scopedb/scopeql/workflows/CI/badge.svg
[actions-url]:https://github.com/scopedb/scopeql/actions?query=workflow%3ACI

## Overview

`scopeql` is a one-shot command-line client for ScopeDB. It reads ScopeQL from a
script, stdin, or an explicit command and submits each top-level statement
independently.

This repository documents the CLI, not the ScopeQL language. For ScopeQL syntax
and examples, use the canonical language documentation:

- [ScopeDB documentation](https://docs.scopedb.io/)
- [ScopeQL quickstart](https://docs.scopedb.io/guides/quickstart)
- [ScopeQL reference](https://docs.scopedb.io/reference/)

## Installation

You can install `scopeql` with Cargo:

```bash
cargo install scopeql
```

Or you can download pre-built binaries from the [releases page](https://github.com/scopedb/scopeql/releases).

Or you can run the CLI client with Docker:

```bash
docker run --rm scopedb/scopeql --help
```

## Connect to ScopeDB

ScopeDB is a managed service. Open **Connect** in ScopeDB Console and copy the
**ScopeDB API** address for your workspace. Then create an API key in **API
Keys**, or use a key secret you previously stored.

Create a connection interactively:

```bash
scopeql connection add
```

Choose **API Key**, then enter the ScopeDB API address and API key you obtained
from Console. The first connection is named `default` unless you choose another
name.

Use `scopeql connection list` to view configured connections,
`scopeql connection default <connection>` to choose the default connection, and
`scopeql connection remove <connection>` to delete one.

These `connection` commands describe builds from the current `main` branch. The
published v0.6.0 release instead uses:

```bash
scopeql config set-connection <connection>
scopeql config get-connections
scopeql config use-connection <connection>
scopeql config delete-connection <connection>
```

## Run ScopeQL

Pass a script directly to `scopeql run`. A script may contain multiple top-level
statements separated by semicolons:

```bash
scopeql run queries.scopeql
```

Omit the file, or use `-`, to read the entire script from stdin. This is the
preferred way to generate ScopeQL in a shell because the shell does not parse
the statement text:

```bash
scopeql run < queries.scopeql

scopeql run - <<'SCOPEQL'
SHOW DATABASES;
SHOW SCHEMAS;
SCOPEQL
```

For a short statement that does not need shell-sensitive syntax, use
`-c/--command`:

```bash
scopeql run --command 'SHOW DATABASES;'
```

Use `--format table|json|csv|jsonl` to select the result format,
`-o/--output <FILE>` to write results to a file, and `-q/--quiet` to suppress
normal output. Running `scopeql` without a subcommand displays command help.

## Configuration

`scopeql` stores connection settings in `config.toml`. An API Key connection sends
the configured key as an `Authorization: Bearer <key>` header:

```toml
default_connection = "default"

[connections.default]
endpoint = "https://<workspace-endpoint>"
auth = "api_key"
api_key = "<api-key>"
```

You can also supply connection settings with environment variables such as
`SCOPEQL_CONFIG_DEFAULT_CONNECTION`,
`SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_ENDPOINT`,
`SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_AUTH`, and
`SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_API_KEY`. Environment-only
configuration must include enough fields to define the default connection.

## Logs

Logs are written to the `.scopeql/logs/` subdirectory of the platform's [cache directory](https://docs.rs/dirs/latest/dirs/fn.cache_dir.html) (falling back to `$HOME/.scopeql/logs/`). The default log level is `INFO`. To change the log level, set the [`RUST_LOG`](https://docs.rs/logforth/latest/logforth/filter/env_filter/index.html) environment variable, e.g., `RUST_LOG=debug` for more verbose output.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
