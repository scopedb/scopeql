# ScopeQL: ScopeDB Command Line Interface

[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[license-badge]: https://img.shields.io/crates/l/scopeql
[license-url]: LICENSE
[actions-badge]: https://github.com/scopedb/scopeql/workflows/CI/badge.svg
[actions-url]:https://github.com/scopedb/scopeql/actions?query=workflow%3ACI

## Overview

`scopeql` provides a command line interface and interactive shell for ScopeDB.

## Configuration

`scopeql` reads its default connection settings from `config.toml`. Each connection can optionally define an API key, which is sent as an `Authorization: Bearer <key>` header on requests.

```toml
default_connection = "default"

[connections.default]
endpoint = "https://api.scopedb.example"
api_key = "your-api-key"
```

You can also override connection settings with environment variables such as `SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_ENDPOINT` and `SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_API_KEY`.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
