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

```toml
default_connection = "default"

[connections.default]
endpoint = "https://<cell>.<provider>.scopedb.cloud"
api_key = "your-api-key"
headers = ["X-Tenant: acme"]
```

You can also override connection settings with environment variables such as `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_ENDPOINT`, `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_API_KEY`, and `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_HEADERS`. The `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_HEADERS` value should use newline-separated `KEY: VALUE` entries, matching the `headers = ["KEY: VALUE"]` TOML format.

## Logs

Logs are written in JSON format to the `scopeql/logs/` subdirectory of the platform's [cache directory](https://docs.rs/dirs/latest/dirs/fn.cache_dir.html) (falling back to `~/.scopeql/logs/`). Logs are rotated daily with a 7-day retention. The default log level is `INFO`. To change the log level, set the `RUST_LOG` environment variable, e.g., `RUST_LOG=debug` for more verbose output.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
