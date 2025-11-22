use ab_glyph::{FontRef, PxScale};
use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::io::Cursor;

use crate::{Context, Error, Res, parse_hex_color::parse_hex_color};

pub struct AsciiRenderer {
    font: FontRef<'static>,
    char_width: u32,
    char_height: u32,
    background_color: Rgba<u8>,
    max_width_chars: u32,
    brightness_boost: f32,
}

impl AsciiRenderer {
    pub fn new(brightness_boost: f32, background_color: Rgba<u8>, max_width: u32) -> Res<Self> {
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
    pub fn process_image(&self, image_bytes: &[u8], charset: &str) -> Res<String> {
        // Load the image to get dimensions
        let img = image::load_from_memory(image_bytes)?;
        let (img_width, img_height) = img.dimensions();

        let (target_width, target_height) = self.calculate_ascii_dimensions(img_width, img_height);

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
    fn calculate_ascii_dimensions(&self, img_width: u32, img_height: u32) -> (u32, u32) {
        let aspect_ratio = img_width as f32 / img_height as f32;

        // Characters are roughly 2x taller than wide in most monospace fonts
        // So we need to compensate for this in our aspect ratio calculation
        let char_aspect_correction = 2.0;

        let target_width = self.max_width_chars;

        // Calculate height: height = width / (aspect_ratio * correction)
        let target_height = (target_width as f32 / (aspect_ratio * char_aspect_correction)) as u32;

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

        let mut image = ImageBuffer::from_pixel(img_width, img_height, self.background_color);

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

use poise::{
    command,
    serenity_prelude::{Attachment, CreateAttachment, Message, User},
};

const DEFAULT_BACKGROUND: Rgba<u8> = Rgba([20, 20, 20, 255]);

#[command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn image_to_ascii(
    ctx: Context<'_>,
    #[description = "Image to convert to ASCII"] attachment: Attachment,
    #[description = "Custom charset"] charset: Option<String>,
    #[description = "A Brightness boost value. 50 = 50% boost, 100 = 100% boost and so on"]
    brightness_boost: Option<u32>,
    #[description = "The image's background colour, accepts hex RGBA, default = #141414"]
    background_color: Option<String>,
    #[description = "Sets the maximum size of the image (Accepts up to 500)"] max_size: Option<u32>,
) -> Result<(), Error> {
    let brightness_boost = brightness_boost.unwrap_or(100);
    let background_color = parse_hex_color(background_color.as_deref().unwrap_or("#141414"))?;
    let size = max_size.unwrap_or(150);
    _image_to_ascii(
        ctx,
        &attachment.download().await?,
        charset.as_deref(),
        (100 + brightness_boost) as f32 / 100.0,
        background_color,
        size,
    )
    .await
}

#[command(
    context_menu_command = "Attachment to ASCII",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn attachment_to_ascii(ctx: Context<'_>, msg: Message) -> Res<()> {
    let attachment = msg
        .attachments
        .first()
        .ok_or("No attachment in this message")?;

    _image_to_ascii(
        ctx,
        &attachment.download().await?,
        None,
        1.0,
        DEFAULT_BACKGROUND,
        150,
    )
    .await
}

#[command(
    context_menu_command = "User Avatar to ASCII",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn avatar_to_ascii(ctx: Context<'_>, user: User) -> Res<()> {
    _image_to_ascii(
        ctx,
        &reqwest::get(user.static_face()).await?.bytes().await?,
        None,
        1.0,
        DEFAULT_BACKGROUND,
        150,
    )
    .await
}

async fn _image_to_ascii(
    ctx: Context<'_>,
    image_bytes: &[u8],
    charset: Option<&str>,
    brightness_boost: f32,
    background_color: Rgba<u8>,
    size: u32,
) -> Res<()> {
    ctx.defer().await?;

    let charset = charset.unwrap_or(".+P0#@");

    let renderer: AsciiRenderer = AsciiRenderer::new(brightness_boost, background_color, size)?;

    let ascii_art = renderer.process_image(image_bytes, charset)?;

    let output_image: RgbaImage = renderer.render_to_image(&ascii_art)?;

    let mut png_bytes = Vec::new();
    output_image.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )?;

    let files = CreateAttachment::bytes(png_bytes, "ascii.png");

    ctx.send(poise::CreateReply::default().attachment(files))
        .await?;
    Ok(())
}
