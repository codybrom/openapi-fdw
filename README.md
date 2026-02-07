# OpenAPI WASM Foreign Data Wrapper

This is a WASM-based Foreign Data Wrapper (FDW) for integrating any OpenAPI 3.0+ compliant REST API into PostgreSQL through Supabase Wrappers.

Point this at an OpenAPI spec and query the API with SQL. The FDW parses the spec, figures out the endpoints and response schemas, and lets you `IMPORT FOREIGN SCHEMA` to generate tables automatically.

Handles pagination, rate limiting (429 backoff), path parameter substitution from WHERE clauses, and stops fetching early when you use LIMIT.

## Installation

Requires [Supabase Wrappers](https://github.com/supabase/wrappers) >= 0.4.1.

### 1. Enable Wrappers

```sql
create extension if not exists wrappers with schema extensions;

create foreign data wrapper wasm_wrapper
  handler wasm_fdw_handler
  validator wasm_fdw_validator;
```

### 2. Create a foreign server

```sql
create server my_api_server
  foreign data wrapper wasm_wrapper
  options (
    fdw_package_url 'https://github.com/codybrom/openapi-fdw/releases/download/v0.1.0/openapi_fdw.wasm',
    fdw_package_name 'supabase:openapi-fdw',
    fdw_package_version '0.1.0',
    fdw_package_checksum '3f559457ba5c28972fd638e4ae8376e6c5d15051ba9b5bc703ea6295bf24e98f',
    base_url 'https://api.example.com',
    spec_url 'https://api.example.com/openapi.json'
  );
```

### 3. Import tables from the API

```sql
import foreign schema openapi from server my_api_server into public;
```

### 4. Query

```sql
select * from users limit 10;
```

## Usage Examples

### Weather API (NWS)

```sql
create server weather_api
  foreign data wrapper wasm_wrapper
  options (
    fdw_package_url 'https://github.com/codybrom/openapi-fdw/releases/download/v0.1.0/openapi_fdw.wasm',
    fdw_package_name 'supabase:openapi-fdw',
    fdw_package_version '0.1.0',
    fdw_package_checksum '3f559457ba5c28972fd638e4ae8376e6c5d15051ba9b5bc703ea6295bf24e98f',
    base_url 'https://api.weather.gov',
    spec_url 'https://api.weather.gov/openapi.json',
    user_agent 'openapi-fdw'
  );

import foreign schema openapi from server weather_api into public;

select * from stations limit 5;
```

### Authenticated API

```sql
create server my_api
  foreign data wrapper wasm_wrapper
  options (
    fdw_package_url 'https://github.com/codybrom/openapi-fdw/releases/download/v0.1.0/openapi_fdw.wasm',
    fdw_package_name 'supabase:openapi-fdw',
    fdw_package_version '0.1.0',
    fdw_package_checksum '3f559457ba5c28972fd638e4ae8376e6c5d15051ba9b5bc703ea6295bf24e98f',
    base_url 'https://api.example.com/v1',
    spec_url 'https://api.example.com/openapi.json',
    api_key_id '<vault_secret_id>'  -- or use api_key for inline key
  );
```

## Development

### Building

```bash
cargo component build --release --target wasm32-unknown-unknown
```

### Running Tests

```bash
cargo test
```

## Limitations

- Read-only (no INSERT/UPDATE/DELETE support)
- Only GET endpoints are supported
- Authentication limited to API key and Bearer token (No OAuth2 flow support yet - use pre-obtained tokens)

## Changelog

| Version | Date       | Notes                                                |
| ------- | ---------- | ---------------------------------------------------- |
| 0.1.0   | 2026-01-25 | Initial version                                      |
