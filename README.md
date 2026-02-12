# OpenAPI WASM Foreign Data Wrapper

> **Note:** This standalone repository will be archived once the OpenAPI FDW is merged into [supabase/wrappers](https://github.com/supabase/wrappers) ([PR #566](https://github.com/supabase/wrappers/pull/566)). Future releases will be published from the main wrappers repo. Documentation will move to [fdw.dev](https://fdw.dev/).

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
    fdw_package_url 'https://github.com/codybrom/openapi-fdw/releases/download/v0.2.0/openapi_fdw.wasm',
    fdw_package_name 'supabase:openapi-fdw',
    fdw_package_version '0.2.0',
    fdw_package_checksum '<sha256-checksum>',
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
    fdw_package_url 'https://github.com/codybrom/openapi-fdw/releases/download/v0.2.0/openapi_fdw.wasm',
    fdw_package_name 'supabase:openapi-fdw',
    fdw_package_version '0.2.0',
    fdw_package_checksum '<sha256-checksum>',
    base_url 'https://api.weather.gov',
    spec_url 'https://api.weather.gov/openapi.json',
    user_agent 'openapi-fdw'
  );

import foreign schema openapi from server weather_api into public;

select * from stations limit 5;
```

### Authenticated API

For Supabase, store credentials in Vault:

```sql
select vault.create_secret('<your_api_key>', 'my_api_key');
-- returns a secret UUID, e.g. 'a]b2c3d4-...'
```

Then reference the Vault secret ID:

```sql
create server my_api
  foreign data wrapper wasm_wrapper
  options (
    fdw_package_url 'https://github.com/codybrom/openapi-fdw/releases/download/v0.2.0/openapi_fdw.wasm',
    fdw_package_name 'supabase:openapi-fdw',
    fdw_package_version '0.2.0',
    fdw_package_checksum '<sha256-checksum>',
    base_url 'https://api.example.com/v1',
    spec_url 'https://api.example.com/openapi.json',
    api_key_id '<vault_secret_id>'
  );
```

Or pass the key inline (for non-Supabase setups):

```sql
    api_key 'sk-your-api-key-here'
```

### Manual Table Definition

Instead of `IMPORT FOREIGN SCHEMA`, you can define tables manually for more control:

```sql
create foreign table api_users (
    id text,
    name text,
    email text,
    created_at timestamptz,
    attrs jsonb
)
server my_api
options (
    endpoint '/users',
    rowid_column 'id'
);

select id, name, email from api_users limit 10;
```

### Path Parameters

Endpoint templates like `/users/{user_id}/posts` are substituted from your WHERE clause:

```sql
create foreign table user_posts (
    user_id text,
    id text,
    title text,
    body text,
    attrs jsonb
)
server my_api
options (
    endpoint '/users/{user_id}/posts',
    rowid_column 'id'
);

-- user_id is substituted into the URL path
select title, body from user_posts where user_id = '123';
```

### GeoJSON / Nested Responses

Use `response_path` and `object_path` to dig into wrapped or nested response structures:

```sql
create foreign table zone_alerts (
    zone_id text,
    event text,
    headline text,
    severity text
)
server weather_api
options (
    endpoint '/alerts/active/zone/{zone_id}',
    response_path '/features',
    object_path '/properties'
);

select event, severity, headline
from zone_alerts
where zone_id = 'OKC143';
```

## Server Options

| Option | Required | Default | Description |
| -------- | ---------- | --------- | ------------- |
| `base_url` | yes* | | API base URL. *Optional if `spec_url` provides servers. |
| `spec_url` | no | | URL to OpenAPI 3.0+ JSON spec. Required for `IMPORT FOREIGN SCHEMA`. |
| `api_key` | no | | API key (inline) |
| `api_key_id` | no | | Vault secret ID for API key |
| `bearer_token` | no | | Bearer token (inline) |
| `bearer_token_id` | no | | Vault secret ID for bearer token |
| `api_key_header` | no | `Authorization` | Header name for API key |
| `api_key_prefix` | no | `Bearer` | Prefix before key value |
| `user_agent` | no | | Custom User-Agent header |
| `accept` | no | | Accept header for content negotiation |
| `headers` | no | | Custom headers as JSON object, e.g. `'{"X-Custom": "value"}'` |
| `page_size` | no | `0` | Records per page (`0` = no limit param sent) |
| `page_size_param` | no | `limit` | Query param name for page size |
| `cursor_param` | no | `after` | Query param name for pagination cursor |
| `include_attrs` | no | `true` | Include `attrs` jsonb column in `IMPORT FOREIGN SCHEMA` output. Set to `'false'` to omit. |

## Table Options

| Option | Required | Default | Description |
| -------- | ---------- | --------- | ------------- |
| `endpoint` | yes | | API path. Supports `{param}` substitution from WHERE clauses. |
| `rowid_column` | no | `id` | Row ID column for single-resource access |
| `response_path` | no | | JSON pointer to data array, e.g. `/data`, `/features` |
| `object_path` | no | | JSON pointer into each row, e.g. `/properties` for GeoJSON |
| `cursor_path` | no | | JSON pointer to next-page cursor in response |
| `cursor_param` | no | | Override server-level `cursor_param` |
| `page_size_param` | no | | Override server-level `page_size_param` |
| `page_size` | no | | Override server-level `page_size` |

### Special Columns

- **`attrs`** (jsonb) — automatically added to all imported tables; contains the full raw JSON response for each row
- Any column matching a `{path_param}` in the endpoint gets the WHERE clause value injected back into the result

### Auto-detection

The FDW automatically detects:

- **Pagination** — cursor-based (`has_more` + cursor fields), URL-based (`next` link), or offset-based
- **Response wrapping** — unwraps common keys: `data`, `results`, `items`, `records`, `entries`, `features`
- **Column names** — `camelCase` in the API response maps to `snake_case` PostgreSQL columns

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

| Version | Date       | Notes                                                                                         |
| ------- | ---------- | --------------------------------------------------------------------------------------------- |
| 0.2.0   | 2026-02-10 | POST-for-read, OpenAPI 3.1 type arrays, new PG types, cached columns, integration tests       |
| 0.1.4   | 2026-02-09 | Type coercion, auth validation, table naming, URL fixes, include_attrs option                 |
| 0.1.3   | 2026-02-07 | Perf: avoid cloning JSON response data during row extraction                                  |
| 0.1.2   | 2026-02-06 | Fix: prefer WHERE clause value for query/path param columns                                   |
| 0.1.1   | 2026-02-06 | Fix: inject query param values back into result rows                                          |
| 0.1.0   | 2026-01-25 | Initial version                                                                               |
