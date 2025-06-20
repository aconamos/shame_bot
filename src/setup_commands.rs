//! Contains commands for configuring the bot's usage in a given server.

use ::serenity::all::{ChannelId, CreateCommand, Permissions};
use poise::serenity_prelude as serenity;
use regex::Regex;

use crate::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Returns a [`CreateCommand`] that represents the general kennel command object for the Discord API.
pub fn get_kennel_command_struct(command: &str) -> CreateCommand {
    serenity::CreateCommand::new(command)
        .description("Punish a user!")
        .add_option(
            serenity::CreateCommandOption::new(
                serenity::CommandOptionType::User,
                "user",
                "User to be punished",
            )
            .required(true),
        )
        .add_option(
            serenity::CreateCommandOption::new(
                serenity::CommandOptionType::String,
                "time",
                "How long to punish the user",
            )
            .required(true),
        )
        .default_member_permissions(Permissions::MODERATE_MEMBERS)
}

/// Sets the kennel role.
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn set_kennel_role(
    ctx: Context<'_>,
    #[description = "The kenneling role. Must be set for the command to work"] role: serenity::Role,
) -> Result<(), Error> {
    let Data { pool } = ctx.data();
    let role_id = role.id.get();
    let guild_id = role.guild_id.get();

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
    let Data { pool } = ctx.data();
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

    let cmd = get_kennel_command_struct(&command);

    println!(
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
    let Data { pool } = ctx.data();

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
    let Data { pool } = ctx.data();

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
    let Data { pool } = ctx.data();

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
    let Data { pool } = ctx.data();

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
