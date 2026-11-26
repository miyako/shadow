use image::{Rgba, RgbaImage};

/// Extract the alpha channel as a grayscale (luminance) image where each pixel
/// value equals the alpha of the corresponding source pixel.
pub fn extract_alpha(img: &RgbaImage) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let mut alpha = vec![0u8; (w as usize) * (h as usize)];
    for y in 0..h {
        for x in 0..w {
            let idx = (y as usize) * (w as usize) + (x as usize);
            alpha[idx] = img.get_pixel(x, y).0[3];
        }
    }
    alpha
}

/// Apply optional binary threshold to the alpha mask.
/// Pixels with alpha > 128 become fully opaque (255); others become transparent (0).
pub fn threshold_alpha(alpha: &[u8], threshold: u8) -> Vec<u8> {
    alpha.iter().map(|&a| if a > threshold { 255 } else { 0 }).collect()
}

/// Flood-fill a mask with a given color, producing an RGBA image where
/// the alpha of each pixel is `mask_value * opacity / 255`.
pub fn color_mask(
    mask: &[u8],
    width: u32,
    height: u32,
    color: Rgba<u8>,
    opacity: f64,
) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let scale = (opacity * 255.0).round() as f64;
    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize) * (width as usize) + (x as usize);
            let m = mask[idx] as f64;
            let a = ((m * scale) / 255.0).round().clamp(0.0, 255.0) as u8;
            img.put_pixel(x, y, Rgba([color.0[0], color.0[1], color.0[2], a]));
        }
    }
    img
}

/// Translate (offset) a mask by (dx, dy) in pixels.
/// Positive dx = right, positive dy = down.
/// Areas outside the original mask become 0.
pub fn offset_mask(mask: &[u8], width: u32, height: u32, dx: i32, dy: i32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut result = vec![0u8; w * h];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let src_x = x - dx;
            let src_y = y - dy;
            if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                let src_idx = (src_y as usize) * w + (src_x as usize);
                let dst_idx = (y as usize) * w + (x as usize);
                result[dst_idx] = mask[src_idx];
            }
        }
    }
    result
}

/// Translate (offset) an RGBA image by (dx, dy) in pixels.
pub fn offset_image(img: &RgbaImage, dx: i32, dy: i32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut result = RgbaImage::new(w, h);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let src_x = x - dx;
            let src_y = y - dy;
            if src_x >= 0 && src_x < w as i32 && src_y >= 0 && src_y < h as i32 {
                let pixel = img.get_pixel(src_x as u32, src_y as u32);
                result.put_pixel(x as u32, y as u32, *pixel);
            }
        }
    }
    result
}

/// Invert a mask: 255 - value.
pub fn invert_mask(mask: &[u8]) -> Vec<u8> {
    mask.iter().map(|&v| 255 - v).collect()
}

/// Element-wise saturating subtraction of two masks: `a - b`, clamped at 0.
pub fn subtract_masks(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter()
        .zip(b.iter())
        .map(|(&va, &vb)| va.saturating_sub(vb))
        .collect()
}

/// Element-wise multiply of two masks (alpha multiply / bitwise AND equivalent).
pub fn multiply_masks(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter()
        .zip(b.iter())
        .map(|(&va, &vb)| {
            let result = (va as u32 * vb as u32) / 255;
            result as u8
        })
        .collect()
}

/// Dilate a mask with a circular structuring element of given radius.
/// Uses a simple box-dilation approximation for performance.
pub fn dilate_mask(mask: &[u8], width: u32, height: u32, radius: u32) -> Vec<u8> {
    if radius == 0 {
        return mask.to_vec();
    }
    let w = width as usize;
    let h = height as usize;
    let r = radius as usize;
    let mut result = vec![0u8; w * h];

    // For each output pixel, check if any pixel in the radius neighborhood is non-zero
    for y in 0..h {
        for x in 0..w {
            let y_start = y.saturating_sub(r);
            let y_end = (y + r + 1).min(h);
            let x_start = x.saturating_sub(r);
            let x_end = (x + r + 1).min(w);

            let mut max_val: u8 = 0;
            'inner: for ny in y_start..y_end {
                for nx in x_start..x_end {
                    let idx = ny * w + nx;
                    if mask[idx] > max_val {
                        max_val = mask[idx];
                        if max_val == 255 {
                            break 'inner;
                        }
                    }
                }
            }
            result[y * w + x] = max_val;
        }
    }
    result
}

/// Crop a mask to the bounding box of non-zero pixels.
/// Returns (offset_x, offset_y, cropped_mask, cropped_width, cropped_height).
pub fn crop_mask_to_bounds(
    mask: &[u8],
    width: u32,
    height: u32,
) -> (u32, u32, Vec<u8>, u32, u32) {
    let w = width as usize;
    let h = height as usize;

    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] > 0 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return (0, 0, vec![], 0, 0);
    }

    let cw = (max_x - min_x + 1) as u32;
    let ch = (max_y - min_y + 1) as u32;
    let mut cropped = vec![0u8; (cw as usize) * (ch as usize)];
    for y in 0..ch as usize {
        for x in 0..cw as usize {
            let src_idx = (min_y + y) * w + (min_x + x);
            cropped[y * cw as usize + x] = mask[src_idx];
        }
    }

    (min_x as u32, min_y as u32, cropped, cw, ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_image(alpha_grid: &[[u8; 4]; 4]) -> RgbaImage {
        let mut img = RgbaImage::new(4, 4);
        for (y, row) in alpha_grid.iter().enumerate() {
            for (x, &a) in row.iter().enumerate() {
                img.put_pixel(x as u32, y as u32, Rgba([255, 0, 0, a]));
            }
        }
        img
    }

    #[test]
    fn test_extract_alpha() {
        let img = make_image(&[
            [255, 128, 0, 0],
            [255, 128, 0, 0],
            [255, 128, 0, 0],
            [255, 128, 0, 0],
        ]);
        let alpha = extract_alpha(&img);
        assert_eq!(alpha, vec![255, 128, 0, 0, 255, 128, 0, 0, 255, 128, 0, 0, 255, 128, 0, 0]);
    }

    #[test]
    fn test_threshold() {
        let alpha = vec![255, 128, 100, 129];
        let thresh = threshold_alpha(&alpha, 128);
        assert_eq!(thresh, vec![255, 0, 0, 255]);
    }

    #[test]
    fn test_offset_mask() {
        let mask = vec![
            255, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
        ];
        let result = offset_mask(&mask, 4, 4, 1, 0);
        // Original pixel at (0,0)=255 moves to (1,0)
        assert_eq!(result[0], 0);
        assert_eq!(result[1], 255);
    }

    #[test]
    fn test_invert_mask() {
        let mask = vec![0, 128, 255];
        assert_eq!(invert_mask(&mask), vec![255, 127, 0]);
    }

    #[test]
    fn test_multiply_masks() {
        let a = vec![255, 128, 0];
        let b = vec![255, 255, 255];
        let result = multiply_masks(&a, &b);
        assert_eq!(result[0], 255);
        assert_eq!(result[1], 128); // 128*255/255 = 128
        assert_eq!(result[2], 0);
    }

    #[test]
    fn test_dilate_mask() {
        let mask = vec![
            0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
            0, 0, 255, 0, 0,
            0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];
        let result = dilate_mask(&mask, 5, 5, 1);
        // Center pixel and all 8 neighbors should be 255
        assert_eq!(result[2 * 5 + 2], 255); // center
        assert_eq!(result[1 * 5 + 2], 255); // above
        assert_eq!(result[3 * 5 + 2], 255); // below
        assert_eq!(result[2 * 5 + 1], 255); // left
        assert_eq!(result[2 * 5 + 3], 255); // right
        assert_eq!(result[0 * 5 + 0], 0);   // far corner
    }

    #[test]
    fn test_crop_mask_to_bounds() {
        let mask = vec![
            0, 0, 0, 0,
            0, 128, 0, 0,
            0, 0, 255, 0,
            0, 0, 0, 0,
        ];
        let (ox, oy, cropped, cw, ch) = crop_mask_to_bounds(&mask, 4, 4);
        assert_eq!(ox, 1);
        assert_eq!(oy, 1);
        assert_eq!(cw, 2);
        assert_eq!(ch, 2);
        assert_eq!(cropped, vec![128, 0, 0, 255]);
    }

    #[test]
    fn test_crop_empty() {
        let mask = vec![0u8; 16];
        let (_, _, cropped, cw, ch) = crop_mask_to_bounds(&mask, 4, 4);
        assert!(cropped.is_empty());
        assert_eq!(cw, 0);
        assert_eq!(ch, 0);
    }
}

