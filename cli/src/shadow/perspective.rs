use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::{Rgba, RgbaImage};

/// Generate a perspective shadow using homography projection.
///
/// The object is treated as being at elevation z = light_z.
/// A light source at (light_x, light_y, above) projects the silhouette onto
/// the ground plane (z = 0).
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let color = params.parse_color()?;

    // Expand canvas
    let expanded = if params.canvas_expand > 0 {
        composite::expand_canvas(input, params.canvas_expand)
    } else {
        input.clone()
    };
    let (w, h) = expanded.dimensions();

    // Extract alpha
    let alpha_mask = alpha::extract_alpha(&expanded);

    // Light source position
    let lx = params.light_x;
    let ly = params.light_y;
    let lz = params.light_z; // elevation of the object

    // If light_z is 0, the object is on the ground — no perspective shadow
    if lz.abs() < 0.001 {
        // Fall back to a simple drop shadow
        return crate::shadow::drop::generate(input, params);
    }

    // Project each pixel onto the ground plane
    // For a point (x, y) at height z = lz:
    //   x' = x - lz * (x - Lx) / lz = x - (x - Lx) = Lx  (if light at same height)
    // More correctly, with light at infinite distance in direction (lx, ly):
    //   x' = x + lz * lx / (light_height)
    // We'll use a simplified model: the shadow is projected along (lx, ly) direction
    // by a distance proportional to lz

    let mut shadow = RgbaImage::new(w, h);

    let proj_x = lx; // projection direction x component
    let proj_y = ly; // projection direction y component
    let proj_dist = lz; // projection distance factor

    // For perspective: shadow stretches and moves
    // The further from the "light source", the longer the shadow
    // Simple approach: for each opaque pixel, trace a shadow point

    // Calculate center of the object
    let mut cx_sum = 0.0f64;
    let mut cy_sum = 0.0f64;
    let mut count = 0u32;
    for y in 0..h {
        for x in 0..w {
            let idx = (y as usize) * (w as usize) + (x as usize);
            if alpha_mask[idx] > 128 {
                cx_sum += x as f64;
                cy_sum += y as f64;
                count += 1;
            }
        }
    }

    let (cx, cy) = if count > 0 {
        (cx_sum / count as f64, cy_sum / count as f64)
    } else {
        return Ok(RgbaImage::new(w, h));
    };

    // For each pixel in the shadow, determine its source
    // We use inverse mapping: for each output pixel, find where it comes from
    let max_proj = proj_dist * 2.0; // max projection distance

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            // Distance from center (normalized)
            let dx_c = x as f64 - cx;
            let dy_c = y as f64 - cy;

            // Project the silhouette: pixels further from center in projection direction
            // get stretched. We use a forward projection approach:
            // For each source pixel (sx, sy), the shadow pixel is at:
            //   shadow_x = sx + proj_x * proj_dist * (1 - dist_from_light / max_dist)
            // This creates a perspective effect

            // Inverse mapping: find source pixel
            // shadow_x = src_x + proj_x * proj_dist * factor(src_x, src_y)
            // Simplified: shadow is offset and stretched

            // Find what source pixel maps to this output pixel
            // Approximate inverse: shift back
            let src_x = x as f64 - proj_x * proj_dist / 10.0;
            let src_y = y as f64 - proj_y * proj_dist / 10.0;

            if src_x < 0.0 || src_x >= w as f64 || src_y < 0.0 || src_y >= h as f64 {
                continue;
            }

            let sx = src_x as usize;
            let sy = src_y as usize;
            let src_idx = sy * (w as usize) + sx;

            if alpha_mask[src_idx] > 0 {
                // Calculate opacity based on distance from center (further = more transparent)
                let dist_from_center = ((dx_c * dx_c + dy_c * dy_c)).sqrt();
                let fade = (1.0 - dist_from_center / (max_proj + dist_from_center)).max(0.0);
                let alpha = (params.opacity * fade * (alpha_mask[src_idx] as f64 / 255.0) * 255.0).round() as u8;

                if alpha > 0 {
                    let existing = shadow.get_pixel(x as u32, y as u32);
                    if alpha > existing.0[3] {
                        shadow.put_pixel(x as u32, y as u32, Rgba([color.0[0], color.0[1], color.0[2], alpha]));
                    }
                }
            }
        }
    }

    // Also add the projection stretch: pixels in the projection direction
    // get elongated
    let mut stretched = RgbaImage::new(w, h);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let p = shadow.get_pixel(x as u32, y as u32);
            if p.0[3] > 0 {
                // Elongate in projection direction
                let elongation = proj_dist / 50.0;
                let ex = x as f64 + proj_x * elongation;
                let ey = y as f64 + proj_y * elongation;
                if ex >= 0.0 && ex < w as f64 && ey >= 0.0 && ey < h as f64 {
                    let existing = stretched.get_pixel(ex as u32, ey as u32);
                    if p.0[3] > existing.0[3] {
                        stretched.put_pixel(ex as u32, ey as u32, *p);
                    }
                }
                // Keep original too
                let existing = stretched.get_pixel(x as u32, y as u32);
                if p.0[3] > existing.0[3] {
                    stretched.put_pixel(x as u32, y as u32, *p);
                }
            }
        }
    }

    // Blur the result
    let blurred = blur::blur_alpha_only(&stretched, params.blur);

    Ok(blurred)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_perspective_shadow() {
        let mut input = RgbaImage::new(64, 64);
        for y in 16..48 {
            for x in 16..48 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Perspective,
            opacity: 0.5,
            blur: 5.0,
            light_x: 1.0,
            light_y: 1.0,
            light_z: 50.0,
            canvas_expand: 10,
            ..Default::default()
        };
        let result = generate(&input, &params).unwrap();
        assert!(result.width() > 64);
        // Should have some shadow pixels
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
    fn test_perspective_zero_elevation_fallback() {
        let mut input = RgbaImage::new(32, 32);
        for y in 8..24 {
            for x in 8..24 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Perspective,
            light_z: 0.0,
            blur: 5.0,
            canvas_expand: 5,
            ..Default::default()
        };
        // Should fall back to drop shadow and not error
        let result = generate(&input, &params);
        assert!(result.is_ok());
    }
}
