use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::RgbaImage;

/// Generate a drop shadow.
///
/// Pipeline: expand canvas → extract alpha → scale (dilate) → color flood → offset → Gaussian blur
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let color = params.parse_color()?;

    // Expand canvas so the shadow isn't clipped
    let expanded_input = if params.canvas_expand > 0 {
        composite::expand_canvas(input, params.canvas_expand)
    } else {
        input.clone()
    };
    let (w, h) = expanded_input.dimensions();

    // 1. Extract alpha mask
    let alpha_mask = alpha::extract_alpha(&expanded_input);

    // 2. Optionally scale the mask (simple dilation-based scaling)
    let scaled_mask = if params.scale > 1.0 {
        let radius = ((params.scale - 1.0) * 10.0).round() as u32;
        alpha::dilate_mask(&alpha_mask, w, h, radius)
    } else {
        alpha_mask
    };

    // 3. Color flood: create colored layer from mask at specified opacity
    let colored = alpha::color_mask(&scaled_mask, w, h, color, params.opacity);

    // 4. Offset
    let offset_colored = alpha::offset_image(&colored, params.offset_x, params.offset_y);

    // 5. Gaussian blur (alpha channel only)
    let blurred = blur::blur_alpha_only(&offset_colored, params.blur);

    Ok(blurred)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn make_test_image(size: u32) -> RgbaImage {
        let mut img = RgbaImage::new(size, size);
        let q = size / 4;
        for y in q..size - q {
            for x in q..size - q {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        img
    }

    #[test]
    fn test_drop_shadow_shadow_only() {
        let input = make_test_image(64);
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Drop,
            opacity: 0.5,
            offset_x: 5,
            offset_y: 5,
            blur: 5.0,
            canvas_expand: 10,
            ..Default::default()
        };
        let result = generate(&input, &params).unwrap();
        // Result should be expanded
        assert!(result.width() >= 64);
        // Should have some non-transparent pixels (the shadow)
        let mut has_shadow = false;
        for y in 0..result.height() {
            for x in 0..result.width() {
                if result.get_pixel(x, y).0[3] > 0 {
                    has_shadow = true;
                    break;
                }
            }
            if has_shadow { break; }
        }
        assert!(has_shadow);
    }

    #[test]
    fn test_drop_shadow_composited() {
        let input = make_test_image(64);
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Drop,
            opacity: 0.5,
            offset_x: 5,
            offset_y: 5,
            blur: 5.0,
            canvas_expand: 10,
            shadow_only: false,
            ..Default::default()
        };
        let shadow = generate(&input, &params).unwrap();
        let expanded_input = crate::composite::expand_canvas(&input, params.canvas_expand);
        let result = crate::composite::composite_shadow_beneath(&expanded_input, &shadow);
        assert!(result.width() > 64);
    }
}
