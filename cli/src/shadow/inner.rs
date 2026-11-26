use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::RgbaImage;

/// Generate an inner shadow (inset shadow).
///
/// Pipeline: extract alpha → offset mask → blur → subtract from original → clip to shape
///
/// Blurring the shape's own alpha and subtracting it from the original yields a
/// gradient that is strongest at the shape's edge and fades to nothing toward
/// the centre — the classic inset-shadow look.
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let color = params.parse_color()?;

    // Expand canvas
    let expanded = if params.canvas_expand > 0 {
        composite::expand_canvas(input, params.canvas_expand)
    } else {
        input.clone()
    };
    let (ew, eh) = expanded.dimensions();

    // 1. Extract original alpha mask
    let alpha_mask = alpha::extract_alpha(&expanded);

    // 2. Offset the mask in the light-opposite direction
    let offset_mask = alpha::offset_mask(&alpha_mask, ew, eh, -params.offset_x, -params.offset_y);

    // 3. Blur the offset mask — the blur bleeds the shape's silhouette inward
    let blurred = blur::blur_mask(&offset_mask, ew, eh, params.blur);

    // 4. Inner shadow = original minus blurred (strong at edges, 0 in the centre)
    let inner_mask = alpha::subtract_masks(&alpha_mask, &blurred);

    // 5. Clip strictly to the shape interior
    let inner_mask = alpha::multiply_masks(&inner_mask, &alpha_mask);

    // 6. Color flood at the requested opacity
    let colored = alpha::color_mask(&inner_mask, ew, eh, color, params.opacity);

    Ok(colored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_inner_shadow() {
        let mut input = RgbaImage::new(64, 64);
        for y in 16..48 {
            for x in 16..48 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Inner,
            opacity: 0.5,
            offset_x: 5,
            offset_y: 5,
            blur: 5.0,
            canvas_expand: 10,
            ..Default::default()
        };
        let result = generate(&input, &params).unwrap();
        assert!(result.width() > 64);
        // Should have some shadow pixels inside
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
    fn test_inner_shadow_is_inset_gradient() {
        // Shadow must be strongest at the edge and fade toward the centre.
        let mut input = RgbaImage::new(128, 128);
        for y in 16..112 {
            for x in 16..112 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Inner,
            opacity: 0.8,
            offset_x: 0,
            offset_y: 0,
            blur: 8.0,
            canvas_expand: 20,
            ..Default::default()
        };
        let result = generate(&input, &params).unwrap();

        // Canvas is expanded by 20px, so the shape spans x = 36..132.
        let row = 20 + 64;
        let edge_x = 20 + 18; // just inside the left edge of the shape
        let mid_x = 20 + 64; // middle of the shape
        let edge_alpha = result.get_pixel(edge_x, row).0[3];
        let mid_alpha = result.get_pixel(mid_x, row).0[3];

        assert!(
            edge_alpha > mid_alpha,
            "edge alpha ({}) should exceed centre alpha ({})",
            edge_alpha,
            mid_alpha
        );
        assert!(edge_alpha > 0, "shadow should be present at the shape edge");
    }
}

