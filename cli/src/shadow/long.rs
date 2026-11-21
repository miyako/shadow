use super::ShadowParams;
use crate::alpha;
use crate::composite;
use image::RgbaImage;

/// Generate a long shadow extending at a fixed angle.
///
/// Pipeline: extract alpha → for each boundary pixel, trace a ray at angle θ →
/// color every pixel along the ray with fading opacity → composite
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let color = params.parse_color()?;

    // Expand canvas
    let expanded = if params.canvas_expand > 0 {
        composite::expand_canvas(input, params.canvas_expand)
    } else {
        input.clone()
    };
    let (w, h) = expanded.dimensions();
    let ew = w as usize;
    let eh = h as usize;

    // Extract alpha
    let alpha_mask = alpha::extract_alpha(&expanded);

    // Calculate ray direction
    let angle_rad = (params.angle as f64).to_radians();
    let dx = angle_rad.cos();
    let dy = angle_rad.sin();

    // Determine max length
    let max_length = match &params.length {
        Some(s) if s == "auto" => {
            // Diagonal of canvas
            ((w as f64).powi(2) + (h as f64).powi(2)).sqrt() as i32
        }
        Some(s) => s.parse::<i32>().map_err(|_| format!("invalid length: {}", s))?,
        None => ((w as f64).powi(2) + (h as f64).powi(2)).sqrt() as i32,
    };

    let mut shadow_alpha = vec![0u8; ew * eh];

    // For each pixel, check if the shadow ray from this pixel hits any opaque pixel
    // We iterate in reverse: for each opaque pixel, trace a ray and mark pixels
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let idx = (y as usize) * ew + (x as usize);
            if alpha_mask[idx] > 0 {
                // Trace ray from this pixel in the shadow direction
                for step in 0..max_length {
                    let px = x as f64 + step as f64 * dx;
                    let py = y as f64 + step as f64 * dy;
                    if px < 0.0 || px >= w as f64 || py < 0.0 || py >= h as f64 {
                        break;
                    }
                    let px_i = px as usize;
                    let py_i = py as usize;
                    let ray_idx = py_i * ew + px_i;

                    // Calculate opacity based on fade mode and distance
                    let t = if max_length > 0 {
                        step as f64 / max_length as f64
                    } else {
                        1.0
                    };

                    let fade_alpha = match params.fade.as_str() {
                        "linear" => 1.0 - t,
                        "exp" => (-3.0 * t).exp(),
                        "none" => 1.0,
                        _ => 1.0 - t,
                    };

                    let final_alpha = (fade_alpha * params.opacity * 255.0).round() as u8;

                    if final_alpha > shadow_alpha[ray_idx] {
                        shadow_alpha[ray_idx] = final_alpha;
                    }
                }
            }
        }
    }

    // Color flood the shadow alpha mask
    let result = alpha::color_mask(&shadow_alpha, w, h, color, 1.0);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_long_shadow() {
        let mut input = RgbaImage::new(64, 64);
        for y in 20..44 {
            for x in 20..44 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let params = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Long,
            opacity: 0.5,
            angle: 45,
            blur: 0.0,
            canvas_expand: 10,
            fade: "linear".to_string(),
            length: Some("auto".to_string()),
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

    #[test]
    fn test_long_shadow_fade_modes() {
        let mut input = RgbaImage::new(32, 32);
        for y in 8..24 {
            for x in 8..24 {
                input.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }

        // Test that different fade modes produce different results
        let params_linear = ShadowParams {
            shadow_type: crate::protocol::ShadowType::Long,
            opacity: 0.5,
            angle: 45,
            fade: "linear".to_string(),
            canvas_expand: 5,
            ..Default::default()
        };
        let params_none = ShadowParams {
            fade: "none".to_string(),
            ..params_linear.clone()
        };

        let r_linear = generate(&input, &params_linear).unwrap();
        let r_none = generate(&input, &params_none).unwrap();

        // "none" fade should have higher alpha values overall
        let linear_sum: u32 = r_linear.pixels().map(|p| p.0[3] as u32).sum();
        let none_sum: u32 = r_none.pixels().map(|p| p.0[3] as u32).sum();
        assert!(none_sum > linear_sum, "none fade should have more shadow than linear");
    }
}
