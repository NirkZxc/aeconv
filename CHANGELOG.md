# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2026-11-20

### Added
- `--resize WxH` option: resize images to exact dimensions using a high-quality Lanczos3 filter; rejects zero dimensions and oversized requests
- `--round-corners Npx` / `N%` option: round image corners with pixel radius or percent of the shorter side; transparent anti-aliased corners, original resolution preserved
- Resize and rounded corners can be combined (resize runs before rounding); no intermediate files created

### Fixed (Codex Audit Compliance)
- Fixed all Clippy warnings (redundant closures, unnecessary casts, unused code, etc.)
- Removed unused `parse_shorthand` function and dead code
- Fixed `unwrap()` after `is_some()` checks with proper pattern matching
- ICNS format: corrected to encode-only (no decoder available in image crate)
- ICO format: verified self-created ICO decode works, multi-resolution encoding confirmed
- CLI interface: aligned README with actual subcommand interface (`list`, `formats`, `info` instead of global flags)
- Transparency handling: PNG→JPEG flattens alpha against white background (documented behavior)
- Documentation: updated format table to reflect ICNS encode-only, corrected CLI usage examples

### Added (Output Validation)
- `--output` / `-o` option now validates that output file extension matches target format
- Rejects mismatched extensions with clear error message (e.g., "Output file extension 'png' does not match target format 'WebP'. Expected one of: webp")
- Case-insensitive extension matching supported
- Subdirectory paths supported (e.g., `--output converted/result.webp`)

### Known Limitations (Phase 1)
- GIF: Multi-frame decoding not supported (first frame only)
- TIFF: Multi-frame decoding not supported (first frame only)
- ICO: Multi-resolution decoding not fully supported (largest frame only)
- ICNS: Decoding not supported (encode only)
- SVG: Encoding (vectorization) not supported
- EXIF metadata preservation not implemented
- ICC color profile handling not implemented

### Planned for Phase 2
- Individual command binaries with PATH registration
- Windows installer (MSI) and Linux packages (deb, rpm, AppImage)
- EXIF metadata preservation
- ICC color profile support
- Animated format frame handling
- Batch conversion with glob patterns
- Progress indicators for large files
- Configuration file support

## [0.1.2] - 2026-11-19

### Added
- Custom `--background` / `-b` color for flattening transparency when converting to formats without alpha support; accepts `#RGB`, `#RRGGBB`, or `R,G,B` (default: white)
- QOI now classified as a transparency-preserving format; RGBA → QOI preserves the alpha channel even when `--background` is supplied

### Fixed
- `ImageFormat::supports_transparency()` now includes QOI, matching the documented format capabilities

## [0.1.1] - 2026-11-16

### Added
- Output extension validation against the target format
- Case-insensitive extension matching
- Support for custom output paths in subdirectories

### Fixed
- Clear error messages for missing or mismatched output extensions

## [0.1.0] - 2026-11-15

### Added
- Initial release of AEConv (Any Extension Converter)
- Cross-platform image conversion CLI written in Rust
- Support for 12 image formats:
  - **Raster**: PNG, JPEG, WebP, BMP, TIFF, TGA, GIF, QOI, PNM (PPM/PGM/PBM)
  - **Icons**: ICO (multi-resolution), ICNS (multi-resolution with Retina variants)
  - **Vector**: SVG (decode/rasterize only)
- Predefined conversion commands: `jpg-to-ico`, `png-to-webp`, `jpg-to-png`
- Generic conversion command: `convert --from <fmt> --to <fmt> <input>`
- Quality control for lossy formats (JPEG: 1-100, WebP: 1-100)
- Atomic file writes for safety
- Unicode path and filename support
- Comprehensive error handling with user-friendly messages
- Format information command (`--formats`)
- Available operations listing (`--list`)
- Image info command (`--info`)
- Modular architecture with separate crates for:
  - CLI argument parsing
  - Conversion engine
  - Format registry
  - Codecs (decode/encode)
  - Metadata handling
  - Output management
  - Error handling
- Automated tests for all core functionality
- MIT license

### Known Limitations
- SVG encoding (vectorization) not supported - use dedicated tools like potrace
- Animated GIF/WebP/TIFF frame extraction not fully implemented
- EXIF metadata preservation not implemented
- ICC color profile handling not implemented
- Windows installer and PATH registration planned for Phase 2
- Individual command binaries (jpgtoico, pngtowebp, etc.) planned for Phase 2
