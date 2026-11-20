# AEConv — Any Extension Converter

[![Crates.io](https://img.shields.io/crates/v/aeconv.svg)](https://crates.io/crates/aeconv)
[![Latest Release](https://img.shields.io/github/v/release/NirkZxc/aeconv)](https://github.com/NirkZxc/aeconv/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A cross-platform command-line utility for converting images between various formats. Built with Rust for performance and reliability.

## Features

- **Wide format support**: PNG, JPEG, WebP, BMP, TIFF, TGA, GIF, QOI, PNM (PPM/PGM/PBM), ICO, ICNS (encode only), SVG
- **Cross-platform**: Windows, Linux, macOS
- **High performance**: Built with Rust's `image` crate and `resvg` for SVG rasterization
- **Multi-resolution icons**: ICO (decode/encode) and ICNS (encode only) with multiple standard sizes
- **SVG rasterization**: Convert SVG to any raster format
- **Quality control**: Adjustable quality for lossy formats (JPEG, WebP)
- **Atomic writes**: Safe file operations with atomic rename
- **Unicode paths**: Full support for Unicode filenames and paths with spaces
- **Image resize**: Resize images to exact dimensions with a high-quality Lanczos3 filter (`--resize WxH`)
- **Rounded corners**: Round image corners with pixel or percent radii (`--round-corners Npx` / `N%`)

## Installation

### From Cargo (Recommended)

```bash
cargo install aeconv
```

### Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/NirkZxc/aeconv/releases).

### From Source

```bash
git clone https://github.com/NirkZxc/aeconv
cd aeconv
cargo build --release
# Binary will be at target/release/aeconv
```

## Usage

### Basic Conversions

```bash
# Convert using predefined commands
aeconv jpg-to-ico photo.jpg
aeconv png-to-webp image.png
aeconv jpg-to-png photo.jpg

# Generic conversion with explicit formats
aeconv convert --from png --to jpg image.png
aeconv convert --from jpg --to ico photo.jpg

# With custom output path
aeconv jpg-to-ico photo.jpg -o /path/to/output.ico

# Output extension must match target format (validated)
aeconv convert --from png --to webp image.png --output result.webp   # OK
aeconv convert --from png --to webp image.png --output result.png    # Error: extension mismatch

# With quality control (1-100)
aeconv convert --from png --to jpg image.png -q 85

# Choose the background used when flattening transparency for JPEG
aeconv convert --from png --to jpg image.png --background "#222222"
aeconv convert --from png --to jpg image.png -b "#fff"
aeconv convert --from png --to jpg image.png --background "255,0,128"

# Force overwrite existing files
aeconv png-to-webp image.png --force
```

### Information Commands

```bash
# List all available conversion operations
aeconv list

# List all supported formats
aeconv formats

# Show image information
aeconv info photo.jpg

# Show version
aeconv --version
```

### Supported Formats

| Format | Extensions | Decode | Encode | Transparency | Animation |
|--------|-----------|--------|--------|--------------|-----------|
| PNG | .png | ✅ | ✅ | ✅ | ❌ |
| JPEG | .jpg, .jpeg, .jpe, .jfif | ✅ | ✅ | ❌ | ❌ |
| WebP | .webp | ✅ | ✅ | ✅ | ✅ |
| BMP | .bmp, .dib | ✅ | ✅ | ❌ | ❌ |
| TIFF | .tiff, .tif | ✅ | ✅ | ✅ | ✅ |
| TGA | .tga | ✅ | ✅ | ✅ | ❌ |
| GIF | .gif | ✅ | ✅ | ✅ | ✅ |
| QOI | .qoi | ✅ | ✅ | ✅ | ❌ |
| PNM | .ppm, .pgm, .pbm, .pnm | ✅ | ✅ | ❌ | ❌ |
| ICO | .ico | ✅ | ✅ | ✅ | ❌ |
| ICNS | .icns | ❌ | ✅ | ✅ | ❌ |
| SVG | .svg, .svgz | ✅ | ❌ | ✅ | ❌ |

**Note**: SVG encoding (vectorization) is not supported in Phase 1. Use dedicated tools like `potrace` for raster-to-vector conversion.

**Note on transparency**: GIF supports binary transparency (transparent or opaque palette entries), not full 8-bit alpha.

**Note on animated formats**: WebP, TIFF, and GIF are marked as supporting animation in the format table because they *can* contain multiple frames. However, AEConv Phase 1 only processes the first frame when decoding these formats. Multi-frame decoding and encoding will be added in Phase 2.

## Options

| Option | Short | Description |
|--------|-------|-------------|
| `--output` | `-o` | Output file path (default: same directory as input with new extension) |
| `--force` | `-f` | Overwrite existing output file |
| `--quality` | `-q` | Quality for lossy formats (1-100, default: 90 for JPEG, 80 for WebP) |
| `--background` | `-b` | Background color when flattening transparency (`#RGB`, `#RRGGBB`, or `R,G,B`; default: white) |
| `--resize` | | Resize to exact WIDTHxHEIGHT pixels using a Lanczos3 filter (e.g. `--resize 512x512`) |
| `--round-corners` | | Round corners: radius in pixels (`32px`) or percent of shorter side (`20%`) |
| `--verbose` | `-v` | Show detailed conversion information |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

## Image Processing

### Resize (`--resize`)

Resize an image to exact dimensions using a high-quality Lanczos3 filter:

```bash
# Downscale an image
aeconv input.png output.png --resize 512x512

# Upscale an image
aeconv input.png output.png --resize 2000x1500

# Non-square resize (aspect ratio may change)
aeconv input.png output.png --resize 800x600

# Resize combined with format conversion
aeconv input.png output.webp --resize 512x512
```

Dimensions must be greater than zero. Oversized requests are rejected to prevent excessive memory allocation.

### Rounded Corners (`--round-corners`)

Round the corners of an image. The radius can be given in pixels or as a percent of the shorter side:

```bash
# Fixed pixel radius
aeconv input.png output.png --round-corners 32px

# Percent of the shorter side
aeconv input.png output.png --round-corners 20%

# Radius auto-clamps so corners never overlap
aeconv input.png output.png --round-corners 50%
```

Corners become transparent with smooth anti-aliased edges; the image resolution is unchanged. For formats without transparency (e.g. JPEG), the corners are flattened against the `--background` color.

### Combined Processing

Resize is applied before rounding corners, so both operations work together without intermediate files:

```bash
# Resize then round corners, converting to WebP
aeconv input.png output.webp --resize 512x512 --round-corners 15%

# Resize, round corners, and set a custom background for non-alpha formats
aeconv input.png output.jpg --resize 400x400 --round-corners 10% --background "#222222"
```

## Examples

```bash
# Convert PNG to WebP with high quality
aeconv png-to-webp image.png -q 95

# Batch convert all JPEGs to WebP (using shell)
for f in *.jpg; do aeconv jpg-to-webp "$f"; done

# Convert SVG to PNG for web use
aeconv convert --from svg --to png logo.svg

# Create multi-resolution ICO from PNG
aeconv png-to-ico icon.png

# Convert to ICNS for macOS
aeconv convert --from png --to icns icon.png

# Get image details
aeconv info photo.jpg
```

## Architecture

AEConv is built with a modular architecture:

- **cli** — Command-line argument parsing using `clap`
- **engine** — Core conversion pipeline
- **formats** — Format registry with capabilities
- **codecs** — Decoding/encoding for each format
- **metadata** — Image metadata handling
- **output** — Output file path resolution and atomic writes
- **errors** — Centralized error handling with user-friendly messages

## Building

### Requirements

- Rust 1.70+ (stable)
- Cargo

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Cross-compile for Windows from Linux
cargo build --release --target x86_64-pc-windows-gnu

# Cross-compile for Linux from Windows
cargo build --release --target x86_64-unknown-linux-gnu
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.
