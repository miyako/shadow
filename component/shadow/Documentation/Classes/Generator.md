# Generator

> **Namespace**: `cs.Shadow.Generator`

Generates shadow effects for images using the `shadow-cli` binary.

## Overview

The `Generator` class provides a structured API to generate six types of image shadows: drop, contact, curved, long, perspective, and inner. It wraps the `shadow-cli` Rust binary via `4D.SystemWorker`.

## Constructor

```4d
var $shadow : cs.Shadow.Generator
$shadow := cs.Shadow.Generator.new()
```

No parameters required. The constructor initializes the internal `_CLI` and `_Process` helper classes.

## Methods

### `version() → Object`

Returns the CLI version and protocol version.

**Returns**: `{ok: Boolean, value: {protocol: Integer, version: Text}}`

```4d
var $result : Object
$result := $shadow.version()
// $result = {ok: True, value: {protocol: 1, version: "0.1.0"}}
```

### `capabilities() → Object`

Returns supported shadow types and parameters.

**Returns**: `{ok: Boolean, value: {protocol: Integer, capabilities: {types: Text[], params: Text[]}}}`

```4d
var $result : Object
$result := $shadow.capabilities()
// types: ["drop", "contact", "curved", "long", "perspective", "inner"]
```

### `generate(input; output; options) → Object`

Generates a shadow and composites it with the original image.

**Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `input` | `4D.File` or `Blob` | Input image (PNG, JPEG, WebP, BMP, GIF) |
| `output` | `4D.File` | Output file path |
| `options` | `Object` | Shadow parameters (see below) |

**Returns**: `{ok: Boolean, value: {protocol, ok, output, width, height}}` or `{ok: False, error: {code, message}}`

### `shadowOnly(input; output; options) → Object`

Same as `generate` but outputs only the shadow layer (no original image).

## Options

| Option | Type | Default | Applies to |
|--------|------|---------|------------|
| `type` | Text | `"drop"` | all |
| `opacity` | Real | 0.5 | all |
| `offset-x` | Integer | 5 | drop, inner |
| `offset-y` | Integer | 5 | drop, inner |
| `blur` | Real | 10.0 | drop, contact, inner, perspective |
| `color` | Text | `"#000000"` | all |
| `scale` | Real | 1.0 | drop, contact |
| `angle` | Integer | 45 | long |
| `length` | Text/Integer | `"auto"` | long |
| `fade` | Text | `"linear"` | long |
| `edge` | Text | `"bottom"` | curved |
| `curve-depth` | Real | 20.0 | curved |
| `light-x` | Real | 0.0 | perspective |
| `light-y` | Real | 0.0 | perspective |
| `light-z` | Real | 100.0 | perspective |
| `canvas-expand` | Integer | 20 | all |
| `format` | Text | `"png"` | output format |
| `background` | Text | `"transparent"` | composite background |

## Shadow Types

| Type | Description |
|------|-------------|
| `drop` | Standard elevation shadow with offset and blur |
| `contact` | Ambient occlusion — tight blur, minimal offset |
| `curved` | Bézier curve lift shadow with gradient fill |
| `long` | Extended shadow at a fixed angle with opacity fade |
| `perspective` | Homography projection onto ground plane |
| `inner` | Inset shadow inside the object boundary |

## Examples

### Drop shadow (file mode)

```4d
var $shadow : cs.Shadow.Generator
var $result : Object

$shadow := cs.Shadow.Generator.new()

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
  // $result.value contains {width, height, output}
Else 
  ALERT($result.error.code + ": " + $result.error.message)
End if 
```

### Contact shadow (stdin/blob mode)

```4d
var $blob : Blob
$blob := File("/DATA/large.png").getContent()

$result := $shadow.generate( \
  $blob; \
  File("/DATA/large_shadowed.png"); \
  New object("type"; "contact"; "blur"; 5.0))
```

## Error Codes

| Code | Exit Code | Description |
|------|-----------|-------------|
| `E_RUNTIME` | 1 | Decode/encode/processing error |
| `E_ARGS` | 2 | Invalid arguments or contract violation |
| `E_IO` | 3 | File not found or permission error |

## Technical Notes

- Execution is synchronous with a 30-second timeout.
- All arguments are safely quoted — no shell interpretation.
- NUL characters in arguments are rejected for security.
- The CLI binary is resolved from `/RESOURCES/bin/shadow-cli` (macOS) or `shadow-cli.exe` (Windows).
- Protocol version 1: one JSON line, LF-terminated.
