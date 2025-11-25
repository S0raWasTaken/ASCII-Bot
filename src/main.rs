use poise::{
    Framework, FrameworkError, FrameworkOptions,
    samples::register_globally,
    serenity_prelude::{ClientBuilder, GatewayIntents},
};

use crate::commands::{attachment_to_ascii, avatar_to_ascii, image_to_ascii};

struct Data;
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

type Res<T> = Result<T, Error>;

mod commands;
mod image_to_ascii;
mod macros;

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
    if let FrameworkError::Command { error, ctx, .. } = error {
        ctx.send(embed!(
            title: format!("Error in command `/{}`", ctx.command().name),
            description: format!(
                "```diff\n- {}```",
                error.to_string().replace('\n', "\n- ").trim()
            ),
            ephemeral: true,
            mentions: None,
            reply: true,
        ))
        .await
        .ok();
    } else {
        poise::builtins::on_error(error).await.ok();
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
