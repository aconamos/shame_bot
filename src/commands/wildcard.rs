use anyhow::{Context as _, Result};
use poise::{ApplicationContext, FrameworkContext};
use serenity::all::{FullEvent, Interaction, UserId};
use serenity::client::Context as SerenityCtx;
use shame_bot::util::get_guild_id::GetGuildID;
use shame_bot::{Context, types::*};
use std::time::Duration;

use crate::ShameBotData;
use crate::set_activity;
use shame_bot::util::stefan_traits::*;

/// Kennels someone.
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
async fn kennel_user(
    ctx: Context<'_>,
    #[description = "User to kennel"] user: UserId,
    #[description = "Time to kennel"] time: String,
) -> Result<()> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let guild_id = ctx.require_guild().await?;

    let Ok(dur_time) = humantime::parse_duration(&time) else {
        return ctx
            .reply_ephemeral("Invalid time format! Say something like '3m' or '1h'")
            .await
            .context("Couldn't send reply!");
    };

    if dur_time < Duration::from_secs(1) {
        return ctx.reply_ephemeral("Over 1 second, please...").await;
    }

    let Ok(kennel) = sqlx::query_as!(
        KennelRow,
        r#"
        SELECT * FROM kennels
        WHERE
            guild_id = $1
            AND command = $2
            ;
        "#,
        guild_id.get() as i64,
        ctx.invoked_command_name()
    )
    .fetch_one(pool)
    .await
    else {
        ctx.reply_ephemeral("Set kennel role first!").await?;

        return Ok(());
    };

    let kennel: Kennel = kennel.into();

    let kenneling = kennel
        .kennel_someone(ctx, ctx.author().id, user, dur_time)
        .await?;

    let _ = ctx.reply_ephemeral("punishment administered!").await;

    set_activity(ctx.serenity_context(), pool).await;

    tokio::time::sleep(dur_time).await;

    kennel.unkennel_someone(ctx, &kenneling).await?;

    Ok(())
}

/// A more-or-less from-scratch implementation of the Poise framework's command handler.
/// Necessary so that new commands can be created by users and registered while the bot is running.
pub async fn wildcard_command_handler(
    ctx: &SerenityCtx,
    event: &FullEvent,
    framework_ctx: FrameworkContext<'_, ShameBotData, anyhow::Error>,
    data: &ShameBotData,
) -> Result<()> {
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
            .with_context(|| "Command structure mismatch")?;

        let result = action(app_ctx).await;

        if let Err(fw_err) = result {
            (framework_ctx.options.on_error)(fw_err).await;
        }
    }

    Ok(())
}
