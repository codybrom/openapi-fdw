# Examples

Each example is a self-contained Docker setup that connects to a real API. Use `run.sh` to build, start, verify, and clean up.

**Prerequisites:** Docker, Rust 1.88+, `cargo-component` v0.21.1, `wasm32-unknown-unknown` target.

## No Auth Required

| Example | API | Features |
| --- | --- | --- |
| [pokeapi](pokeapi/) | [PokéAPI](https://pokeapi.co/) | Offset-based pagination, path params, auto-detected `results` wrapper |
| [carapi](carapi/) | [CarAPI](https://carapi.app/) | Page-based pagination, query pushdown, auto-detected `data` wrapper |
| [nws](nws/) | [National Weather Service](https://www.weather.gov/documentation/services-web-api) | GeoJSON responses, nested path extraction, custom User-Agent |

## Auth Required

| Example | API | Auth | Features |
| --- | --- | --- | --- |
| [github](github/) | [GitHub REST API](https://docs.github.com/en/rest) | Bearer token | Path params, custom headers, `items` wrapper, search pushdown |
| [threads](threads/) | [Meta Threads API](https://developers.facebook.com/docs/threads) | OAuth token (query param) | Cursor-based pagination, path params, query pushdown |

## Usage

```bash
# Run a single example (builds, starts Postgres, verifies, cleans up)
./examples/run.sh pokeapi

# Run all examples
./examples/run.sh

# Keep containers running to explore interactively
./examples/run.sh nws --no-cleanup
psql postgresql://postgres:postgres@localhost:54322/postgres
docker compose -f examples/docker-compose.yml down -v
```

For auth-required examples, copy `.env.example` to `.env` and add your tokens:

```bash
cp examples/.env.example examples/.env
# edit examples/.env with your tokens
./examples/run.sh github
```
