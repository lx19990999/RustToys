# RustToys

A Swiss Army knife for developers, built with Rust and [egui](https://github.com/emilk/egui). Inspired by [DevToys](https://github.com/DevToys-app/DevToys).

## Features

30 developer tools across 7 categories, all running natively with no browser dependency.

### Converters

| Tool | Description |
|------|-------------|
| Cron Parser | Parse cron expressions, show human-readable description and upcoming executions |
| Date Converter | Convert timestamps and date strings between formats, supports 13 timezones |
| JSON > Table | Convert JSON arrays to tabular view, export as CSV / TSV / Markdown |
| JSON <> YAML | Bidirectional conversion between JSON and YAML |
| Number Base | Convert integers between binary, octal, decimal, hex |

### Encoders / Decoders

| Tool | Description |
|------|-------------|
| Base64 Text | Encode / decode text to / from Base64 |
| Base64 Image | Encode images to Base64 or decode Base64 to view images |
| Certificate Decoder | Parse PEM / DER certificates, display subject, issuer, validity, extensions |
| GZip | Compress text to GZip or decompress GZip back to text |
| HTML | Encode / decode HTML entities |
| JWT | Decode and encode JSON Web Tokens, verify signatures (HS / RS / ES / PS) |
| QR Code | Generate and decode QR codes, supports SVG and PNG export |
| URL | URL-encode / decode with multiline support |

### Formatters

| Tool | Description |
|------|-------------|
| JSON Formatter | Format, minify and validate JSON, configurable indent and key sorting |
| SQL Formatter | Format SQL with keyword normalization, supports Standard / MySQL / PostgreSQL / PL/SQL |
| XML Formatter | Format and validate XML, configurable indent and minify |

### Generators

| Tool | Description |
|------|-------------|
| Hash / Checksum | Compute MD5, SHA-1, SHA-256, SHA-384, SHA-512 for text and files |
| Lorem Ipsum | Generate placeholder text, 4 word libraries, multiple output modes |
| Password Generator | Generate random passwords with configurable length, count and charset |
| UUID Generator | Generate UUID v1, v4, v7 with uppercase and hyphen options |

### Graphic

| Tool | Description |
|------|-------------|
| Color Blindness Simulator | Simulate Protanopia, Deuteranopia, Tritanopia on any image |
| Image Converter | Convert between PNG / JPEG / BMP / GIF / WebP, resize with quality control |

### Testers

| Tool | Description |
|------|-------------|
| JSONPath Tester | Evaluate JSONPath expressions against JSON data with built-in cheat sheet |
| Regex Tester | Test regex patterns, show matches with capture groups and positions |
| XML / XSD Tester | Validate XML against XSD schemas with detailed error reporting |

### Text

| Tool | Description |
|------|-------------|
| Analyzer & Utilities | Text statistics, 14 case conversions, line sorting, line-break conversion |
| Escape / Unescape | Escape and unescape special characters |
| List Comparer | Compare two lists with set operations (intersection, difference, union) |
| Markdown Preview | Live Markdown preview with GitHub-style rendering |
| Text Comparer | LCS-based line diff between two texts |

## Configuration

Settings are saved to `~/.config/rusttoys.json` (Windows: `%USERPROFILE%\.config\rusttoys.json`).

```json
{
  "theme": "system",
  "dpi": 2.0,
  "lastsavefolder": "/home/user/Documents"
}
```

- **theme** — `system`, `light` or `dark`. Applied at startup, updated on change.
- **dpi** — Interface scale factor. Auto-detected on first launch, updated on change.
- **lastsavefolder** — Default directory for file save dialogs, updated on each save.

## Build

```bash
cd RustToys
cargo build --release
```

Requires a Rust toolchain (stable). Supports Linux, macOS and Windows.

## License

[GPL-3.0](LICENSE)
