# learn-rust

A personal collection of small Rust projects, each in its own subdirectory,
written to practice idiomatic Rust patterns and the standard ecosystem.

## Projects

| Project | What it teaches | Status |
|---------|-----------------|--------|
| [`uconv/`](uconv/) | CLI app: clap v4 derive, async/tokio, reqwest+rustls, traits, `Result`-based error handling, thread-local test isolation, atomic file writes | ✓ 84 tests pass, clippy clean |

## uconv

A unit converter for currency, temperature, length, weight, volume, and
time. Direct, natural-language, REPL, and batch modes. Live currency rates
with a 1-hour on-disk cache.

```sh
cd uconv
cargo run -- 100 km mi      # 100 km = 62.137119 mi
cargo run -- 25 C F          # 25 °C = 77 °F
cargo run -- --list length   # list length units
cargo test                   # 84 tests, offline
```

See [`uconv/README.md`](uconv/README.md) for full documentation.

## License

Each subproject carries its own license header in its `Cargo.toml`.
