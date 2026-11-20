use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::RgbaImage;

/// Generate a drop shadow.
///
/// Pipeline: extract alpha → color flood → offset → Gaussian blur → (optionally composite beneath)
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let (w, h) = input.dimensions();
    let color = params.parse_color()?;

    // 1. Extract alpha mask
    let alpha_mask = alpha::extract_alpha(input);

    // 2. Optionally scale the mask (simple dilation-based scaling)
    let scaled_mask = if params.scale > 1.0 {
        let radius = ((params.scale - 1.0) * 10.0).round() as u32;
        alpha::dilate_mask(&alpha_mask, w, h, radius)
    } else if params.scale < 1.0 {
        // Erosion approximation — for scale < 1.0, we don't erode in the simple version
        // but could be implemented with min-filter
        alpha_mask
    } else {
        alpha_mask
    };

    // 3. Color flood: create colored layer from mask at specified opacity
    let colored = alpha::color_mask(&scaled_mask, w, h, color, params.opacity);

    // 4. Offset
    let offset_img = composite::expand_canvas_offset(&colored, params.canvas_expand, params.offset_x, params.offset_y);

    // But we need to work within the expanded canvas
    // Actually, let's handle canvas expansion at the shadow layer level:
    // The shadow should be created in an expanded canvas so it's not clipped
    let expanded_input = if params.canvas_expand > 0 {
        crate::composite::expand_canvas(input, params.canvas_expand)
    } else {
        input.clone()
    };

    let (ew, eh) = expanded_input.dimensions();

    // Re-extract alpha from expanded input
    let expanded_alpha = alpha::extract_alpha(&expanded_input);

    // Scale
    let expanded_scaled = if params.scale > 1.0 {
        let radius = ((params.scale - 1.0) * 10.0).round() as u32;
        alpha::dilate_mask(&expanded_alpha, ew, eh, radius)
    } else {
        expanded_alpha
    };

    // Color flood
    let expanded_colored = alpha::color_mask(&expanded_scaled, ew, eh, color, params.opacity);

    // Offset
    let offset_colored = alpha::offset_image(&expanded_colored, params.offset_x, params.offset_y);

    // Blur
    let blurred = blur::blur_alpha_only(&offset_colored, params.blur);

    if params.shadow_only {
        Ok(blurred)
    } else {
        Ok(blurred)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn make_test_image(size: u32) -> RgbaImage {
        let mut img = RgbaImage::new(size, size);
        // Fill center quadrant with opaque red
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
        let result = crate::composite::composite_shadow_beneath(
            &crate::composite::expand_canvas(&input, params.canvas_expand),
            &shadow,
        );
        // Should be expanded
        assert!(result.width() > 64);
    }
}
