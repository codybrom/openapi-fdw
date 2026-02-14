# GitHub API Example

Query the [GitHub REST API](https://docs.github.com/en/rest) using SQL. This example demonstrates bearer token authentication, page-based pagination, path parameter substitution, query parameter pushdown, and custom HTTP headers.

## Quick Start

**Prerequisites:** Docker, Rust 1.88+, `cargo-component` v0.21.1, `wasm32-unknown-unknown` target, and a [GitHub personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens).

```bash
# Create .env with your GitHub token
cp examples/github/.env.example examples/github/.env
# edit .env with your token

# Start everything (builds WASM, starts Postgres, configures auth)
./examples/github/setup.sh

# Connect
psql postgresql://postgres:postgres@localhost:54324/postgres

# When done
./examples/github/teardown.sh
```

> All queries below hit the live GitHub API. Results will reflect real data.

---

## 1. Your Profile

Single object response. The FDW returns one row with your GitHub profile info.

```sql
SELECT login, name, public_repos, followers
FROM my_profile;
```

| login | name | public_repos | followers |
| --- | --- | --- | --- |
| youruser | Your Name | 42 | 150 |

> Your results will reflect your own GitHub profile.

## 2. Your Repositories

Paginated list of your repos. The FDW auto-detects page-based pagination via `Link` headers.

```sql
SELECT name, language, stargazers_count, fork
FROM my_repos
LIMIT 5;
```

| name | language | stargazers_count | fork |
| --- | --- | --- | --- |
| my-project | TypeScript | 24 | f |
| dotfiles | Shell | 3 | f |
| cool-app | Rust | 12 | f |
| some-fork | | 0 | t |
| api-client | Python | 8 | f |

> Your results will reflect your own repositories.

Filter with query pushdown:

```sql
-- Pushes down to: GET /user/repos?type=owner&sort=updated
SELECT name, language, updated_at
FROM my_repos
WHERE type = 'owner' AND sort = 'updated'
LIMIT 5;
```

## 3. Repository Detail (Path Parameters)

Look up a specific repository. The `{owner}` and `{repo}` placeholders in the endpoint are replaced with values from your WHERE clause.

```sql
SELECT name, stargazers_count, forks_count, language
FROM repo_detail
WHERE owner = 'supabase' AND repo = 'wrappers';
```

| name | stargazers_count | forks_count | language |
| --- | --- | --- | --- |
| wrappers | 811 | 92 | Rust |

## 4. Repository Issues

Issues for a repository. Two path parameters plus query pushdown for state filtering:

```sql
SELECT number, title, state
FROM repo_issues
WHERE owner = 'supabase' AND repo = 'wrappers'
LIMIT 5;
```

| number | title | state |
| --- | --- | --- |
| 571 | chore(deps): bump aws-sdk-s3 from 1.109.0 to 1.112.0 in the cargo group across 1 directory | open |
| 549 | feat: add aggregate pushdown support via GetForeignUpperPaths | open |
| 472 | AWS Cognito wrapper, ERROR: HV000: unhandled error | open |
| 461 | Hubspot FDW requires API Keys which are deprecated | open |
| 459 | Auth0 FDW API Key | open |

Filter by state:

```sql
SELECT number, title, state
FROM repo_issues
WHERE owner = 'supabase' AND repo = 'wrappers' AND state = 'closed'
LIMIT 5;
```

## 5. Pull Requests

Pull requests with state filtering via query pushdown:

```sql
SELECT number, title, state
FROM repo_pulls
WHERE owner = 'supabase' AND repo = 'wrappers' AND state = 'closed'
LIMIT 5;
```

| number | title | state |
| --- | --- | --- |
| 572 | docs(openapi): update wasm module checksum and improve docs | closed |
| 570 | chore(deps): bump time from 0.3.44 to 0.3.47 in the cargo group across 1 directory | closed |
| 569 | feat: add comprehensive AI assistant guide for Wrappers project | closed |
| 568 | chore(deps): bump bytes from 1.10.1 to 1.11.1 in the cargo group across 1 directory | closed |
| 567 | chore(deps): bump wasmtime from 36.0.3 to 36.0.5 in the cargo group across 1 directory | closed |

## 6. Releases

Paginated list of releases for a repository:

```sql
SELECT tag_name, name, prerelease
FROM repo_releases
WHERE owner = 'supabase' AND repo = 'wrappers'
LIMIT 5;
```

| tag_name | name | prerelease |
| --- | --- | --- |
| wasm_openapi_fdw_v0.1.4 | wasm_openapi_fdw_v0.1.4 | f |
| wasm_snowflake_fdw_v0.2.1 | wasm_snowflake_fdw_v0.2.1 | f |
| wasm_infura_fdw_v0.1.0 | wasm_infura_fdw_v0.1.0 | f |
| wasm_clerk_fdw_v0.2.1 | wasm_clerk_fdw_v0.2.1 | f |
| v0.5.7 | v0.5.7 | f |

## 7. Search Repositories (Query Pushdown)

When a WHERE clause references `q`, the FDW sends it as a query parameter to the `/search/repositories` endpoint. The FDW auto-detects the `items` wrapper key in the response.

```sql
-- Pushes down to: GET /search/repositories?q=openapi+foreign+data+wrapper
SELECT name, full_name, stargazers_count
FROM search_repos
WHERE q = 'openapi foreign data wrapper'
LIMIT 5;
```

| name | full_name | stargazers_count |
| --- | --- | --- |
| openapi_fdw | sabino/openapi_fdw | 2 |
| openapi-fdw | user/openapi-fdw | 1 |
| fdw-api | user/fdw-api | 0 |

## 8. Debug Mode

The `search_repos_debug` table uses the `github_debug` server which has `debug 'true'`. This emits HTTP request details as PostgreSQL INFO messages.

```sql
SELECT id FROM search_repos_debug WHERE q = 'supabase' LIMIT 1;
```

Look for INFO output like:

```log
INFO:  [openapi_fdw] HTTP GET https://api.github.com/search/repositories?per_page=30&q=supabase -> 200 (176333 bytes)
INFO:  [openapi_fdw] Scan complete: 1 rows, 2 columns
```

## The `attrs` Column

Every table includes an `attrs jsonb` column that captures all fields not mapped to named columns:

```sql
SELECT name, attrs->>'visibility' AS visibility,
       attrs->>'has_wiki' AS has_wiki
FROM my_repos
LIMIT 3;
```

| name | visibility | has_wiki |
| --- | --- | --- |
| my-project | public | true |
| dotfiles | public | false |
| cool-app | public | true |

## Features Demonstrated

| Feature | Table(s) |
| --- | --- |
| Bearer token auth (Authorization header) | All tables |
| Custom HTTP headers (X-GitHub-Api-Version) | All tables |
| Page-based pagination (auto-detected) | `my_repos`, `repo_issues`, `repo_pulls`, `repo_releases`, `search_repos` |
| Path parameter substitution | `repo_detail`, `repo_issues`, `repo_pulls`, `repo_releases` |
| Query parameter pushdown | `my_repos` (`type`, `sort`), `repo_issues` (`state`), `repo_pulls` (`state`), `search_repos` (`q`) |
| Single object response | `my_profile`, `repo_detail` |
| Auto-detected wrapper key (`items`) | `search_repos`, `search_repos_debug` |
| Type coercion (timestamptz, boolean, bigint) | All tables |
| Debug mode | `search_repos_debug` |
| `attrs` catch-all column | All tables |
| `rowid_column` | `my_repos`, `repo_issues`, `repo_pulls`, `repo_releases`, `search_repos` |
