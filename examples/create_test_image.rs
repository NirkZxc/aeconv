use image::{DynamicImage, ImageBuffer, Rgba};

fn main() {
    let mut img = ImageBuffer::new(100, 100);
    for (x, _y, pixel) in img.enumerate_pixels_mut() {
        if x < 50 {
            *pixel = Rgba([255, 0, 0, 255]); // Red
        } else {
            *pixel = Rgba([0, 0, 255, 255]); // Blue
        }
    }
    let dynamic = DynamicImage::ImageRgba8(img);
    dynamic.save("test_input.png").unwrap();
    println!("Created test_input.png");
}
