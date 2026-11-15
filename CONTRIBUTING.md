# Contributing to AEConv

Thank you for your interest in contributing to AEConv! This document provides guidelines for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and constructive in all interactions.

## How to Contribute

### Reporting Bugs

Before reporting a bug, please check if it has already been reported in the [Issues](https://github.com/almaz/aeconv/issues).

When reporting a bug, please include:
- AEConv version (`aeconv --version`)
- Operating system and version
- Steps to reproduce the issue
- Input file format and command used
- Expected vs actual behavior
- Error messages (if any)

### Suggesting Features

Feature requests are welcome! Please open an issue with:
- Clear description of the feature
- Use cases and motivation
- Possible implementation approach (if known)

### Pull Requests

1. **Fork** the repository
2. **Create a branch** for your change: `git checkout -b feature/your-feature-name`
3. **Make your changes** following the coding standards below
4. **Add tests** for new functionality
5. **Run tests** to ensure they pass: `cargo test`
6. **Format code**: `cargo fmt`
7. **Check for warnings**: `cargo clippy`
8. **Commit** with a clear message
9. **Push** to your fork
10. **Open a Pull Request**

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git

### Building

```bash
git clone https://github.com/almaz/aeconv
cd aeconv
cargo build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_flatten_alpha
```

### Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy`)
- Write clear, concise commit messages
- Add documentation for public APIs
- Keep functions small and focused

## Architecture Overview

AEConv uses a modular architecture:

```
src/
├── main.rs          # Entry point
├── cli.rs           # Command-line interface
├── engine.rs        # Conversion pipeline
├── formats.rs       # Format registry & capabilities
├── codecs.rs        # Decoding/encoding per format
├── metadata.rs      # Image metadata handling
├── output.rs        # Output path resolution
└── errors.rs        # Error types & handling
```

### Adding a New Format

1. Add the format to `ImageFormat` enum in `formats.rs`
2. Add extensions, MIME type, and capabilities
3. Register in `FormatRegistry::register_builtin()`
4. Implement decoder in `codecs.rs` `decode_raster()` or add new decode function
5. Implement encoder in `codecs.rs` `encode_image()` match arm
6. Add format-specific handling in `prepare_for_format()` if needed
7. Add tests for the new format

## Testing Guidelines

- Write unit tests for new functions in the same module
- Add integration tests in `tests/` directory (if created)
- Test both success and error cases
- Test with various input formats
- Verify output files are valid (not just exit codes)

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create a git tag: `git tag v0.x.x`
4. Push tag: `git push origin v0.x.x`
5. GitHub Actions will build and publish release

## Questions?

Feel free to open an issue for any questions about contributing!

## License

By contributing, you agree that your contributions will be licensed under the MIT License.