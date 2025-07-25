use std::sync::Arc;
use std::time::Duration;

use ::serenity::all::{ActivityData, CacheHttp, UserId};
use commands::setup_commands::*;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::commands::utility::time_kenneled;
use crate::commands::wildcard::wildcard_command_handler;
use crate::util::pgint_dur::PgIntervalToDuration;

mod healthcheck;
mod commands {
    pub mod setup_commands;
    pub mod utility;
    pub mod wildcard;
}
mod util {
    pub mod pgint_dur;
    pub mod stefan_traits;
}

/// The timeout between healthcehcks.
const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(30);

// User data, which is stored and accessible in all command invocations
pub struct Data {
    pool: Arc<PgPool>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub fn get_formatted_message(
    message: &str,
    victim: &UserId,
    kenneler: &UserId,
    time: &str,
    return_time: &str,
) -> String {
    message
        .replace("$victim", format!("<@{}>", victim).as_str())
        .replace("$kenneler", format!("<@{}>", kenneler).as_str())
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

    let token = std::env::var("BOT_TOKEN").expect("missing BOT_TOKEN");
    let postgres_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
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

                for row in data {
                    let cmd = get_kennel_command_struct(&row.command_name);

                    let guild_id = row
                        .guild_id
                        .parse::<u64>()
                        .expect("Malformed guild_id data was inserted into the database! WTF?");

                    ctx.http()
                        .create_guild_commands(guild_id.into(), &vec![cmd])
                        .await?;
                }

                set_activity(ctx, pool.as_ref()).await;

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
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

    let _ = tokio::spawn(async move {
        let http = thread_http.as_ref();
        let pool = thread_pool.as_ref();

        loop {
            if let Err(e) = healthcheck::check(http, pool).await {
                println!("Healthcheck failed!: {:?}", e);
            } else {
                println!("Healthcheck succeeded!");
            }
            tokio::time::sleep(HEALTHCHECK_TIMEOUT).await;
        }
    });

    client.start().await.unwrap();
}
