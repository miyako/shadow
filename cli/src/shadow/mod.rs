pub mod contact;
pub mod curved;
pub mod drop;
pub mod inner;
pub mod long;
pub mod perspective;

use crate::image_io::parse_color;
use crate::protocol::ShadowType;
use image::RgbaImage;
use serde::Deserialize;

/// Parameters for shadow generation, parsed from CLI options.
#[derive(Debug, Clone, Deserialize)]
pub struct ShadowParams {
    #[serde(rename = "type")]
    pub shadow_type: ShadowType,

    pub opacity: f64,
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur: f32,
    pub color: String,
    pub scale: f64,
    pub angle: i32,
    pub length: Option<String>,
    pub fade: String,
    pub edge: String,
    pub curve_depth: f32,
    pub light_x: f64,
    pub light_y: f64,
    pub light_z: f64,
    pub canvas_expand: u32,
    pub shadow_only: bool,
}

impl Default for ShadowParams {
    fn default() -> Self {
        ShadowParams {
            shadow_type: ShadowType::Drop,
            opacity: 0.5,
            offset_x: 5,
            offset_y: 5,
            blur: 10.0,
            color: "#000000".to_string(),
            scale: 1.0,
            angle: 45,
            length: None,
            fade: "linear".to_string(),
            edge: "bottom".to_string(),
            curve_depth: 20.0,
            light_x: -1.0,
            light_y: -1.0,
            light_z: 40.0,
            canvas_expand: 20,
            shadow_only: false,
        }
    }
}

impl ShadowParams {
    pub fn parse_color(&self) -> Result<image::Rgba<u8>, String> {
        parse_color(&self.color)
    }
}

/// Generate a shadow for the given input image using the specified parameters.
/// Returns either the composited image (shadow + original) or just the shadow layer.
pub fn generate(input: &RgbaImage, params: &ShadowParams) -> Result<RgbaImage, String> {
    let result = match params.shadow_type {
        ShadowType::Drop => drop::generate(input, params)?,
        ShadowType::Contact => contact::generate(input, params)?,
        ShadowType::Curved => curved::generate(input, params)?,
        ShadowType::Long => long::generate(input, params)?,
        ShadowType::Perspective => perspective::generate(input, params)?,
        ShadowType::Inner => inner::generate(input, params)?,
    };

    if params.shadow_only {
        Ok(result)
    } else {
        // Expand the input to match the shadow canvas size for compositing
        let expanded_input = if params.canvas_expand > 0 {
            crate::composite::expand_canvas(input, params.canvas_expand)
        } else {
            input.clone()
        };

        // For inner shadow, composite above (darkening); for all others, beneath
        match params.shadow_type {
            ShadowType::Inner => Ok(crate::composite::composite_shadow_above(&expanded_input, &result)),
            _ => Ok(crate::composite::composite_shadow_beneath(&expanded_input, &result)),
        }
    }
}
