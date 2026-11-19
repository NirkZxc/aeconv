use crate::codecs::{EncodeOptions, convert_image, decode_file};
use crate::errors::{AeconvError, Result};
use crate::formats::ImageFormat;
use crate::metadata::ImageMetadata;
use crate::output::{
    OutputConfig, check_output_path, determine_output_path, validate_quality, write_atomic,
};
use camino::Utf8PathBuf;

/// Conversion request
#[derive(Debug, Clone)]
pub struct ConversionRequest {
    pub input_path: Utf8PathBuf,
    pub target_format: ImageFormat,
    pub config: OutputConfig,
}

/// Conversion result
#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub output_path: Utf8PathBuf,
    pub source_format: ImageFormat,
    pub target_format: ImageFormat,
    pub input_metadata: ImageMetadata,
    pub output_size: u64,
}

/// Conversion engine
pub struct ConversionEngine;

impl ConversionEngine {
    /// Perform a single conversion
    pub fn convert(request: ConversionRequest) -> Result<ConversionResult> {
        // 1. Validate input file exists
        if !request.input_path.exists() {
            return Err(AeconvError::FileNotFound {
                path: request.input_path.to_string(),
            });
        }

        // 2. Validate quality
        let quality = validate_quality(request.config.quality)?;

        // 3. Detect source format
        let source_format = crate::codecs::detect_format(&request.input_path)?;

        // 4. Check if conversion is valid
        if !crate::formats::can_decode(source_format) {
            return Err(AeconvError::UnsupportedSourceFormat {
                format: source_format.name().to_string(),
            });
        }
        if !crate::formats::can_encode(request.target_format) {
            return Err(AeconvError::UnsupportedTargetFormat {
                format: request.target_format.name().to_string(),
            });
        }

        // 5. Determine output path
        let output_path =
            determine_output_path(&request.input_path, request.target_format, &request.config)?;

        // 6. Check overwrite
        check_output_path(&output_path, request.config.force_overwrite)?;

        // 7. Decode source image
        let decoded = decode_file(&request.input_path)?;

        // 8. Convert and encode
        let encode_options = EncodeOptions {
            quality,
            preserve_transparency: decoded.metadata.has_alpha
                && request.target_format.supports_transparency(),
            background_color: request.config.background_color,
            _dpi: decoded.metadata.dpi,
            _icc_profile: decoded.metadata.color_profile.clone(),
        };

        let output_data = convert_image(&decoded, request.target_format, &encode_options)?;

        // 9. Write output atomically
        write_atomic(&output_path, &output_data)?;

        // 10. Get output file size
        let output_size = std::fs::metadata(&output_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(ConversionResult {
            output_path,
            source_format,
            target_format: request.target_format,
            input_metadata: decoded.metadata,
            output_size,
        })
    }

    /// Batch convert multiple files (for future use)
    pub fn _convert_batch(requests: Vec<ConversionRequest>) -> Vec<Result<ConversionResult>> {
        use rayon::prelude::*;
        requests.into_par_iter().map(Self::convert).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::ImageFormat;

    fn create_test_png() -> image::DynamicImage {
        let mut img = image::ImageBuffer::new(100, 100);
        for (x, _y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 256) as u8, (x % 256) as u8, 128, 255]);
        }
        image::DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn test_convert_png_to_jpg() {
        let dynamic = create_test_png();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Jpeg,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::Jpeg);
        assert_eq!(result.source_format, ImageFormat::Png);
    }

    #[test]
    fn test_convert_png_to_webp() {
        let dynamic = create_test_png();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::WebP,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::WebP);
        assert_eq!(result.source_format, ImageFormat::Png);
    }

    #[test]
    fn test_convert_png_to_ico() {
        let dynamic = create_test_png();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Ico,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::Ico);
        assert_eq!(result.source_format, ImageFormat::Png);
    }

    #[test]
    fn test_ico_multi_resolution() {
        // Create a larger test image (256x256) to test all 7 standard ICO sizes
        let mut img = image::ImageBuffer::new(256, 256);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255]);
        }
        let dynamic = image::DynamicImage::ImageRgba8(img);

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test_large.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Ico,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());

        // Verify the ICO file contains all 7 standard sizes
        let ico_data = std::fs::read(&result.output_path).unwrap();
        let cursor = std::io::Cursor::new(ico_data);
        let dir = ico::IconDir::read(cursor).unwrap();

        // Should have 7 entries for 16, 24, 32, 48, 64, 128, 256
        assert_eq!(
            dir.entries().len(),
            7,
            "ICO should contain 7 standard resolutions"
        );

        let expected_sizes = [16, 24, 32, 48, 64, 128, 256];
        for (i, entry) in dir.entries().iter().enumerate() {
            let decoded = entry.decode().unwrap();
            assert_eq!(
                decoded.width(),
                expected_sizes[i] as u32,
                "ICO entry {} should be {}x{}",
                i,
                expected_sizes[i],
                expected_sizes[i]
            );
            assert_eq!(decoded.height(), expected_sizes[i] as u32);
        }
    }

    #[test]
    fn test_convert_svg_to_png() {
        let svg_data = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="blue"/></svg>"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.svg");
        std::fs::write(&input_path, svg_data).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Png,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::Png);
        assert_eq!(result.source_format, ImageFormat::Svg);
    }

    #[test]
    fn test_convert_png_alpha_to_jpg_flattens_against_default_white() {
        let dynamic = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgba([200, 10, 20, 0]),
        ));

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test_alpha.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Jpeg,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        let converted = image::open(&result.output_path).unwrap().to_rgb8();
        assert_eq!(result.target_format, ImageFormat::Jpeg);
        assert_eq!(converted.get_pixel(0, 0), &image::Rgb([255, 255, 255]));
    }

    #[test]
    fn test_convert_rgb_png_to_jpg_without_alpha() {
        let dynamic = image::DynamicImage::ImageRgb8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgb([40, 80, 120]),
        ));
        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("rgb.png");
        dynamic.save(&input_path).unwrap();

        let result = ConversionEngine::convert(ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Jpeg,
            config: OutputConfig::default(),
        })
        .unwrap();
        let converted = image::open(&result.output_path).unwrap().to_rgb8();
        let pixel = converted.get_pixel(0, 0);
        assert!(pixel[0].abs_diff(40) <= 2);
        assert!(pixel[1].abs_diff(80) <= 2);
        assert!(pixel[2].abs_diff(120) <= 2);
    }

    #[test]
    fn test_convert_png_alpha_to_jpg_uses_custom_background() {
        let dynamic = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgba([200, 10, 20, 0]),
        ));
        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("custom_background.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Jpeg,
            config: OutputConfig {
                background_color: crate::codecs::RgbColor::new(34, 51, 68),
                ..OutputConfig::default()
            },
        };

        let result = ConversionEngine::convert(request).unwrap();
        let converted = image::open(&result.output_path).unwrap().to_rgb8();
        let pixel = converted.get_pixel(0, 0);
        assert!(pixel[0].abs_diff(34) <= 2);
        assert!(pixel[1].abs_diff(51) <= 2);
        assert!(pixel[2].abs_diff(68) <= 2);
    }

    #[test]
    fn test_convert_png_to_bmp() {
        let dynamic = create_test_png();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Bmp,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::Bmp);
        assert_eq!(result.source_format, ImageFormat::Png);
    }

    #[test]
    fn test_convert_png_to_tiff() {
        let dynamic = create_test_png();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Tiff,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::Tiff);
        assert_eq!(result.source_format, ImageFormat::Png);
    }

    #[test]
    fn test_convert_png_to_gif() {
        let dynamic = create_test_png();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Gif,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::Gif);
        assert_eq!(result.source_format, ImageFormat::Png);
    }

    fn write_rgba_source(dir: &tempfile::TempDir, name: &str) -> Utf8PathBuf {
        let source = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgba([120, 80, 40, 128]),
        ));
        let path = dir.path().join(format!("{name}.png"));
        source.save(&path).unwrap();
        Utf8PathBuf::from_path_buf(path).unwrap()
    }

    #[test]
    fn test_convert_rgba_to_qoi_preserves_alpha_despite_background_option() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = write_rgba_source(&temp_dir, "qoi_alpha");
        let request = ConversionRequest {
            input_path,
            target_format: ImageFormat::Qoi,
            config: OutputConfig {
                background_color: crate::codecs::RgbColor::new(10, 20, 30),
                ..OutputConfig::default()
            },
        };

        let result = ConversionEngine::convert(request).unwrap();
        let converted = image::open(&result.output_path).unwrap().to_rgba8();
        assert_eq!(converted.get_pixel(0, 0), &image::Rgba([120, 80, 40, 128]));
    }

    #[test]
    fn test_convert_rgba_to_png_webp_and_tiff_preserves_alpha_with_background_option() {
        for (target_format, output_ext) in [
            (ImageFormat::Png, "png"),
            (ImageFormat::WebP, "webp"),
            (ImageFormat::Tiff, "tiff"),
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let input_path = write_rgba_source(&temp_dir, "source");
            let output_path = temp_dir.path().join(format!("output.{output_ext}"));
            let result = ConversionEngine::convert(ConversionRequest {
                input_path,
                target_format,
                config: OutputConfig {
                    output_path: Some(Utf8PathBuf::from_path_buf(output_path).unwrap()),
                    background_color: crate::codecs::RgbColor::new(10, 20, 30),
                    ..OutputConfig::default()
                },
            })
            .unwrap();
            let converted = image::open(&result.output_path).unwrap().to_rgba8();
            assert_eq!(converted.get_pixel(0, 0)[3], 128, "{target_format:?} alpha");
        }
    }

    #[test]
    fn test_convert_rgba_to_bmp_and_pnm_flattens_to_custom_background() {
        let temp_dir = tempfile::tempdir().unwrap();
        for target_format in [ImageFormat::Bmp, ImageFormat::Pnm] {
            let input_path = write_rgba_source(&temp_dir, &format!("{target_format:?}"));
            let result = ConversionEngine::convert(ConversionRequest {
                input_path,
                target_format,
                config: OutputConfig {
                    background_color: crate::codecs::RgbColor::new(34, 51, 68),
                    ..OutputConfig::default()
                },
            })
            .unwrap();
            let converted = image::open(&result.output_path).unwrap().to_rgb8();
            assert_eq!(
                converted.get_pixel(0, 0),
                &image::Rgb([77, 66, 54]),
                "{target_format} flattened pixel"
            );
        }
    }

    #[test]
    fn test_convert_rgba_to_tga_preserves_alpha_with_background_option() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = write_rgba_source(&temp_dir, "tga_alpha");
        let output_path = temp_dir.path().join("output.tga");
        let result = ConversionEngine::convert(ConversionRequest {
            input_path,
            target_format: ImageFormat::Tga,
            config: OutputConfig {
                output_path: Some(Utf8PathBuf::from_path_buf(output_path).unwrap()),
                background_color: crate::codecs::RgbColor::new(10, 20, 30),
                ..OutputConfig::default()
            },
        })
        .unwrap();
        let converted = image::open(&result.output_path).unwrap().to_rgba8();
        assert_eq!(converted.get_pixel(0, 0), &image::Rgba([120, 80, 40, 128]));
    }

    #[test]
    fn test_rgb_to_jpeg_ignores_custom_background() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("rgb_background.png");
        image::DynamicImage::ImageRgb8(image::ImageBuffer::from_pixel(
            1,
            1,
            image::Rgb([40, 80, 120]),
        ))
        .save(&input_path)
        .unwrap();
        let result = ConversionEngine::convert(ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Jpeg,
            config: OutputConfig {
                background_color: crate::codecs::RgbColor::new(0, 255, 0),
                ..OutputConfig::default()
            },
        })
        .unwrap();
        let converted = image::open(&result.output_path).unwrap().to_rgb8();
        let pixel = converted.get_pixel(0, 0);
        assert!(pixel[0].abs_diff(40) <= 2);
        assert!(pixel[1].abs_diff(80) <= 2);
        assert!(pixel[2].abs_diff(120) <= 2);
    }

    #[test]
    fn test_error_missing_input_file() {
        let request = ConversionRequest {
            input_path: Utf8PathBuf::from("/nonexistent/path/test.png"),
            target_format: ImageFormat::Jpeg,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request);
        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            crate::errors::AeconvError::FileNotFound { .. }
        );
    }

    #[test]
    fn test_error_unsupported_target() {
        let dynamic = create_test_png();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Svg, // SVG encoding not supported
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request);
        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            crate::errors::AeconvError::UnsupportedTargetFormat { .. }
        );
    }
}
