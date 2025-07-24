use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ::serenity::all::{
    ActivityData, CacheHttp, Context as SerenityCtx, CreateCommand, CreateCommandOption,
    EditMessage, FullEvent, Interaction, Message, RoleId, UserId,
};
use ::serenity::all::{GuildId, Permissions};
use dotenv::dotenv;
use humantime::format_duration;
use humantime::parse_duration;
use poise::{
    ApplicationContext, CreateReply, FrameworkContext, ReplyHandle, serenity_prelude as serenity,
};
use setup_commands::*;
use sqlx::postgres::types::PgInterval;
use sqlx::{PgPool, postgres::PgPoolOptions};

use stefan_traits::*;

mod healthcheck;
mod setup_commands;
mod stefan_traits;

/// The timeout between healthcehcks.
const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(30);

// User data, which is stored and accessible in all command invocations
pub(crate) struct Data {
    pool: Arc<PgPool>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn get_formatted_message(
    message: &String,
    victim: &UserId,
    kenneler: &UserId,
    time: &String,
    return_time: &String,
) -> String {
    message
        .replace("$victim", format!("<@{}>", victim).as_str())
        .replace("$kenneler", format!("<@{}>", kenneler).as_str())
        .replace("$time", &time)
        .replace("$return", &return_time)
}

/// Tells you the total time kenneled in case you can't read the status
#[poise::command(slash_command)]
async fn time_kenneled(ctx: Context<'_>) -> Result<(), Error> {
    let Data { pool } = ctx.data();
    let pool = pool.as_ref();

    match sqlx::query!(
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
        Ok(res) => {
            if let Some(sum) = res.sum {
                ctx.reply(format!(
                    "Kenneled users for {}",
                    format_duration(
                        Duration::from_micros(sum.microseconds as u64)
                            + Duration::from_secs(sum.days as u64 * 24 * 60 * 60)
                            + Duration::from_secs(sum.months as u64 * 30 * 24 * 60 * 60)
                    )
                ))
                .await?;
            }
        }
        Err(_) => {
            ctx.reply("Database error???").await?;
        }
    }

    Ok(())
}

/// Kennels someone.
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
async fn kennel_user(
    ctx: Context<'_>,
    #[description = "User to kennel"] user: UserId,
    #[description = "Time to kennel"] time: String,
) -> Result<(), Error> {
    let Data { pool } = ctx.data();
    let pool = pool.as_ref();
    let guild_id = ctx.guild_id().expect("If this command gets called outside of a guild somehow, the world is on fire, and everyone explodes.");

    let Ok(dur_time) = parse_duration(&time) else {
        return ctx
            .reply_ephemeral("Invalid time format! Say something like '3m' or '1h'")
            .await;
    };

    if dur_time < Duration::from_secs(1) {
        return ctx.reply_ephemeral("Over 1 second, please...").await;
    }

    let Ok(data) = sqlx::query!(
        r#"
        SELECT * FROM servers
        WHERE
            guild_id = $1
            AND command_name = $2
            ;
        "#,
        guild_id.get().to_string(),
        ctx.invoked_command_name()
    )
    .fetch_one(pool)
    .await
    else {
        ctx.reply_ephemeral("Set kennel role first!").await?;

        return Ok(());
    };

    let http = ctx.http();
    let member = guild_id.member(http, user).await?;
    let role_id = RoleId::new(
        data.role_id
            .parse::<u64>()
            .expect("Invalid role_id data inserted into database! WTF?"),
    );
    let author_id = ctx.author().id;
    let return_timestamp = chrono::Utc::now() + dur_time.clone();
    let announcement = get_formatted_message(
        &data.announcement_message,
        &user,
        &ctx.author().id,
        &time,
        &return_timestamp.discord_relative_timestamp(),
    );
    let kennel_message = get_formatted_message(
        &data.kennel_message,
        &user,
        &ctx.author().id,
        &time,
        &return_timestamp.discord_relative_timestamp(),
    );
    let pg_int = PgInterval::try_from(dur_time).expect("Some fuckwit put in a microsecond value?");

    member.add_role(http, &role_id).await?;

    // Send the announcement message in the channel where kenneling was executed
    let announcement_reply_handle = ctx.reply(&announcement).await?;

    // Send the kenneling message in the dedicated kennel channel, if it exists
    let mut kennel_reply_handle: Option<Message> = None;

    if let Some(kennel_channel) = data.kennel_channel {
        let kennel_channel = kennel_channel
            .parse::<u64>()
            .expect("Invalid kennel_channel data inserted into database!");

        if let Ok(channel) = http.get_channel(kennel_channel.into()).await {
            if channel.id() != ctx.channel_id() {
                kennel_reply_handle = channel.id().say(http, &kennel_message).await.ok();
            }
        }
    }

    sqlx::query!(
        r#"
        INSERT INTO kennelings
            (guild_id, victim, kenneler, kennel_length)
        VALUES
            ($1, $2, $3, $4)
            ;
        "#,
        guild_id.get().to_string(),
        user.get().to_string(),
        author_id.get().to_string(),
        pg_int,
    )
    .execute(pool)
    .await?;

    // Set kennel length activity
    // TODO: Turn this into a helper function
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
            ctx.serenity_context()
                .set_activity(Some(ActivityData::custom(format!(
                    "Kenneled users for {}",
                    format_duration(
                        Duration::from_micros(sum.microseconds as u64)
                            + Duration::from_secs(sum.days as u64 * 24 * 60 * 60)
                            + Duration::from_secs(sum.months as u64 * 30 * 24 * 60 * 60)
                    )
                ))));
        }
    }

    tokio::time::sleep(dur_time).await;

    member.remove_role(http, &role_id).await?;

    let release_message = get_formatted_message(
        &data.release_message,
        &user,
        &author_id,
        &time,
        &return_timestamp.discord_relative_timestamp(),
    );

    // For now, both will be the same release message. Maybe this will be changed?
    announcement_reply_handle
        .edit(ctx, CreateReply::default().content(&release_message))
        .await?;

    if let Some(mut kennel_msg) = kennel_reply_handle {
        let _ = kennel_msg
            .edit(ctx, EditMessage::default().content(release_message))
            .await;
    }

    Ok(())
}

async fn wildcard_command_handler(
    ctx: &SerenityCtx,
    event: &FullEvent,
    framework_ctx: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction } = event {
        if let Interaction::Command(command_interaction) = interaction {
            println!("{}", command_interaction.data.name);
            // This isn't strictly bulletproof, but it works well enough as long as the only
            // commands we want to ignore in this event listener are the globally registered
            // ones.
            if command_interaction.data.guild_id.is_none() {
                return Ok(());
            }

            // Apparently you can just... do this???? It feels so wrong.
            // No options validation here. I didn't even bother to find the
            // command - everything will use kennel_user().
            let app_ctx = ApplicationContext {
                data,
                serenity_context: ctx,
                interaction: command_interaction,
                interaction_type: poise::CommandInteractionType::Command,
                args: &command_interaction.data.options(),
                has_sent_initial_response: &std::sync::atomic::AtomicBool::new(false),
                framework: framework_ctx,
                parent_commands: &vec![],
                command: &kennel_user(),
                invocation_data: &tokio::sync::Mutex::new(Box::new(()) as _),
                __non_exhaustive: (),
            };

            let action = app_ctx
                .command
                .slash_action
                .ok_or("command structure mismatch")?;

            let _ = action(app_ctx).await;
        }
    }

    Ok(())
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

                if let Ok(res) = sqlx::query!(
                    r#"
                    SELECT SUM(kennel_length)
                    FROM kennelings
                    WHERE 
                        NOT guild_id = '849505364764524565'
                        ;
                    "#
                )
                .fetch_one(pool.as_ref())
                .await
                {
                    if let Some(sum) = res.sum {
                        ctx.set_activity(Some(ActivityData::custom(format!(
                            "Kenneled users for {}",
                            format_duration(
                                Duration::from_micros(sum.microseconds as u64)
                                    + Duration::from_secs(sum.days as u64 * 24 * 60 * 60)
                                    + Duration::from_secs(sum.months as u64 * 30 * 24 * 60 * 60)
                            )
                        ))));
                    }
                }

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
