# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of `uconv`.
- Direct conversion: `uconv <value> <from> <to>`.
- Natural language conversion: `uconv "100 km to miles"`.
- Interactive REPL: `uconv --repl`.
- Currency conversion with live rates from `open.er-api.com`.
- 1-hour on-disk cache at `~/.cache/uconv/rates.json`.
- `--offline` and `--refresh` flags for cache control.
- Temperature (C, F, K, R), Length (mm...nmi), Weight (g...t),
  Volume (ml...gal), Time (s...yr) categories.
- `--precision N` and `--scientific` output formatting.
- `--list [CATEGORY]` to enumerate supported units.
- `--file PATH` batch mode.
- Fuzzy unit-name suggestions via Levenshtein distance.
- Unit and integration tests; `cargo clippy -- -D warnings` clean.

### Notes
- Time `month` and `year` use Julian approximations (365.25-day year).
- Volume units are US customary; Imperial variants are not yet supported.

## [0.1.0] — 2026-11-24

First public release.
