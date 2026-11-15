use crate::formats::ImageFormat;
use std::collections::HashMap;

/// Image metadata container
#[derive(Debug, Clone, Default)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub color_type: ColorType,
    pub bit_depth: u8,
    pub has_alpha: bool,
    pub is_animated: bool,
    pub frame_count: usize,
    pub dpi: Option<(f64, f64)>,
    pub color_profile: Option<Vec<u8>>,
    pub exif: HashMap<String, String>,
    #[allow(dead_code)]
    pub format_specific: HashMap<String, String>,
}

/// Color type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum ColorType {
    #[default]
    Grayscale,
    GrayscaleAlpha,
    Rgb,
    Rgba,
    Bgr,
    Bgra,
    Palette,
    PaletteAlpha,
}

impl ColorType {
    pub fn from_image_color_type(ct: image::ColorType) -> Self {
        match ct {
            image::ColorType::L8 | image::ColorType::L16 => ColorType::Grayscale,
            image::ColorType::La8 | image::ColorType::La16 => ColorType::GrayscaleAlpha,
            image::ColorType::Rgb8 | image::ColorType::Rgb16 | image::ColorType::Rgb32F => {
                ColorType::Rgb
            }
            image::ColorType::Rgba8 | image::ColorType::Rgba16 | image::ColorType::Rgba32F => {
                ColorType::Rgba
            }
            // Bgr/Bgra variants may not exist in all versions
            _ => ColorType::Rgb,
        }
    }

    pub fn has_alpha(&self) -> bool {
        matches!(
            self,
            ColorType::GrayscaleAlpha | ColorType::Rgba | ColorType::Bgra | ColorType::PaletteAlpha
        )
    }

    #[allow(dead_code)]
    pub fn channel_count(&self) -> u8 {
        match self {
            ColorType::Grayscale => 1,
            ColorType::GrayscaleAlpha => 2,
            ColorType::Rgb | ColorType::Bgr => 3,
            ColorType::Rgba | ColorType::Bgra => 4,
            ColorType::Palette | ColorType::PaletteAlpha => 4,
        }
    }
}

/// Extract metadata from a dynamic image
pub fn extract_metadata(img: &image::DynamicImage, _format: ImageFormat) -> ImageMetadata {
    let color_type = ColorType::from_image_color_type(img.color());

    ImageMetadata {
        width: img.width(),
        height: img.height(),
        color_type,
        bit_depth: bit_depth_from_color_type(img.color()),
        has_alpha: color_type.has_alpha() || img.color().has_alpha(),
        is_animated: false,
        frame_count: 1,
        dpi: None,
        color_profile: None,
        exif: HashMap::new(),
        format_specific: HashMap::new(),
    }
}

/// Get bit depth from color type
fn bit_depth_from_color_type(ct: image::ColorType) -> u8 {
    match ct {
        image::ColorType::L8
        | image::ColorType::La8
        | image::ColorType::Rgb8
        | image::ColorType::Rgba8
        | image::ColorType::L16
        | image::ColorType::La16
        | image::ColorType::Rgb16
        | image::ColorType::Rgba16 => 8,
        _ => 8,
    }
}

/// Preserve metadata when converting between formats
pub fn preserve_metadata(source: &ImageMetadata, target_format: ImageFormat) -> ImageMetadata {
    let mut target = source.clone();

    // Adjust for target format capabilities
    if !target_format.supports_transparency() {
        target.has_alpha = false;
    }

    if !target_format.supports_animation() {
        target.is_animated = false;
        target.frame_count = 1;
    }

    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_type_alpha() {
        assert!(ColorType::Rgba.has_alpha());
        assert!(ColorType::GrayscaleAlpha.has_alpha());
        assert!(!ColorType::Rgb.has_alpha());
        assert!(!ColorType::Grayscale.has_alpha());
    }
}
