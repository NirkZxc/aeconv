use crate::errors::{AeconvError, Result};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum ImageFormat {
    // Raster formats
    Png,
    Jpeg,
    Bmp,
    WebP,
    Tiff,
    Tga,
    Gif,
    Qoi,
    Pnm, // PPM/PGM/PBM/PNM

    // Icon formats
    Ico,
    Icns,

    // Vector formats
    Svg,
}

impl ImageFormat {
    /// Get all supported formats (for future use)
    pub fn _all() -> Vec<ImageFormat> {
        vec![
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Bmp,
            ImageFormat::WebP,
            ImageFormat::Tiff,
            ImageFormat::Tga,
            ImageFormat::Gif,
            ImageFormat::Qoi,
            ImageFormat::Pnm,
            ImageFormat::Ico,
            ImageFormat::Icns,
            ImageFormat::Svg,
        ]
    }

    /// Get raster formats (can be both source and target)
    pub fn raster_formats() -> Vec<ImageFormat> {
        vec![
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Bmp,
            ImageFormat::WebP,
            ImageFormat::Tiff,
            ImageFormat::Tga,
            ImageFormat::Gif,
            ImageFormat::Qoi,
            ImageFormat::Pnm,
        ]
    }

    /// Get icon formats
    pub fn icon_formats() -> Vec<ImageFormat> {
        vec![ImageFormat::Ico, ImageFormat::Icns]
    }

    /// Get vector formats
    pub fn vector_formats() -> Vec<ImageFormat> {
        vec![ImageFormat::Svg]
    }

    /// Check if format supports transparency
    pub fn supports_transparency(&self) -> bool {
        matches!(
            self,
            ImageFormat::Png
                | ImageFormat::WebP
                | ImageFormat::Tiff
                | ImageFormat::Tga
                | ImageFormat::Gif
                | ImageFormat::Ico
                | ImageFormat::Icns
        )
    }

    /// Check if format supports animation
    pub fn supports_animation(&self) -> bool {
        matches!(
            self,
            ImageFormat::Gif | ImageFormat::WebP | ImageFormat::Tiff
        )
    }

    /// Check if format supports multiple resolutions (ICO/ICNS)
    pub fn supports_multiple_resolutions(&self) -> bool {
        matches!(self, ImageFormat::Ico | ImageFormat::Icns)
    }

    /// Check if format is lossy
    pub fn is_lossy(&self) -> bool {
        matches!(self, ImageFormat::Jpeg | ImageFormat::WebP)
    }

    /// Get file extensions for this format
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            ImageFormat::Png => &["png"],
            ImageFormat::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            ImageFormat::Bmp => &["bmp", "dib"],
            ImageFormat::WebP => &["webp"],
            ImageFormat::Tiff => &["tiff", "tif"],
            ImageFormat::Tga => &["tga"],
            ImageFormat::Gif => &["gif"],
            ImageFormat::Qoi => &["qoi"],
            ImageFormat::Pnm => &["ppm", "pgm", "pbm", "pnm"],
            ImageFormat::Ico => &["ico"],
            ImageFormat::Icns => &["icns"],
            ImageFormat::Svg => &["svg", "svgz"],
        }
    }

    /// Get MIME type (for future use)
    pub fn _mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Tiff => "image/tiff",
            ImageFormat::Tga => "image/x-tga",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Qoi => "image/qoi",
            ImageFormat::Pnm => "image/x-portable-anymap",
            ImageFormat::Ico => "image/x-icon",
            ImageFormat::Icns => "image/icns",
            ImageFormat::Svg => "image/svg+xml",
        }
    }

    /// Get format name for display
    pub fn name(&self) -> &'static str {
        match self {
            ImageFormat::Png => "PNG",
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Bmp => "BMP",
            ImageFormat::WebP => "WebP",
            ImageFormat::Tiff => "TIFF",
            ImageFormat::Tga => "TGA",
            ImageFormat::Gif => "GIF",
            ImageFormat::Qoi => "QOI",
            ImageFormat::Pnm => "PNM (PPM/PGM/PBM)",
            ImageFormat::Ico => "ICO",
            ImageFormat::Icns => "ICNS",
            ImageFormat::Svg => "SVG",
        }
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for ImageFormat {
    type Err = AeconvError;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.to_lowercase();
        match s.as_str() {
            "png" => Ok(ImageFormat::Png),
            "jpg" | "jpeg" | "jpe" | "jfif" => Ok(ImageFormat::Jpeg),
            "bmp" | "dib" => Ok(ImageFormat::Bmp),
            "webp" => Ok(ImageFormat::WebP),
            "tiff" | "tif" => Ok(ImageFormat::Tiff),
            "tga" => Ok(ImageFormat::Tga),
            "gif" => Ok(ImageFormat::Gif),
            "qoi" => Ok(ImageFormat::Qoi),
            "ppm" | "pgm" | "pbm" | "pnm" => Ok(ImageFormat::Pnm),
            "ico" => Ok(ImageFormat::Ico),
            "icns" => Ok(ImageFormat::Icns),
            "svg" | "svgz" => Ok(ImageFormat::Svg),
            _ => Err(AeconvError::UnsupportedSourceFormat { format: s }),
        }
    }
}

/// Format capabilities
#[derive(Debug, Clone, Default)]
pub struct FormatCapabilities {
    pub can_decode: bool,
    pub can_encode: bool,
    #[allow(dead_code)]
    pub supports_transparency: bool,
    #[allow(dead_code)]
    pub supports_animation: bool,
    #[allow(dead_code)]
    pub supports_multiple_resolutions: bool,
    #[allow(dead_code)]
    pub is_lossy: bool,
    #[allow(dead_code)]
    pub max_dimensions: Option<(u32, u32)>,
}

/// Format registry for managing supported operations
pub struct FormatRegistry {
    capabilities: HashMap<ImageFormat, FormatCapabilities>,
    decode_extensions: HashMap<String, ImageFormat>,
    encode_extensions: HashMap<String, ImageFormat>,
}

impl FormatRegistry {
    fn new() -> Self {
        let mut registry = Self {
            capabilities: HashMap::new(),
            decode_extensions: HashMap::new(),
            encode_extensions: HashMap::new(),
        };
        registry.register_builtin();
        registry
    }

    fn register_builtin(&mut self) {
        // Raster formats - full support
        for fmt in ImageFormat::raster_formats() {
            let caps = FormatCapabilities {
                can_decode: true,
                can_encode: true,
                supports_transparency: fmt.supports_transparency(),
                supports_animation: fmt.supports_animation(),
                supports_multiple_resolutions: fmt.supports_multiple_resolutions(),
                is_lossy: fmt.is_lossy(),
                max_dimensions: None,
            };
            self.register_format(fmt, caps);
        }

        // Icon formats - ICO supports decode/encode, ICNS only encode (no decoder available)
        // ICO
        {
            let fmt = ImageFormat::Ico;
            let caps = FormatCapabilities {
                can_decode: true,
                can_encode: true,
                supports_transparency: fmt.supports_transparency(),
                supports_animation: false,
                supports_multiple_resolutions: fmt.supports_multiple_resolutions(),
                is_lossy: false,
                max_dimensions: Some((256, 256)),
            };
            self.register_format(fmt, caps);
        }
        // ICNS
        {
            let fmt = ImageFormat::Icns;
            let caps = FormatCapabilities {
                can_decode: false, // No ICNS decoder available
                can_encode: true,
                supports_transparency: fmt.supports_transparency(),
                supports_animation: false,
                supports_multiple_resolutions: fmt.supports_multiple_resolutions(),
                is_lossy: false,
                max_dimensions: Some((1024, 1024)),
            };
            self.register_format(fmt, caps);
        }

        // Vector formats - decode only (rasterization), encode not supported in Phase 1
        let svg_caps = FormatCapabilities {
            can_decode: true,
            can_encode: false, // SVG encoding would require vectorization (separate operation)
            supports_transparency: true,
            supports_animation: false,
            supports_multiple_resolutions: false,
            is_lossy: false,
            max_dimensions: None,
        };
        self.register_format(ImageFormat::Svg, svg_caps);
    }

    fn register_format(&mut self, fmt: ImageFormat, caps: FormatCapabilities) {
        for ext in fmt.extensions() {
            self.decode_extensions.insert(ext.to_lowercase(), fmt);
            if caps.can_encode {
                self.encode_extensions.insert(ext.to_lowercase(), fmt);
            }
        }
        self.capabilities.insert(fmt, caps);
    }

    pub fn get_capabilities(&self, fmt: ImageFormat) -> Option<&FormatCapabilities> {
        self.capabilities.get(&fmt)
    }

    pub fn can_decode(&self, fmt: ImageFormat) -> bool {
        self.capabilities
            .get(&fmt)
            .map(|c| c.can_decode)
            .unwrap_or(false)
    }

    pub fn can_encode(&self, fmt: ImageFormat) -> bool {
        self.capabilities
            .get(&fmt)
            .map(|c| c.can_encode)
            .unwrap_or(false)
    }

    pub fn format_from_extension(&self, ext: &str) -> Option<ImageFormat> {
        let ext = ext.to_lowercase();
        self.decode_extensions.get(&ext).copied()
    }

    pub fn _target_format_from_extension(&self, ext: &str) -> Option<ImageFormat> {
        let ext = ext.to_lowercase();
        self.encode_extensions.get(&ext).copied()
    }

    pub fn _all_decodable_formats(&self) -> Vec<ImageFormat> {
        self.capabilities
            .iter()
            .filter(|(_, c)| c.can_decode)
            .map(|(f, _)| *f)
            .collect()
    }

    pub fn _all_encodable_formats(&self) -> Vec<ImageFormat> {
        self.capabilities
            .iter()
            .filter(|(_, c)| c.can_encode)
            .map(|(f, _)| *f)
            .collect()
    }

    pub fn valid_conversions(&self) -> Vec<(ImageFormat, ImageFormat)> {
        let mut conversions = Vec::new();
        for (src_fmt, src_caps) in &self.capabilities {
            if !src_caps.can_decode {
                continue;
            }
            for (dst_fmt, dst_caps) in &self.capabilities {
                if !dst_caps.can_encode {
                    continue;
                }
                // SVG -> SVG not supported (would be identity)
                if *src_fmt == ImageFormat::Svg && *dst_fmt == ImageFormat::Svg {
                    continue;
                }
                // Allow all other combinations
                conversions.push((*src_fmt, *dst_fmt));
            }
        }
        conversions
    }
}

static REGISTRY: OnceLock<FormatRegistry> = OnceLock::new();

pub fn registry() -> &'static FormatRegistry {
    REGISTRY.get_or_init(FormatRegistry::new)
}

pub fn format_from_extension(ext: &str) -> Option<ImageFormat> {
    registry().format_from_extension(ext)
}

pub fn _target_format_from_extension(ext: &str) -> Option<ImageFormat> {
    registry()._target_format_from_extension(ext)
}

pub fn can_decode(fmt: ImageFormat) -> bool {
    registry().can_decode(fmt)
}

pub fn can_encode(fmt: ImageFormat) -> bool {
    registry().can_encode(fmt)
}

pub fn all_operations() -> Vec<String> {
    registry()
        .valid_conversions()
        .iter()
        .map(|(src, dst)| {
            format!(
                "{}to{}",
                src.name().to_lowercase(),
                dst.name().to_lowercase()
            )
        })
        .collect()
}
