use crate::codecs::RgbColor;
use crate::engine::{ConversionEngine, ConversionRequest};
use crate::errors::{AeconvError, Result};
use crate::formats::{ImageFormat, format_from_extension};
use crate::output::{OutputConfig, validate_quality};
use crate::processing::RoundCorners;
use camino::Utf8PathBuf;
use std::fs;

/// Result of processing a single batch file.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BatchItemResult {
    pub input_path: Utf8PathBuf,
    pub output_path: Option<Utf8PathBuf>,
    pub status: BatchStatus,
    pub message: String,
}

/// Status of a single batch item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStatus {
    Success,
    Skipped,
    Error,
}

/// Aggregated result of a batch run.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct BatchSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub items: Vec<BatchItemResult>,
}

impl BatchSummary {
    pub fn has_errors(&self) -> bool {
        self.failed > 0
    }
}

/// Configuration for a batch conversion run.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub input_dir: Utf8PathBuf,
    pub output_dir: Utf8PathBuf,
    pub source_format: ImageFormat,
    pub target_format: ImageFormat,
    pub quality: Option<u8>,
    pub background_color: RgbColor,
    pub force: bool,
    pub resize: Option<(u32, u32)>,
    pub round_corners: Option<RoundCorners>,
    pub verbose: bool,
}

/// Run a batch conversion over all files in `input_dir` matching `source_format`.
///
/// Every file is converted independently through the regular
/// [`ConversionEngine::convert`] pipeline, so one damaged file never aborts
/// the whole batch. Results are collected per file and the final summary
/// reports succeeded / failed / skipped counts.
///
/// The output directory is created when missing. Existing output files are
/// skipped unless `force` is set, and an output path that would overwrite a
/// source file is always skipped.
pub fn run_batch(config: BatchConfig) -> Result<BatchSummary> {
    if !config.input_dir.exists() {
        return Err(AeconvError::FileNotFound {
            path: config.input_dir.to_string(),
        });
    }
    if !config.input_dir.is_dir() {
        return Err(AeconvError::InvalidArgument(format!(
            "Input path is not a directory: {}",
            config.input_dir
        )));
    }

    if config.output_dir.exists() && !config.output_dir.is_dir() {
        return Err(AeconvError::InvalidArgument(format!(
            "Output path is not a directory: {}",
            config.output_dir
        )));
    }
    fs::create_dir_all(&config.output_dir).map_err(AeconvError::IoError)?;

    let quality = validate_quality(config.quality)?;

    let target_ext = config.target_format.extensions().first().ok_or_else(|| {
        AeconvError::UnsupportedTargetFormat {
            format: config.target_format.name().to_string(),
        }
    })?;

    let mut entries: Vec<_> = fs::read_dir(&config.input_dir)
        .map_err(AeconvError::IoError)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .collect();
    entries.sort_by_key(|entry| entry.file_name());

    let total = entries.len();
    if total == 0 {
        println!(
            "Batch: directory '{}' is empty — nothing to convert.",
            config.input_dir
        );
        return Ok(BatchSummary::default());
    }

    let mut summary = BatchSummary {
        total,
        ..BatchSummary::default()
    };

    for entry in &entries {
        let input_path = match Utf8PathBuf::from_path_buf(entry.path().to_path_buf()) {
            Ok(path) => path,
            Err(_) => {
                summary.failed += 1;
                summary.items.push(BatchItemResult {
                    input_path: entry.path().to_string_lossy().into_owned().into(),
                    output_path: None,
                    status: BatchStatus::Error,
                    message: "path is not valid Unicode".to_string(),
                });
                println!(
                    "ERR: {} — path is not valid Unicode",
                    entry.path().display()
                );
                continue;
            }
        };

        // Only convert files whose extension matches the requested source format.
        if format_from_extension(input_path.extension().unwrap_or("")) != Some(config.source_format)
        {
            summary.skipped += 1;
            let message = format!(
                "skipped: extension does not match source format '{}'",
                config.source_format.name()
            );
            summary.items.push(BatchItemResult {
                input_path: input_path.clone(),
                output_path: None,
                status: BatchStatus::Skipped,
                message,
            });
            if config.verbose {
                println!(
                    "SKIP: {} — {}",
                    input_path,
                    summary.items.last().unwrap().message
                );
            }
            continue;
        }

        let stem = input_path.file_stem().ok_or_else(|| {
            AeconvError::InvalidArgument(format!("Input file has no name: {}", input_path))
        })?;
        let output_path = config.output_dir.join(format!("{}.{}", stem, target_ext));

        // Never overwrite a source file, even with --force.
        if output_path == input_path {
            summary.skipped += 1;
            let message = "skipped: output would overwrite the source file".to_string();
            summary.items.push(BatchItemResult {
                input_path: input_path.clone(),
                output_path: Some(output_path.clone()),
                status: BatchStatus::Skipped,
                message,
            });
            if config.verbose {
                println!(
                    "SKIP: {} — output would overwrite the source file",
                    input_path
                );
            }
            continue;
        }

        // Do not overwrite an existing output file unless --force is set.
        if output_path.exists() && !config.force {
            summary.skipped += 1;
            let message = format!(
                "skipped: output file already exists (use --force to overwrite): {}",
                output_path
            );
            summary.items.push(BatchItemResult {
                input_path: input_path.clone(),
                output_path: Some(output_path.clone()),
                status: BatchStatus::Skipped,
                message,
            });
            if config.verbose {
                println!(
                    "SKIP: {} — {}",
                    input_path,
                    summary.items.last().unwrap().message
                );
            }
            continue;
        }

        let request = ConversionRequest {
            input_path: input_path.clone(),
            target_format: config.target_format,
            config: OutputConfig {
                output_path: Some(output_path.clone()),
                force_overwrite: config.force,
                quality,
                background_color: config.background_color,
            },
            resize: config.resize,
            round_corners: config.round_corners,
        };

        match ConversionEngine::convert(request) {
            Ok(result) => {
                summary.succeeded += 1;
                summary.items.push(BatchItemResult {
                    input_path: input_path.clone(),
                    output_path: Some(result.output_path.clone()),
                    status: BatchStatus::Success,
                    message: format!("{} bytes", result.output_size),
                });
                if config.verbose {
                    println!(
                        "OK: {} -> {} ({} bytes, {} -> {})",
                        input_path,
                        result.output_path,
                        result.output_size,
                        config.source_format.name(),
                        config.target_format.name()
                    );
                } else {
                    println!("{}", result.output_path);
                }
            }
            Err(error) => {
                summary.failed += 1;
                let message = error.user_message();
                summary.items.push(BatchItemResult {
                    input_path: input_path.clone(),
                    output_path: None,
                    status: BatchStatus::Error,
                    message: message.clone(),
                });
                println!("ERR: {} — {}", input_path, message);
            }
        }
    }

    println!(
        "Batch complete: {} succeeded, {} failed, {} skipped of {} file(s).",
        summary.succeeded, summary.failed, summary.skipped, summary.total
    );

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::RgbColor;
    use crate::formats::ImageFormat;
    use camino::Utf8Path;

    fn write_png(path: &Utf8Path, width: u32, height: u32) {
        let img = image::ImageBuffer::from_pixel(width, height, image::Rgba([10, 20, 30, 255]));
        image::DynamicImage::ImageRgba8(img)
            .save(path.as_std_path())
            .unwrap();
    }

    fn write_bytes(path: &Utf8Path, bytes: &[u8]) {
        std::fs::write(path.as_std_path(), bytes).unwrap();
    }

    fn config_for(input_dir: &Utf8Path, output_dir: &Utf8Path) -> BatchConfig {
        BatchConfig {
            input_dir: input_dir.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            source_format: ImageFormat::Png,
            target_format: ImageFormat::WebP,
            quality: Some(80),
            background_color: RgbColor::default(),
            force: false,
            resize: None,
            round_corners: None,
            verbose: false,
        }
    }

    #[test]
    fn batch_png_to_webp_converts_all_files() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("a.png")).unwrap(),
            100,
            100,
        );
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("b.png")).unwrap(),
            50,
            60,
        );

        let mut config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );
        config.verbose = true;

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);

        let out_a = output_dir.path().join("a.webp");
        let out_b = output_dir.path().join("b.webp");
        assert!(out_a.exists());
        assert!(out_b.exists());

        let decoded_a = image::open(&out_a).unwrap();
        assert_eq!((decoded_a.width(), decoded_a.height()), (100, 100));
        let decoded_b = image::open(&out_b).unwrap();
        assert_eq!((decoded_b.width(), decoded_b.height()), (50, 60));
    }

    #[test]
    fn batch_png_to_jpeg_flattens_alpha() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("alpha.png")).unwrap(),
            10,
            10,
        );

        let mut config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );
        config.target_format = ImageFormat::Jpeg;

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 1);

        let decoded = image::open(output_dir.path().join("alpha.jpg"))
            .unwrap()
            .to_rgb8();
        // Opaque source pixels keep their color through flattening.
        assert_eq!(decoded.get_pixel(0, 0), &image::Rgb([10, 20, 30]));
    }

    #[test]
    fn batch_with_resize_changes_output_dimensions() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("in.png")).unwrap(),
            200,
            200,
        );

        let mut config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );
        config.resize = Some((80, 40));

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 1);

        let decoded = image::open(output_dir.path().join("in.webp")).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (80, 40));
    }

    #[test]
    fn batch_with_round_corners_makes_corners_transparent() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("in.png")).unwrap(),
            100,
            100,
        );

        let mut config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );
        config.target_format = ImageFormat::Png;
        config.round_corners = Some(RoundCorners::Pixels(20));

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 1);

        let decoded = image::open(output_dir.path().join("in.png"))
            .unwrap()
            .to_rgba8();
        assert_eq!(decoded.get_pixel(0, 0)[3], 0);
        assert_eq!(decoded.get_pixel(50, 50)[3], 255);
    }

    #[test]
    fn batch_resize_and_round_corners_combined() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("in.png")).unwrap(),
            200,
            200,
        );

        let mut config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );
        config.target_format = ImageFormat::Png;
        config.resize = Some((60, 60));
        config.round_corners = Some(RoundCorners::Pixels(15));

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 1);

        let decoded = image::open(output_dir.path().join("in.png"))
            .unwrap()
            .to_rgba8();
        assert_eq!((decoded.width(), decoded.height()), (60, 60));
        assert_eq!(decoded.get_pixel(0, 0)[3], 0);
        assert_eq!(decoded.get_pixel(30, 30)[3], 255);
    }

    #[test]
    fn batch_empty_directory_converts_nothing() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();

        let config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.total, 0);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);
        assert!(!summary.has_errors());
    }

    #[test]
    fn batch_missing_input_directory_is_an_error() {
        let output_dir = tempfile::tempdir().unwrap();
        let config = BatchConfig {
            input_dir: Utf8PathBuf::from("/nonexistent/batch-input"),
            output_dir: Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
            source_format: ImageFormat::Png,
            target_format: ImageFormat::WebP,
            quality: None,
            background_color: RgbColor::default(),
            force: false,
            resize: None,
            round_corners: None,
            verbose: false,
        };

        let result = run_batch(config);
        assert!(result.is_err());
        matches!(result.unwrap_err(), AeconvError::FileNotFound { .. });
    }

    #[test]
    fn batch_corrupted_image_does_not_abort_whole_batch() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("good.png")).unwrap(),
            100,
            100,
        );
        write_bytes(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("broken.png")).unwrap(),
            b"not a png file",
        );

        let config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 0);
        assert!(summary.has_errors());
        assert!(output_dir.path().join("good.webp").exists());
        assert!(!output_dir.path().join("broken.webp").exists());
    }

    #[test]
    fn batch_existing_output_without_force_is_skipped() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("in.png")).unwrap(),
            100,
            100,
        );
        // Pre-create the output file.
        write_bytes(
            &Utf8PathBuf::from_path_buf(output_dir.path().join("in.webp")).unwrap(),
            b"placeholder",
        );

        let config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 1);
        assert!(!summary.has_errors());
        // Placeholder untouched.
        assert_eq!(
            std::fs::read(output_dir.path().join("in.webp")).unwrap(),
            b"placeholder"
        );
    }

    #[test]
    fn batch_existing_output_with_force_is_overwritten() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("in.png")).unwrap(),
            100,
            100,
        );
        write_bytes(
            &Utf8PathBuf::from_path_buf(output_dir.path().join("in.webp")).unwrap(),
            b"placeholder",
        );

        let mut config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );
        config.force = true;

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.skipped, 0);
        // Output is now a real WebP, not the placeholder.
        let decoded = image::open(output_dir.path().join("in.webp")).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (100, 100));
    }

    #[test]
    fn batch_output_overwrite_of_source_is_skipped() {
        // Converting PNG to PNG into the same directory must not clobber sources.
        let dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(dir.path().join("in.png")).unwrap(),
            100,
            100,
        );

        let config = BatchConfig {
            input_dir: Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
            output_dir: Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap(),
            source_format: ImageFormat::Png,
            target_format: ImageFormat::Png,
            quality: None,
            background_color: RgbColor::default(),
            force: true,
            resize: None,
            round_corners: None,
            verbose: false,
        };

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.skipped, 1);
        assert!(!summary.has_errors());
        // Source file still present and unchanged.
        assert!(dir.path().join("in.png").exists());
    }

    #[test]
    fn batch_paths_with_spaces_and_unicode() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let unicode_dir = input_dir.path().join("папка с пробелом");
        std::fs::create_dir_all(&unicode_dir).unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(unicode_dir.join("изображение.png")).unwrap(),
            40,
            40,
        );

        let config = BatchConfig {
            input_dir: Utf8PathBuf::from_path_buf(unicode_dir).unwrap(),
            output_dir: Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
            source_format: ImageFormat::Png,
            target_format: ImageFormat::WebP,
            quality: None,
            background_color: RgbColor::default(),
            force: false,
            resize: None,
            round_corners: None,
            verbose: false,
        };

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.succeeded, 1);
        assert!(output_dir.path().join("изображение.webp").exists());
    }

    #[test]
    fn batch_non_matching_extension_is_skipped() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        write_png(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("image.png")).unwrap(),
            10,
            10,
        );
        write_bytes(
            &Utf8PathBuf::from_path_buf(input_dir.path().join("notes.txt")).unwrap(),
            b"not an image",
        );

        let config = config_for(
            &Utf8PathBuf::from_path_buf(input_dir.path().to_path_buf()).unwrap(),
            &Utf8PathBuf::from_path_buf(output_dir.path().to_path_buf()).unwrap(),
        );

        let summary = run_batch(config).unwrap();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.failed, 0);
        assert!(!summary.has_errors());
    }
}
