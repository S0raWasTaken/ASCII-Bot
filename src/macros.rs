#[macro_export]
macro_rules! embed {
    (
        $(title: $title:expr,)?
        $(description: $description:expr,)?
        $(ephemeral: $ephemeral:expr,)?
        $(mentions: $mentions:expr,)?
        $(reply: $reply:expr,)?
    ) => {{
        poise::CreateReply {
            embeds: vec![
                poise::serenity_prelude::CreateEmbed::new()
                    $(.title($title))?
                    $(.description($description))?
            ],
            $(ephemeral: Some($ephemeral),)?
            $(reply: $reply,)?
            $(allowed_mentions: $mentions,)?
            ..Default::default()
        }

    }};
}
