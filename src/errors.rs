use thiserror::Error;

#[derive(Error, Debug)]
pub enum AeconvError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Unsupported source format: {format}")]
    UnsupportedSourceFormat { format: String },

    #[error("Unsupported target format: {format}")]
    UnsupportedTargetFormat { format: String },

    #[error("Invalid conversion: {0} -> {1}")]
    #[allow(dead_code)]
    InvalidConversion(String, String),

    #[error("Corrupted image: {path}")]
    CorruptedImage { path: String },

    #[error("Output file already exists: {path} (use --force to overwrite)")]
    OutputExists { path: String },

    #[error("Permission denied: {path}")]
    #[allow(dead_code)]
    PermissionDenied { path: String },

    #[error("Encoding failed: {format}: {reason}")]
    EncodingFailed { format: String, reason: String },

    #[error("Decoding failed: {format}: {reason}")]
    DecodingFailed { format: String, reason: String },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image error: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("SVG error: {0}")]
    SvgError(String),

    #[error("ICNS error: {0}")]
    IcnsError(String),

    #[error("Invalid quality value: {value} (must be 1-100)")]
    InvalidQuality { value: u8 },

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Color profile error: {0}")]
    #[allow(dead_code)]
    ColorProfileError(String),
}

impl AeconvError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AeconvError::FileNotFound { .. } => 2,
            AeconvError::UnsupportedSourceFormat { .. } => 3,
            AeconvError::UnsupportedTargetFormat { .. } => 4,
            AeconvError::InvalidConversion { .. } => 5,
            AeconvError::CorruptedImage { .. } => 6,
            AeconvError::OutputExists { .. } => 7,
            AeconvError::PermissionDenied { .. } => 13,
            AeconvError::EncodingFailed { .. } => 8,
            AeconvError::DecodingFailed { .. } => 9,
            AeconvError::IoError(e) => match e.kind() {
                std::io::ErrorKind::NotFound => 2,
                std::io::ErrorKind::PermissionDenied => 13,
                _ => 1,
            },
            AeconvError::ImageError(_) => 10,
            AeconvError::SvgError(_) => 11,
            AeconvError::IcnsError(_) => 12,
            AeconvError::InvalidQuality { .. } => 14,
            AeconvError::InvalidArgument(_) => 15,
            AeconvError::UnsupportedOperation(_) => 16,
            AeconvError::ColorProfileError(_) => 17,
        }
    }
}

pub type Result<T> = std::result::Result<T, AeconvError>;

/// User-friendly error message without technical details
impl AeconvError {
    pub fn user_message(&self) -> String {
        match self {
            AeconvError::FileNotFound { path } => format!("File not found: {}", path),
            AeconvError::UnsupportedSourceFormat { format } => {
                format!("Unsupported source format: {}", format)
            }
            AeconvError::UnsupportedTargetFormat { format } => {
                format!("Unsupported target format: {}", format)
            }
            AeconvError::InvalidConversion(source, target) => {
                format!("Cannot convert {} to {}", source, target)
            }
            AeconvError::CorruptedImage { path } => {
                format!("Corrupted or unreadable image: {}", path)
            }
            AeconvError::OutputExists { path } => format!(
                "Output file already exists: {} (use --force to overwrite)",
                path
            ),
            AeconvError::PermissionDenied { path } => format!("Permission denied: {}", path),
            AeconvError::EncodingFailed { format, reason } => {
                format!("Failed to encode {}: {}", format, reason)
            }
            AeconvError::DecodingFailed { format, reason } => {
                format!("Failed to decode {}: {}", format, reason)
            }
            AeconvError::IoError(e) => format!("I/O error: {}", e),
            AeconvError::ImageError(e) => format!("Image processing error: {}", e),
            AeconvError::SvgError(e) => format!("SVG error: {}", e),
            AeconvError::IcnsError(e) => format!("ICNS error: {}", e),
            AeconvError::InvalidQuality { value } => {
                format!("Invalid quality value: {} (must be 1-100)", value)
            }
            AeconvError::InvalidArgument(msg) => format!("Invalid argument: {}", msg),
            AeconvError::UnsupportedOperation(msg) => format!("Unsupported operation: {}", msg),
            AeconvError::ColorProfileError(msg) => format!("Color profile error: {}", msg),
        }
    }
}
