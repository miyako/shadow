use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

/// Load an image from a file path, converting to RGBA.
pub fn load_from_file(path: &str) -> Result<RgbaImage, String> {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(format!("input file not found: {}", path));
    }
    let img = image::open(path_obj).map_err(|e| format!("failed to decode image: {}", e))?;
    Ok(img.to_rgba8())
}

/// Load an image from raw bytes (stdin), converting to RGBA.
pub fn load_from_bytes(data: &[u8]) -> Result<RgbaImage, String> {
    if data.is_empty() {
        return Err("stdin input is empty".to_string());
    }
    let img = image::load_from_memory(data)
        .map_err(|e| format!("failed to decode image from stdin: {}", e))?;
    Ok(img.to_rgba8())
}

/// Unified loader: if `input` is "-", reads stdin bytes; otherwise treats as file path.
pub fn load(input: &str) -> Result<RgbaImage, String> {
    if input == "-" {
        let bytes = crate::protocol::read_stdin()
            .map_err(|e| format!("failed to read stdin: {}", e))?;
        load_from_bytes(&bytes)
    } else {
        load_from_file(input)
    }
}

/// Save an RGBA image to a file path, inferring format from extension.
pub fn save_to_file(
    img: &RgbaImage,
    path: &str,
    format: Option<image::ImageFormat>,
) -> Result<(), String> {
    let fmt = match format {
        Some(f) => f,
        None => {
            let ext = Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            image::ImageFormat::from_extension(ext).unwrap_or(image::ImageFormat::Png)
        }
    };
    img.save_with_format(path, fmt)
        .map_err(|e| format!("failed to encode image: {}", e))
}

/// Encode an RGBA image to bytes in the given format.
pub fn encode_to_bytes(
    img: &RgbaImage,
    format: image::ImageFormat,
) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, format)
        .map_err(|e| format!("failed to encode image: {}", e))?;
    Ok(buf.into_inner())
}

/// Parse a hex color string like "#RRGGBB" or "#RRGGBBAA" into an RGBA pixel.
pub fn parse_color(s: &str) -> Result<Rgba<u8>, String> {
    let s = s.strip_prefix('#').unwrap_or(s);
    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| format!("invalid color: {}", e))?;
            let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| format!("invalid color: {}", e))?;
            let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| format!("invalid color: {}", e))?;
            Ok(Rgba([r, g, b, 255]))
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| format!("invalid color: {}", e))?;
            let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| format!("invalid color: {}", e))?;
            let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| format!("invalid color: {}", e))?;
            let a = u8::from_str_radix(&s[6..8], 16).map_err(|e| format!("invalid color: {}", e))?;
            Ok(Rgba([r, g, b, a]))
        }
        _ => Err(format!("invalid color format: expected #RRGGBB or #RRGGBBAA, got: #{}", s)),
    }
}

/// Create a blank transparent RGBA image of the given dimensions.
pub fn create_transparent(width: u32, height: u32) -> RgbaImage {
    ImageBuffer::new(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_rgb() {
        let c = parse_color("#FF0000").unwrap();
        assert_eq!(c.0, [255, 0, 0, 255]);
    }

    #[test]
    fn test_parse_color_rgba() {
        let c = parse_color("#0000FF80").unwrap();
        assert_eq!(c.0, [0, 0, 255, 128]);
    }

    #[test]
    fn test_parse_color_no_hash() {
        let c = parse_color("00FF00").unwrap();
        assert_eq!(c.0, [0, 255, 0, 255]);
    }

    #[test]
    fn test_parse_color_invalid() {
        assert!(parse_color("#GGG").is_err());
        assert!(parse_color("#12345").is_err());
    }

    #[test]
    fn test_create_transparent() {
        let img = create_transparent(10, 10);
        assert_eq!(img.dimensions(), (10, 10));
        assert_eq!(img.get_pixel(0, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn test_load_from_bytes_empty() {
        assert!(load_from_bytes(&[]).is_err());
    }

    #[test]
    fn test_round_trip_png() {
        let img = create_transparent(32, 32);
        let bytes = encode_to_bytes(&img, image::ImageFormat::Png).unwrap();
        let loaded = load_from_bytes(&bytes).unwrap();
        assert_eq!(loaded.dimensions(), (32, 32));
    }
}
