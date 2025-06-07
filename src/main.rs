use std::time::Duration;

use ::serenity::all::Permissions;
use ::serenity::all::{
    ActivityData, CacheHttp, Context as SerenityCtx, CreateCommand, CreateCommandOption, FullEvent,
    Interaction, RoleId, UserId,
};
use dotenv::dotenv;
use humantime::format_duration;
use humantime::parse_duration;
use poise::{ApplicationContext, CreateReply, FrameworkContext, serenity_prelude as serenity};
use setup_commands::*;
use sqlx::postgres::types::PgInterval;
use sqlx::{PgPool, postgres::PgPoolOptions};

mod setup_commands;

// User data, which is stored and accessible in all command invocations
pub(crate) struct Data {
    pool: PgPool,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// https://github.com/coravacav/uofu-cs-discord-bot/blob/0513983163f0563a26709004dccb954948dffb2a/bot-lib/src/utils.rs
pub trait GetRelativeTimestamp {
    fn discord_relative_timestamp(&self) -> String;
}

impl GetRelativeTimestamp for chrono::DateTime<chrono::Utc> {
    fn discord_relative_timestamp(&self) -> String {
        format!("<t:{}:R>", self.timestamp())
    }
}

trait SendReplyEphemeral {
    async fn reply_ephemeral(&self, content: impl Into<String>) -> Result<(), Error>;
}

impl SendReplyEphemeral for Context<'_> {
    async fn reply_ephemeral(&self, content: impl Into<String>) -> Result<(), Error> {
        let reply = CreateReply::default()
            .reply(true)
            .ephemeral(true)
            .content(content);

        self.send(reply).await?;

        Ok(())
    }
}

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

/// Kennels someone.
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
async fn kennel_user(
    ctx: Context<'_>,
    #[description = "User to kennel"] user: UserId,
    #[description = "Time to kennel"] time: String,
) -> Result<(), Error> {
    let Ok(dur_time) = parse_duration(&time) else {
        return ctx
            .reply_ephemeral("Invalid time format! Say something like '3m' or '1h'")
            .await;
    };

    if dur_time < Duration::from_secs(1) {
        return ctx.reply_ephemeral("Over 1 second, please...").await;
    }

    let data = sqlx::query!(
        r#"
        SELECT * FROM servers
        WHERE
        guild_id = $1
        AND command_name = $2
        ;
        "#,
        format!("{}", ctx.guild_id().ok_or("No guild ID!")?.get()),
        ctx.invoked_command_name()
    )
    .fetch_one(&ctx.data().pool)
    .await?;

    let Some(role_id) = data.role_id else {
        return ctx.reply_ephemeral("Set kennel role first!").await;
    };
    let http = ctx.http();
    let guild = ctx.guild_id().expect("No guild ID!");
    let member = guild.member(http, user).await?;
    let role_id = RoleId::new(role_id.parse::<u64>()?);
    let return_time = chrono::Utc::now() + dur_time.clone();

    let reply = get_formatted_message(
        &data.command_verb.unwrap_or(
            "$kenneler has kenneled $victim for $time.\n\nThey will be released $return."
                .to_string(),
        ),
        &user,
        &ctx.author().id,
        &time,
        &return_time.discord_relative_timestamp(),
    );

    member.add_role(http, &role_id).await?;

    let reply_handle = ctx.reply(reply).await?;

    if let Ok(pg_int) = PgInterval::try_from(dur_time) {
        sqlx::query!(
            r#"
            INSERT INTO kennelings(guild_id, user_id, kennel_length, kenneled_at)
            VALUES($1, $2, $3, NOW())
            ;
        "#,
            format!("{}", guild.get()),
            format!("{}", user.get()),
            pg_int,
        )
        .execute(&ctx.data().pool)
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
    .fetch_one(&ctx.data().pool)
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

    reply_handle
        .edit(
            ctx,
            CreateReply::default().content(get_formatted_message(
                &data
                    .release_message
                    .unwrap_or("$victim has been released from the kennel.".to_string()),
                &user,
                &ctx.author().id,
                &time,
                &return_time.discord_relative_timestamp(),
            )),
        )
        .await?;

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
            if command_interaction.data.name == "set_kennel_command"
                || command_interaction.data.name == "set_kennel_role"
            {
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

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                set_kennel_role(),
                set_kennel_command(),
                set_kennel_message(),
            ],
            event_handler: |w, x, y, z| Box::pin(wildcard_command_handler(w, x, y, z)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(postgres_url.as_str())
                    .await?;

                let data = sqlx::query!(
                    r#"
                SELECT * FROM servers
                ;
                "#
                )
                .fetch_all(&pool)
                .await?;

                for thing in data {
                    if let Some(name) = thing.command_name {
                        let cmd = CreateCommand::new(&name)
                            .description("Punish a user!")
                            .add_option(
                                CreateCommandOption::new(
                                    serenity::CommandOptionType::User,
                                    "user",
                                    "User to be punished",
                                )
                                .required(true),
                            )
                            .add_option(
                                CreateCommandOption::new(
                                    serenity::CommandOptionType::String,
                                    "time",
                                    "How long to punish the user",
                                )
                                .required(true),
                            )
                            .default_member_permissions(Permissions::MODERATE_MEMBERS);

                        ctx.http()
                            .create_guild_commands(
                                thing.guild_id.parse::<u64>()?.into(),
                                &vec![cmd],
                            )
                            .await?;
                    }
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
                .fetch_one(&pool)
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
                Ok(Data { pool })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
