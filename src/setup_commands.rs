use poise::serenity_prelude as serenity;
use regex::Regex;

use crate::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Sets the kennel role.
#[poise::command(slash_command)]
pub async fn set_kennel_role(
    ctx: Context<'_>,
    #[description = "The kenneling role"] role: serenity::Role,
) -> Result<(), Error> {
    let Data { pool } = ctx.data();

    sqlx::query(
        r#"
        INSERT INTO servers(guild_id, role_id) 
        VALUES ($1, $2)
        ON CONFLICT (guild_id)
        DO UPDATE SET
            role_id=$2
            ;
    "#,
    )
    .bind(format!("{}", role.guild_id.get()))
    .bind(format!("{}", role.id.get()))
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
#[poise::command(slash_command)]
pub async fn set_kennel_command(
    ctx: Context<'_>,
    #[description = "The command to kennel someone"] command: String,
) -> Result<(), Error> {
    let Data { pool } = ctx.data();

    let guild_id = ctx.guild_id().expect("No guild ID???");

    let re = Regex::new(r"^[a-zA-Z_]+$").unwrap();

    if !re.is_match(command.as_str()) {
        ctx.reply(format!(
            "Cannot set the command to {}! Make sure it only uses letters!",
            &command
        ))
        .await?;

        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO servers(guild_id, command_name) 
        VALUES ($1, $2)
        ON CONFLICT (guild_id)
        DO UPDATE SET
            command_name=$2
            ;
    "#,
    )
    .bind(format!("{}", &guild_id))
    .bind(&command)
    .execute(pool)
    .await?;

    let cmd = serenity::CreateCommand::new(&command)
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
        );

    ctx.http()
        .create_guild_commands(guild_id, &vec![cmd])
        .await?;

    ctx.reply(format!(
        "Successfully set this guild's kennel command to: /{}",
        &command
    ))
    .await?;

    Ok(())
}

/// Sets the message sent when kenneling someone.
#[poise::command(slash_command)]
pub async fn set_kennel_message(
    ctx: Context<'_>,
    #[description = "The message to send when kenneling someone. Use $victim, $kenneler, $time, and $return to format."]
    message: String,
) -> Result<(), Error> {
    sqlx::query!(
        r#"
            INSERT INTO servers(guild_id, command_verb) 
            VALUES ($1, $2)
            ON CONFLICT (guild_id)
            DO UPDATE SET
                command_verb=$2
                ;            
        "#,
        format!(
            "{}",
            ctx.guild_id()
                .ok_or("stop it stop it stop it right now")?
                .get()
        ),
        message,
    )
    .execute(&ctx.data().pool)
    .await?;

    ctx.reply(format!("Set message to: {}", message)).await?;

    Ok(())
}

/// Sets the message sent when someone is released from the kennel.
#[poise::command(slash_command)]
pub async fn set_release_message(
    ctx: Context<'_>,
    #[description = "The message to send when someone is released from the kennel. Use $victim, $kenneler, $time, and $return to format."]
    message: String,
) -> Result<(), Error> {
    sqlx::query!(
        r#"
            INSERT INTO servers(guild_id, release_message) 
            VALUES ($1, $2)
            ON CONFLICT (guild_id)
            DO UPDATE SET
                release_message=$2
                ;            
        "#,
        format!(
            "{}",
            ctx.guild_id()
                .ok_or("stop it stop it stop it right now")?
                .get()
        ),
        message,
    )
    .execute(&ctx.data().pool)
    .await?;

    ctx.reply(format!("Set message to: {}", message)).await?;

    Ok(())
}
