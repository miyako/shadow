# Shadow CLI — Project Charter & Specification

> **Status:** Draft for review by agents and supervisors
> **Date:** 2026-11-19
> **Repository (proposed):** `miyako/shadow`
> **License:** MIT
> **Language:** Rust (1.99+)
> **Architecture:** CLI-backed 4D component (derived from `miyako/4d-component-template`)

---

## 1. Executive Summary

### 1.1 Objective

Build a **portable, self-contained CLI utility** that accepts an image (file path or stdin data stream), generates a **shadow image** behind it, and outputs a composited result (or a standalone shadow layer). The CLI will be wrapped by a **4D component** so that host 4D applications can invoke it through a structured class API via `4D.SystemWorker`.

### 1.2 Scope

| In Scope | Out of Scope |
|----------|-------------|
| Rust CLI binary (`shadow-cli`) | Native 4D Plug-in SDK integration |
| 4D component wrapper (`Shadow.Generator` class) | 4D-side raster manipulation |
| Shadow types: drop, contact, curved, long, perspective, inner | Real-time GPU rendering |
| Parameters: opacity, offset (x/y), blur (σ), color, scale, angle | Animated shadows / video output |
| Input: PNG, JPEG, WebP, BMP, GIF (RGBA) | Vector-only SVG input parsing |
| Output: PNG (primary), optional shadow-only layer | On-the-fly network streaming |
| macOS universal (arm64 + x86_64), Windows x64 | Linux builds (future) |
| stdin binary streaming support | Interactive/TUI mode |

### 1.3 Reference Implementations

| Reference | Role | Key Takeaway |
|-----------|------|--------------|
| [`miyako/4d-component-template`](https://github.com/miyako/4d-component-template) | Project scaffold & workflow contract | CLI-backed component architecture; THINK/ASK→RUN paradigm; tool4d compilation; asset-lock provenance; AGENTS.md playbook |
| [`miyako/quickdraw`](https://github.com/miyako/quickdraw) | Functional precedent | Same architecture wrapping a Rust CLI (`quickdraw-cli`); asset-lock.json pinning; synchronous 30s timeout; structured JSON protocol |

---

## 2. Technical Background

### 2.1 Shadow Effect Taxonomy & Algorithms

Each shadow type is defined by its algorithmic pipeline. All pipelines share a common first step: **alpha-channel extraction** to obtain the object silhouette.

#### 2.1.1 Alpha Channel Extraction (Common Foundation)

For an RGBA image with pixel coordinates \((x, y)\):

1. **Isolation**: Extract the 2D matrix of \(\alpha\) values, ignoring RGB.
2. **Optional thresholding**: Apply binary threshold \(f(\alpha) = 1\) if \(\alpha > 128\), else \(0\).
3. **Stencil**: The \(\alpha\) matrix serves as a mask for the shadow color: \(C_{final} = C_{shadow} \times \alpha\).

#### 2.1.2 Drop Shadow

The standard elevation shadow. Pipeline:

1. **Silhouette** → extract alpha mask
2. **Color flood** → fill silhouette with shadow color at specified opacity
3. **Offset** → translate by \((t_x, t_y)\): \(S_{shadow}(x,y) = S(x - t_x, y - t_y)\)
4. **Blur** → Gaussian convolution: \(S_{blurred} = S_{shadow} * G(x,y)\) where \(G(x,y) = \frac{1}{2\pi\sigma^2} e^{-\frac{x^2+y^2}{2\sigma^2}}\)
5. **Composite** → place shadow layer beneath original image

**Parameters**: `offset-x`, `offset-y`, `blur` (σ), `color`, `opacity`

#### 2.1.3 Contact Shadow

Simulates ambient occlusion — object resting on a surface. Characterized by:

- **Zero or minimal offset** (\(t_x \approx 0, t_y \approx 0\))
- **Tight blur** (small σ relative to object size)
- Often slightly **dilated** before blurring to create a wider base

Pipeline: identical to drop shadow but with constrained parameters and optional dilation step.

#### 2.1.4 Curved Shadow

Creates the illusion of an object lifting/curling away from a surface at one edge. Uses **Bézier curves**:

1. Define a path along the lifting edge of the object
2. Draw a quadratic Bézier: \(B(t) = (1-t)^2P_0 + 2(1-t)tP_1 + t^2P_2\)
3. Fill the area beneath the curve with a **gradient** fading from opaque shadow color to transparent
4. Optionally blur the gradient edges

**Parameters**: `edge` (top/bottom/left/right), `curve-depth`, `blur`, `color`, `opacity`

#### 2.1.5 Long Shadow

Extends dramatically at a fixed angle (typically 45°). Algorithm:

1. For each boundary pixel of the silhouette, trace a ray at angle \(\theta\)
2. Color every pixel along the ray until canvas boundary is hit
3. Apply **opacity gradient**: fade from full opacity at the object edge to zero at the ray terminus

**Parameters**: `angle` (degrees), `length` (or `auto` for canvas-bound), `color`, `opacity`, `fade` (gradient mode: linear/exponential/none)

#### 2.1.6 Perspective Shadow

Projects the silhouette onto a theoretical ground plane using **homography**:

1. Treat the object as a plane at height \(z = h\)
2. Define light source at \((L_x, L_y, L_z)\)
3. For each vertex \((x, y)\): \(x' = x - h\frac{x - L_x}{h - L_z}\), \(y' = y - h\frac{y - L_y}{h - L_z}\)
4. Rasterize the distorted polygon and blur

**Parameters**: `light-x`, `light-y`, `light-z` (or `elevation`), `vanishing-point`, `blur`, `color`, `opacity`

#### 2.1.7 Inner Shadow (Inset)

Draws shadow inside the element's boundary — inversion of drop shadow:

1. Extract alpha mask
2. **Invert** the mask
3. **Offset** the inverted mask
4. **Intersect** with original mask (bitwise AND / alpha multiply)
5. **Blur** the intersection
6. **Composite** on top of the original (darkening)

**Parameters**: `offset-x`, `offset-y`, `blur`, `color`, `opacity`

### 2.2 SVG Filter Specification Reference

The SVG Filter Effects Module Level 1 defines shadow primitives that serve as a conceptual spec baseline:

| Primitive | Function | Maps to Parameter |
|-----------|----------|-------------------|
| `feDropShadow` | Convenience wrapper (blur + offset + flood + composite + merge) | All-in-one |
| `feOffset` | Shifts input by (dx, dy) | offset-x, offset-y |
| `feGaussianBlur` | Gaussian convolution with stdDeviation | blur (σ) |
| `feFlood` | Fills region with solid color + opacity | color, opacity |
| `feComposite` | Masking/compositing operations | silhouette masking |
| `feMerge` | Layer stacking | compositing order |

> **Design note**: The CLI implements these concepts in raster space (pixel buffer operations), not SVG XML. However, the parameter model mirrors SVG semantics for familiarity.

### 2.3 Rust Image Processing Ecosystem

| Crate | Version | Role in Project |
|-------|---------|-----------------|
| `image` | latest | Core decode/encode: PNG, JPEG, WebP, BMP, GIF. `ImageBuffer<Rgba>` as primary pixel container |
| `imageproc` | latest | Drawing primitives, morphological ops (dilate/erode), spatial filters, geometric transforms |
| `fastblur` or `gaussian-blur` | latest | Optimized Gaussian blur (box-blur approximation or separable kernel) |
| `clap` | 4.x | CLI argument parsing with derive macros, subcommands, help generation |
| `serde` + `serde_json` | 1.x | JSON protocol serialization/deserialization for 4D communication |
| `resvg` / `usvg` | latest | (Optional future) SVG filter rendering for curved/perspective shadows |

### 2.4 4D SystemWorker Integration Contract

Based on the 4D SystemWorker class documentation and the template's AGENTS.md:

- **Invocation**: `4D.SystemWorker.new(commandLine: Text; options: Object)` — accepts command-line Text, NOT an argv Collection
- **Arguments**: Safely quoted by the `_Process` adapter class; no shell interpretation (`cmd /c`, `bash -c` forbidden)
- **stdin**: Binary blob input via `.postMessage(blob)` then `.closeInput()`
- **stdout/stderr**: Captured via `.response` (Text) and `.responseError` (Text)
- **Timeout**: 30 seconds default; synchronous execution
- **exitCode**: Integer from the CLI process
- **dataType**: `"blob"` for binary image input via stdin; `"text"` for JSON protocol responses
- **Encoding**: UTF-8 for all text I/O

---

## 3. Architecture Design

### 3.1 System Overview

```
┌─────────────────────────────────────────────────────────┐
│                    4D Host Application                   │
│                                                         │
│  cs.Shadow.Generator.new()                              │
│    │                                                    │
│    ├─ .version()        → JSON {protocol, version}      │
│    ├─ .capabilities()   → JSON {types[], params[]}      │
│    ├─ .generate(input; output; options) → shadow image  │
│    └─ .shadowOnly(input; output; options) → shadow only │
│                                                         │
│    ┌──────────────────────────────────┐                 │
│    │       _Process (4D Class)        │                 │
│    │  Quote args → SystemWorker       │                 │
│    │  stdin (blob) → closeInput       │                 │
│    │  wait() → response/exitCode      │                 │
│    └──────────┬───────────────────────┘                 │
│               │                                          │
│    ┌──────────▼───────────────────────┐                 │
│    │       _CLI (4D Class)            │                 │
│    │  Resolve /RESOURCES/bin path     │                 │
│    │  macOS-universal / Windows-x64   │                 │
│    └──────────┬───────────────────────┘                 │
└───────────────┼──────────────────────────────────────────┘
                │
                │  (CLI process, separate)
                │
┌───────────────▼──────────────────────────────────────────┐
│                  shadow-cli (Rust binary)                 │
│                                                          │
│  clap argument parsing                                   │
│    │                                                     │
│    ├─ load image (file path OR stdin blob)              │
│    ├─ extract alpha channel → silhouette                │
│    ├─ shadow type dispatch:                             │
│    │    ├─ drop    → offset + gaussian blur             │
│    │    ├─ contact → dilate + tight blur                │
│    │    ├─ curved  → bézier path + gradient fill        │
│    │    ├─ long    → ray trace + opacity gradient        │
│    │    ├─ perspective → homography project + blur      │
│    │    └─ inner   → invert + offset + intersect + blur │
│    ├─ composite (shadow beneath original)               │
│    └─ encode output → file or stdout                    │
│                                                          │
│  JSON protocol on stdout (one line, LF-terminated)       │
│  Errors on stderr as JSON                                │
└──────────────────────────────────────────────────────────┘
```

### 3.2 CLI Contract

#### 3.2.1 Protocol

- **Version**: 1
- **Format**: Exactly one JSON line, LF-terminated, on stdout for protocol responses
- **Image data**: Never embedded in JSON; always file path or raw binary on stdin/stdout
- **Error format**: JSON on stderr with exit code 2

#### 3.2.2 Commands

```
shadow-cli --version
  → {"protocol":1,"version":"0.1.0"}

shadow-cli capabilities
  → {"protocol":1,"capabilities":{"types":["drop","contact","curved","long","perspective","inner"],"params":["opacity","offset-x","offset-y","blur","color","scale","angle","edge","curve-depth","fade","light-x","light-y","light-z"]}}

shadow-cli generate --input <path|-> --output <path|-> --type <type> [options...]
  → {"protocol":1,"ok":true,"output":"/path/to/output.png","width":W,"height":H}

shadow-cli shadow-only --input <path|-> --output <path|-> --type <type> [options...]
  → {"protocol":1,"ok":true,"output":"/path/to/shadow.png","width":W,"height":H}
```

#### 3.2.3 Input Modes

| Mode | `--input` value | Data flow |
|------|----------------|-----------|
| File path | `/path/to/image.png` | CLI reads file directly |
| stdin | `-` | CLI reads binary blob from stdin until EOF |

#### 3.2.4 Output Modes

| Mode | `--output` value | Data flow |
|------|-----------------|-----------|
| File path | `/path/to/output.png` | CLI writes file directly |
| stdout | `-` | CLI writes binary blob to stdout; JSON protocol suppressed to stderr |

#### 3.2.5 Options Matrix

| Option | Type | Default | Applies to shadow types |
|--------|------|---------|------------------------|
| `--type` | enum | `drop` | all |
| `--opacity` | float 0.0–1.0 | 0.5 | all |
| `--offset-x` | int (px) | 5 | drop, inner |
| `--offset-y` | int (px) | 5 | drop, inner |
| `--blur` | float (σ px) | 10.0 | drop, contact, inner, perspective |
| `--color` | hex string | `#000000` | all |
| `--scale` | float | 1.0 | drop, contact |
| `--angle` | int (deg) | 45 | long |
| `--length` | int (px) or `auto` | `auto` | long |
| `--fade` | enum: linear/exp/none | `linear` | long |
| `--edge` | enum: top/bottom/left/right | `bottom` | curved |
| `--curve-depth` | float (px) | 20.0 | curved |
| `--light-x` | float | 0.0 | perspective |
| `--light-y` | float | 0.0 | perspective |
| `--light-z` | float (elevation) | 100.0 | perspective |
| `--canvas-expand` | int (px) | 20 | all (padding around output) |
| `--format` | enum: png/jpeg/webp | `png` | output format override |
| `--background` | hex string or `transparent` | `transparent` | composite background |
| `--shadow-only` | flag | false | output only shadow layer |

### 3.3 4D Component Architecture

#### 3.3.1 Class Structure

| Class | Visibility | Responsibility |
|-------|-----------|----------------|
| `_CLI` | internal | Resolve `/RESOURCES/bin/shadow-cli` path; platform dispatch |
| `_Process` | internal | Argument quoting; SystemWorker execution; stdin/stdout; error capture |
| `Generator` | public | Feature API: `version()`, `capabilities()`, `generate()`, `shadowOnly()` |

#### 3.3.2 Host API Example

```4d
// Create generator
var $shadow : cs.Shadow.Generator
$shadow := cs.Shadow.Generator.new()

// Generate a drop shadow
var $result : Object
$result := $shadow.generate( \
  File("/DATA/icon.png"); \
  File("/DATA/icon_shadowed.png"); \
  New object( \
    "type"; "drop"; \
    "opacity"; 0.5; \
    "offset-x"; 5; \
    "offset-y"; 5; \
    "blur"; 10.0; \
    "color"; "#000000" ))

If ($result.ok)
  // $result.value contains {width, height, outputPath}
Else
  ALERT($result.error.code + ": " + $result.error.message)
End if
```

#### 3.3.3 stdin Streaming Mode (for large images)

```4d
var $blob : Blob
$blob := File("/DATA/large.png").getContent()
$result := $shadow.generate( \
  $blob; \
  File("/DATA/large_shadowed.png"); \
  New object("type"; "contact"; "blur"; 5.0))
```

### 3.4 Error Model

| Exit Code | Condition | stderr JSON |
|-----------|-----------|-------------|
| 0 | Success | (empty) |
| 1 | Runtime error (decode/encode/processing) | `{"protocol":1,"error":{"code":"E_RUNTIME","message":"..."}}` |
| 2 | Invalid arguments / contract violation | `{"protocol":1,"error":{"code":"E_ARGS","message":"..."}}` |
| 3 | I/O error (file not found, permission) | `{"protocol":1,"error":{"code":"E_IO","message":"..."}}` |

---

## 4. Project Roadmap & Milestones

### Milestone 0: Project Initialization (M0)

**Goal**: Scaffold the repository from `4d-component-template` with correct identity.

| Task | Owner | Deliverable |
|------|-------|-------------|
| Initialize template with `--repository miyako/shadow --namespace Shadow --cli shadow-cli --language rust --feature "Image shadow generator"` | Agent-CLI | Initialized repo with correct identity |
| Verify `doctor.py --root .` passes | Agent-CLI | JSON report |
| Define `template.json`, `package.json` metadata | Agent-CLI | Committed config |
| Create AGENTS.md adapted for shadow project | Supervisor | Reviewed playbook |

**Exit criteria**: `initialize.py` completes; all identity surfaces match `shadow`/`Shadow`/`shadow-cli`.

---

### Milestone 1: CLI Core — Alpha Extraction & Drop Shadow (M1)

**Goal**: Minimal working CLI that produces a drop shadow.

| Task | Owner | Deliverable |
|------|-------|-------------|
| Set up Rust workspace: `Cargo.toml` with `image`, `imageproc`, `clap`, `serde`, `serde_json` | Agent-CLI | Compiling workspace |
| Implement `--version` and `capabilities` JSON protocol responses | Agent-CLI | CLI contract test vectors |
| Implement image loading (file path + stdin blob) for PNG/JPEG | Agent-CLI | Load + decode pipeline |
| Implement alpha channel extraction → silhouette mask | Agent-CLI | Mask extraction module |
| Implement Gaussian blur (via `fastblur` or separable kernel) | Agent-CLI | Blurred shadow module |
| Implement drop shadow: offset + blur + color + opacity + composite | Agent-CLI | Working `generate --type drop` |
| Implement output encoding (file + stdout) | Agent-CLI | PNG output verified |
| Write `cli-contract.json` test vectors for `--version`, `capabilities`, `generate drop` | Agent-CLI | Passing tests |

**Exit criteria**: `shadow-cli generate --input test.png --output out.png --type drop` produces a correct drop shadow; CLI contract tests pass.

---

### Milestone 2: Extended Shadow Types (M2)

**Goal**: Implement all six shadow types.

| Task | Owner | Deliverable |
|------|-------|-------------|
| Contact shadow: dilate + tight blur pipeline | Agent-CLI | `--type contact` working |
| Long shadow: ray trace + opacity gradient | Agent-CLI | `--type long` working |
| Inner shadow: invert + offset + intersect + blur | Agent-CLI | `--type inner` working |
| Curved shadow: Bézier path + gradient fill | Agent-CLI | `--type curved` working |
| Perspective shadow: homography projection + blur | Agent-CLI | `--type perspective` working |
| `--shadow-only` flag: output just the shadow layer | Agent-CLI | Shadow-only output verified |
| `--canvas-expand`: pad output canvas | Agent-CLI | Padded output verified |
| Extended CLI contract vectors for all types | Agent-CLI | All vectors pass |

**Exit criteria**: All six shadow types produce visually correct output; extended contract tests pass.

---

### Milestone 3: Advanced Features & Robustness (M3)

**Goal**: Production-grade input handling and format support.

| Task | Owner | Deliverable |
|------|-------|-------------|
| WebP, BMP, GIF decode support | Agent-CLI | Multi-format input tests |
| Output format selection (`--format png/jpeg/webp`) | Agent-CLI | Format override verified |
| Background color option (`--background`) | Agent-CLI | Background compositing |
| Large image handling (memory-efficient streaming for stdin) | Agent-CLI | 50MB+ image test |
| Error handling: all exit codes and JSON error structures | Agent-CLI | Error vectors pass |
| CLI self-test: `shadow-cli selftest` runs internal assertions | Agent-CLI | Selftest command |

**Exit criteria**: All supported formats round-trip; error handling is deterministic and tested.

---

### Milestone 4: 4D Component Wrapper (M4)

**Goal**: Replace template `Greeting` class with `Generator` class; functional host API.

| Task | Owner | Deliverable |
|------|-------|-------------|
| Implement `Generator.4dm`: constructor, `version()`, `capabilities()` | Agent-4D | Class file |
| Implement `Generator.generate(input; output; options)` | Agent-4D | File-path mode working |
| Implement stdin streaming variant (blob input) | Agent-4D | Blob mode working |
| Implement `Generator.shadowOnly(input; output; options)` | Agent-4D | Shadow-only API |
| Update `_CLI.4dm` for `shadow-cli` binary resolution | Agent-4D | Path resolution |
| Update `_Process.4dm` for blob stdin + JSON protocol | Agent-4D | Process execution |
| Update `Documentation/Classes/Generator.md` | Agent-4D | API docs |
| Update `tests/host/Smoke.4dm` with real namespace calls | Agent-4D | Smoke tests |
| Update `tests/host/contract.json` with feature cases | Agent-4D | Contract registry |

**Exit criteria**: Source host test passes — `cs.Shadow.Generator.new().generate(...)` returns `{ok:true, value:{...}}`.

---

### Milestone 5: Build, Compile & Package (M5)

**Goal**: Cross-platform build; tool4d compilation; packaged `.4dbase`.

| Task | Owner | Deliverable |
|------|-------|-------------|
| Build CLI: `macos-arm64`, `macos-x64`, `windows-x64` | Agent-Build | Three binaries + manifests |
| `lipo` universal macOS binary | Agent-Build | Universal binary verified |
| Assemble resources: `ci_config.py assemble` | Agent-Build | Resources assembled |
| tool4d compilation: `compile_component.py` | Agent-Build | Compiled component |
| Package: `package_component.py` | Agent-Build | `.4DZ` + `.4dbase` |
| Compiled host test: unpacked `.4dbase` runs | Agent-Build | `Is compiled mode=True` |
| `dependencies.json` present in test host | Agent-Build | No modal dialog blocker |

**Exit criteria**: `accept_local.py` reports `result:partial, finished:true`; compiled host test passes.

---

### Milestone 6: Release & Signing (M6)

**Goal**: Tagged GitHub release with signed macOS binary.

| Task | Owner | Deliverable |
|------|-------|-------------|
| `release-policy.json` with Apple notarization config | Supervisor | Policy file |
| Configure 7 repository secrets for signing | Supervisor (user-owned) | Secrets set |
| `bump-version.yml`: synchronize version to `0.1.0` | Agent-Build | Version commit |
| `build.yml`: full release pipeline from immutable SHA | Agent-Build | Release artifacts |
| Apple Developer ID signing + notarization + stapling | Agent-Build | Signed universal binary |
| `release_assets.py` verification | Agent-Build | Assets validated |
| `gh release create --verify-tag 0.1.0` | Agent-Build | Published release |

**Exit criteria**: GitHub release `0.1.0` published with signed macOS universal binary and Windows x64 binary; dependency manager install verified.

---

## 5. Team Structure & Responsibilities

### 5.1 Roles

| Role | Assignee | Responsibilities |
|------|----------|-----------------|
| **Supervisor** | (human) | Contract decisions; credential setup; release policy; final approval |
| **Agent-CLI** | AI agent | Rust CLI implementation; CLI contract tests; binary builds |
| **Agent-4D** | AI agent | 4D class implementation; host tests; documentation |
| **Agent-Build** | AI agent | Cross-platform builds; tool4d compilation; packaging; CI |
| **Agent-QA** | AI agent | Visual correctness verification; regression tests |

### 5.2 RACI Matrix

| Task | Agent-CLI | Agent-4D | Agent-Build | Agent-QA | Supervisor |
|------|-----------|----------|-------------|----------|------------|
| CLI implementation | **R** | C | I | C | A |
| 4D wrapper | I | **R** | C | C | A |
| Build pipeline | C | I | **R** | I | A |
| Test vectors | **R** | **R** | I | C | I |
| Visual QA | I | I | I | **R** | A |
| Release | I | I | **R** | I | **A** |
| Signing credentials | I | I | I | I | **R** |

*R=Responsible, A=Accountable, C=Consulted, I=Informed*

---

## 6. Testing Strategy

### 6.1 CLI Contract Tests (`tests/cli-contract.json`)

Deterministic JSON vectors testing exact stdout/exitCode for:
- `--version` → exact version string
- `capabilities` → schema-validated JSON
- `generate --type drop` with fixed input → exact output hash
- `generate --type <each>` → exact output hash
- Invalid args → exit code 2, structured error
- Missing input file → exit code 3
- stdin input → exact output hash

### 6.2 Visual Regression Tests

For each shadow type, a reference input image and expected output are committed:
- `tests/fixtures/input_alpha_128.png` (128×128, semi-transparent circle)
- `tests/fixtures/expected/drop_default.png`
- `tests/fixtures/expected/contact_default.png`
- `tests/fixtures/expected/curved_bottom.png`
- etc.

CLI output is compared pixel-by-pixel (with small tolerance for blur edge anti-aliasing).

### 6.3 4D Host Tests (`tests/host/`)

- `Smoke.4dm`: Real namespace call to `cs.Shadow.Generator.new().generate(...)`
- `contract.json`: Feature cases with exact JSON assertion on decoded `value`
- Negative cases: missing options, invalid type, NUL in arguments

### 6.4 Platform Tests

| Platform | Test | Requirement |
|----------|------|-------------|
| macOS arm64 | Native build + run | All CLI tests pass |
| macOS x64 | `lipo` universal + run | All CLI tests pass |
| Windows x64 | Native build + run | All CLI tests pass |
| macOS (4D) | Source host test | `cs.Shadow.Generator` loads and executes |
| macOS (4D) | Compiled host test | `Is compiled mode=True` |

---

## 7. Dependencies & Provenance

### 7.1 Rust Dependencies

| Crate | License | Purpose | Redistribution Notice |
|-------|---------|---------|----------------------|
| `image` | MIT | Image decode/encode | MIT notice |
| `imageproc` | MIT | Image processing ops | MIT notice |
| `fastblur` | MIT | Gaussian blur | MIT notice |
| `clap` | MIT/Apache-2.0 | CLI parsing | Dual notice |
| `serde` | MIT/Apache-2.0 | Serialization | Dual notice |
| `serde_json` | MIT/Apache-2.0 | JSON | Dual notice |

### 7.2 Build Dependencies

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.99+ (CI lock: latest stable) | CLI compilation |
| Python | 3.10+ | Build/test scripts |
| tool4d | 20.7 build 101905 | 4D compilation (free, headless) |
| `lipo` | (macOS system) | Universal binary creation |

### 7.3 Asset Lock (`cli/asset-lock.json`)

Following the `quickdraw` pattern, the 4D component repository will pin the CLI release:
```json
{
  "schema": 1,
  "source": {
    "repository": "miyako/shadow",
    "tag": "0.1.0",
    "commit": "<SHA>"
  },
  "cli_version": "0.1.0",
  "targets": {
    "macos-universal": {
      "url": "https://github.com/miyako/shadow/releases/download/0.1.0/shadow-cli-macos-universal.zip",
      "zip_sha256": "...",
      "exe_sha256": "..."
    },
    "windows-x64": {
      "url": "https://github.com/miyako/shadow/releases/download/0.1.0/shadow-cli-windows-x64.zip",
      "zip_sha256": "...",
      "exe_sha256": "..."
    }
  }
}
```

---

## 8. Feasibility Assessment

### 8.1 Technical Feasibility: HIGH

| Factor | Assessment | Evidence |
|--------|------------|---------|
| Rust image processing ecosystem | Mature | `image` crate is the de facto standard; `imageproc` provides drawing + morphology; `fastblur` provides optimized blur |
| Alpha-channel silhouette extraction | Trivial | Standard RGBA pixel manipulation; well-documented algorithm |
| Gaussian blur | Solved | Separable kernel or box-blur approximation; `fastblur` crate provides O(n) implementation |
| Bezier curves (curved shadow) | Moderate | `imageproc` has drawing primitives; quadratic bezier path + gradient fill is straightforward |
| Homography (perspective shadow) | Moderate | Requires 2D→2D projective transform; can be implemented with matrix math + `imageproc::geometric_transformations` |
| Cross-platform builds | Proven | Template already builds Go/Rust/C++ for macOS universal + Windows x64 |
| 4D SystemWorker integration | Proven | `quickdraw` demonstrates the exact same pattern with a Rust CLI |

### 8.2 Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Large image memory consumption | Medium | Medium | Stream decode where possible; document max image size; 30s timeout |
| Blur performance on very large images | Medium | Low | Use `fastblur` (O(n)); benchmark with 50MP images; document limits |
| Perspective shadow geometric correctness | Low | Medium | Unit test with known geometric inputs; visual QA |
| 4D blob stdin for large images | Low | Medium | File-path mode as primary; stdin as fallback; test with real 4D |
| macOS notarization failures | Low | High | Follow template's proven `sign_release.py` pipeline; test early |
| Windows path/encoding edge cases | Medium | Medium | Strict UTF-8 everywhere; quote all paths; test with non-ASCII paths |

### 8.3 Architecture Decision: Rust over Go/C++

| Criterion | Rust | Go | C++ |
|-----------|------|----|-----|
| Image processing ecosystem | **Excellent** (`image`, `imageproc`, `fastblur`) | Limited (`gg`, `gift`) | Excellent (OpenCV, Skia) but heavy |
| Memory safety | **Guaranteed** | Garbage collected | Manual |
| Binary size | Small (~2-5MB) | Medium (~10MB+) | Varies |
| Cross-compilation | Good (cargo + cross) | **Excellent** (native cross-compile) | Complex (CMake toolchains) |
| Performance | **Top-tier** (zero-cost abstractions) | Good (GC overhead) | **Top-tier** |
| Template precedent | **Yes** (`quickdraw` uses Rust) | Yes (template supports) | Yes (template supports) |
| Learning curve | Steep but manageable | Gentle | Steep |

**Decision: Rust** — best combination of ecosystem maturity, performance, memory safety, binary size, and precedent with `quickdraw`.

---

## 9. File Structure (Target Repository)

```
shadow/
├── .github/
│   ├── skills/shadow-cli/SKILL.md
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── bump-version.yml
│   │   └── build.yml
│   └── copilot-instructions.md
├── AGENTS.md
├── assets/
│   ├── logo.png
│   └── logo.svg
├── cli/
│   ├── asset-lock.json
│   └── starters/rust/           ← Rust CLI source (or moved to cli/src/)
│       ├── Cargo.toml
│       ├── Cargo.lock
│       └── src/
│           ├── main.rs           ← clap + protocol dispatch
│           ├── alpha.rs          ← alpha extraction / silhouette
│           ├── blur.rs           ← Gaussian blur wrapper
│           ├── shadow/
│           │   ├── mod.rs
│           │   ├── drop.rs       ← drop shadow pipeline
│           │   ├── contact.rs    ← contact shadow (dilate + blur)
│           │   ├── curved.rs     ← Bézier + gradient
│           │   ├── long.rs       ← ray trace + fade
│           │   ├── perspective.rs ← homography projection
│           │   └── inner.rs      ← inset shadow
│           ├── composite.rs      ← layer compositing
│           └── protocol.rs       ← JSON protocol I/O
├── component/shadow/
│   ├── Project/
│   │   ├── shadow.4DProject
│   │   ├── Sources/
│   │   │   ├── Classes/
│   │   │   │   ├── _CLI.4dm
│   │   │   │   ├── _Process.4dm
│   │   │   │   └── Generator.4dm
│   │   │   └── Methods/
│   │   │       └── _CaptureError.4dm
│   │   └── Sources/dependencies.json
│   ├── Documentation/Classes/Generator.md
│   └── Resources/
│       ├── bin/
│       │   ├── shadow-cli              ← macOS universal
│       │   └── shadow-cli.exe          ← Windows x64
│       ├── logo.png
│       ├── logo.svg
│       ├── licenses/
│       └── en.lproj/syntaxEN.json
├── scripts/
│   ├── initialize.py
│   ├── build_cli.py
│   ├── compile_component.py
│   ├── package_component.py
│   ├── test_4d.py
│   ├── accept_local.py
│   ├── sign_release.py
│   ├── generate_icons.py
│   └── ci_config.py
├── tests/
│   ├── cli-contract.json
│   ├── test_cli.py
│   ├── test_binding_runtime.py
│   ├── fixtures/
│   │   ├── input_alpha_128.png
│   │   ├── input_photo.jpg
│   │   └── expected/
│   │       ├── drop_default.png
│   │       ├── contact_default.png
│   │       ├── curved_bottom.png
│   │       ├── long_45deg.png
│   │       ├── perspective_default.png
│   │       └── inner_default.png
│   └── host/
│       ├── contract.json
│       └── Project/Sources/Methods/Smoke.4dm
├── tooling/
│   └── compiler/
├── package.json
├── template.json
├── workflow-contract.json
├── release-policy.json
└── README.md
```

---

## 10. Agent Work Instructions

### 10.1 For Agent-CLI (Rust Developer)

**Starting point**: `cli/starters/rust/` after `initialize.py` runs with `--language rust`.

**Priority order**:
1. Get `--version` and `capabilities` working first — establish the JSON protocol
2. Implement image loading (PNG only initially) + alpha extraction
3. Implement Gaussian blur as a standalone module
4. Implement drop shadow as the first complete pipeline
5. Add remaining shadow types one at a time, with visual verification
6. Add stdin/stdout streaming
7. Add remaining format support

**Rules**:
- Every public function must have a unit test
- All output must be deterministic for the same input + params (no random seeds)
- Never panic on user input — return structured errors
- Binary must be statically linked where possible (no runtime DLL dependencies on Windows)
- Respect the `cli-contract.json` vectors — they are the source of truth

### 10.2 For Agent-4D (4D Component Developer)

**Starting point**: `component/shadow/Project/Sources/Classes/Greeting.4dm` (to be replaced with `Generator.4dm`).

**Priority order**:
1. Replace `Greeting.4dm` with `Generator.4dm` using `rename_feature.py`
2. Implement `version()` and `capabilities()` first
3. Implement `generate(input; output; options)` with file-path mode
4. Add blob stdin variant
5. Add `shadowOnly()` method
6. Update `_CLI.4dm` for `shadow-cli` binary name
7. Update `_Process.4dm` for blob stdin support
8. Write `Documentation/Classes/Generator.md`
9. Write `tests/host/Smoke.4dm` and `contract.json`

**Rules**:
- Use `cs.Generator` internally, `cs.Shadow.Generator` for host
- Validate all options with `Value type` + `OB Instance of` before processing
- Reject embedded NUL with strict `Position(Char(0); $argument; *)>0`
- Never use `cmd /c`, `bash -c`, or unquoted paths
- `dataType:"text"`, `encoding:"UTF-8"`, `hideWindow:true`, `timeout:30`
- Close stdin with `.closeInput()` before `.wait()`
- Separate `.response`, `.responseError`, `.exitCode`, `.errors`

### 10.3 For Agent-Build (CI/Release Engineer)

**Starting point**: Template scripts in `scripts/`.

**Priority order**:
1. Verify `doctor.py --root .` passes locally
2. Build CLI for native target first (`macos-arm64`)
3. Build all three targets; create universal macOS binary with `lipo`
4. Assemble resources; run `ci_config.py assemble`
5. Download tool4d; compile component
6. Package; test unpacked compiled package
7. Set up GitHub Actions workflows
8. Configure signing (with supervisor)

**Rules**:
- Build all targets from the same commit/SHA
- Every binary gets a `build-manifest.json` with SHA256
- `lipo -verify_arch x86_64 arm64` (order matters)
- ZIP must have explicit nested directory records
- `dependencies.json` must exist in test host project
- Local acceptance is always `result:partial, release_ready:false`
- Never bypass compilation — zero exit without JSON is not success

### 10.4 For Agent-QA (Quality Assurance)

**Starting point**: `tests/fixtures/` and `tests/cli-contract.json`.

**Priority order**:
1. Create test fixture images (known geometry, known alpha)
2. Generate expected output for each shadow type
3. Run CLI contract tests; verify exact JSON responses
4. Run visual regression: pixel-diff output against expected
5. Test edge cases: fully opaque image, fully transparent image, 1×1 pixel, very large image
6. Test 4D host smoke tests

**Rules**:
- Use deterministic fixtures (procedurally generated, not photographs)
- Allow tolerance of ±2 per channel for blur anti-aliasing
- Document any non-deterministic behavior
- Verify all six shadow types produce distinct visual results

---

## 11. Workflow Contract Summary

### 11.1 Development Flow

```
THINK/ASK → Contract decisions → RUN → Implement → Test → Verify → Accept
```

### 11.2 CI Workflow Triggers

| Workflow | Trigger | Publishing? |
|----------|---------|-------------|
| `ci.yml` | Branch push / PR | No |
| `bump-version.yml` | Manual dispatch | No (commit only) |
| `build.yml` | Manual on `main` | Yes (tag + release) |

### 11.3 Acceptance Gate

Before any release:

- [ ] `python3 -m unittest discover -s tests -v` passes
- [ ] `generate_icons.py --check` passes
- [ ] CLI builds for `macos-arm64`, `macos-x64`, `windows-x64`
- [ ] `lipo` universal binary verified
- [ ] Resources assembled
- [ ] tool4d compilation: `Compile project` returns fresh success JSON
- [ ] `package_component.py` creates deterministic ZIP
- [ ] Source host test passes (real namespace call)
- [ ] Compiled host test passes (`Is compiled mode=True`)
- [ ] `workflow_contract.py` passes
- [ ] (Optional) macOS binary signed + notarized
- [ ] `release_assets.py` validation passes

---

## 12. Open Questions for Supervisor

| # | Question | Impact | Default if Unanswered |
|---|----------|--------|----------------------|
| 1 | Should the CLI also support SVG output (vector shadows with filter elements)? | Scope expansion | No — raster only for v0.1 |
| 2 | Is a GUI preview mode needed (e.g., `--preview` opens a window)? | Scope expansion | No — CLI only |
| 3 | Should batch processing (multiple images in one invocation) be supported? | API design | No — one image per call, matching template's synchronous model |
| 4 | Are there specific image format priorities beyond PNG? | M3 priorities | PNG primary; JPEG/WebP/BMP/GIF secondary |
| 5 | What is the maximum expected image dimension? | Memory planning | 8K (7680×4320) as documented ceiling; larger via stdin streaming |
| 6 | Should the component support async execution (CALL WORKER pattern)? | API design | No — synchronous 30s timeout per template convention |
| 7 | Is Apple notarization required for v0.1.0? | Release pipeline | Yes — include in release-policy.json |
| 8 | Should Windows Authenticode signing be implemented? | Release pipeline | No — documented limitation; future enhancement |

---

## 13. Glossary

| Term | Definition |
|------|-----------|
| **CLI-backed component** | 4D component that wraps a native CLI executable launched via `SystemWorker` |
| **Alpha channel** | The transparency component of an RGBA pixel (0=transparent, 255=opaque) |
| **Silhouette** | The 2D shape defined by an image's alpha channel |
| **Gaussian blur** | Convolution with a Gaussian kernel; controlled by standard deviation σ |
| **Homography** | A projective transformation mapping between two planes |
| **Bézier curve** | Parametric curve defined by control points; used for curved shadows |
| **tool4d** | 4D's free standalone headless compiler tool |
| **`.4DZ`** | Compiled 4D component ZIP archive |
| **`.4dbase`** | 4D component bundle directory structure |
| **asset-lock.json** | Provenance file pinning exact CLI release with SHA256 digests |
| **SystemWorker** | 4D class for launching and communicating with external processes |
| **Protocol v1** | One-line JSON, LF-terminated communication format between CLI and 4D |

---

*End of document. Prepared 2026-11-19 for review and distribution to the project team.*
