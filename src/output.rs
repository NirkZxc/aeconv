use crate::errors::{AeconvError, Result};
use crate::formats::ImageFormat;
use camino::{Utf8Path, Utf8PathBuf};

/// Output configuration
#[derive(Debug, Clone, Default)]
pub struct OutputConfig {
    pub output_path: Option<Utf8PathBuf>,
    pub force_overwrite: bool,
    pub quality: Option<u8>,
}

/// Determine output path based on input and configuration
pub fn determine_output_path(
    input_path: &Utf8Path,
    target_format: ImageFormat,
    config: &OutputConfig,
) -> Result<Utf8PathBuf> {
    // If explicit output path provided, use it
    if let Some(ref output) = config.output_path {
        return Ok(output.clone());
    }

    // Otherwise, create output next to input with new extension
    let parent = input_path
        .parent()
        .ok_or_else(|| AeconvError::InvalidArgument("Input path has no parent directory".into()))?;

    let stem = input_path
        .file_stem()
        .ok_or_else(|| AeconvError::InvalidArgument("Input path has no file name".into()))?;

    let new_extension =
        target_format
            .extensions()
            .first()
            .ok_or_else(|| AeconvError::UnsupportedTargetFormat {
                format: target_format.name().to_string(),
            })?;

    let output_filename = format!("{}.{}", stem, new_extension);
    let output_path = parent.join(output_filename);

    Ok(output_path)
}

/// Check if output file exists and handle overwrite
pub fn check_output_path(path: &Utf8Path, force: bool) -> Result<()> {
    if path.exists() {
        if !force {
            return Err(AeconvError::OutputExists {
                path: path.to_string(),
            });
        }
        // Remove existing file if force is enabled
        std::fs::remove_file(path).map_err(AeconvError::IoError)?;
    }
    Ok(())
}

/// Ensure parent directory exists
pub fn ensure_parent_dir(path: &Utf8Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        std::fs::create_dir_all(parent).map_err(AeconvError::IoError)?;
    }
    Ok(())
}

/// Write bytes to file atomically (write to temp, then rename)
pub fn write_atomic(path: &Utf8Path, data: &[u8]) -> Result<()> {
    ensure_parent_dir(path)?;

    // Create temp file in same directory
    let temp_path = path.with_extension(format!("{}.tmp", path.extension().unwrap_or("tmp")));

    std::fs::write(&temp_path, data).map_err(AeconvError::IoError)?;

    // Atomic rename
    std::fs::rename(&temp_path, path).map_err(AeconvError::IoError)?;

    Ok(())
}

/// Validate quality value
pub fn validate_quality(quality: Option<u8>) -> Result<Option<u8>> {
    if let Some(q) = quality {
        if q == 0 || q > 100 {
            return Err(AeconvError::InvalidQuality { value: q });
        }
        Ok(Some(q))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::ImageFormat;

    #[test]
    fn test_determine_output_path() {
        let input = Utf8Path::new("photo.jpg");
        let config = OutputConfig::default();
        let output = determine_output_path(input, ImageFormat::Png, &config).unwrap();
        assert_eq!(output.file_name(), Some("photo.png"));
    }

    #[test]
    fn test_determine_output_path_with_custom() {
        let input = Utf8Path::new("photo.jpg");
        let config = OutputConfig {
            output_path: Some(Utf8PathBuf::from("output.ico")),
            ..Default::default()
        };
        let output = determine_output_path(input, ImageFormat::Ico, &config).unwrap();
        assert_eq!(output.file_name(), Some("output.ico"));
    }

    #[test]
    fn test_validate_quality() {
        assert_eq!(validate_quality(Some(80)).unwrap(), Some(80));
        assert!(validate_quality(Some(0)).is_err());
        assert!(validate_quality(Some(101)).is_err());
        assert_eq!(validate_quality(None).unwrap(), None);
    }
}
