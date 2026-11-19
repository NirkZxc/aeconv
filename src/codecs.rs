use crate::errors::{AeconvError, Result};
use crate::formats::ImageFormat;
use crate::metadata::{ImageMetadata, extract_metadata, preserve_metadata};
use camino::Utf8Path;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageReader};
use std::io::Cursor;

/// RGB background color used when an image's alpha channel must be flattened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor([u8; 3]);

impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self([red, green, blue])
    }

    pub const fn channels(self) -> [u8; 3] {
        self.0
    }
}

impl Default for RgbColor {
    fn default() -> Self {
        Self::new(255, 255, 255)
    }
}

impl std::str::FromStr for RgbColor {
    type Err = AeconvError;

    fn from_str(value: &str) -> Result<Self> {
        let invalid = || {
            AeconvError::InvalidArgument(format!(
                "Invalid background color '{}'. Use #RGB, #RRGGBB, or R,G,B (each value 0-255)",
                value
            ))
        };

        if let Some(hex) = value.strip_prefix('#') {
            if !hex.is_ascii() {
                return Err(invalid());
            }
            let channels = match hex.len() {
                3 => hex
                    .chars()
                    .map(|digit| {
                        digit
                            .to_digit(16)
                            .map(|value| (value * 17) as u8)
                            .ok_or_else(invalid)
                    })
                    .collect::<Result<Vec<_>>>()?,
                6 => (0..3)
                    .map(|index| {
                        u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                            .map_err(|_| invalid())
                    })
                    .collect::<Result<Vec<_>>>()?,
                _ => return Err(invalid()),
            };
            return Ok(Self::new(channels[0], channels[1], channels[2]));
        }

        let channels = value
            .split(',')
            .map(|component| component.trim().parse::<u8>().map_err(|_| invalid()))
            .collect::<Result<Vec<_>>>()?;
        if channels.len() != 3 {
            return Err(invalid());
        }

        Ok(Self::new(channels[0], channels[1], channels[2]))
    }
}

/// Decoded image with metadata
#[derive(Debug)]
pub struct DecodedImage {
    pub image: DynamicImage,
    pub metadata: ImageMetadata,
    pub source_format: ImageFormat,
    pub _frames: Vec<DynamicImage>, // For animated formats (future use)
}

/// Encode options
#[derive(Debug, Clone, Default)]
pub struct EncodeOptions {
    pub quality: Option<u8>,
    pub preserve_transparency: bool,
    pub background_color: RgbColor,
    pub _dpi: Option<(f64, f64)>,
    pub _icc_profile: Option<Vec<u8>>,
}

/// Decode image from file
pub fn decode_file(path: &Utf8Path) -> Result<DecodedImage> {
    let format = detect_format(path)?;

    if !crate::formats::can_decode(format) {
        return Err(AeconvError::UnsupportedSourceFormat {
            format: format.name().to_string(),
        });
    }

    match format {
        ImageFormat::Svg => decode_svg(path),
        _ => decode_raster(path, format),
    }
}

/// Detect format from file extension (public for use by other modules)
pub fn detect_format(path: &Utf8Path) -> Result<ImageFormat> {
    let ext = path
        .extension()
        .ok_or_else(|| AeconvError::InvalidArgument("File has no extension".to_string()))?;
    crate::formats::format_from_extension(ext).ok_or_else(|| AeconvError::UnsupportedSourceFormat {
        format: ext.to_string(),
    })
}

/// Decode raster formats using image crate
fn decode_raster(path: &Utf8Path, format: ImageFormat) -> Result<DecodedImage> {
    let reader = ImageReader::open(path).map_err(|e| AeconvError::DecodingFailed {
        format: format.name().to_string(),
        reason: e.to_string(),
    })?;

    let reader = reader
        .with_guessed_format()
        .map_err(|e| AeconvError::DecodingFailed {
            format: format.name().to_string(),
            reason: e.to_string(),
        })?;

    let img = reader.decode().map_err(|_e| AeconvError::CorruptedImage {
        path: path.to_string(),
    })?;

    let metadata = extract_metadata(&img, format);

    // For animated formats, we'd need to extract frames
    // The image crate's decoder doesn't easily expose all frames for all formats
    // This is a simplified version - full frame extraction would need format-specific handling
    let _frames = vec![img.clone()];

    Ok(DecodedImage {
        image: img,
        metadata,
        source_format: format,
        _frames,
    })
}

/// Decode SVG using resvg
fn decode_svg(path: &Utf8Path) -> Result<DecodedImage> {
    let svg_data = std::fs::read(path).map_err(AeconvError::IoError)?;

    // Parse SVG - use usvg's Tree::from_data with TreeParsing trait
    use usvg::TreeParsing;
    let opt = usvg::Options::default();
    let tree =
        usvg::Tree::from_data(&svg_data, &opt).map_err(|e| AeconvError::SvgError(e.to_string()))?;

    // Determine size
    let size = tree.size;
    let width = size.width() as u32;
    let height = size.height() as u32;

    if width == 0 || height == 0 {
        return Err(AeconvError::SvgError("SVG has zero dimensions".to_string()));
    }

    // Rasterize to pixmap - use resvg's re-exported tiny_skia
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| AeconvError::SvgError("Failed to create pixmap".to_string()))?;

    resvg::render(&tree, usvg::Transform::default(), &mut pixmap.as_mut());

    // Convert to DynamicImage
    let img = DynamicImage::ImageRgba8(
        image::ImageBuffer::from_raw(width, height, pixmap.take())
            .ok_or_else(|| AeconvError::SvgError("Failed to create image buffer".to_string()))?,
    );

    let metadata = ImageMetadata {
        width,
        height,
        color_type: crate::metadata::ColorType::Rgba,
        bit_depth: 8,
        has_alpha: true,
        is_animated: false,
        frame_count: 1,
        dpi: None,
        color_profile: None,
        exif: std::collections::HashMap::new(),
        format_specific: std::collections::HashMap::new(),
    };

    Ok(DecodedImage {
        image: img.clone(),
        metadata,
        source_format: ImageFormat::Svg,
        _frames: vec![img],
    })
}

/// Encode image to bytes
pub fn encode_image(
    img: &DynamicImage,
    target_format: ImageFormat,
    options: &EncodeOptions,
) -> Result<Vec<u8>> {
    if !crate::formats::can_encode(target_format) {
        return Err(AeconvError::UnsupportedTargetFormat {
            format: target_format.name().to_string(),
        });
    }

    let mut buf = Cursor::new(Vec::new());

    match target_format {
        ImageFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buf);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::from(img.color()),
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "PNG".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::Jpeg => {
            let quality = options.quality.unwrap_or(90);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::from(img.color()),
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "JPEG".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::Bmp => {
            let encoder = image::codecs::bmp::BmpEncoder::new(&mut buf);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::from(img.color()),
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "BMP".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::WebP => {
            // Use webp crate for better quality control
            let quality = options.quality.unwrap_or(80) as f32 / 100.0;
            let webp_data = webp::Encoder::from_image(img)
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "WebP".to_string(),
                    reason: e.to_string(),
                })?
                .encode(quality);
            buf.get_mut().extend_from_slice(&webp_data);
        }
        ImageFormat::Tiff => {
            let encoder = image::codecs::tiff::TiffEncoder::new(&mut buf);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::from(img.color()),
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "TIFF".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::Tga => {
            let encoder = image::codecs::tga::TgaEncoder::new(&mut buf);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::from(img.color()),
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "TGA".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::Gif => {
            // GIF encoding - use Rgba8 and let encoder handle palette conversion
            let encoder = image::codecs::gif::GifEncoder::new(&mut buf);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "GIF".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::Qoi => {
            let encoder = image::codecs::qoi::QoiEncoder::new(&mut buf);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::from(img.color()),
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "QOI".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::Pnm => {
            // PNM (PPM/PGM/PBM) - use PPM for color, PGM for grayscale
            let encoder = image::codecs::pnm::PnmEncoder::new(&mut buf);
            encoder
                .write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::from(img.color()),
                )
                .map_err(|e| AeconvError::EncodingFailed {
                    format: "PNM".to_string(),
                    reason: e.to_string(),
                })?;
        }
        ImageFormat::Ico => {
            return encode_ico(img, options);
        }
        ImageFormat::Icns => {
            return encode_icns(img, options);
        }
        ImageFormat::Svg => {
            return Err(AeconvError::UnsupportedOperation(
                "SVG encoding not supported (use vectorization)".to_string(),
            ));
        }
    }

    Ok(buf.into_inner())
}

/// Encode ICO with multiple resolutions using ico crate
fn encode_ico(img: &DynamicImage, _options: &EncodeOptions) -> Result<Vec<u8>> {
    // Standard ICO sizes
    let sizes = [16, 24, 32, 48, 64, 128, 256];
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    let mut has_entries = false;

    for &size in &sizes {
        if img.width() >= size && img.height() >= size {
            let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
            let rgba = resized.to_rgba8();

            let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());

            icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);
            has_entries = true;
        }
    }

    if !has_entries {
        // If image is smaller than 16x16, just use original size
        let rgba = img.to_rgba8();
        let icon_image = ico::IconImage::from_rgba_data(img.width(), img.height(), rgba.into_raw());

        icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);
    }

    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    icon_dir
        .write(&mut cursor)
        .map_err(|e| AeconvError::EncodingFailed {
            format: "ICO".to_string(),
            reason: e.to_string(),
        })?;

    Ok(buf)
}

/// Encode ICNS with multiple representations
fn encode_icns(img: &DynamicImage, _options: &EncodeOptions) -> Result<Vec<u8>> {
    // For ICNS, we'll create a basic ICNS file with PNG data for each size
    // ICNS format is a container format with typed entries
    let mut buf = Vec::new();
    let mut has_data = false;

    // Standard ICNS sizes with their type codes
    let sizes: Vec<(u32, &[u8; 4])> = vec![
        (16, b"it32"),   // 16x16
        (32, b"it32"),   // 32x32
        (48, b"it32"),   // 48x48
        (128, b"ic08"),  // 128x128
        (256, b"ic09"),  // 256x256
        (512, b"ic10"),  // 512x512
        (1024, b"ic11"), // 1024x1024
    ];

    for (size, icon_type) in sizes {
        if img.width() >= size && img.height() >= size {
            let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
            let png_data = encode_png_bytes(&resized)?;

            // Write ICNS entry: type (4 bytes) + length (4 bytes, big endian) + data
            buf.extend_from_slice(icon_type);
            let len = (png_data.len() + 8) as u32; // data + header
            buf.extend_from_slice(&len.to_be_bytes());
            buf.extend_from_slice(&png_data);
            has_data = true;
        }
    }

    // Also add Retina variants (@2x)
    let retina_sizes: Vec<(u32, &[u8; 4])> = vec![
        (32, b"ic04"),  // 16x16@2x
        (64, b"ic05"),  // 32x32@2x
        (256, b"ic06"), // 128x128@2x
        (512, b"ic07"), // 256x256@2x
    ];

    for (size, icon_type) in retina_sizes {
        if img.width() >= size && img.height() >= size {
            let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
            let png_data = encode_png_bytes(&resized)?;

            buf.extend_from_slice(icon_type);
            let len = (png_data.len() + 8) as u32;
            buf.extend_from_slice(&len.to_be_bytes());
            buf.extend_from_slice(&png_data);
            has_data = true;
        }
    }

    if !has_data {
        return Err(AeconvError::IcnsError(
            "No valid icon sizes generated".to_string(),
        ));
    }

    Ok(buf)
}

/// Helper to encode as PNG bytes
fn encode_png_bytes(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder
        .write_image(
            img.as_bytes(),
            img.width(),
            img.height(),
            ExtendedColorType::from(img.color()),
        )
        .map_err(|e| AeconvError::EncodingFailed {
            format: "PNG".to_string(),
            reason: e.to_string(),
        })?;
    Ok(buf.into_inner())
}

/// Convert image to target format with proper color space handling
pub fn convert_image(
    decoded: &DecodedImage,
    target_format: ImageFormat,
    options: &EncodeOptions,
) -> Result<Vec<u8>> {
    let mut img = decoded.image.clone();
    let mut metadata = preserve_metadata(&decoded.metadata, target_format);

    // Handle transparency for target format before metadata is adjusted to target capabilities.
    if !target_format.supports_transparency() && decoded.image.color().has_alpha() {
        img = flatten_alpha(&img, options.background_color);
        metadata.has_alpha = false;
    }

    // Handle color type conversion for specific formats
    img = prepare_for_format(img, target_format)?;

    // Encode
    let mut encode_options = options.clone();
    encode_options.preserve_transparency =
        metadata.has_alpha && target_format.supports_transparency();

    encode_image(&img, target_format, &encode_options)
}

/// Flatten alpha channel against the requested background color.
fn flatten_alpha(img: &DynamicImage, background: RgbColor) -> DynamicImage {
    let rgba = img.to_rgba8();
    let mut rgb = image::ImageBuffer::new(img.width(), img.height());

    let background = background.channels();
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let alpha = u16::from(pixel[3]);
        let inverse_alpha = u16::from(u8::MAX) - alpha;
        let composite = |source: u8, background: u8| {
            ((u16::from(source) * alpha + u16::from(background) * inverse_alpha + 127) / 255) as u8
        };
        let r = composite(pixel[0], background[0]);
        let g = composite(pixel[1], background[1]);
        let b = composite(pixel[2], background[2]);
        rgb.put_pixel(x, y, image::Rgb([r, g, b]));
    }

    DynamicImage::ImageRgb8(rgb)
}

/// Prepare image for specific target format
fn prepare_for_format(img: DynamicImage, target_format: ImageFormat) -> Result<DynamicImage> {
    match target_format {
        ImageFormat::Jpeg => {
            // JPEG doesn't support alpha, convert to RGB
            if img.color().has_alpha() {
                Ok(img.to_rgb8().into())
            } else {
                Ok(img)
            }
        }
        ImageFormat::Gif => {
            // GIF uses palette, convert to RGBA for encoder
            Ok(img.to_rgba8().into())
        }
        ImageFormat::Bmp => {
            // BMP typically uses BGR or BGRA - use RGB/RGBA as fallback
            if img.color().has_alpha() {
                Ok(img.to_rgba8().into())
            } else {
                Ok(img.to_rgb8().into())
            }
        }
        ImageFormat::Ico | ImageFormat::Icns => {
            // Icons need RGBA
            Ok(img.to_rgba8().into())
        }
        _ => Ok(img),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_short_hex_background_color() {
        assert_eq!(
            "#f08".parse::<RgbColor>().unwrap(),
            RgbColor::new(255, 0, 136)
        );
    }

    #[test]
    fn parses_long_hex_background_color_case_insensitively() {
        assert_eq!(
            "#aBcD09".parse::<RgbColor>().unwrap(),
            RgbColor::new(171, 205, 9)
        );
        assert_eq!(
            "#ABCD09".parse::<RgbColor>().unwrap(),
            RgbColor::new(171, 205, 9)
        );
    }

    #[test]
    fn parses_comma_separated_rgb_background_color() {
        assert_eq!(
            "255,0,128".parse::<RgbColor>().unwrap(),
            RgbColor::new(255, 0, 128)
        );
    }

    #[test]
    fn rejects_invalid_hex_background_colors() {
        for value in ["#", "#12", "#12345", "#ggg", "fff"] {
            assert!(value.parse::<RgbColor>().is_err(), "accepted {value}");
        }
    }

    #[test]
    fn rejects_malformed_or_out_of_range_rgb_background_colors() {
        for value in ["1,2", "1,2,3,4", "a,2,3", "1,,3", "256,0,0", "-1,2,3"] {
            assert!(value.parse::<RgbColor>().is_err(), "accepted {value}");
        }
    }

    #[test]
    fn flatten_alpha_composites_transparent_semi_transparent_and_opaque_pixels() {
        let mut img = image::ImageBuffer::new(3, 1);
        img.put_pixel(0, 0, image::Rgba([200, 100, 50, 0]));
        img.put_pixel(1, 0, image::Rgba([110, 120, 130, 128]));
        img.put_pixel(2, 0, image::Rgba([7, 8, 9, 255]));

        let dynamic = DynamicImage::ImageRgba8(img);
        let flattened = flatten_alpha(&dynamic, RgbColor::new(10, 20, 30)).to_rgb8();

        assert_eq!(flattened.get_pixel(0, 0), &image::Rgb([10, 20, 30]));
        assert_eq!(flattened.get_pixel(1, 0), &image::Rgb([60, 70, 80]));
        assert_eq!(flattened.get_pixel(2, 0), &image::Rgb([7, 8, 9]));

        let rounding_input = DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgba([1, 0, 0, 128]),
        ));
        let rounded = flatten_alpha(&rounding_input, RgbColor::new(0, 0, 0)).to_rgb8();
        assert_eq!(rounded.get_pixel(0, 0), &image::Rgb([1, 0, 0]));
    }

    #[test]
    fn default_background_is_white() {
        assert_eq!(RgbColor::default(), RgbColor::new(255, 255, 255));

        let transparent = DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgba([0, 0, 0, 0]),
        ));
        let flattened = flatten_alpha(&transparent, RgbColor::default()).to_rgb8();
        assert_eq!(flattened.get_pixel(0, 0), &image::Rgb([255, 255, 255]));
    }

    #[test]
    fn transparent_image_keeps_alpha_when_target_supports_it() {
        let image = DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgba([10, 20, 30, 0]),
        ));
        let decoded = DecodedImage {
            metadata: extract_metadata(&image, ImageFormat::Png),
            image,
            source_format: ImageFormat::Png,
            _frames: Vec::new(),
        };
        let encoded = convert_image(&decoded, ImageFormat::Png, &EncodeOptions::default()).unwrap();
        let converted = image::load_from_memory(&encoded).unwrap().to_rgba8();
        assert_eq!(converted.get_pixel(0, 0), &image::Rgba([10, 20, 30, 0]));
    }
}
