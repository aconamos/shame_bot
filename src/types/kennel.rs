use std::time::Duration;

use anyhow::Result;
use serenity::all::{ChannelId, CreateMessage, GuildId, Http, MessageId, RoleId, UserId};
use sqlx::{PgPool, postgres::types::PgInterval};

use crate::{
    Context, ShameBotData, get_formatted_message, util::stefan_traits::GetRelativeTimestamp,
};

/// Represents the fields available from the `kennels` table.
#[derive(Debug)]
pub struct KennelRow {
    pub id: i32,
    pub name: String,
    pub guild_id: i64,
    pub role_id: i64,
    pub msg_announce: Option<String>,
    pub msg_announce_edit: Option<String>,
    pub msg_release: Option<String>,
    pub kennel_channel_id: Option<i64>,
    pub kennel_msg: Option<String>,
    pub kennel_msg_edit: Option<String>,
    pub kennel_release_msg: Option<String>,
}

/// A Kennel
#[derive(Debug)]
pub struct Kennel {
    pub id: i32,
    pub name: String,
    pub guild_id: GuildId,
    pub role_id: RoleId,
    pub msg_announce: Option<String>,
    pub msg_announce_edit: Option<String>,
    pub msg_release: Option<String>,
    pub kennel_channel_id: Option<ChannelId>,
    pub kennel_msg: Option<String>,
    pub kennel_msg_edit: Option<String>,
    pub kennel_release_msg: Option<String>,
}

impl From<KennelRow> for Kennel {
    fn from(value: KennelRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            guild_id: GuildId::new(value.guild_id as u64),
            role_id: RoleId::new(value.guild_id as u64),
            msg_announce: value.msg_announce,
            msg_announce_edit: value.msg_announce_edit,
            msg_release: value.msg_release,
            kennel_channel_id: value
                .kennel_channel_id
                .and_then(|id| Some(ChannelId::new(id as u64))),
            kennel_msg: value.kennel_msg,
            kennel_msg_edit: value.kennel_msg_edit,
            kennel_release_msg: value.kennel_release_msg,
        }
    }
}

impl Kennel {
    /// Inserts a new kennel into the database and returns the full Kennel struct with an id.
    async fn insert(
        pool: &PgPool,
        name: String,
        guild_id: GuildId,
        role_id: RoleId,
        msg_announce: Option<String>,
        msg_announce_edit: Option<String>,
        msg_release: Option<String>,
        kennel_channel_id: Option<ChannelId>,
        kennel_msg: Option<String>,
        kennel_msg_edit: Option<String>,
        kennel_release_msg: Option<String>,
    ) -> Result<Kennel> {
        let query_res = sqlx::query_as!(
            KennelRow,
            r#"
            INSERT INTO kennels
                (
                    name, 
                    guild_id, 
                    role_id, 
                    msg_announce, 
                    msg_announce_edit, 
                    msg_release, 
                    kennel_channel_id, 
                    kennel_msg, 
                    kennel_msg_edit, 
                    kennel_release_msg
                )
            VALUES
                (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    $9,
                    $10
                )
            RETURNING *
                ;
            "#,
            name,
            guild_id.get() as i64,
            role_id.get() as i64,
            msg_announce,
            msg_announce_edit,
            msg_release,
            kennel_channel_id.and_then(|id| Some(id.get() as i64)),
            kennel_msg,
            kennel_msg_edit,
            kennel_release_msg
        )
        .fetch_one(pool)
        .await?;

        Ok(query_res.into())
    }

    /// Creates a new kenneling.
    async fn kennel_someone(
        &self,
        ctx: Context<'_>,
        author_id: UserId,
        victim_id: UserId,
        kennel_length: Duration,
    ) -> Result<()> {
        let Self {
            id,
            name,
            guild_id,
            role_id,
            msg_announce,
            kennel_channel_id,
            kennel_msg,
            ..
        } = self;

        let http = ctx.http();
        let ShameBotData { pool } = ctx.data();
        let pool = pool.as_ref();

        // Set up objects for Discord API and DB/replies
        let guild = guild_id.to_partial_guild(http).await?;
        let victim = guild.member(http, victim_id).await?;

        let current_time = chrono::Utc::now();
        let return_time = current_time + kennel_length;

        tracing::trace!(
            "Adding role to user {} for kennel {} in server {}",
            victim.display_name(),
            name,
            &guild.name
        );
        victim.add_role(http, role_id);
        tracing::trace!("Added successfully!");

        // Send announcement messages if applicable
        let mut msg_announce_id: Option<MessageId> = None;
        let mut kennel_msg_id: Option<MessageId> = None;

        if let Some(msg) = msg_announce {
            let formatted_msg = get_formatted_message(
                msg,
                &victim_id,
                &author_id,
                &humantime::format_duration(kennel_length).to_string(),
                &return_time.discord_relative_timestamp(),
            );

            let res = ctx
                .channel_id()
                .send_message(http, CreateMessage::new().content(formatted_msg))
                .await;

            if let Ok(reply_handle) = res {
                msg_announce_id = Some(reply_handle.id);
            } else {
                tracing::error!("Replying to kenneling failed!");
            }
        }

        if let Some(channel) = kennel_channel_id
            && let Some(msg) = kennel_msg
        {
            let formatted_msg = get_formatted_message(
                msg,
                &victim_id,
                &author_id,
                &humantime::format_duration(kennel_length).to_string(),
                &return_time.discord_relative_timestamp(),
            );

            let res = channel
                .send_message(http, CreateMessage::new().content(formatted_msg))
                .await;

            if let Ok(reply_handle) = res {
                kennel_msg_id = Some(reply_handle.id);
            } else {
                tracing::error!("Announcement in kenneling channel failed!");
            }
        }

        // Insert kenneling into database - hope this doesn't fail!
        let kennel_length_pgint: PgInterval = kennel_length
            .try_into()
            .expect("Microsecond duration encountered in kennel_length!");

        let res = sqlx::query!(
            r#"
            INSERT INTO kennelings
                (
                    kennel_id,
                    guild_id,
                    author_id,
                    victim_id,
                    kennel_length,
                    msg_announce_id,
                    kennel_msg_id
                )
            VALUES
                (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7
                )
            RETURNING *
                ;
            "#,
            id,
            guild_id.get() as i64,
            author_id.get() as i64,
            victim_id.get() as i64,
            kennel_length_pgint,
            msg_announce_id.map(|id| id.get() as i64),
            kennel_msg_id.map(|id| id.get() as i64),
        )
        .fetch_one(pool)
        .await;

        if let Err(e) = res {
            tracing::error!("Error encountered inserting kenneling into database! {e:?}");
        } else {
            tracing::trace!("Kenneling inserted");
        }

        Ok(())
    }
}
