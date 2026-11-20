use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::RgbaImage;

/// Generate a contact shadow.
///
/// Pipeline: extract alpha → dilate (slightly) → color flood → minimal offset → tight blur
/// Contact shadows simulate ambient occlusion with zero offset and tight blur.
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

    // 1. Extract alpha
    let alpha_mask = alpha::extract_alpha(&expanded);

    // 2. Dilate slightly (creates wider base)
    let dilation_radius = if params.scale > 1.0 {
        ((params.scale - 1.0) * 5.0).round() as u32
    } else {
        1u32 // default slight dilation for contact shadow
    };
    let dilated = alpha::dilate_mask(&alpha_mask, ew, eh, dilation_radius);

    // 3. Color flood
    let colored = alpha::color_mask(&dilated, ew, eh, color, params.opacity);

    // 4. Minimal/zero offset (contact shadows have ~0 offset)
    // Use the provided offset_x/offset_y but they're typically 0 for contact
    let offset_img = alpha::offset_image(&colored, params.offset_x, params.offset_y);

    // 5. Tight blur (use the blur param, but contact shadows typically have smaller blur)
    let blurred = blur::blur_alpha_only(&offset_img, params.blur);

    Ok(blurred)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_contact_shadow() {
        let mut input = RgbaImage::new(64, 64);
        for y in 16..48 {
            for x in 16..48 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Contact,
            opacity: 0.5,
            blur: 3.0,
            offset_x: 0,
            offset_y: 0,
            canvas_expand: 10,
            ..Default::default()
        };
        let result = generate(&input, &params).unwrap();
        assert!(result.width() > 64);
        // Should have shadow pixels
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
