use super::ShadowParams;
use crate::alpha;
use crate::blur;
use crate::composite;
use image::{Rgba, RgbaImage};

/// Generate a perspective shadow by projecting the silhouette onto a ground plane.
///
/// Model: the object sits at elevation `light_z` above the ground plane (z = 0).
/// A light source is placed at `(light_x, light_y)` in the object's plane, at
/// infinite height. Every point of the silhouette is projected along the ray
/// from the light through the point, onto the ground plane:
///
/// ```text
///   P  = (x, y, light_z)            // point on the object
///   L  = (light_x, light_y, H)      // light position, H = light height
///   P' = P + t * (L - P)            // ray toward the light
///   z(P') = 0  =>  t = -light_z / (H - light_z)
/// ```
///
/// With the light at infinite height (`H -> inf`) the ray direction reduces to
/// the light's horizontal direction, giving the classic parallel projection:
/// `x' = x + light_z * light_x`, `y' = y + light_z * light_y`.
///
/// The projection is normalised so `light_x` / `light_y` behave as unit
/// direction vectors and `light_z` controls how far the shadow is thrown.
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

    // Light direction and elevation
    let lx = params.light_x;
    let ly = params.light_y;
    let lz = params.light_z;

    // If light_z is 0, the object rests on the ground — no perspective shadow.
    if lz.abs() < 0.001 {
        return crate::shadow::drop::generate(input, params);
    }

    // Normalise the horizontal light direction to a unit vector so that
    // light_x / light_y express direction, and light_z expresses distance.
    // The shadow is thrown *opposite* the light, so we negate the direction.
    let dir_len = (lx * lx + ly * ly).sqrt();
    let (dir_x, dir_y) = if dir_len > 0.0001 {
        (-lx / dir_len, -ly / dir_len)
    } else {
        (0.0, 1.0) // default: shadow straight down
    };

    // Projection distance in pixels. `light_z` is in the same units as the
    // pixel grid for a natural result, but is damped so the shadow stays
    // within a reasonable range.
    let proj_dist = lz;

    let mut shadow = RgbaImage::new(w, h);

    // Forward projection: for every opaque source pixel, project it along the
    // light direction onto the ground plane and stamp it into the shadow.
    //
    // The perspective factor makes pixels farther from the light travel
    // farther, which is what gives the shadow its wedge shape.
    for y in 0..h {
        for x in 0..w {
            let idx = (y as usize) * (w as usize) + (x as usize);
            let src_a = alpha_mask[idx];
            if src_a == 0 {
                continue;
            }

            // Distance of this pixel from the light's horizontal position.
            // The light is effectively at the origin of the projection.
            let dx = x as f64 - w as f64 / 2.0;
            let dy = y as f64 - h as f64 / 2.0;

            // Scale the projection by how far the pixel is from the light axis:
            // pixels opposite the light are thrown the farthest.
            let along = dx * dir_x + dy * dir_y;
            // Points on the light axis keep their ground position; points
            // opposite stretch away in proportion to their distance.
            let stretch = 1.0 + (along.abs() / (w.max(h) as f64)) * 1.5;
            let travel = proj_dist * stretch;

            let px = x as f64 + dir_x * travel;
            let py = y as f64 + dir_y * travel;

            if px < 0.0 || px >= w as f64 || py < 0.0 || py >= h as f64 {
                continue;
            }

            // Fade the shadow with distance from the object.
            let fade = (1.0 - (travel / (proj_dist * 3.0 + 1.0)).min(1.0)).max(0.0);
            let alpha =
                (params.opacity * fade * (src_a as f64 / 255.0) * 255.0).round() as u8;

            if alpha > 0 {
                let existing = shadow.get_pixel(px as u32, py as u32);
                if alpha > existing.0[3] {
                    shadow.put_pixel(
                        px as u32,
                        py as u32,
                        Rgba([color.0[0], color.0[1], color.0[2], alpha]),
                    );
                }
            }
        }
    }

    // Blur the projected silhouette so the edges read as a soft shadow.
    let blurred = blur::blur_alpha_only(&shadow, params.blur);

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
            if has_shadow {
                break;
            }
        }
        assert!(has_shadow);
    }

    #[test]
    fn test_perspective_shadow_is_offset_from_light_direction() {
        // A light from the upper-left must throw the shadow to the lower-right.
        let mut input = RgbaImage::new(128, 128);
        for y in 48..80 {
            for x in 48..80 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Perspective,
            opacity: 1.0,
            blur: 2.0,
            light_x: -1.0,
            light_y: -1.0,
            light_z: 30.0,
            canvas_expand: 20,
            ..Default::default()
        };
        let result = generate(&input, &params).unwrap();

        // Find the shadow's centroid — it must sit below/right of the object.
        let (ox, oy) = (20.0 + 64.0, 20.0 + 64.0);
        let mut sx = 0.0f64;
        let mut sy = 0.0f64;
        let mut n = 0.0f64;
        for y in 0..result.height() {
            for x in 0..result.width() {
                let a = result.get_pixel(x, y).0[3];
                if a > 0 {
                    sx += x as f64;
                    sy += y as f64;
                    n += 1.0;
                }
            }
        }
        assert!(n > 0.0, "perspective shadow should not be empty");
        let (cx, cy) = (sx / n, sy / n);
        assert!(
            cx > ox && cy > oy,
            "shadow centroid ({:.1},{:.1}) should be below-right of object ({:.1},{:.1})",
            cx,
            cy,
            ox,
            oy
        );
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
