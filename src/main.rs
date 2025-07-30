use std::sync::Arc;
use std::time::Duration;

use ::serenity::all::{ActivityData, CacheHttp, GuildId, UserId};
use commands::setup_commands::*;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use shame_bot::{Error, ShameBotData};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::commands::utility::time_kenneled;
use crate::commands::wildcard::wildcard_command_handler;
use shame_bot::util::pgint_dur::PgIntervalToDuration;

mod healthcheck;
mod commands {
    pub mod setup_commands;
    pub mod utility;
    pub mod wildcard;
}

/// The timeout between healthcehcks.
const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(30);

pub fn get_formatted_message(
    message: &str,
    victim_id: &UserId,
    author_id: &UserId,
    time: &str,
    return_time: &str,
) -> String {
    message
        .replace("$victim", format!("<@{}>", victim_id).as_str())
        .replace("$kenneler", format!("<@{}>", author_id).as_str())
        .replace("$time", time)
        .replace("$return", return_time)
}

pub async fn set_activity(ctx: &serenity::prelude::Context, pool: &PgPool) {
    if let Ok(res) = sqlx::query!(
        r#"
        SELECT SUM(kennel_length)
        FROM kennelings
        WHERE 
            NOT guild_id = '849505364764524565'
            ;
        "#
    )
    .fetch_one(pool)
    .await
    {
        if let Some(sum) = res.sum {
            ctx.set_activity(Some(ActivityData::custom(format!(
                "Kenneled users for {}",
                humantime::format_duration(sum.as_duration())
            ))));
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let token = std::env::var("BOT_TOKEN").expect("missing BOT_TOKEN");
    let postgres_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
    tracing::debug!("Connecting to database: {postgres_url}");
    let intents = serenity::GatewayIntents::non_privileged();
    let pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&postgres_url)
            .await
            .expect("Couldn't connect to database! Aborting..."),
    );
    // Pool reference for the healthcheck thread
    let thread_pool = Arc::clone(&pool);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                set_kennel_role(),
                set_kennel_command(),
                set_announcement_message(),
                set_kennel_message(),
                set_release_message(),
                set_kennel_channel(),
                time_kenneled(),
            ],
            event_handler: |w, x, y, z| Box::pin(wildcard_command_handler(w, x, y, z)),
            on_error: |error| {
                async fn error_cb(error: poise::FrameworkError<'_, ShameBotData, Error>) {
                    // Get rid of the unknown interaction errors because the kennel command triggers this.
                    if let poise::FrameworkError::UnknownInteraction { .. } = error {
                        return;
                    }

                    tracing::error!("{}", error.to_string())
                }

                Box::pin(error_cb(error))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let data = sqlx::query!(
                    r#"
                    SELECT * FROM servers
                    ;
                    "#
                )
                .fetch_all(pool.as_ref())
                .await?;
                tracing::info!("Setting up guild commands...");
                for row in data {
                    let cmd = shame_bot::get_kennel_command_struct(&row.command_name);
                    tracing::debug!(
                        "Initializing kennel command \"{}\" for guild {}",
                        &row.command_name,
                        &row.guild_id
                    );

                    let guild_id: GuildId = shame_bot::string_to_id(&row.guild_id)?;

                    ctx.http()
                        .create_guild_commands(guild_id.into(), &vec![cmd])
                        .await?;
                }

                set_activity(ctx, pool.as_ref()).await;

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                tracing::info!("Bot started!");
                Ok(ShameBotData {
                    pool: Arc::clone(&pool),
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .unwrap();

    let thread_http = Arc::clone(&client.http);

    // TODO: Should this be moved to inside the ready callback?
    let _ = tokio::spawn(async move {
        let http = thread_http.as_ref();
        let pool = thread_pool.as_ref();

        loop {
            if let Err(e) = healthcheck::check(http, pool).await {
                tracing::error!("Healthcheck failed!: {}", (*e).to_string());
            }
            tokio::time::sleep(HEALTHCHECK_TIMEOUT).await;
        }
    });

    tracing::info!("Bot starting...");
    client.start().await.unwrap();
    tracing::info!("Exiting...");
}
