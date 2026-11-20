use crate::errors::{AeconvError, Result};
use image::{DynamicImage, ImageBuffer, Rgba};

/// Maximum allowed dimension for a single side of a resized image.
/// Prevents excessive memory allocation from unreasonable resize requests.
const MAX_RESIZE_DIMENSION: u32 = 16_384;

/// Maximum total pixel budget for a resized image (~256 megapixels).
/// Guards against allocating many hundreds of megabytes for a single frame.
const MAX_RESIZE_PIXELS: u64 = 268_435_456;

/// Rounded corners specification: radius either in pixels or as a percent of
/// the shorter side of the image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoundCorners {
    /// Radius as a percentage of `min(width, height)`.
    Percent(f64),
    /// Radius in pixels.
    Pixels(u32),
}

/// Resize an image to exact dimensions using a high-quality Lanczos3 filter.
///
/// Dimensions are validated to be non-zero and within a safe budget so that
/// no overflow or excessive allocation can occur.
pub fn resize_image(img: &DynamicImage, width: u32, height: u32) -> Result<DynamicImage> {
    if width == 0 || height == 0 {
        return Err(AeconvError::InvalidArgument(format!(
            "Invalid resize dimensions: {width}x{height} (both must be greater than zero)"
        )));
    }
    if width > MAX_RESIZE_DIMENSION || height > MAX_RESIZE_DIMENSION {
        return Err(AeconvError::InvalidArgument(format!(
            "Invalid resize dimensions: {width}x{height} (each side must not exceed {MAX_RESIZE_DIMENSION})"
        )));
    }
    if u64::from(width) * u64::from(height) > MAX_RESIZE_PIXELS {
        return Err(AeconvError::InvalidArgument(format!(
            "Invalid resize dimensions: {width}x{height} (exceeds the maximum pixel budget)"
        )));
    }

    let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    Ok(resized)
}

/// Apply rounded corners to an image.
///
/// Returns a new image with an alpha mask applied: the corners become
/// transparent with anti-aliased edges, while the image content outside the
/// corners is left unchanged. The original resolution is preserved.
///
/// The effective radius is clamped to half of the shorter side so that the
/// four corners never overlap and the image cannot turn fully transparent.
/// A radius of zero leaves the image unchanged.
pub fn apply_round_corners(img: &DynamicImage, corners: RoundCorners) -> Result<DynamicImage> {
    let (width, height) = (img.width(), img.height());
    let min_dim = u32::min(width, height) as f64;

    let radius = match corners {
        RoundCorners::Percent(percent) => {
            if percent.is_nan() || percent < 0.0 {
                return Err(AeconvError::InvalidArgument(format!(
                    "Invalid round-corners radius: {percent}% (must be a non-negative number)"
                )));
            }
            (percent / 100.0) * min_dim
        }
        RoundCorners::Pixels(pixels) => pixels as f64,
    };

    // Clamp so corners never overlap; the image can never become fully transparent.
    let radius = radius.min(min_dim / 2.0);
    if radius <= 0.0 {
        return Ok(img.clone());
    }

    let rgba = img.to_rgba8();
    let mut out = ImageBuffer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let mask = corner_mask_alpha(x, y, width, height, radius);
            let src = rgba.get_pixel(x, y);
            // Existing alpha and the corner mask combine multiplicatively so a
            // semi-transparent source pixel keeps its transparency on the edge:
            // out_alpha = src_alpha * mask_alpha / 255.
            let alpha = (u16::from(src[3]) * u16::from(mask) / 255) as u8;
            out.put_pixel(x, y, Rgba([src[0], src[1], src[2], alpha]));
        }
    }

    Ok(DynamicImage::ImageRgba8(out))
}

/// Alpha (0..=255) for one pixel within a rounded corner, or `255` when the
/// pixel lies outside every corner region.
///
/// Edges are anti-aliased with a smooth falloff over one pixel so corners stay
/// smooth without visible steps.
fn corner_mask_alpha(x: u32, y: u32, width: u32, height: u32, radius: f64) -> u8 {
    // Pixel centres (x+0.5, y+0.5) guarantee mirror symmetry across both axes:
    // distance from (x+0.5, y+0.5) to the top-left centre equals distance from
    // (width-1-x+0.5, y+0.5) to the top-right centre, and likewise vertically.
    let px = x as f64 + 0.5;
    let py = y as f64 + 0.5;
    let r = radius;
    let w = width as f64;
    let h = height as f64;

    // A pixel is affected by a corner if its centre lies inside the corner
    // square ([0, r] × [0, r] for top-left, etc.) OR within r + 1 of the
    // corner's circle centre (anti-aliased edge just outside the square).
    // Pixels outside every corner region remain fully opaque (255).
    let centres = [
        (r, r),         // top-left
        (w - r, r),     // top-right
        (r, h - r),     // bottom-left
        (w - r, h - r), // bottom-right
    ];
    let in_square = [
        px <= r && py <= r,         // TL
        px >= w - r && py <= r,     // TR
        px <= r && py >= h - r,     // BL
        px >= w - r && py >= h - r, // BR
    ];

    // Among all corners whose region contains this pixel, pick the one whose
    // circle centre is closest. This matters when corners touch (r == min/2)
    // and a pixel sits between two centres — the closer centre wins.
    let mut best_dist = f64::INFINITY;
    let mut best: Option<(f64, f64)> = None;

    for i in 0..4 {
        let (cx, cy) = centres[i];
        let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
        let in_region = in_square[i] || dist <= r + 1.0;
        if in_region && dist < best_dist {
            best_dist = dist;
            best = Some((cx, cy));
        }
    }

    match best {
        Some((cx, cy)) => circle_alpha(px, py, cx, cy, r),
        None => 255,
    }
}

/// Coverage alpha for one pixel against a quarter-circle of radius `r` centred
/// at `(cx, cy)`.
///
/// Pixels inside the circle are fully opaque; pixels more than one pixel
/// outside are fully transparent; pixels on the edge fade smoothly.
fn circle_alpha(px: f64, py: f64, cx: f64, cy: f64, r: f64) -> u8 {
    let dx = px - cx;
    let dy = py - cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist <= r {
        return 255;
    }
    if dist >= r + 1.0 {
        return 0;
    }
    // Smoothstep falloff over one pixel for a smooth, step-free edge.
    let t = dist - r;
    let smooth = t * t * (3.0 - 2.0 * t);
    (255.0 * (1.0 - smooth)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_rgba(width: u32, height: u32, pixel: Rgba<u8>) -> DynamicImage {
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(width, height, pixel))
    }

    #[test]
    fn resize_downscale_changes_dimensions() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = resize_image(&img, 50, 50).unwrap();
        assert_eq!((out.width(), out.height()), (50, 50));
    }

    #[test]
    fn resize_upscale_changes_dimensions() {
        let img = solid_rgba(10, 10, Rgba([10, 20, 30, 255]));
        let out = resize_image(&img, 100, 100).unwrap();
        assert_eq!((out.width(), out.height()), (100, 100));
    }

    #[test]
    fn resize_non_square_dimensions() {
        let img = solid_rgba(200, 100, Rgba([10, 20, 30, 255]));
        let out = resize_image(&img, 80, 40).unwrap();
        assert_eq!((out.width(), out.height()), (80, 40));
    }

    #[test]
    fn resize_rejects_zero_dimensions() {
        let img = solid_rgba(10, 10, Rgba([0, 0, 0, 255]));
        assert!(resize_image(&img, 0, 10).is_err());
        assert!(resize_image(&img, 10, 0).is_err());
    }

    #[test]
    fn resize_preserves_opaque_content() {
        let img = solid_rgba(100, 100, Rgba([255, 0, 0, 255]));
        let out = resize_image(&img, 50, 50).unwrap();
        let rgba = out.to_rgba8();
        let p = rgba.get_pixel(25, 25);
        assert_eq!(p[3], 255);
        assert!(p[0] > 200);
    }

    #[test]
    fn round_corners_pixels_keeps_center_opaque() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        assert_eq!(rgba.get_pixel(50, 50)[3], 255);
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        // Content outside the mask is unchanged.
        assert_eq!(rgba.get_pixel(50, 50).0, [10, 20, 30, 255]);
    }

    #[test]
    fn round_corners_percent_uses_shorter_side() {
        // Image is 200x100; 50% of the shorter side (100) = 50px radius.
        let img = solid_rgba(200, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Percent(50.0)).unwrap();
        let rgba = out.to_rgba8();
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        assert_eq!(rgba.get_pixel(100, 50)[3], 255);
    }

    #[test]
    fn round_corners_preserves_existing_alpha() {
        let mut buffer = ImageBuffer::new(100, 100);
        for (_, _, p) in buffer.enumerate_pixels_mut() {
            *p = Rgba([10, 20, 30, 100]);
        }
        let img = DynamicImage::ImageRgba8(buffer);
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        // Existing alpha is preserved (not raised to 255) outside the corners.
        assert_eq!(rgba.get_pixel(50, 50)[3], 100);
        // Corners become fully transparent regardless of prior alpha.
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn round_corners_has_anti_aliased_edge() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(40)).unwrap();
        let rgba = out.to_rgba8();
        // Pixel (20, 4) has centre (20.5, 4.5), distance to circle centre (40, 40)
        // is ~40.50, which is just outside r=40 but within r+1=41, so it carries
        // an intermediate alpha value.
        let alpha = rgba.get_pixel(20, 4)[3];
        assert!(
            alpha > 0 && alpha < 255,
            "expected anti-aliased edge, got {alpha}"
        );
    }

    #[test]
    fn round_corners_radius_zero_is_noop() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(0)).unwrap();
        assert_eq!(out.to_rgba8().get_pixel(0, 0)[3], 255);
    }

    #[test]
    fn round_corners_percent_zero_is_noop() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Percent(0.0)).unwrap();
        assert_eq!(out.to_rgba8().get_pixel(0, 0)[3], 255);
    }

    #[test]
    fn round_corners_large_radius_is_clamped() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(1000)).unwrap();
        let rgba = out.to_rgba8();
        // Clamped to 50 (half the shorter side): corners touch at the center.
        assert_eq!(rgba.get_pixel(50, 50)[3], 255);
        // The image must not turn fully transparent.
        let has_opaque = (0..100).any(|y| (0..100).any(|x| rgba.get_pixel(x, y)[3] > 0));
        assert!(has_opaque);
    }

    #[test]
    fn round_corners_rejects_negative_percent() {
        let img = solid_rgba(10, 10, Rgba([0, 0, 0, 255]));
        assert!(apply_round_corners(&img, RoundCorners::Percent(-5.0)).is_err());
    }

    #[test]
    fn round_corners_output_is_rgba_even_for_rgb_input() {
        // An RGB input gains an alpha channel so corners can be transparent.
        let img =
            DynamicImage::ImageRgb8(ImageBuffer::from_pixel(100, 100, image::Rgb([10, 20, 30])));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        assert!(matches!(out, DynamicImage::ImageRgba8(_)));
        assert_eq!(out.to_rgba8().get_pixel(0, 0)[3], 0);
    }

    // Source alpha 128 with mask 200 must combine multiplicatively:
    // (128 * 200) / 255 = 100 (integer division), not min(128, 200) = 128.
    #[test]
    fn round_corners_combines_alpha_multiplicatively() {
        // Formula check: src 128, mask 200 -> 100.
        assert_eq!((128u16 * 200u16) / 255u16, 100u16);

        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 128]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();

        // Pixel on the anti-aliased corner edge has an intermediate mask.
        // Pixel centre (0.5, 14.5) is at distance ~20.26 from corner centre
        // (20, 20): just outside r=20 but within r+1=21.
        let mask = corner_mask_alpha(0, 14, 100, 100, 20.0);
        assert!(
            mask > 0 && mask < 255,
            "expected intermediate mask, got {mask}"
        );
        // out = src_alpha * mask / 255 (multiplicative), not min(src, mask).
        let expected = ((128u16 * mask as u16) / 255u16) as u8;
        assert_eq!(rgba.get_pixel(0, 14)[3], expected);
    }

    #[test]
    fn round_corners_alpha_zero_stays_zero() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 0]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        // mask 255 at the centre, src alpha 0 -> 0.
        assert_eq!(rgba.get_pixel(50, 50)[3], 0);
        // mask 0 in the corner -> 0.
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn round_corners_alpha_255_preserves_mask() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        // mask 255 at the centre -> 255.
        assert_eq!(rgba.get_pixel(50, 50)[3], 255);
        // mask 0 in the corner -> 0.
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        // src alpha 255 yields exactly the mask value on the edge.
        let mask = corner_mask_alpha(0, 16, 100, 100, 20.0);
        assert_eq!(rgba.get_pixel(0, 16)[3], mask);
    }

    #[test]
    fn round_corners_mask_255_preserves_source_alpha() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 128]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        // centre pixel has mask 255, so source alpha 128 is preserved.
        assert_eq!(rgba.get_pixel(50, 50)[3], 128);
    }

    #[test]
    fn round_corners_horizontal_symmetry() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        // Pixel (x, y) and (99-x, y) must have identical alpha (mirror symmetry).
        for y in 0..100 {
            for x in 0..50 {
                let left = rgba.get_pixel(x, y)[3];
                let right = rgba.get_pixel(99 - x, y)[3];
                assert_eq!(left, right, "horizontal asymmetry at ({x}, {y})");
            }
        }
    }

    #[test]
    fn round_corners_vertical_symmetry() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        // Pixel (x, y) and (x, 99-y) must have identical alpha (mirror symmetry).
        for x in 0..100 {
            for y in 0..50 {
                let top = rgba.get_pixel(x, y)[3];
                let bottom = rgba.get_pixel(x, 99 - y)[3];
                assert_eq!(top, bottom, "vertical asymmetry at ({x}, {y})");
            }
        }
    }

    #[test]
    fn round_corners_all_four_corners_symmetric() {
        let img = solid_rgba(100, 100, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(20)).unwrap();
        let rgba = out.to_rgba8();
        // All four corners at the same offset from their corner must match.
        for offset in 0..20 {
            let tl = rgba.get_pixel(offset, offset)[3];
            let tr = rgba.get_pixel(99 - offset, offset)[3];
            let bl = rgba.get_pixel(offset, 99 - offset)[3];
            let br = rgba.get_pixel(99 - offset, 99 - offset)[3];
            assert_eq!(tl, tr, "TL vs TR asymmetry at offset {offset}");
            assert_eq!(tl, bl, "TL vs BL asymmetry at offset {offset}");
            assert_eq!(tl, br, "TL vs BR asymmetry at offset {offset}");
        }
    }

    #[test]
    fn round_corners_fractional_radius_32x32() {
        // 32×32 at 20% → radius = 6.4 px (fractional, not rounded to 6).
        let img = solid_rgba(32, 32, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Percent(20.0)).unwrap();
        let rgba = out.to_rgba8();
        // Corner pixel must be transparent.
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        // Center must be opaque.
        assert_eq!(rgba.get_pixel(16, 16)[3], 255);
        // Horizontal symmetry.
        for y in 0..32 {
            for x in 0..16 {
                assert_eq!(
                    rgba.get_pixel(x, y)[3],
                    rgba.get_pixel(31 - x, y)[3],
                    "horizontal asymmetry at ({x}, {y})"
                );
            }
        }
        // Vertical symmetry.
        for x in 0..32 {
            for y in 0..16 {
                assert_eq!(
                    rgba.get_pixel(x, y)[3],
                    rgba.get_pixel(x, 31 - y)[3],
                    "vertical asymmetry at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn round_corners_size_32x32() {
        let img = solid_rgba(32, 32, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(8)).unwrap();
        let rgba = out.to_rgba8();
        assert_eq!((out.width(), out.height()), (32, 32));
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        assert_eq!(rgba.get_pixel(16, 16)[3], 255);
    }

    #[test]
    fn round_corners_size_64x64() {
        let img = solid_rgba(64, 64, Rgba([10, 20, 30, 255]));
        let out = apply_round_corners(&img, RoundCorners::Pixels(16)).unwrap();
        let rgba = out.to_rgba8();
        assert_eq!((out.width(), out.height()), (64, 64));
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        assert_eq!(rgba.get_pixel(32, 32)[3], 255);
    }
}
