# ScopeQL: ScopeDB Command Line Interface

[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[license-badge]: https://img.shields.io/crates/l/scopeql
[license-url]: LICENSE
[actions-badge]: https://github.com/scopedb/scopeql/workflows/CI/badge.svg
[actions-url]:https://github.com/scopedb/scopeql/actions?query=workflow%3ACI

## Overview

`scopeql` provides a command line interface and interactive shell for ScopeDB.

## Installation

You can install `scopeql` with Cargo:

```bash
cargo install scopeql
```

Or you can download pre-built binaries from the [releases page](https://github.com/scopedb/scopeql/releases).

Or you can use the Docker image:

```bash
docker run -it --rm scopedb/scopeql
```

## Configuration

`scopeql` reads its default connection settings from `config.toml`. Each connection can optionally define an API key, which is sent as an `Authorization: Bearer <key>` header on requests.

Use `scopeql connection list` to view configured connections, `scopeql connection default <connection>` to choose the default connection, `scopeql connection add` to create a connection interactively, and `scopeql connection remove <connection>` to delete one. The add prompt always runs the full first-launch setup flow and defaults to a `default` connection in API Key mode. Starting `scopeql` without a config file opens the same connection setup prompt before entering the REPL.

```toml
default_connection = "default"

[connections.default]
endpoint = "https://<cell>.<provider>.scopedb.cloud"
auth = "api_key"
api_key = "your-api-key"
headers = ["X-Tenant: acme"]
```

You can also supply connection settings with environment variables such as `SCOPEQL_CONFIG_DEFAULT_CONNECTION`, `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_ENDPOINT`, `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_AUTH`, `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_API_KEY`, and `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_HEADERS`. Environment-only configuration must include enough fields to define the default connection. The `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_HEADERS` value should use newline-separated `KEY: VALUE` entries, matching the `headers = ["KEY: VALUE"]` TOML format.

## Logs

Logs are written to the `.scopeql/logs/` subdirectory of the platform's [cache directory](https://docs.rs/dirs/latest/dirs/fn.cache_dir.html) (falling back to `$HOME/.scopeql/logs/`). The default log level is `INFO`. To change the log level, set the [`RUST_LOG`](https://docs.rs/logforth/latest/logforth/filter/env_filter/index.html) environment variable, e.g., `RUST_LOG=debug` for more verbose output.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
