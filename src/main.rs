use poise::{
    CreateReply, Framework, FrameworkError, FrameworkOptions,
    samples::register_globally,
    serenity_prelude::{ClientBuilder, CreateEmbed, GatewayIntents},
};

use crate::image_to_ascii::{
    attachment_to_ascii, avatar_to_ascii, image_to_ascii,
};

struct Data;
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

type Res<T> = Result<T, Error>;

mod image_to_ascii;
mod parse_hex_color;

#[tokio::main]
async fn main() -> Res<()> {
    let intents = GatewayIntents::non_privileged();

    // Token file is generated through
    // ./xor_token.sh "MTQTHISIS.ANEXAMPLE.TOKEN"
    let token = String::from_utf8(
        include_bytes!("../.token.xor")
            .iter()
            .map(|b| b ^ 66)
            .collect::<Vec<_>>(),
    )?;

    let mut client =
        ClientBuilder::new(token, intents).framework(framework()).await?;

    client.start().await?;
    Ok(())
}

async fn on_error(error: FrameworkError<'_, Data, Error>) {
    match error {
        FrameworkError::Command { error, ctx, .. } => {
            ctx.send(CreateReply {
                embeds: vec![
                    CreateEmbed::new()
                        .title(format!(
                            "Error in command `/{}`",
                            ctx.command().name
                        ))
                        .description(format!(
                            "```diff\n- {}```",
                            error.to_string().replace('\n', "\n- ").trim()
                        )),
                ],
                ephemeral: Some(true),
                allowed_mentions: None,
                reply: true,
                ..Default::default()
            })
            .await
            .ok();
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                eprintln!("Error while... handling an error... oops\n\n{e}");
            }
        }
    }
}

fn framework() -> Framework<Data, Error> {
    let options = FrameworkOptions {
        commands: vec![
            image_to_ascii(),
            attachment_to_ascii(),
            avatar_to_ascii(),
        ],
        on_error: |e| Box::pin(on_error(e)),
        ..Default::default()
    };

    Framework::builder()
        .options(options)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                println!("{} is on!", ready.user.name);
                register_globally(ctx, &framework.options().commands).await?;
                Ok(Data)
            })
        })
        .build()
}
