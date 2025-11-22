use image::Rgba;

/// Parse a hex color string to Rgba<u8>
/// Accepts formats: "#RRGGBB", "RRGGBB", "#RGB", "RGB", "0xRRGGBB"
/// With optional alpha: "#RRGGBBAA", "RRGGBBAA", "#RGBA", "RGBA"
/// If no alpha is provided, defaults to 255 (fully opaque)
pub fn parse_hex_color(hex: &str) -> Result<Rgba<u8>, String> {
    // Lowercase and remove '#' or '0x' if present
    let hex = hex.to_lowercase();
    let hex = hex
        .strip_prefix('#')
        .or_else(|| hex.strip_prefix("0x"))
        .unwrap_or(&hex);

    match hex.len() {
        // Short format: "RGB" -> "RRGGBB"
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;

            // Double each digit: F -> FF
            Ok(Rgba([r * 17, g * 17, b * 17, 255]))
        }
        // Short format with alpha: "RGBA" -> "RRGGBBAA"
        4 => {
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let a = u8::from_str_radix(&hex[3..4], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;

            // Double each digit: F -> FF
            Ok(Rgba([r * 17, g * 17, b * 17, a * 17]))
        }
        // Full format: "RRGGBB"
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;

            Ok(Rgba([r, g, b, 255]))
        }
        // Full format with alpha: "RRGGBBAA"
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;
            let a = u8::from_str_radix(&hex[6..8], 16)
                .map_err(|_| format!("Invalid hex color: {}", hex))?;

            Ok(Rgba([r, g, b, a]))
        }
        _ => Err(format!("Invalid hex color length: {}", hex)),
    }
}
