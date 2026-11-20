use image::{GrayImage, Rgba, RgbaImage};

/// Apply Gaussian blur to a single-channel (grayscale/alpha) buffer in-place.
/// Uses fastblur's single-channel implementation.
pub fn blur_single_channel(data: &mut Vec<u8>, width: usize, height: usize, sigma: f32) {
    if sigma <= 0.0 || data.is_empty() {
        return;
    }
    fastblur::gaussian_blur_asymmetric_single_channel(data, width, height, sigma, sigma);
}

/// Apply Gaussian blur to the alpha channel only of an RGBA image.
/// RGB values are preserved; only alpha is blurred.
pub fn blur_alpha_only(img: &RgbaImage, sigma: f32) -> RgbaImage {
    if sigma <= 0.0 {
        return img.clone();
    }

    let (w, h) = img.dimensions();
    let w_us = w as usize;
    let h_us = h as usize;

    // Extract alpha as a Vec<u8>
    let mut alpha_data: Vec<u8> = (0..h_us * w_us)
        .map(|i| {
            let x = (i % w_us) as u32;
            let y = (i / w_us) as u32;
            img.get_pixel(x, y).0[3]
        })
        .collect();

    // Blur the alpha channel in-place
    blur_single_channel(&mut alpha_data, w_us, h_us, sigma);

    // Reconstruct RGBA with blurred alpha
    let mut result = RgbaImage::new(w, h);
    for i in 0..h_us * w_us {
        let x = (i % w_us) as u32;
        let y = (i / w_us) as u32;
        let p = img.get_pixel(x, y);
        result.put_pixel(x, y, Rgba([p.0[0], p.0[1], p.0[2], alpha_data[i]]));
    }

    result
}

/// Apply Gaussian blur to an entire RGBA image (all channels).
/// This blurs R, G, B channels independently plus alpha.
pub fn gaussian_blur_rgba(img: &RgbaImage, sigma: f32) -> RgbaImage {
    if sigma <= 0.0 {
        return img.clone();
    }

    let (w, h) = img.dimensions();
    let w_us = w as usize;
    let h_us = h as usize;

    let mut r_data: Vec<u8> = Vec::with_capacity(w_us * h_us);
    let mut g_data: Vec<u8> = Vec::with_capacity(w_us * h_us);
    let mut b_data: Vec<u8> = Vec::with_capacity(w_us * h_us);
    let mut a_data: Vec<u8> = Vec::with_capacity(w_us * h_us);

    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            r_data.push(p.0[0]);
            g_data.push(p.0[1]);
            b_data.push(p.0[2]);
            a_data.push(p.0[3]);
        }
    }

    blur_single_channel(&mut r_data, w_us, h_us, sigma);
    blur_single_channel(&mut g_data, w_us, h_us, sigma);
    blur_single_channel(&mut b_data, w_us, h_us, sigma);
    blur_single_channel(&mut a_data, w_us, h_us, sigma);

    let mut result = RgbaImage::new(w, h);
    for i in 0..h_us * w_us {
        let x = (i % w_us) as u32;
        let y = (i / w_us) as u32;
        result.put_pixel(x, y, Rgba([
            r_data[i],
            g_data[i],
            b_data[i],
            a_data[i],
        ]));
    }

    result
}

/// Blur a grayscale image in-place wrapper (returns a new image).
pub fn blur_gray(img: &GrayImage, sigma: f32) -> GrayImage {
    if sigma <= 0.0 {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let w_us = w as usize;
    let h_us = h as usize;
    let mut data: Vec<u8> = img.pixels().map(|p| p.0[0]).collect();
    blur_single_channel(&mut data, w_us, h_us, sigma);

    let mut result = GrayImage::new(w, h);
    for i in 0..h_us * w_us {
        let x = (i % w_us) as u32;
        let y = (i / w_us) as u32;
        result.put_pixel(x, y, image::Luma([data[i]]));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_blur_zero_sigma_returns_clone() {
        let mut img = RgbaImage::new(8, 8);
        img.put_pixel(4, 4, Rgba([255, 0, 0, 255]));
        let blurred = blur_alpha_only(&img, 0.0);
        assert_eq!(blurred.get_pixel(4, 4).0, [255, 0, 0, 255]);
    }

    #[test]
    fn test_blur_spreads_alpha() {
        let mut img = RgbaImage::new(16, 16);
        img.put_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let blurred = blur_alpha_only(&img, 3.0);
        let center = blurred.get_pixel(8, 8).0[3];
        assert!(center > 0);
        let neighbor = blurred.get_pixel(9, 8).0[3];
        assert!(neighbor > 0, "neighbor should have some alpha after blur");
    }

    #[test]
    fn test_blur_preserves_rgb() {
        let mut img = RgbaImage::new(16, 16);
        for x in 0..16 {
            for y in 0..16 {
                img.put_pixel(x, y, Rgba([100, 50, 200, 128]));
            }
        }
        let blurred = blur_alpha_only(&img, 2.0);
        let p = blurred.get_pixel(5, 5);
        assert_eq!(p.0[0], 100);
        assert_eq!(p.0[1], 50);
        assert_eq!(p.0[2], 200);
    }

    #[test]
    fn test_gaussian_blur_rgba_preserves_dimensions() {
        let img = RgbaImage::new(16, 16);
        let blurred = gaussian_blur_rgba(&img, 3.0);
        assert_eq!(blurred.dimensions(), (16, 16));
    }

    #[test]
    fn test_blur_gray() {
        let mut img = GrayImage::new(16, 16);
        img.put_pixel(8, 8, image::Luma([255]));
        let blurred = blur_gray(&img, 3.0);
        assert!(blurred.get_pixel(8, 8).0[0] > 0);
        assert!(blurred.get_pixel(9, 8).0[0] > 0);
    }
}
