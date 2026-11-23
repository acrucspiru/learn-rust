# uconv

A fast, offline-capable command-line unit converter written in idiomatic Rust.

`uconv` converts between units of **currency**, **temperature**, **length**,
**weight**, **volume**, and **time**. Currency rates are fetched live from a
free public API and cached for one hour; every other category is fully
offline.

## Features

- **Direct conversion**: `uconv 100 km mi`, `uconv 25 C F`, `uconv 50 USD EUR`
- **Natural language** (bonus): `uconv "100 km to miles"`, `uconv "25 C in F"`
- **Interactive REPL**: `uconv --repl`
- **Live currency rates** with a 1-hour disk cache (`~/.cache/uconv/rates.json`)
- **`--offline`** uses cache only; **`--refresh`** forces a fresh fetch
- **Precision control**: `--precision 3`, `--scientific`
- **List units**: `uconv --list`, `uconv --list currency`
- **Batch mode**: `uconv --file conversions.txt`
- **Actionable errors**: "Unknown unit 'metre'. Did you mean 'meter'?"
- **No telemetry, no tracking**

## Install

### From source

```sh
git clone https://github.com/acrucspiru/learn-rust
cd learn-rust/uconv
cargo install --path .
```

Requires Rust 1.74 or later (Rust 2021 edition). The build is fully static;
no system OpenSSL is required thanks to `rustls-tls`.

## Usage

### Direct conversion

```
$ uconv 100 km mi
100 km = 62.137119 mi

$ uconv 25 C F
25 °C = 77 °F

$ uconv 50 USD EUR
50 USD = 46 EUR  (rates as of 2025-04-01 12:34 UTC)

$ uconv 1.5 kg lb --precision 3
1.5 kg = 3.307 lb

$ uconv 1 day s --scientific
1 d = 8.64e4 s
```

### Natural language

```
$ uconv "100 km to miles"
100 km = 62.137119 mi

$ uconv "1 m in ft"
1 m = 3.280840 ft
```

Separators accepted: `to`, `in`, `->`, `as`.

### REPL

```
$ uconv --repl
uconv REPL — type a conversion, or `:help`. Ctrl-D to exit.
uconv> 100 km mi
100 km = 62.137119 mi
uconv> 25 C F
25 °C = 77 °F
uconv> :list currency
currency
  USD
  EUR
  ...
uconv> :quit
```

### List units

```
$ uconv --list            # every category
$ uconv --list currency   # just currency codes
$ uconv --list length
```

### Batch mode

`uconv --file batch.txt` reads one conversion per line. Lines starting with
`#` are comments; blank lines are skipped.

```
# batch.txt
100 km mi
25 C F
1.5 kg to lb --precision 3
```

### Offline and refresh

```
$ uconv --offline 50 USD EUR        # cache only, no network
$ uconv --refresh 50 USD EUR        # force fresh fetch
```

## Supported units

| Category    | Units                                                                                  |
|-------------|----------------------------------------------------------------------------------------|
| Temperature | Celsius (`C`), Fahrenheit (`F`), Kelvin (`K`), Rankine (`R`, `°R`)                     |
| Length      | mm, cm, m, km, in, ft, yd, mi, nmi                                                    |
| Weight      | g, kg, oz, lb, st, t (metric tonne)                                                   |
| Volume      | ml, l, tsp, tbsp, cup, fl oz, pt, qt, gal (US customary)                              |
| Time        | s, min, h, d, wk, mo, yr (Julian year = 365.25 days)                                  |
| Currency    | ISO-4217 codes (USD, EUR, GBP, JPY, ...) — whatever the API returns                   |

Run `uconv --list <category>` to see all aliases.

## Exit codes

| Code | Meaning            |
|------|--------------------|
| 0    | Success            |
| 1    | Generic error      |
| 2    | Network error      |
| 3    | Invalid CLI args   |

## Design notes

- **Base-unit conversion**: each category converts through a single base unit
  (metres, grams, litres, seconds, Kelvin). Currency converts through the API
  base (USD).
- **Temperature is affine**, not linear — handled with a dedicated `convert()`
  function that rejects sub-absolute-zero inputs.
- **Atomic cache writes**: writes go to `rates.json.tmp`, then `rename` for
  POSIX-atomic replacement. A crash mid-write cannot corrupt the cache.
- **`rustls-tls`**: the binary does not link against system OpenSSL, so it
  builds and runs anywhere with a working `libc`.
- **No `unwrap` in hot paths**: every fallible operation returns `Result`,
  and `main` translates errors to the correct exit code.

## Development

```sh
cargo build              # debug build
cargo test               # unit + integration tests (84 tests, all offline)
cargo clippy -- -D warnings
cargo fmt --check
```

Integration tests use `wiremock` to stub the currency API, so `cargo test`
runs entirely offline.

## License

MIT OR Apache-2.0, dual-licensed for maximum compatibility.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
