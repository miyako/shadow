use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::{Rgba, RgbaImage};

/// Generate a curved shadow using a Bézier curve with a gradient fill.
///
/// The shadow appears to lift from one edge of the object.
/// Pipeline: extract alpha → determine lifting edge → draw Bézier curve →
/// fill area beneath curve with gradient → blur → composite
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let color = params.parse_color()?;

    // Expand canvas
    let expanded = if params.canvas_expand > 0 {
        composite::expand_canvas(input, params.canvas_expand)
    } else {
        input.clone()
    };
    let (w, h) = expanded.dimensions();

    // Extract alpha to find object bounds
    let alpha_mask = alpha::extract_alpha(&expanded);

    // Find bounding box of the object
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;
    for y in 0..h {
        for x in 0..w {
            if alpha_mask[(y as usize) * (w as usize) + (x as usize)] > 0 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return Ok(RgbaImage::new(w, h));
    }

    let curve_depth = params.curve_depth;

    // Create gradient shadow along the specified edge
    let mut shadow = RgbaImage::new(w, h);
    let shadow_color = [color.0[0], color.0[1], color.0[2]];

    match params.edge.as_str() {
        "bottom" => {
            // Shadow extends below the object's bottom edge
            let top = max_y;
            let bottom = h.min(max_y + (curve_depth as u32 * 3));
            let left = min_x;
            let right = max_x;
            for y in top..bottom {
                for x in left..=right {
                    // Bézier curve depth varies along x
                    let t = (x - left) as f64 / (right - left + 1) as f64;
                    let curve_y = top as f64
                        + curve_depth as f64 * 4.0 * t * (1.0 - t);
                    if y as f64 >= curve_y {
                        let dist = (y as f64 - curve_y) / (bottom - top + 1) as f64;
                        let alpha = ((1.0 - dist.min(1.0)) * params.opacity * 255.0).round() as u8;
                        if alpha > 0 {
                            shadow.put_pixel(x, y, Rgba([shadow_color[0], shadow_color[1], shadow_color[2], alpha]));
                        }
                    }
                }
            }
        }
        "top" => {
            let bottom = min_y;
            let top = 0.max(min_y.saturating_sub((curve_depth as u32 * 3)));
            let left = min_x;
            let right = max_x;
            for y in (top..bottom).rev() {
                for x in left..=right {
                    let t = (x - left) as f64 / (right - left + 1) as f64;
                    let curve_y = bottom as f64
                        - curve_depth as f64 * 4.0 * t * (1.0 - t);
                    if (y as f64) <= curve_y {
                        let dist = (curve_y - y as f64) / (bottom - top + 1) as f64;
                        let alpha = ((1.0 - dist.min(1.0)) * params.opacity * 255.0).round() as u8;
                        if alpha > 0 {
                            shadow.put_pixel(x, y, Rgba([shadow_color[0], shadow_color[1], shadow_color[2], alpha]));
                        }
                    }
                }
            }
        }
        "left" => {
            let right_edge = min_x;
            let left_edge = 0.max(min_x.saturating_sub((curve_depth as u32 * 3)));
            let top = min_y;
            let bottom = max_y;
            for x in (left_edge..right_edge).rev() {
                for y in top..=bottom {
                    let t = (y - top) as f64 / (bottom - top + 1) as f64;
                    let curve_x = right_edge as f64
                        - curve_depth as f64 * 4.0 * t * (1.0 - t);
                    if (x as f64) <= curve_x {
                        let dist = (curve_x - x as f64) / (right_edge - left_edge + 1) as f64;
                        let alpha = ((1.0 - dist.min(1.0)) * params.opacity * 255.0).round() as u8;
                        if alpha > 0 {
                            shadow.put_pixel(x, y, Rgba([shadow_color[0], shadow_color[1], shadow_color[2], alpha]));
                        }
                    }
                }
            }
        }
        "right" => {
            let left_edge = max_x;
            let right_edge = w.min(max_x + (curve_depth as u32 * 3));
            let top = min_y;
            let bottom = max_y;
            for x in left_edge..right_edge {
                for y in top..=bottom {
                    let t = (y - top) as f64 / (bottom - top + 1) as f64;
                    let curve_x = left_edge as f64
                        + curve_depth as f64 * 4.0 * t * (1.0 - t);
                    if x as f64 >= curve_x {
                        let dist = (x as f64 - curve_x) / (right_edge - left_edge + 1) as f64;
                        let alpha = ((1.0 - dist.min(1.0)) * params.opacity * 255.0).round() as u8;
                        if alpha > 0 {
                            shadow.put_pixel(x, y, Rgba([shadow_color[0], shadow_color[1], shadow_color[2], alpha]));
                        }
                    }
                }
            }
        }
        _ => {
            return Err(format!("invalid edge: {} (expected top/bottom/left/right)", params.edge));
        }
    }

    // Mask shadow to only areas outside or at the edge of the original object
    // (so the shadow doesn't cover the object itself)
    let masked_shadow = mask_outside_object(&shadow, &alpha_mask, w, h);

    // Blur the shadow
    let blurred = blur::blur_alpha_only(&masked_shadow, params.blur);

    Ok(blurred)
}

/// Keep only shadow pixels that are outside the original object's alpha mask.
fn mask_outside_object(shadow: &RgbaImage, alpha_mask: &[u8], w: u32, h: u32) -> RgbaImage {
    let mut result = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let idx = (y as usize) * (w as usize) + (x as usize);
            if alpha_mask[idx] == 0 {
                let p = shadow.get_pixel(x, y);
                result.put_pixel(x, y, *p);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_curved_shadow_bottom() {
        let mut input = RgbaImage::new(64, 64);
        for y in 16..48 {
            for x in 16..48 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Curved,
            opacity: 0.5,
            blur: 3.0,
            edge: "bottom".to_string(),
            curve_depth: 10.0,
            canvas_expand: 10,
            ..Default::default()
        };
        let result = generate(&input, &params).unwrap();
        assert!(result.width() > 64);
        // Should have some shadow pixels below the object
        let mut has_shadow = false;
        for y in 50..result.height() {
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
    fn test_curved_shadow_invalid_edge() {
        let input = RgbaImage::new(64, 64);
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Curved,
            edge: "diagonal".to_string(),
            ..Default::default()
        };
        assert!(generate(&input, &params).is_err());
    }
}
