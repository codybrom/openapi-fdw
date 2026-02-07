# Contributing

Thanks for your interest in contributing to OpenAPI FDW!

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) 1.88+
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [cargo-component](https://github.com/bytecodealliance/cargo-component) v0.21.1: `cargo install --locked cargo-component --version 0.21.1`

### Building

```bash
cargo component build --release --target wasm32-unknown-unknown
```

### Running Tests

Unit tests run on the native target:

```bash
cargo test
```

### Formatting & Linting

```bash
cargo fmt --check
cargo clippy --all --tests --no-deps
```

## Submitting Changes

1. Fork the repo and create a branch from `main`
2. Make your changes and add tests if applicable
3. Ensure `cargo test`, `cargo fmt --check`, and `cargo clippy` all pass
4. Open a pull request

## Reporting Issues

Please open an issue on GitHub with:

- What you were trying to do
- What happened instead
- The API you were querying (if relevant)
- Any error messages

## License

By contributing, you agree that your contributions will be licensed under the [Apache License 2.0](LICENSE).
