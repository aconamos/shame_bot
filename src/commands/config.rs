use anyhow::{Result, anyhow};
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, RoleId};
use shame_bot::{
    Context, ShameBotData,
    util::{get_guild_id::GetGuildID, stefan_traits::SendReplyEphemeral},
};

/// Useless stub for command grouping.
#[poise::command(slash_command, subcommands("create", "set_message"))]
pub async fn kennels(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

/// Creates a new kennel in the given server.
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn create(
    ctx: Context<'_>,
    #[description = "The name of this kennel. Must be unique."] kennel_name: String,
    #[description = "The role of this kennel. Can't be shared with other kennels."]
    role: serenity::Role,
) -> Result<()> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();
    let guild_id = ctx.require_guild().await?;

    // validate uniqueness of kennel name and role id
    let Err(_) = sqlx::query!(
        r#"
        SELECT *
        FROM kennels
        WHERE
            role_id = $1
            ;
        "#,
        role.guild_id.get() as i64
    )
    .fetch_all(pool)
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
            name = $1
            ;
        "#,
        &kennel_name
    )
    .fetch_all(pool)
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
            (name, guild_id, role_id)
        VALUES
            ($1, $2, $3)
            ;
        "#,
        &kennel_name,
        guild_id.get() as i64,
        role.id.get() as i64
    )
    .execute(pool)
    .await;

    // hacky error handling for reply
    match res {
        Ok(_) => {
            ctx.reply_ephemeral(format!("New kennel {} was created!\nIt's recommended to set the announcement messages now and the kennel channel, if applicable.", &kennel_name))
                .await?;
        }
        Err(e) => {
            ctx.reply_ephemeral("Couldn't create new kennel!").await?;
            return Err(anyhow!("Error inserting kennel: {e:?}"));
        }
    }

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
    name: String,
}

async fn autocomplete_kennel(ctx: Context<'_>, partial: &str) -> impl Iterator<Item = String> {
    let ShameBotData { pool } = ctx.data();
    let Ok(guild_id) = ctx.require_guild().await else {
        return vec!["This comand should only be used inside of a guild!".into()].into_iter();
    };
    let pool = pool.as_ref();

    let potential_kennels = sqlx::query_as!(
        AutocompleteRow,
        r#"
        SELECT name
        FROM kennels
        WHERE
            guild_id = $1
            AND name ~ $2
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
        .map(|row| row.name)
        .collect();

    kennel_names.into_iter()
}

#[poise::command(slash_command)]
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

    let res = sqlx::query(
        r#"
        UPDATE kennels
        SET
            $1 = $2
        WHERE
            name = $3
            ;
        "#,
    )
    .bind(property.to_string())
    .bind(value)
    .bind(&kennel)
    .execute(pool)
    .await;

    Ok(())
}
