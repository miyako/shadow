mod alpha;
mod blur;
mod composite;
mod image_io;
mod protocol;
mod shadow;

use clap::{Parser, Subcommand};
use image::ImageFormat;
use protocol::{write_stderr_error, ShadowType};
use shadow::{generate, ShadowParams};
use std::process::ExitCode;

/// Shadow CLI — Image shadow generator
#[derive(Parser)]
#[command(name = "shadow-cli")]
#[command(version = "0.1.0")]
#[command(about = "Generate shadow effects for images", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version and protocol info
    #[command(name = "--version")]
    Version,
    /// Show supported shadow types and parameters
    Capabilities,
    /// Generate a shadow and composite with the original image
    Generate {
        /// Input image path, or "-" for stdin
        #[arg(long)]
        input: String,

        /// Output image path, or "-" for stdout
        #[arg(long)]
        output: String,

        /// Shadow type
        #[arg(long, default_value = "drop")]
        r#type: String,

        /// Opacity 0.0–1.0
        #[arg(long, default_value_t = 0.5)]
        opacity: f64,

        /// X offset in pixels
        #[arg(long, default_value_t = 5)]
        offset_x: i32,

        /// Y offset in pixels
        #[arg(long, default_value_t = 5)]
        offset_y: i32,

        /// Blur sigma in pixels
        #[arg(long, default_value_t = 10.0)]
        blur: f32,

        /// Shadow color as hex string (#RRGGBB or #RRGGBBAA)
        #[arg(long, default_value = "#000000")]
        color: String,

        /// Scale factor (for drop/contact)
        #[arg(long, default_value_t = 1.0)]
        scale: f64,

        /// Angle in degrees (for long shadow)
        #[arg(long, default_value_t = 45)]
        angle: i32,

        /// Length in pixels or "auto" (for long shadow)
        #[arg(long)]
        length: Option<String>,

        /// Fade mode: linear, exp, none (for long shadow)
        #[arg(long, default_value = "linear")]
        fade: String,

        /// Edge: top, bottom, left, right (for curved shadow)
        #[arg(long, default_value = "bottom")]
        edge: String,

        /// Curve depth in pixels (for curved shadow)
        #[arg(long, default_value_t = 20.0)]
        curve_depth: f32,

        /// Light X position (for perspective)
        #[arg(long, default_value_t = 0.0)]
        light_x: f64,

        /// Light Y position (for perspective)
        #[arg(long, default_value_t = 0.0)]
        light_y: f64,

        /// Light Z elevation (for perspective)
        #[arg(long, default_value_t = 100.0)]
        light_z: f64,

        /// Canvas expansion in pixels (padding around output)
        #[arg(long, default_value_t = 20)]
        canvas_expand: u32,

        /// Output format: png, jpeg, webp
        #[arg(long, default_value = "png")]
        format: String,

        /// Background color or "transparent"
        #[arg(long, default_value = "transparent")]
        background: String,

        /// Output only the shadow layer (no original)
        #[arg(long, default_value_t = false)]
        shadow_only: bool,
    },
    /// Generate only the shadow layer (alias for generate --shadow-only)
    ShadowOnly {
        #[arg(long)]
        input: String,
        #[arg(long)]
        output: String,
        #[arg(long, default_value = "drop")]
        r#type: String,
        #[arg(long, default_value_t = 0.5)]
        opacity: f64,
        #[arg(long, default_value_t = 5)]
        offset_x: i32,
        #[arg(long, default_value_t = 5)]
        offset_y: i32,
        #[arg(long, default_value_t = 10.0)]
        blur: f32,
        #[arg(long, default_value = "#000000")]
        color: String,
        #[arg(long, default_value_t = 1.0)]
        scale: f64,
        #[arg(long, default_value_t = 45)]
        angle: i32,
        #[arg(long)]
        length: Option<String>,
        #[arg(long, default_value = "linear")]
        fade: String,
        #[arg(long, default_value = "bottom")]
        edge: String,
        #[arg(long, default_value_t = 20.0)]
        curve_depth: f32,
        #[arg(long, default_value_t = 0.0)]
        light_x: f64,
        #[arg(long, default_value_t = 0.0)]
        light_y: f64,
        #[arg(long, default_value_t = 100.0)]
        light_z: f64,
        #[arg(long, default_value_t = 20)]
        canvas_expand: u32,
        #[arg(long, default_value = "png")]
        format: String,
        #[arg(long, default_value = "transparent")]
        background: String,
    },
    /// Run internal self-tests
    Selftest,
}

fn parse_shadow_type(s: &str) -> Result<ShadowType, String> {
    s.parse::<ShadowType>()
        .map_err(|e| format!("invalid --type: {}", e))
}

fn build_params(
    shadow_type: &str,
    opacity: f64,
    offset_x: i32,
    offset_y: i32,
    blur: f32,
    color: &str,
    scale: f64,
    angle: i32,
    length: Option<String>,
    fade: &str,
    edge: &str,
    curve_depth: f32,
    light_x: f64,
    light_y: f64,
    light_z: f64,
    canvas_expand: u32,
    shadow_only: bool,
) -> Result<ShadowParams, String> {
    let st = parse_shadow_type(shadow_type)?;

    // Validate opacity range
    if !(0.0..=1.0).contains(&opacity) {
        return Err(format!("opacity must be 0.0–1.0, got: {}", opacity));
    }

    // Validate fade mode
    match fade {
        "linear" | "exp" | "none" => {}
        _ => return Err(format!("invalid fade: {} (expected linear/exp/none)", fade)),
    }

    // Validate edge
    match edge {
        "top" | "bottom" | "left" | "right" => {}
        _ => return Err(format!("invalid edge: {} (expected top/bottom/left/right)", edge)),
    }

    // Validate length if specified
    if let Some(ref l) = length {
        if l != "auto" {
            l.parse::<i32>()
                .map_err(|_| format!("invalid length: {} (expected integer or 'auto')", l))?;
        }
    }

    Ok(ShadowParams {
        shadow_type: st,
        opacity,
        offset_x,
        offset_y,
        blur,
        color: color.to_string(),
        scale,
        angle,
        length: length.clone(),
        fade: fade.to_string(),
        edge: edge.to_string(),
        curve_depth,
        light_x,
        light_y,
        light_z,
        canvas_expand,
        shadow_only,
    })
}

fn parse_format(s: &str) -> Result<ImageFormat, String> {
    match s.to_lowercase().as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpeg" | "jpg" => Ok(ImageFormat::Jpeg),
        "webp" => Ok(ImageFormat::WebP),
        _ => Err(format!("invalid format: {} (expected png/jpeg/webp)", s)),
    }
}

fn handle_generate(
    input: &str,
    output: &str,
    shadow_type: &str,
    opacity: f64,
    offset_x: i32,
    offset_y: i32,
    blur: f32,
    color: &str,
    scale: f64,
    angle: i32,
    length: Option<String>,
    fade: &str,
    edge: &str,
    curve_depth: f32,
    light_x: f64,
    light_y: f64,
    light_z: f64,
    canvas_expand: u32,
    format: &str,
    background: &str,
    shadow_only: bool,
) -> Result<(), String> {
    // Build params
    let params = build_params(
        shadow_type, opacity, offset_x, offset_y, blur, color, scale, angle,
        length, fade, edge, curve_depth, light_x, light_y, light_z,
        canvas_expand, shadow_only,
    )?;

    // Load image
    let img = image_io::load(input)?;

    // Generate shadow
    let result = shadow::generate(&img, &params)?;

    // Apply background if specified
    let final_img = if background != "transparent" {
        let bg = image_io::parse_color(background)?;
        composite::apply_background(&result, bg)
    } else {
        result
    };

    // Parse output format
    let fmt = parse_format(format)?;

    // Determine output mode
    let stdout_mode = output == "-";
    let output_path = if stdout_mode {
        String::new()
    } else {
        output.to_string()
    };

    if stdout_mode {
        // Write image bytes to stdout, suppress JSON protocol (write to stderr)
        let bytes = image_io::encode_to_bytes(&final_img, fmt)?;
        protocol::write_stdout_bytes(&bytes)?;
        // Protocol response goes to stderr when using stdout for image
        let resp = protocol::GenerateResponse {
            protocol: protocol::PROTOCOL_VERSION,
            ok: true,
            output: "stdout".to_string(),
            width: final_img.width(),
            height: final_img.height(),
        };
        let mut json = serde_json::to_string(&resp).unwrap();
        json.push('\n');
        use std::io::Write;
        std::io::stderr().write_all(json.as_bytes()).ok();
        std::io::stderr().flush().ok();
    } else {
        // Write to file
        image_io::save_to_file(&final_img, &output_path, Some(fmt))?;

        // JSON protocol response on stdout
        let resp = protocol::GenerateResponse {
            protocol: protocol::PROTOCOL_VERSION,
            ok: true,
            output: output_path.clone(),
            width: final_img.width(),
            height: final_img.height(),
        };
        protocol::write_stdout(&resp)?;
    }

    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Version) => {
            protocol::write_stdout(&protocol::version_response())
                .map_err(|e| e.to_string())
        }
        Some(Commands::Capabilities) => {
            protocol::write_stdout(&protocol::capabilities_response())
                .map_err(|e| e.to_string())
        }
        Some(Commands::Generate {
            input, output, r#type, opacity, offset_x, offset_y, blur, color,
            scale, angle, length, fade, edge, curve_depth, light_x, light_y,
            light_z, canvas_expand, format, background, shadow_only,
        }) => {
            handle_generate(
                &input, &output, &r#type, opacity, offset_x, offset_y, blur, &color,
                scale, angle, length, &fade, &edge, curve_depth, light_x, light_y,
                light_z, canvas_expand, &format, &background, shadow_only,
            )
        }
        Some(Commands::ShadowOnly {
            input, output, r#type, opacity, offset_x, offset_y, blur, color,
            scale, angle, length, fade, edge, curve_depth, light_x, light_y,
            light_z, canvas_expand, format, background,
        }) => {
            handle_generate(
                &input, &output, &r#type, opacity, offset_x, offset_y, blur, &color,
                scale, angle, length, &fade, &edge, curve_depth, light_x, light_y,
                light_z, canvas_expand, &format, &background, true,
            )
        }
        Some(Commands::Selftest) => {
            run_selftest()
        }
        None => {
            // No subcommand — show help
            clap::Command::new("shadow-cli")
                .about("Generate shadow effects for images")
                .print_help()
                .map_err(|e| e.to_string())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Determine error code
            let (code, message) = if e.contains("not found") || e.contains("I/O") || e.contains("io") {
                ("E_IO", e)
            } else if e.contains("invalid") || e.contains("parse") || e.contains("must be") {
                ("E_ARGS", e)
            } else {
                ("E_RUNTIME", e)
            };

            let _ = write_stderr_error(code, message);
            let exit_code = match code {
                "E_ARGS" => 2,
                "E_IO" => 3,
                _ => 1,
            };
            ExitCode::from(exit_code)
        }
    }
}

fn run_selftest() -> Result<(), String> {
    println!("Running shadow-cli selftest...");

    // Test 1: Version response
    let v = protocol::version_response();
    assert_eq!(v.protocol, 1);
    assert_eq!(v.version, "0.1.0");
    println!("  [PASS] version response");

    // Test 2: Capabilities response
    let c = protocol::capabilities_response();
    assert!(c.capabilities.types.len() == 6);
    assert!(c.capabilities.params.len() > 0);
    println!("  [PASS] capabilities response");

    // Test 3: Color parsing
    let color = image_io::parse_color("#FF0000")?;
    assert_eq!(color.0, [255, 0, 0, 255]);
    println!("  [PASS] color parsing");

    // Test 4: Alpha extraction
    let mut img = image::RgbaImage::new(8, 8);
    img.put_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    let alpha = alpha::extract_alpha(&img);
    assert_eq!(alpha[4 * 8 + 4], 255);
    assert_eq!(alpha[0], 0);
    println!("  [PASS] alpha extraction");

    // Test 5: Drop shadow generation
    let mut test_img = image::RgbaImage::new(32, 32);
    for y in 8..24 {
        for x in 8..24 {
            test_img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
        }
    }
    let params = ShadowParams {
        shadow_type: ShadowType::Drop,
        opacity: 0.5,
        offset_x: 5,
        offset_y: 5,
        blur: 5.0,
        canvas_expand: 10,
        ..Default::default()
    };
    let result = shadow::generate(&test_img, &params)?;
    assert!(result.width() >= 32);
    println!("  [PASS] drop shadow generation");

    // Test 6: All shadow types
    for st in ShadowType::all() {
        let mut p = params.clone();
        p.shadow_type = st.parse::<ShadowType>().unwrap();
        let r = shadow::generate(&test_img, &p)?;
        assert!(r.width() >= 32, "shadow type {} produced too small image", st);
        println!("  [PASS] {} shadow generation", st);
    }

    println!("\nAll selftest checks passed.");
    Ok(())
}
