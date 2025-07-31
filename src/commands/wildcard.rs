use std::time::Duration;

use poise::{ApplicationContext, FrameworkContext};
use serenity::all::{FullEvent, Interaction, UserId};
use serenity::client::Context as SerenityCtx;
use shame_bot::util::get_guild_id::GetGuildID;
use shame_bot::{Context, types::*};

use crate::set_activity;
use crate::{Error, ShameBotData};
use shame_bot::util::stefan_traits::*;

/// Kennels someone.
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
async fn kennel_user(
    ctx: Context<'_>,
    #[description = "User to kennel"] user: UserId,
    #[description = "Time to kennel"] time: String,
) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let guild_id = ctx.require_guild().await?;

    let Ok(dur_time) = humantime::parse_duration(&time) else {
        return ctx
            .reply_ephemeral("Invalid time format! Say something like '3m' or '1h'")
            .await;
    };

    if dur_time < Duration::from_secs(1) {
        return ctx.reply_ephemeral("Over 1 second, please...").await;
    }

    let Ok(data) = sqlx::query_as!(
        ServerRow,
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
    let server: Server = data.try_into()?;

    let http = ctx.http();
    let now = chrono::Utc::now();
    let return_timestamp = now + dur_time;

    let kenneling = Kenneling {
        guild_id,
        kennel_length: dur_time,
        kenneled_at: now,
        author_id: ctx.author().id,
        released_at: return_timestamp,
        victim_id: user,
        id: None,
    };

    let reply_handle = kenneling.apply_kennel(http, &server, Some(&ctx)).await?;
    let kenneling_row = KennelingRow::try_from(&kenneling)?;
    kenneling_row.assume_current_and_insert(pool).await?;

    set_activity(ctx.serenity_context(), pool).await;

    tokio::time::sleep(dur_time).await;

    kenneling
        .unapply_kennel(http, pool, true, reply_handle.as_ref(), Some(&ctx))
        .await?;

    Ok(())
}

/// A more-or-less from-scratch implementation of the Poise framework's command handler.
/// Necessary so that new commands can be created by users and registered while the bot is running.
pub async fn wildcard_command_handler(
    ctx: &SerenityCtx,
    event: &FullEvent,
    framework_ctx: FrameworkContext<'_, ShameBotData, Error>,
    data: &ShameBotData,
) -> Result<(), Error> {
    if let FullEvent::InteractionCreate {
        interaction: Interaction::Command(command_interaction),
    } = event
    {
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
            parent_commands: &[],
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
