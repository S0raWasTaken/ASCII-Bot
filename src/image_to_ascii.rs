use ab_glyph::{FontRef, PxScale};
use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::io::Cursor;

use crate::Res;

pub struct AsciiRenderer {
    font: FontRef<'static>,
    char_width: u32,
    char_height: u32,
    background_color: Rgba<u8>,
    max_width_chars: u32,
    brightness_boost: f32,
}

impl AsciiRenderer {
    pub fn new(
        brightness_boost: f32,
        background_color: Rgba<u8>,
        max_width: u32,
    ) -> Res<Self> {
        let font_data = include_bytes!("../fonts/RobotoMono-Regular.ttf");
        let font = FontRef::try_from_slice(font_data)?;

        Ok(Self {
            font,
            char_width: 9,
            char_height: 18,
            background_color,
            max_width_chars: max_width.min(500),
            brightness_boost,
        })
    }

    fn boost_brightness(&self, color: Rgba<u8>) -> Rgba<u8> {
        let r = (color[0] as f32 * self.brightness_boost).min(255.0) as u8;
        let g = (color[1] as f32 * self.brightness_boost).min(255.0) as u8;
        let b = (color[2] as f32 * self.brightness_boost).min(255.0) as u8;
        Rgba([r, g, b, color[3]])
    }

    /// Convert image bytes to ASCII art with proper aspect ratio
    pub fn process_image(
        &self,
        image_bytes: &[u8],
        charset: &str,
    ) -> Res<String> {
        // Load the image to get dimensions
        let img = image::load_from_memory(image_bytes)?;
        let (img_width, img_height) = img.dimensions();

        let (target_width, target_height) =
            self.calculate_ascii_dimensions(img_width, img_height);

        // Convert to ASCII using libasciic
        let cursor = Cursor::new(image_bytes);
        let ascii_art = libasciic::AsciiBuilder::new(cursor)?
            .dimensions(target_width, target_height)
            .colorize(true)
            .style(libasciic::Style::FgPaint)
            .threshold(0)
            .filter_type(libasciic::FilterType::Lanczos3)
            .charset(charset)?
            .make_ascii()?;

        Ok(ascii_art)
    }

    /// Calculate ASCII dimensions maintaining aspect ratio
    /// Width is clamped to max_width_chars (120)
    fn calculate_ascii_dimensions(
        &self,
        img_width: u32,
        img_height: u32,
    ) -> (u32, u32) {
        let aspect_ratio = img_width as f32 / img_height as f32;

        // Characters are roughly 2x taller than wide in most monospace fonts
        // So we need to compensate for this in our aspect ratio calculation
        let char_aspect_correction = 2.0;

        let target_width = self.max_width_chars;

        // Calculate height: height = width / (aspect_ratio * correction)
        let target_height = (target_width as f32
            / (aspect_ratio * char_aspect_correction))
            as u32;

        // Ensure at least 1 row
        (target_width, target_height.max(1))
    }

    /// Render ASCII art with ANSI RGB color codes back to an image
    pub fn render_to_image(&self, ascii_text: &str) -> Res<RgbaImage> {
        let lines: Vec<&str> = ascii_text.lines().collect();
        let height = lines.len() as u32;

        // Get max width by stripping ANSI codes
        let width = lines
            .iter()
            .map(|l| self.count_visible_chars(l))
            .max()
            .unwrap_or(0) as u32;

        let img_width = width * self.char_width;
        let img_height = height * self.char_height;

        let mut image = ImageBuffer::from_pixel(
            img_width,
            img_height,
            self.background_color,
        );

        let scale = PxScale::from(self.char_height as f32);

        for (line_idx, line) in lines.iter().enumerate() {
            let parsed = self.parse_colored_line(line);

            for (col_idx, (ch, color)) in parsed.iter().enumerate() {
                let x = col_idx as i32 * self.char_width as i32;
                let y = line_idx as i32 * self.char_height as i32;

                let boosted_colour = self.boost_brightness(*color);

                draw_text_mut(
                    &mut image,
                    boosted_colour,
                    x,
                    y,
                    scale,
                    &self.font,
                    &ch.to_string(),
                );
            }
        }

        Ok(image)
    }

    /// Count visible characters (excluding ANSI escape sequences)
    fn count_visible_chars(&self, line: &str) -> usize {
        let mut count = 0;
        let mut chars = line.chars();

        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip ANSI escape sequence
                if chars.next() == Some('[') {
                    loop {
                        match chars.next() {
                            Some('m') => break,
                            Some(_) => continue,
                            None => break,
                        }
                    }
                }
            } else {
                count += 1;
            }
        }

        count
    }

    /// Parse a line with RGB ANSI escape codes
    /// Format: \x1b[38;2;R;G;Bm (foreground) or \x1b[48;2;R;G;Bm (background)
    /// Color persists until next code or reset (\x1b[0m)
    fn parse_colored_line(&self, line: &str) -> Vec<(char, Rgba<u8>)> {
        let mut result = Vec::new();
        let mut current_color = Rgba([255, 255, 255, 255]); // Default white
        let mut chars = line.chars();

        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Start of ANSI escape sequence
                if chars.next() == Some('[') {
                    let mut code = String::new();

                    // Read until 'm' (end of color code)
                    loop {
                        match chars.next() {
                            Some('m') => break,
                            Some(c) => code.push(c),
                            None => break,
                        }
                    }

                    // Parse RGB code or reset
                    if let Some(color) = self.parse_ansi_rgb(&code) {
                        current_color = color;
                    }
                }
            } else {
                // Regular character - use current color
                result.push((ch, current_color));
            }
        }

        result
    }

    /// Parse ANSI RGB color codes
    /// Formats: 38;2;R;G;B (foreground) or 48;2;R;G;B (background) or 0 (reset)
    fn parse_ansi_rgb(&self, code: &str) -> Option<Rgba<u8>> {
        let parts: Vec<&str> = code.split(';').collect();

        // RGB foreground: 38;2;R;G;B
        if parts.len() >= 5 && parts[0] == "38" && parts[1] == "2" {
            let r = parts[2].parse().ok()?;
            let g = parts[3].parse().ok()?;
            let b = parts[4].parse().ok()?;
            return Some(Rgba([r, g, b, 255]));
        }

        // RGB background: 48;2;R;G;B (for BgPaint or BgOnly styles)
        if parts.len() >= 5 && parts[0] == "48" && parts[1] == "2" {
            let r = parts[2].parse().ok()?;
            let g = parts[3].parse().ok()?;
            let b = parts[4].parse().ok()?;
            return Some(Rgba([r, g, b, 255]));
        }

        // Reset code: 0
        if parts.len() == 1 && parts[0] == "0" {
            return Some(Rgba([255, 255, 255, 255])); // Reset to white
        }

        None
    }
}

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
            let r = parse_hex_single(hex, 0)?;
            let g = parse_hex_single(hex, 1)?;
            let b = parse_hex_single(hex, 2)?;

            Ok(Rgba([r, g, b, 255]))
        }
        // Short format with alpha: "RGBA" -> "RRGGBBAA"
        4 => {
            let r = parse_hex_single(hex, 0)?;
            let g = parse_hex_single(hex, 1)?;
            let b = parse_hex_single(hex, 2)?;
            let a = parse_hex_single(hex, 3)?;

            // Double each digit: F -> FF
            Ok(Rgba([r, g, b, a]))
        }
        // Full format: "RRGGBB"
        6 => {
            let r = parse_hex_pair(hex, 0)?;
            let g = parse_hex_pair(hex, 2)?;
            let b = parse_hex_pair(hex, 4)?;

            Ok(Rgba([r, g, b, 255]))
        }
        // Full format with alpha: "RRGGBBAA"
        8 => {
            let r = parse_hex_pair(hex, 0)?;
            let g = parse_hex_pair(hex, 2)?;
            let b = parse_hex_pair(hex, 4)?;
            let a = parse_hex_pair(hex, 6)?;

            Ok(Rgba([r, g, b, a]))
        }
        _ => Err(format!("Invalid hex color length: {}", hex)),
    }
}

fn parse_hex_pair(hex: &str, start: usize) -> Result<u8, String> {
    u8::from_str_radix(&hex[start..start + 2], 16)
        .map_err(|_| format!("Invalid hex color: {}", hex))
}

fn parse_hex_single(hex: &str, start: usize) -> Result<u8, String> {
    u8::from_str_radix(&hex[start..start + 1], 16)
        .map(|v| v * 17) // Expand: F -> FF
        .map_err(|_| format!("Invalid hex color: {}", hex))
}
