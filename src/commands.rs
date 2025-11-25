use std::time::Duration;

use image::{Rgba, RgbaImage};

use crate::{
    Context, Error, Res,
    image_to_ascii::{AsciiRenderer, parse_hex_color},
};

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
    #[description = "Custom charset (Max 20 chars)"] charset: Option<String>,
    #[description = "A Brightness boost value. 50 = 50% boost, 100 = 100% boost and so on"]
    brightness_boost: Option<u32>,
    #[description = "The image's background colour, accepts hex RGBA, default = #141414"]
    background_color: Option<String>,
    #[description = "Sets the maximum size of the image (Accepts up to 500)"]
    max_size: Option<u32>,
) -> Result<(), Error> {
    let brightness_boost = brightness_boost.unwrap_or(100);
    let background_color =
        parse_hex_color(background_color.as_deref().unwrap_or("#141414"))?;
    let size = max_size.unwrap_or(150);
    let charset = charset.map(|mut c| {
        c.truncate(20);
        c
    });

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
    let attachment =
        msg.attachments.first().ok_or("No attachment in this message")?;

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
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let avatar = client
        .get(user.static_face())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    _image_to_ascii(ctx, &avatar, None, 1.0, DEFAULT_BACKGROUND, 150).await
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
    let renderer: AsciiRenderer =
        AsciiRenderer::new(brightness_boost, background_color, size)?;
    let ascii_art = renderer.process_image(image_bytes, charset)?;
    let output_image: RgbaImage = renderer.render_to_image(&ascii_art)?;
    let mut png_bytes = Vec::new();

    output_image.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )?;

    let files = CreateAttachment::bytes(png_bytes, "ascii.png");

    ctx.send(poise::CreateReply::default().attachment(files)).await?;
    Ok(())
}
