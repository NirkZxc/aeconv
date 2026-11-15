use crate::engine::{ConversionEngine, ConversionRequest};
use crate::errors::Result;
use crate::formats::{ImageFormat, all_operations};
use crate::output::OutputConfig;
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
    };

    let request = ConversionRequest {
        input_path: input,
        target_format: target_fmt,
        config,
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
}
