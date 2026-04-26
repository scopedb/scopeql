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

You can also override connection settings with environment variables such as `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_ENDPOINT` and `SCOPEQL_CONFIG_CONNECTIONS_<CONNECTION_NAME>_API_KEY`.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
