# AGENTS.md — Shadow CLI Project Playbook

> **For**: AI agents working on the `miyako/shadow` repository
> **Paradigm**: THINK/ASK → RUN → Verify → Accept

---

## 1. Project Identity

| Field | Value |
|-------|-------|
| Repository | `miyako/shadow` |
| Namespace | `Shadow` |
| CLI binary | `shadow-cli` |
| Language | Rust 1.99+ |
| License | MIT |
| Protocol version | 1 |

## 2. Architecture

- **CLI** (`cli/`): Rust binary that generates shadow images. JSON protocol on stdout, errors on stderr.
- **Component** (`component/shadow/`): 4D component wrapping the CLI via `SystemWorker`.
- **Tests** (`tests/`): CLI contract vectors, visual fixtures, 4D host smoke tests.

## 3. CLI Protocol

- One JSON line, LF-terminated on stdout for protocol responses.
- Image data: never in JSON — file paths or raw binary via stdin/stdout.
- Exit codes: 0=success, 1=runtime, 2=args, 3=I/O.

## 4. Shadow Types

| Type | Key Parameters |
|------|---------------|
| `drop` | offset-x, offset-y, blur, color, opacity, scale |
| `contact` | blur, scale, color, opacity |
| `curved` | edge, curve-depth, blur, color, opacity |
| `long` | angle, length, fade, color, opacity |
| `perspective` | light-x, light-y, light-z, blur, color, opacity |
| `inner` | offset-x, offset-y, blur, color, opacity |

## 5. Build Commands

```bash
# Build CLI
cd cli && cargo build --release

# Run tests
cd cli && cargo test

# Run selftest
cli/target/debug/shadow-cli selftest

# Generate a shadow
shadow-cli generate --input image.png --output out.png --type drop
```

## 6. 4D Component Rules

- Use `cs.Generator` internally, `cs.Shadow.Generator` for host.
- Validate all options before processing.
- Reject embedded NUL in arguments.
- Never use `cmd /c`, `bash -c`, or unquoted paths.
- `dataType:"text"`, `encoding:"UTF-8"`, `hideWindow:true`, `timeout:30`.
- Close stdin with `.closeInput()` before `.wait()`.

## 7. Development Rules

- All output must be deterministic for the same input + params.
- Never panic on user input — return structured errors.
- Every public function must have a unit test.
- Respect the `cli-contract.json` vectors — they are the source of truth.
- Build all targets from the same commit/SHA.
- Local acceptance is always `result:partial, release_ready:false`.

## 8. File Layout

```
shadow/
├── cli/              ← Rust CLI source
├── component/shadow/ ← 4D component
├── scripts/          ← Build/test scripts
├── tests/            ← Test vectors + fixtures
├── AGENTS.md         ← This file
├── PROJECT_CHARTER.md
├── template.json
├── package.json
└── workflow-contract.json
```
