use ab_glyph::{FontRef, PxScale};
use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use std::io::Cursor;

use crate::Res;

pub struct AsciiRenderer {
    font: FontRef<'static>,
    char_width: u32,
    char_height: u32,
    background_color: Rgba<u8>,
    max_width_chars: u32,
    background_brightness: f32,
}

impl AsciiRenderer {
    pub fn new(background_brightness: f32, max_width: u32) -> Res<Self> {
        let font_data = include_bytes!("../fonts/RobotoMono-Regular.ttf");
        let font = FontRef::try_from_slice(font_data)?;
        let background_color = Rgba([0, 0, 0, 255]);
        let background_brightness = background_brightness.clamp(0.0, 1.0);

        Ok(Self {
            font,
            char_width: 9,
            char_height: 18,
            background_color,
            max_width_chars: max_width.min(200),
            background_brightness,
        })
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
        let ascii_art = libasciic::AsciiBuilder::new(cursor)
            .dimensions(target_width, target_height)
            .colorize(true)
            .style(libasciic::Style::Mixed)
            .threshold(0)
            .filter_type(libasciic::FilterType::Lanczos3)
            .charset(charset)
            .background_brightness(self.background_brightness)
            .make_ascii()?;

        Ok(ascii_art)
    }

    /// Calculate ASCII dimensions maintaining aspect ratio
    /// Width is clamped to max_width_chars (200)
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
    /// Now supports both foreground and background colors
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

            for (col_idx, (ch, fg_color, bg_color)) in parsed.iter().enumerate()
            {
                let x = col_idx as u32 * self.char_width;
                let y = line_idx as u32 * self.char_height;

                // Draw background rectangle first if background color is set
                if let Some(bg) = bg_color {
                    draw_filled_rect_mut(
                        &mut image,
                        Rect::at(x as i32, y as i32)
                            .of_size(self.char_width, self.char_height),
                        *bg,
                    );
                }

                // Draw character with foreground color
                draw_text_mut(
                    &mut image,
                    *fg_color,
                    x as i32,
                    y as i32,
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
    /// Returns: Vec<(char, foreground_color, optional_background_color)>
    fn parse_colored_line(
        &self,
        line: &str,
    ) -> Vec<(char, Rgba<u8>, Option<Rgba<u8>>)> {
        let mut result = Vec::new();
        let mut current_fg = Rgba([255, 255, 255, 255]); // Default white
        let mut current_bg: Option<Rgba<u8>> = None; // Default no background
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
                    match self.parse_ansi_rgb(&code) {
                        Some(AnsiColor::Foreground(color)) => {
                            current_fg = color;
                        }
                        Some(AnsiColor::Background(color)) => {
                            current_bg = Some(color);
                        }
                        Some(AnsiColor::Reset) => {
                            current_fg = Rgba([255, 255, 255, 255]);
                            current_bg = None;
                        }
                        None => {}
                    }
                }
            } else {
                // Regular character - use current colors
                result.push((ch, current_fg, current_bg));
            }
        }

        result
    }

    /// Parse ANSI RGB color codes
    /// Formats: 38;2;R;G;B (foreground) or 48;2;R;G;B (background) or 0 (reset)
    fn parse_ansi_rgb(&self, code: &str) -> Option<AnsiColor> {
        let parts: Vec<&str> = code.split(';').collect();

        // RGB foreground: 38;2;R;G;B
        if parts.len() >= 5 && parts[0] == "38" && parts[1] == "2" {
            let r = parts[2].parse().ok()?;
            let g = parts[3].parse().ok()?;
            let b = parts[4].parse().ok()?;
            return Some(AnsiColor::Foreground(Rgba([r, g, b, 255])));
        }

        // RGB background: 48;2;R;G;B
        if parts.len() >= 5 && parts[0] == "48" && parts[1] == "2" {
            let r = parts[2].parse().ok()?;
            let g = parts[3].parse().ok()?;
            let b = parts[4].parse().ok()?;
            return Some(AnsiColor::Background(Rgba([r, g, b, 255])));
        }

        // Reset code: 0
        if parts.len() == 1 && parts[0] == "0" {
            return Some(AnsiColor::Reset);
        }

        None
    }
}

/// Represents the type of ANSI color code
enum AnsiColor {
    Foreground(Rgba<u8>),
    Background(Rgba<u8>),
    Reset,
}
