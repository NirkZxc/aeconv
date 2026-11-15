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

    fn create_test_png_with_alpha() -> image::DynamicImage {
        let mut img = image::ImageBuffer::new(100, 100);
        for (x, _y, pixel) in img.enumerate_pixels_mut() {
            let alpha = if x < 50 { 255 } else { 0 };
            *pixel = image::Rgba([255, 0, 0, alpha]);
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
    fn test_convert_png_alpha_to_jpg_flattens() {
        let dynamic = create_test_png_with_alpha();

        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("test_alpha.png");
        dynamic.save(&input_path).unwrap();

        let request = ConversionRequest {
            input_path: Utf8PathBuf::from_path_buf(input_path).unwrap(),
            target_format: ImageFormat::Jpeg,
            config: OutputConfig::default(),
        };

        let result = ConversionEngine::convert(request).unwrap();
        assert!(result.output_path.exists());
        assert_eq!(result.target_format, ImageFormat::Jpeg);
        // JPEG doesn't support alpha, should be flattened
        assert!(!result.input_metadata.has_alpha || result.target_format == ImageFormat::Jpeg);
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
