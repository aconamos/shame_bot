//! Contains commands for configuring the bot's usage in a given server.

use ::serenity::all::{ChannelId, CreateCommand, Permissions, RoleId};
use poise::serenity_prelude as serenity;
use regex::Regex;
use shame_bot::types::kenneling::*;

use crate::ShameBotData;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, ShameBotData, Error>;

/// Sets the kennel role.
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn set_kennel_role(
    ctx: Context<'_>,
    #[description = "The kenneling role. Must be set for the command to work"] role: serenity::Role,
) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let role_id = role.id.get();
    let guild_id = role.guild_id.get();

    if let Ok(res) = sqlx::query!(
        r#"
        SELECT
            role_id
        FROM
            servers
        WHERE
            guild_id=$1
            ;
        "#,
        guild_id.to_string(),
    )
    .fetch_one(pool)
    .await
    {
        let guild = ctx
            .partial_guild()
            .await
            .expect("Why is this called outside of a guild");
        let existing_role_id: RoleId = shame_bot::string_to_id(&res.role_id)?;

        let active_kennelings: Vec<Kenneling> = sqlx::query_as!(
            KennelingRow,
            r#"
            SELECT *
            FROM kennelings
            WHERE
                released_at > CURRENT_TIMESTAMP AND
                guild_id = $1
                ;
            "#,
            guild_id.to_string()
        )
        .fetch_all(pool)
        .await?
        .iter()
        .map(|kr| kr.try_into().expect("malformed data inserted"))
        .collect();

        tracing::trace!(
            "set_kennel_role called: Updating active kennelings for guild {}",
            guild.name
        );

        for kenneling in active_kennelings {
            tracing::trace!("Updating kenneling: {kenneling:?}");

            let member = guild
                .member(ctx.http(), kenneling.victim)
                .await
                .expect("Member must have left!");

            member.remove_role(ctx.http(), existing_role_id).await?;
            member.add_role(ctx.http(), role_id).await?;
        }
    }

    sqlx::query!(
        r#"
        INSERT INTO servers
            (guild_id, role_id) 
        VALUES 
            ($1, $2)
        ON CONFLICT 
            (guild_id)
        DO 
            UPDATE SET
                role_id=$2
            ;
        "#,
        guild_id.to_string(),
        role_id.to_string()
    )
    .execute(pool)
    .await?;

    ctx.reply(format!(
        "Successfully set this guild's kennel role to <@&{}>!",
        role.id
    ))
    .await?;

    Ok(())
}

/// Sets the command to kennel someone.
#[poise::command(
    slash_command,
    default_member_permissions = "ADMINISTRATOR",
    guild_cooldown = 60
)]
pub async fn set_kennel_command(
    ctx: Context<'_>,
    #[description = "The command to kennel someone. Defaults to 'kennel'"] command: Option<String>,
) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let command = command.unwrap_or_else(|| "kennel".to_string());

    let Some(guild_id) = ctx.guild_id() else {
        ctx.reply("This command can only be used in a server!")
            .await?;

        return Ok(());
    };

    // TODO: Figure out Discord's actual regex. It's on the docs... somewhere
    let re = Regex::new(r"^[a-zA-Z][a-zA-Z_]+$").expect("Idiot coder coded bad code!");

    if !re.is_match(&command) {
        ctx.reply(format!(
            "Cannot set the command to {}! Make sure it only uses letters!",
            &command
        ))
        .await?;

        return Ok(());
    }

    let rows_affected = sqlx::query!(
        r#"
        UPDATE servers
        SET 
            command_name = $1
        WHERE
            guild_id = $2
            ;
        "#,
        &command,
        guild_id.get().to_string()
    )
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        ctx.reply("Couldn't set command! Make sure to set the kennel role using `/set_kennel_role` first!").await?;

        return Ok(());
    }

    let cmd = shame_bot::get_kennel_command_struct(&command);

    tracing::debug!(
        "{:?}",
        ctx.http().create_guild_commands(guild_id, &vec![cmd]).await
    );

    ctx.reply(format!(
        "Successfully set this guild's kennel command to: /{}",
        &command
    ))
    .await?;

    Ok(())
}

/// Sets the message sent when kenneling someone in the channel where it's called.
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn set_announcement_message(
    ctx: Context<'_>,
    #[description = "The message to send when kenneling someone. Use $victim, $kenneler, $time, and $return to format."]
    message: String,
) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();

    let Some(guild_id) = ctx.guild_id() else {
        ctx.reply("This command can only be used in a server!")
            .await?;

        return Ok(());
    };

    let rows_affected = sqlx::query!(
        r#"
        UPDATE servers
        SET
            announcement_message = $1
        WHERE 
            guild_id = $2
            ;
        "#,
        message,
        guild_id.get().to_string(),
    )
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        ctx.reply("Couldn't set announcement message! Make sure to set the kennel role using `/set_kennel_role` first!").await?;
    } else {
        ctx.reply(format!("Set announcement message to: {}", message))
            .await?;
    }

    Ok(())
}

/// Sets the message sent when kenneling someone in the kennel channel.
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn set_kennel_message(
    ctx: Context<'_>,
    #[description = "The message to send in the kennel when kenneling someone."] message: String,
) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();

    let Some(guild_id) = ctx.guild_id() else {
        ctx.reply("This command can only be used in a server!")
            .await?;

        return Ok(());
    };

    let rows_affected = sqlx::query!(
        r#"
        UPDATE servers
        SET
            kennel_message = $1
        WHERE 
            guild_id = $2
            ;
        "#,
        message,
        guild_id.get().to_string(),
    )
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        ctx.reply("Couldn't set kenneling message! Make sure to set the kennel role using `/set_kennel_role` first!").await?;
    } else {
        ctx.reply(format!("Set kenneling message to: {}", message))
            .await?;
    }

    Ok(())
}

/// Sets the message sent when someone is released from the kennel.
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn set_release_message(
    ctx: Context<'_>,
    #[description = "The released from kennel message. Use $victim, $kenneler, $time, and $return to format."]
    message: String,
) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();

    let Some(guild_id) = ctx.guild_id() else {
        ctx.reply("This command can only be used in a server!")
            .await?;

        return Ok(());
    };

    let rows_affected = sqlx::query!(
        r#"
        UPDATE servers
        SET
            release_message = $1
        WHERE
            guild_id = $2
            ;
        "#,
        message,
        guild_id.get().to_string(),
    )
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        ctx.reply("Couldn't set release message! Make sure to set the kennel role using `/set_kennel_role` first!").await?;
    } else {
        ctx.reply(format!("Set release message to: {}", message))
            .await?;
    }

    Ok(())
}

/// An optional channel to send kennel messages to, so that victims know how long they're kenneled for.
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn set_kennel_channel(
    ctx: Context<'_>,
    #[description = "The kennel channel to announce in"] message: ChannelId,
) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();

    let Some(guild_id) = ctx.guild_id() else {
        ctx.reply("This command can only be used in a server!")
            .await?;

        return Ok(());
    };

    let rows_affected = sqlx::query!(
        r#"
        UPDATE servers
        SET
            kennel_channel = $1
        WHERE
            guild_id = $2
            ;
        "#,
        message.get().to_string(),
        guild_id.get().to_string(),
    )
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        ctx.reply("Couldn't set kennel channel! Make sure to set the kennel role using `/set_kennel_role` first!").await?;
    } else {
        ctx.reply(format!("Set kennel channel to: {}", message))
            .await?;
    }

    Ok(())
}
