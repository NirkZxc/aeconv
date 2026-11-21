use crate::batch::{BatchConfig, run_batch};
use crate::codecs::RgbColor;
use crate::engine::{ConversionEngine, ConversionRequest};
use crate::errors::Result;
use crate::formats::{ImageFormat, all_operations};
use crate::output::OutputConfig;
use crate::processing::RoundCorners;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(
    name = "aeconv",
    version = VERSION,
    about = "Any Extension Converter - Cross-platform image conversion CLI",
    long_about = "AEConv converts images between various formats including PNG, JPEG, WebP, BMP, TIFF, TGA, GIF, QOI, PNM, ICO, ICNS, and SVG."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output file path
    #[arg(short, long, global = true)]
    pub output: Option<Utf8PathBuf>,

    /// Force overwrite existing files
    #[arg(short, long, global = true)]
    pub force: bool,

    /// Quality for lossy formats (1-100)
    #[arg(short, long, global = true, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub quality: Option<u8>,

    /// Background color for flattening transparency (#RGB, #RRGGBB, or R,G,B)
    #[arg(short = 'b', long, global = true)]
    pub background: Option<RgbColor>,

    /// Resize image to exact WIDTHxHEIGHT dimensions using a Lanczos3 filter
    #[arg(long, global = true, value_name = "WIDTHxHEIGHT", value_parser = clap::value_parser!(ResizeArg))]
    pub resize: Option<ResizeArg>,

    /// Round corners: radius in pixels (e.g. 32px) or percent of shorter side (e.g. 20%)
    #[arg(long, global = true, value_name = "RADIUS", value_parser = clap::value_parser!(RoundCornersArg))]
    pub round_corners: Option<RoundCornersArg>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Convert JPEG to ICO
    JpgToIco {
        #[arg(value_name = "INPUT")]
        input: Utf8PathBuf,
    },

    /// Convert PNG to WebP
    PngToWebp {
        #[arg(value_name = "INPUT")]
        input: Utf8PathBuf,
    },

    /// Convert JPEG to PNG
    JpgToPng {
        #[arg(value_name = "INPUT")]
        input: Utf8PathBuf,
    },

    /// Generic conversion: aeconv convert --from jpg --to png input.jpg
    Convert {
        /// Source format
        #[arg(long, value_enum)]
        from: Option<FormatArg>,

        /// Target format
        #[arg(long, value_enum)]
        to: Option<FormatArg>,

        #[arg(value_name = "INPUT")]
        input: Utf8PathBuf,
    },

    /// Batch convert all images in a directory
    Batch {
        #[arg(value_name = "INPUT_DIR")]
        input: Utf8PathBuf,

        /// Source image format (e.g. png, jpg, webp)
        #[arg(long, value_enum)]
        from: FormatArg,

        /// Target image format (e.g. webp, jpg, png)
        #[arg(long, value_enum)]
        to: FormatArg,

        /// Output directory for converted files
        #[arg(long)]
        output_dir: Utf8PathBuf,
    },

    /// List all available conversion operations
    List,

    /// List all supported formats
    Formats,

    /// Show image information
    Info {
        #[arg(value_name = "INPUT")]
        input: Utf8PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Png,
    Jpg,
    Jpeg,
    Bmp,
    Webp,
    Tiff,
    Tga,
    Gif,
    Qoi,
    Ppm,
    Pgm,
    Pbm,
    Pnm,
    Ico,
    Icns,
    Svg,
}

impl From<FormatArg> for ImageFormat {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Png => ImageFormat::Png,
            FormatArg::Jpg | FormatArg::Jpeg => ImageFormat::Jpeg,
            FormatArg::Bmp => ImageFormat::Bmp,
            FormatArg::Webp => ImageFormat::WebP,
            FormatArg::Tiff => ImageFormat::Tiff,
            FormatArg::Tga => ImageFormat::Tga,
            FormatArg::Gif => ImageFormat::Gif,
            FormatArg::Qoi => ImageFormat::Qoi,
            FormatArg::Ppm | FormatArg::Pgm | FormatArg::Pbm | FormatArg::Pnm => ImageFormat::Pnm,
            FormatArg::Ico => ImageFormat::Ico,
            FormatArg::Icns => ImageFormat::Icns,
            FormatArg::Svg => ImageFormat::Svg,
        }
    }
}

/// Resize dimensions parsed from a `WIDTHxHEIGHT` CLI value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeArg(pub u32, pub u32);

impl std::str::FromStr for ResizeArg {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('x').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid resize value '{s}'. Expected WIDTHxHEIGHT (e.g. 512x512)"
            ));
        }
        let width = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid resize width '{}' in '{s}'", parts[0]))?;
        let height = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid resize height '{}' in '{s}'", parts[1]))?;
        Ok(ResizeArg(width, height))
    }
}

/// Rounded corners radius: pixels (`32px`) or percent of the shorter side (`20%`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoundCornersArg {
    Percent(f64),
    Pixels(u32),
}

impl std::str::FromStr for RoundCornersArg {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.trim();
        if let Some(rest) = s.strip_suffix('%') {
            let value = rest
                .parse::<f64>()
                .map_err(|_| format!("Invalid round-corners percent '{}' in '{s}'", rest))?;
            Ok(RoundCornersArg::Percent(value))
        } else if let Some(rest) = s.strip_suffix("px") {
            let value = rest
                .parse::<u32>()
                .map_err(|_| format!("Invalid round-corners pixels '{}' in '{s}'", rest))?;
            Ok(RoundCornersArg::Pixels(value))
        } else {
            Err(format!(
                "Invalid round-corners value '{s}'. Use Npx (e.g. 32px) or N% (e.g. 20%)"
            ))
        }
    }
}

/// Run the CLI
pub fn run(cli: Cli) -> Result<()> {
    // Handle shorthand commands (e.g., `aeconv jpgtoico photo.jpg`)
    // This is handled by checking if the first arg matches a pattern
    // But clap handles subcommands, so we'll use the subcommand approach

    match cli.command {
        Commands::JpgToIco { ref input } => run_conversion(input.clone(), ImageFormat::Ico, &cli),
        Commands::PngToWebp { ref input } => run_conversion(input.clone(), ImageFormat::WebP, &cli),
        Commands::JpgToPng { ref input } => run_conversion(input.clone(), ImageFormat::Png, &cli),
        Commands::Convert {
            from: _,
            to,
            ref input,
        } => {
            let target_fmt = to.map(Into::into).unwrap_or(ImageFormat::Png);
            run_conversion(input.clone(), target_fmt, &cli)
        }
        Commands::Batch {
            ref input,
            from,
            to,
            ref output_dir,
        } => run_batch_command(input.clone(), from, to, output_dir.clone(), &cli),
        Commands::List => {
            print_operations();
            Ok(())
        }
        Commands::Formats => {
            print_formats();
            Ok(())
        }
        Commands::Info { ref input } => show_info(input.clone()),
    }
}

fn run_conversion(input: Utf8PathBuf, target_fmt: ImageFormat, cli: &Cli) -> Result<()> {
    let config = OutputConfig {
        output_path: cli.output.clone(),
        force_overwrite: cli.force,
        quality: cli.quality,
        background_color: cli.background.unwrap_or_default(),
    };

    let request = ConversionRequest {
        input_path: input,
        target_format: target_fmt,
        config,
        resize: cli.resize.map(|r| (r.0, r.1)),
        round_corners: cli.round_corners.map(|r| match r {
            RoundCornersArg::Percent(p) => RoundCorners::Percent(p),
            RoundCornersArg::Pixels(p) => RoundCorners::Pixels(p),
        }),
    };

    let result = ConversionEngine::convert(request)?;

    if cli.verbose {
        println!(
            "Converted: {} -> {}",
            result.source_format, result.target_format
        );
        println!(
            "Input: {}x{}",
            result.input_metadata.width, result.input_metadata.height
        );
        println!("Output: {}", result.output_path);
        println!("Size: {} bytes", result.output_size);
    } else {
        println!("{}", result.output_path);
    }

    Ok(())
}

fn run_batch_command(
    input_dir: Utf8PathBuf,
    from: FormatArg,
    to: FormatArg,
    output_dir: Utf8PathBuf,
    cli: &Cli,
) -> Result<()> {
    let config = BatchConfig {
        input_dir,
        output_dir,
        source_format: from.into(),
        target_format: to.into(),
        quality: cli.quality,
        background_color: cli.background.unwrap_or_default(),
        force: cli.force,
        resize: cli.resize.map(|r| (r.0, r.1)),
        round_corners: cli.round_corners.map(|r| match r {
            RoundCornersArg::Percent(p) => RoundCorners::Percent(p),
            RoundCornersArg::Pixels(p) => RoundCorners::Pixels(p),
        }),
        verbose: cli.verbose,
    };

    let summary = run_batch(config)?;
    if summary.has_errors() {
        std::process::exit(1);
    }
    Ok(())
}

fn print_operations() {
    let ops = all_operations();
    println!("Available conversion operations ({}):", ops.len());
    for op in ops {
        println!("  {}", op);
    }
}

fn print_formats() {
    println!("Supported formats:");
    println!("\nRaster formats:");
    for fmt in ImageFormat::raster_formats() {
        let caps = crate::formats::registry().get_capabilities(fmt).unwrap();
        println!(
            "  {} - {} (decode: {}, encode: {})",
            fmt.name(),
            fmt.extensions().join(", "),
            caps.can_decode,
            caps.can_encode
        );
    }
    println!("\nIcon formats:");
    for fmt in ImageFormat::icon_formats() {
        let caps = crate::formats::registry().get_capabilities(fmt).unwrap();
        println!(
            "  {} - {} (decode: {}, encode: {})",
            fmt.name(),
            fmt.extensions().join(", "),
            caps.can_decode,
            caps.can_encode
        );
    }
    println!("\nVector formats:");
    for fmt in ImageFormat::vector_formats() {
        let caps = crate::formats::registry().get_capabilities(fmt).unwrap();
        println!(
            "  {} - {} (decode: {}, encode: {})",
            fmt.name(),
            fmt.extensions().join(", "),
            caps.can_decode,
            caps.can_encode
        );
    }
}

fn show_info(input: Utf8PathBuf) -> Result<()> {
    let decoded = crate::codecs::decode_file(&input)?;
    let meta = &decoded.metadata;

    println!("File: {}", input);
    println!("Format: {}", decoded.source_format);
    println!("Dimensions: {}x{}", meta.width, meta.height);
    println!("Color type: {:?}", meta.color_type);
    println!("Bit depth: {}", meta.bit_depth);
    println!("Has alpha: {}", meta.has_alpha);
    println!("Animated: {}", meta.is_animated);
    println!("Frame count: {}", meta.frame_count);
    if let Some(dpi) = meta.dpi {
        println!("DPI: {:.1}x{:.1}", dpi.0, dpi.1);
    }
    if let Some(profile) = &meta.color_profile {
        println!("Color profile: Yes ({} bytes)", profile.len());
    }
    if !meta.exif.is_empty() {
        println!("EXIF: {} tags", meta.exif.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::ImageFormat;

    #[test]
    fn test_format_arg_conversion() {
        assert_eq!(ImageFormat::from(FormatArg::Png), ImageFormat::Png);
        assert_eq!(ImageFormat::from(FormatArg::Jpg), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from(FormatArg::Jpeg), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from(FormatArg::Ico), ImageFormat::Ico);
    }

    #[test]
    fn parses_background_option_and_short_alias_for_conversion() {
        let long = Cli::try_parse_from([
            "aeconv",
            "convert",
            "--from",
            "png",
            "--to",
            "jpg",
            "image.png",
            "--background",
            "#222222",
        ])
        .unwrap();
        assert_eq!(long.background, Some(RgbColor::new(34, 34, 34)));

        let short = Cli::try_parse_from([
            "aeconv",
            "convert",
            "--from",
            "png",
            "--to",
            "jpg",
            "image.png",
            "-b",
            "#fff",
        ])
        .unwrap();
        assert_eq!(short.background, Some(RgbColor::new(255, 255, 255)));
    }

    #[test]
    fn parses_short_hex_and_whitespace_rgb_at_cli_boundary() {
        for (value, expected) in [
            ("#f08", RgbColor::new(255, 0, 136)),
            ("1, 2, 3", RgbColor::new(1, 2, 3)),
        ] {
            let cli = Cli::try_parse_from([
                "aeconv",
                "convert",
                "--from",
                "png",
                "--to",
                "jpg",
                "image.png",
                "--background",
                value,
            ])
            .unwrap();
            assert_eq!(cli.background, Some(expected));
        }
    }

    #[test]
    fn rejects_malformed_background_values_at_cli_boundary_without_panicking() {
        // These values reach our RgbColor parser and must produce a clear error.
        for value in ["#ggg", "#12", "256,0,0"] {
            let error = Cli::try_parse_from([
                "aeconv",
                "convert",
                "--from",
                "png",
                "--to",
                "jpg",
                "image.png",
                "--background",
                value,
            ])
            .unwrap_err();
            assert!(
                error.to_string().contains("Invalid background color"),
                "unexpected error for {value}: {error}"
            );
        }

        // Negative values are rejected by RgbColor::from_str (each component must be 0..=255).
        let error = Cli::try_parse_from([
            "aeconv",
            "convert",
            "--from",
            "png",
            "--to",
            "jpg",
            "image.png",
            "--background=-1,2,3",
        ])
        .unwrap_err();
        assert!(
            error.to_string().contains("Invalid background color"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parses_resize_dimensions_at_cli_boundary() {
        let cli = Cli::try_parse_from([
            "aeconv",
            "convert",
            "--from",
            "png",
            "image.png",
            "--resize",
            "512x512",
        ])
        .unwrap();
        assert_eq!(cli.resize, Some(ResizeArg(512, 512)));
    }

    #[test]
    fn rejects_malformed_resize_values_at_cli_boundary() {
        // These values reach our ResizeArg parser and must produce a clear error.
        for value in ["512", "512x", "x512", "abcx100", "512xabc"] {
            let error = Cli::try_parse_from([
                "aeconv",
                "convert",
                "--from",
                "png",
                "image.png",
                "--resize",
                value,
            ])
            .unwrap_err();
            assert!(
                error.to_string().contains("Invalid resize"),
                "unexpected error for {value}: {error}"
            );
        }
    }

    #[test]
    fn parses_round_corners_pixels_and_percent_at_cli_boundary() {
        let pixels = Cli::try_parse_from([
            "aeconv",
            "convert",
            "--from",
            "png",
            "image.png",
            "--round-corners",
            "32px",
        ])
        .unwrap();
        assert_eq!(pixels.round_corners, Some(RoundCornersArg::Pixels(32)));

        let percent = Cli::try_parse_from([
            "aeconv",
            "convert",
            "--from",
            "png",
            "image.png",
            "--round-corners",
            "20%",
        ])
        .unwrap();
        assert_eq!(percent.round_corners, Some(RoundCornersArg::Percent(20.0)));
    }

    #[test]
    fn rejects_malformed_round_corners_values_at_cli_boundary() {
        // These values reach our RoundCornersArg parser and must produce a clear error.
        for value in ["32", "20", "abcpx", "abc%"] {
            let error = Cli::try_parse_from([
                "aeconv",
                "convert",
                "--from",
                "png",
                "image.png",
                "--round-corners",
                value,
            ])
            .unwrap_err();
            assert!(
                error.to_string().contains("Invalid round-corners"),
                "unexpected error for {value}: {error}"
            );
        }
    }
}
