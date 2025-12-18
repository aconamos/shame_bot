use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;
use shame_bot::{
    Context, ShameBotData, get_kennel_command_struct,
    types::{Kennel, KennelRow},
    util::{get_guild_id::GetGuildID, stefan_traits::SendReplyEphemeral},
};

static COMMAND_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^[-_'\p{L}\p{N}\p{sc=Deva}\p{sc=Thai}]{1,32}$").unwrap()
});

/// Useless stub for command grouping.
#[poise::command(slash_command, subcommands("create", "set_message", "toggle_metrics"))]
pub async fn kennels(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

/// Creates a new kennel in the given server.
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn create(
    ctx: Context<'_>,
    #[description = "The name of this kennel. Must be unique. This will be the command as well."]
    command: String,
    #[description = "The role of this kennel. Can't be shared with other kennels."]
    role: serenity::Role,
) -> Result<()> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let guild_id = ctx.require_guild().await?;

    if !COMMAND_REGEX.is_match(&command) {
        return ctx
            .reply_ephemeral("Command name must consist of up to 32 characters which are _, -, ', letters, and numbers only!")
            .await;
    };

    // validate uniqueness of kennel name to guild and role id (not necessary to validate role with guild because they are snowflakes)
    let Err(_) = sqlx::query!(
        r#"
        SELECT *
        FROM kennels
        WHERE
            role_id = $1
            ;
        "#,
        role.id.get() as i64
    )
    .fetch_one(pool)
    .await
    else {
        return ctx
            .reply_ephemeral("There is already a kennel with the given role!")
            .await;
    };

    let Err(_) = sqlx::query!(
        r#"
        SELECT *
        FROM kennels
        WHERE
            command = $1
            AND guild_id = $2
            ;
        "#,
        &command,
        guild_id.get() as i64
    )
    .fetch_one(pool)
    .await
    else {
        return ctx
            .reply_ephemeral("There is already a kennel with the given name!")
            .await;
    };

    // insert kennel
    let res = sqlx::query!(
        r#"
        INSERT INTO kennels
            (command, guild_id, role_id)
        VALUES
            ($1, $2, $3)
            ;
        "#,
        &command,
        guild_id.get() as i64,
        role.id.get() as i64
    )
    .execute(pool)
    .await;

    // hacky error handling for reply
    match res {
        Ok(_) => {
            ctx.reply_ephemeral(format!("New kennel `{}` was created!\nIt's recommended to set the announcement messages now and the kennel channel, if applicable.", &command))
                .await?;
        }
        Err(err) => {
            let _ = ctx.reply_ephemeral("Couldn't create new kennel!").await;
            return Err(err).context("Error inserting kennel!");
        }
    }

    let cmd = get_kennel_command_struct(&command);

    match ctx.http().create_guild_command(guild_id, &cmd).await {
        Ok(_) => {
            tracing::debug!("Guild command {command} for {guild_id} created!");
        }
        Err(e) => {
            tracing::error!("Failed to create command {command} for guild {guild_id}! {e}");
            return Err(e.into());
        }
    };

    Ok(())
}

/// This represents a type of message that the bot will use when (un)kenneling.
#[derive(Debug, poise::ChoiceParameter)]
pub enum MessageType {
    #[name = "Announcement"]
    Announce,
    #[name = "Announcement Edit"]
    AnnounceEdit,
    #[name = "Release"]
    Release,
    #[name = "Kennel Announcement"]
    Kennel,
    #[name = "Kennel Announcement Edit"]
    KennelEdit,
    #[name = "Kennel Release"]
    KennelRelease,
}

impl ToString for MessageType {
    fn to_string(&self) -> String {
        match self {
            MessageType::Announce => "msg_announce",
            MessageType::AnnounceEdit => "msg_announce_edit",
            MessageType::Release => "msg_release",
            MessageType::Kennel => "kennel_msg",
            MessageType::KennelEdit => "kennel_msg_edit",
            MessageType::KennelRelease => "kennel_release_msg",
        }
        .into()
    }
}

/// A row from the autocomplete query, so that defined types can be used instead of the anonymous
/// record thingies.
struct AutocompleteRow {
    command: String,
}

async fn autocomplete_kennel(ctx: Context<'_>, partial: &str) -> impl Iterator<Item = String> {
    let ShameBotData { pool } = ctx.data();
    // I seriously doubt that it is possible for this code path to even execute.
    // I'm 99% sure. I'm not doing an unreachable!() though.
    let Ok(guild_id) = ctx.require_guild().await else {
        return vec!["This comand should only be used inside of a guild!".into()].into_iter();
    };
    let pool = pool.as_ref();

    let potential_kennels = sqlx::query_as!(
        AutocompleteRow,
        r#"
        SELECT command
        FROM kennels
        WHERE
            guild_id = $1
            AND command ~ $2
            ;
        "#,
        guild_id.get() as i64,
        partial
    )
    .fetch_all(pool)
    .await;

    let kennel_names: Vec<String> = potential_kennels
        .unwrap_or(vec![])
        .into_iter()
        .map(|row| row.command)
        .collect();

    kennel_names.into_iter()
}

/// Sets a message in the bot config.
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn set_message(
    ctx: Context<'_>,
    #[description = "Which message to modify"] property: MessageType,
    #[description = "The kennel to modify"]
    #[autocomplete = "autocomplete_kennel"]
    kennel: String,
    #[description = "The new message, or leave empty to remove the message"] value: Option<String>,
) -> Result<()> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let guild_id = ctx.require_guild().await?;

    let Ok(_kennel_row) = sqlx::query_as!(
        KennelRow,
        r#"
        SELECT *
        FROM kennels
        WHERE
            command = $1
            AND guild_id = $2
            ;
        "#,
        &kennel,
        guild_id.get() as i64
    )
    .fetch_one(pool)
    .await
    else {
        return ctx
            .reply_ephemeral("No kennel with the given name exists!")
            .await;
    };

    // TODO: might be worth refactoring to use Kennel's .update(), but this works as is.

    let base_statement = format!(
        r#"
        UPDATE kennels
        SET
            {} = $1
        WHERE
            command = $2
            AND guild_id = $3
            ;
        "#,
        // This reeks of SQL injection! It's fun!
        property.to_string()
    );

    let res = sqlx::query(&base_statement)
        .bind(&value)
        .bind(&kennel)
        .bind(guild_id.get() as i64)
        .execute(pool)
        .await;

    if let Err(err) = res {
        let reply = format!(
            "Couldn't set {} for kennel {} to `{}`",
            property.to_string(),
            &kennel,
            &value.unwrap_or("".into())
        );

        let _ = ctx.reply_ephemeral(&reply).await;

        return Err(err).context(reply);
    }

    let reply = match value {
        Some(val) => format!("Set {} to {}!", property.to_string(), &val),
        None => format!("Removed message from {}", property.to_string()),
    };

    let _ = ctx.reply_ephemeral(&reply).await;

    Ok(())
}

/// Opts in or out of metrics
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn toggle_metrics(
    ctx: Context<'_>,
    #[description = "The kennel to modify"]
    #[autocomplete = "autocomplete_kennel"]
    kennel: String,
) -> Result<()> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let guild_id = ctx.require_guild().await?;

    let query_res = sqlx::query_as!(
        KennelRow,
        r#"
        SELECT *
        FROM kennels
        WHERE
            command = $1
            AND guild_id = $2
            ;
        "#,
        &kennel,
        guild_id.get() as i64
    )
    .fetch_one(pool)
    .await;

    let Ok(kennel) = query_res else {
        return ctx.reply_ephemeral("Couldn't find the given kennel!").await;
    };

    let mut kennel: Kennel = kennel.into();

    kennel.opt_in_to_metrics = !kennel.opt_in_to_metrics;

    let res = kennel.update(pool).await;

    if let Err(err) = res {
        let _ = ctx.reply_ephemeral("A database error occurred!").await;

        return Err(err).context(format!("Couldn't toggle metrics for kennel {:?}", kennel));
    }

    let _ = ctx
        .reply_ephemeral(match kennel.opt_in_to_metrics {
            true => "Toggled metrics to on!",
            false => "Toggled metrics to off!",
        })
        .await;

    Ok(())
}
