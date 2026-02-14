# Examples

Each example is a self-contained Docker setup that connects to a real API. Run `setup.sh` to start, `verify.sh` to test, and `teardown.sh` to clean up.

**Prerequisites:** Docker, Rust 1.88+, `cargo-component` v0.21.1, `wasm32-unknown-unknown` target.

## No Auth Required

| Example | API | Port | Features |
| --- | --- | --- | --- |
| [pokeapi](pokeapi/) | [PokéAPI](https://pokeapi.co/) | 54325 | Offset-based pagination, path params, auto-detected `results` wrapper |
| [carapi](carapi/) | [CarAPI](https://carapi.app/) | 54326 | Page-based pagination, query pushdown, auto-detected `data` wrapper |
| [nws](nws/) | [National Weather Service](https://www.weather.gov/documentation/services-web-api) | 54322 | GeoJSON responses, nested path extraction, custom User-Agent |

## Auth Required

| Example | API | Port | Auth | Features |
| --- | --- | --- | --- | --- |
| [github](github/) | [GitHub REST API](https://docs.github.com/en/rest) | 54324 | Bearer token | Path params, custom headers, `items` wrapper, search pushdown |
| [threads](threads/) | [Meta Threads API](https://developers.facebook.com/docs/threads) | 54323 | Bearer token | Cursor-based pagination, path params, query pushdown |

## Usage

```bash
# Pick an example
cd examples/pokeapi

# Start (builds WASM, starts Postgres, copies binary)
./setup.sh

# Run smoke tests
./verify.sh

# Connect directly
psql postgresql://postgres:postgres@localhost:54325/postgres

# Clean up
./teardown.sh
```

For auth-required examples, copy `.env.example` to `.env` and add your token before running `setup.sh`.
