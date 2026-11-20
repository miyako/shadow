use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::RgbaImage;

/// Generate an inner shadow (inset shadow).
///
/// Pipeline: extract alpha → invert mask → offset inverted mask → intersect with original mask → blur → composite on top
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let (w, h) = input.dimensions();
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

    // 2. Invert the mask
    let inverted = alpha::invert_mask(&alpha_mask);

    // 3. Offset the inverted mask
    let offset_inverted = alpha::offset_mask(&inverted, ew, eh, params.offset_x, params.offset_y);

    // 4. Intersect (multiply) with original mask
    // This gives us the area inside the object that will receive the shadow
    let inner_mask = alpha::multiply_masks(&offset_inverted, &alpha_mask);

    // 5. Color flood
    let colored = alpha::color_mask(&inner_mask, ew, eh, color, params.opacity);

    // 6. Blur
    let blurred = blur::blur_alpha_only(&colored, params.blur);

    Ok(blurred)
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
}
