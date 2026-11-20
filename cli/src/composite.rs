use image::{Rgba, RgbaImage};

/// Composite a shadow layer beneath the original image.
/// The output canvas is the same size as the original.
/// Shadow is placed first (bottom), then original on top.
pub fn composite_shadow_beneath(original: &RgbaImage, shadow: &RgbaImage) -> RgbaImage {
    let (w, h) = original.dimensions();
    let mut result = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            // Start with shadow pixel
            let s = shadow.get_pixel(x, y);
            let o = original.get_pixel(x, y);

            // Alpha composite original over shadow
            let pixel = alpha_over(s, o);
            result.put_pixel(x, y, pixel);
        }
    }
    result
}

/// Composite a shadow layer on top of the original (for inner shadows).
pub fn composite_shadow_above(original: &RgbaImage, shadow: &RgbaImage) -> RgbaImage {
    let (w, h) = original.dimensions();
    let mut result = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let o = original.get_pixel(x, y);
            let s = shadow.get_pixel(x, y);
            // Shadow goes on top, darkening the original
            let pixel = alpha_over(o, s);
            result.put_pixel(x, y, pixel);
        }
    }
    result
}

/// Alpha-composite `top` over `bottom` using the source-over operator.
/// Returns the composited RGBA pixel.
pub fn alpha_over(bottom: &Rgba<u8>, top: &Rgba<u8>) -> Rgba<u8> {
    let ba = bottom.0[3] as f32 / 255.0;
    let ta = top.0[3] as f32 / 255.0;
    let out_a = ta + ba * (1.0 - ta);

    if out_a <= 0.0 {
        return Rgba([0, 0, 0, 0]);
    }

    let r = (top.0[0] as f32 * ta + bottom.0[0] as f32 * ba * (1.0 - ta)) / out_a;
    let g = (top.0[1] as f32 * ta + bottom.0[1] as f32 * ba * (1.0 - ta)) / out_a;
    let b = (top.0[2] as f32 * ta + bottom.0[2] as f32 * ba * (1.0 - ta)) / out_a;

    Rgba([
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    ])
}

/// Expand the canvas by `padding` pixels on all sides, placing the original
/// image centered in the expanded canvas.
pub fn expand_canvas(img: &RgbaImage, padding: u32) -> RgbaImage {
    if padding == 0 {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let new_w = w + 2 * padding;
    let new_h = h + 2 * padding;
    let mut result = RgbaImage::new(new_w, new_h);

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);
            result.put_pixel(x + padding, y + padding, *pixel);
        }
    }
    result
}

/// Expand the canvas by `padding` pixels on all sides, placing the given image
/// at the specified offset within the expanded canvas.
pub fn expand_canvas_offset(img: &RgbaImage, padding: u32, offset_x: i32, offset_y: i32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let new_w = w + 2 * padding;
    let new_h = h + 2 * padding;
    let mut result = RgbaImage::new(new_w, new_h);

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let pixel = img.get_pixel(x as u32, y as u32);
            let dst_x = (x + padding as i32 + offset_x).clamp(0, new_w as i32 - 1) as u32;
            let dst_y = (y + padding as i32 + offset_y).clamp(0, new_h as i32 - 1) as u32;
            result.put_pixel(dst_x, dst_y, *pixel);
        }
    }
    result
}

/// Fill the transparent areas of an image with a solid background color.
pub fn apply_background(img: &RgbaImage, bg: Rgba<u8>) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut result = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            if p.0[3] == 0 {
                result.put_pixel(x, y, bg);
            } else if p.0[3] < 255 {
                // Blend with background
                let composited = alpha_over(&bg, p);
                result.put_pixel(x, y, composited);
            } else {
                result.put_pixel(x, y, *p);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_over_fully_opaque() {
        let bottom = Rgba([100, 100, 100, 255]);
        let top = Rgba([200, 0, 0, 255]);
        let result = alpha_over(&bottom, &top);
        assert_eq!(result.0, [200, 0, 0, 255]);
    }

    #[test]
    fn test_alpha_over_fully_transparent_top() {
        let bottom = Rgba([100, 100, 100, 255]);
        let top = Rgba([200, 0, 0, 0]);
        let result = alpha_over(&bottom, &top);
        assert_eq!(result.0, [100, 100, 100, 255]);
    }

    #[test]
    fn test_alpha_over_half_alpha() {
        let bottom = Rgba([0, 0, 0, 255]);
        let top = Rgba([255, 255, 255, 128]);
        let result = alpha_over(&bottom, &top);
        // 128/255 ≈ 0.502
        // out_a = 0.502 + 1.0 * (1 - 0.502) = 1.0
        // r = (255 * 0.502 + 0 * 1.0 * 0.498) / 1.0 ≈ 128
        assert_eq!(result.0[3], 255);
        assert!(result.0[0] > 120 && result.0[0] < 135);
    }

    #[test]
    fn test_expand_canvas() {
        let mut img = RgbaImage::new(10, 10);
        img.put_pixel(5, 5, Rgba([255, 0, 0, 255]));
        let expanded = expand_canvas(&img, 5);
        assert_eq!(expanded.dimensions(), (20, 20));
        // Original pixel should now be at (10, 10)
        assert_eq!(expanded.get_pixel(10, 10).0, [255, 0, 0, 255]);
        // Padding area should be transparent
        assert_eq!(expanded.get_pixel(0, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn test_expand_canvas_zero_padding() {
        let img = RgbaImage::new(10, 10);
        let expanded = expand_canvas(&img, 0);
        assert_eq!(expanded.dimensions(), (10, 10));
    }

    #[test]
    fn test_apply_background() {
        let mut img = RgbaImage::new(4, 4);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        // rest is transparent
        let result = apply_background(&img, Rgba([255, 255, 255, 255]));
        assert_eq!(result.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(result.get_pixel(1, 0).0, [255, 255, 255, 255]);
    }
}
