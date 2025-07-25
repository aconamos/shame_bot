use std::time::Duration;

use poise::{ApplicationContext, CreateReply, FrameworkContext};
use serenity::all::{ActivityData, EditMessage, FullEvent, Interaction, Message, RoleId, UserId};
use serenity::client::Context as SerenityCtx;
use sqlx::postgres::types::PgInterval;

use crate::set_activity;
use crate::{Context, Data, Error, get_formatted_message, util::stefan_traits::*};

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

    let Ok(dur_time) = humantime::parse_duration(&time) else {
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

    set_activity(ctx.serenity_context(), pool).await;

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

/// A more-or-less from-scratch implementation of the Poise framework's command handler.
/// Necessary so that new commands can be created by users and registered while the bot is running.
pub async fn wildcard_command_handler(
    ctx: &SerenityCtx,
    event: &FullEvent,
    framework_ctx: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let FullEvent::InteractionCreate {
        interaction: Interaction::Command(command_interaction),
    } = event
    {
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

    Ok(())
}
