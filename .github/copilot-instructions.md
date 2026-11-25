# Shadow CLI — Copilot Instructions

This repository contains a Rust CLI (`shadow-cli`) that generates shadow effects for images, wrapped as a 4D component.

## Key Facts

- **CLI source**: `cli/src/` (Rust, Cargo workspace)
- **4D component**: `component/shadow/Project/Sources/Classes/`
- **Tests**: `tests/` (CLI contract vectors + fixtures)
- **Protocol**: v1 — JSON on stdout, errors on stderr

## Build & Test

```bash
cd cli && cargo build          # Build CLI
cd cli && cargo test           # Run Rust unit tests
python3 tests/test_cli.py      # Run CLI contract tests
python3 scripts/test_4d.py     # Check component structure + CLI tests
```

## Architecture

- `shadow-cli` is a standalone binary invoked via `4D.SystemWorker`
- The 4D `Generator` class wraps the CLI with argument quoting and blob stdin support
- Six shadow types: drop, contact, curved, long, perspective, inner
- All output is deterministic for the same input + parameters

## Coding Rules

- Never panic on user input — return structured errors
- All public functions must have unit tests
- Respect `tests/cli-contract.json` vectors as source of truth
- Build all targets from the same commit
