use std::time::Duration;

use anyhow::{Context as _, Result};
use serenity::all::{ChannelId, CreateMessage, EditMessage, GuildId, MessageId, RoleId, UserId};
use sqlx::{PgPool, postgres::types::PgInterval, query_as};

use crate::{
    Context, ShameBotData, get_formatted_message,
    types::{Kenneling, KennelingRow},
    util::stefan_traits::GetRelativeTimestamp,
};

/// Represents the fields available from the `kennels` table.
#[derive(Debug)]
pub struct KennelRow {
    pub id: i32,
    pub command: String,
    pub guild_id: i64,
    pub role_id: i64,
    pub msg_announce: Option<String>,
    pub msg_announce_edit: Option<String>,
    pub msg_release: Option<String>,
    pub kennel_channel_id: Option<i64>,
    pub kennel_msg: Option<String>,
    pub kennel_msg_edit: Option<String>,
    pub kennel_release_msg: Option<String>,
    pub opt_in_to_metrics: bool,
}

/// A Kennel
#[derive(Debug)]
pub struct Kennel {
    pub id: i32,
    pub command: String,
    pub guild_id: GuildId,
    pub role_id: RoleId,
    pub msg_announce: Option<String>,
    pub msg_announce_edit: Option<String>,
    pub msg_release: Option<String>,
    pub kennel_channel_id: Option<ChannelId>,
    pub kennel_msg: Option<String>,
    pub kennel_msg_edit: Option<String>,
    pub kennel_release_msg: Option<String>,
    pub opt_in_to_metrics: bool,
}

impl From<KennelRow> for Kennel {
    fn from(row_value: KennelRow) -> Self {
        Self {
            id: row_value.id,
            command: row_value.command,
            guild_id: GuildId::new(row_value.guild_id as u64),
            role_id: RoleId::new(row_value.role_id as u64),
            msg_announce: row_value.msg_announce,
            msg_announce_edit: row_value.msg_announce_edit,
            msg_release: row_value.msg_release,
            kennel_channel_id: row_value
                .kennel_channel_id
                .map(|id| ChannelId::new(id as u64)),
            kennel_msg: row_value.kennel_msg,
            kennel_msg_edit: row_value.kennel_msg_edit,
            kennel_release_msg: row_value.kennel_release_msg,
            opt_in_to_metrics: row_value.opt_in_to_metrics,
        }
    }
}

impl Kennel {
    /// Updates this kennel in the database.
    ///
    /// It is worth noting that this exposes an API internally that allows potentially invalid data entry.
    /// The public-facing API through the bot ensures that data is entered correctly, but be cautious
    /// while calling this method, as it could lead to unexpected/undefined behavior if fields that
    /// aren't normally mutable are changed.
    pub async fn update(&self, pool: &PgPool) -> Result<()> {
        let Self {
            // These fields are generally considered immutable.
            id,
            command,
            guild_id,
            role_id,
            // These fields are mutable.
            msg_announce,
            msg_announce_edit,
            msg_release,
            kennel_channel_id,
            kennel_msg,
            kennel_msg_edit,
            kennel_release_msg,
            opt_in_to_metrics,
        } = self;

        let query_res = sqlx::query!(
            r#"
            UPDATE kennels
                SET 
                command = $2,
                guild_id = $3,
                role_id = $4,
                msg_announce = $5,
                msg_announce_edit = $6,
                msg_release = $7,
                kennel_channel_id = $8,
                kennel_msg = $9,
                kennel_msg_edit = $10,
                kennel_release_msg = $11,
                opt_in_to_metrics = $12
            WHERE
                id = $1
                ;
            "#,
            id,
            command,
            guild_id.get() as i64,
            role_id.get() as i64,
            msg_announce.as_ref(),
            msg_announce_edit.as_ref(),
            msg_release.as_ref(),
            kennel_channel_id.map(|id| id.get() as i64),
            kennel_msg.as_ref(),
            kennel_msg_edit.as_ref(),
            kennel_release_msg.as_ref(),
            opt_in_to_metrics,
        )
        .execute(pool)
        .await;

        query_res
            .map(|_| ())
            .context(format!("Error updating kennel {} to {:?}", id, self))
    }

    /// Creates a new kenneling.
    pub async fn kennel_someone(
        &self,
        ctx: Context<'_>,
        author_id: UserId,
        victim_id: UserId,
        kennel_length: Duration,
    ) -> Result<Kenneling> {
        let Self {
            id,
            command: name,
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

        if let Err(err) = victim.add_role(http, role_id).await {
            tracing::error!("error: {err:?}");
            return Err(err).context("Couldn't add role to victim for kenneling ");
        } else {
            tracing::trace!("Added successfully!");
        }

        // Send announcement messages if applicable
        let mut msg_announce_id: Option<MessageId> = None;
        let mut kennel_msg_id: Option<MessageId> = None;

        // todo: refactor to send_messages fn

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

                if let Err(e) = sqlx::query!(
                    r#"
                    INSERT INTO sent_messages
                        (message_id, channel_id)
                    VALUES
                        ($1, $2)
                        ;
                    "#,
                    reply_handle.id.get() as i64,
                    ctx.channel_id().get() as i64,
                )
                .execute(pool)
                .await
                {
                    tracing::error!("Error inserting message! {:?}", e);
                }
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

                if let Err(e) = sqlx::query!(
                    r#"
                    INSERT INTO sent_messages
                        (message_id, channel_id)
                    VALUES
                        ($1, $2)
                        ;
                    "#,
                    reply_handle.id.get() as i64,
                    channel.get() as i64,
                )
                .execute(pool)
                .await
                {
                    tracing::error!("Error inserting message! {:?}", e);
                }
            } else {
                tracing::error!("Announcement in kenneling channel failed!");
            }
        }

        // Insert kenneling into database - hope this doesn't fail!
        let kennel_length_pgint: PgInterval = kennel_length
            .try_into()
            .expect("Microsecond duration encountered in kennel_length!");

        let res = sqlx::query_as!(
            KennelingRow,
            r#"
            WITH k AS 
            (
                INSERT INTO kennelings
                    (
                        kennel_id,
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
                        $6
                    )
                RETURNING *
            )
            SELECT 
                k.id,
                k.kennel_id,
                k.author_id,
                k.victim_id,
                k.kenneled_at,
                k.kennel_length,
                k.released_at,
                k.msg_announce_id,
                k.kennel_msg_id,
                a.channel_id as msg_announce_channel_id,
                b.channel_id as kennel_msg_channel_id
            FROM k
            LEFT JOIN sent_messages a
            ON
                a.message_id = k.msg_announce_id
            LEFT JOIN sent_messages b
            ON
                b.message_id = k.kennel_msg_id
                ;
            "#,
            id,
            author_id.get() as i64,
            victim_id.get() as i64,
            kennel_length_pgint,
            msg_announce_id.map(|id| id.get() as i64),
            kennel_msg_id.map(|id| id.get() as i64),
        )
        .fetch_one(pool)
        .await;

        if let Err(e) = &res {
            tracing::error!("{e:?}");
            tracing::error!("{}", e.to_string());
        }

        match res {
            Err(err) => Err(err).context(format!("Couldn't insert kenneling into the database!")),
            Ok(kennel) => Ok((&kennel).into()),
        }
    }

    pub async fn unkennel_someone(&self, ctx: Context<'_>, kenneling: &Kenneling) -> Result<()> {
        let ShameBotData { pool } = ctx.data();
        let http = ctx.http();
        let pool = pool.as_ref();

        let Self {
            guild_id, role_id, ..
        } = self;

        let Kenneling { victim_id, .. } = kenneling;

        let guild = guild_id.to_partial_guild(http).await?;
        let victim = guild.member(http, victim_id).await?;

        if let Err(err) = victim.remove_role(http, role_id).await {
            return Err(err).context("Couldn't remove role from victim for kenneling ");
        } else {
            tracing::trace!("Removed successfully!");
        }

        if let Err(err) = self.edit_messages(http, pool, kenneling).await {
            tracing::error!("{err:?}");
        };

        Ok(())
    }

    pub async fn get_current_kennelings(&self, pool: &PgPool) -> Result<Vec<Kenneling>> {
        Ok(query_as!(
            KennelingRow,
            r#"
            SELECT 
                k.id,
                k.kennel_id,
                k.author_id,
                k.victim_id,
                k.kenneled_at,
                k.kennel_length,
                k.released_at,
                k.msg_announce_id,
                k.kennel_msg_id,
                a.channel_id as msg_announce_channel_id,
                b.channel_id as kennel_msg_channel_id
            FROM kennelings k
            LEFT JOIN sent_messages a
            ON
                k.msg_announce_id = a.message_id
            LEFT JOIN sent_messages b
            ON
                k.kennel_msg_id = b.message_id
            WHERE
                released_at > CURRENT_TIMESTAMP
                AND kennel_id = $1
                ;
            "#,
            self.id
        )
        .fetch_all(pool)
        .await?
        .iter()
        .map(|kr| kr.into())
        .collect())
    }

    /// Validates a given kenneling, updating the database and messages if necessary.
    pub async fn validate_kenneling(
        &self,
        http: &serenity::all::Http,
        pool: &PgPool,
        kenneling: &Kenneling,
    ) -> Result<()> {
        let Kenneling {
            id: kenneling_id,
            victim_id,
            ..
        } = kenneling;

        let Self {
            guild_id,
            role_id: kennel_role,
            ..
        } = self;

        let guild = http.get_guild(*guild_id).await?;
        let victim = guild.member(http, victim_id).await?;

        if !victim.roles.iter().any(|role| role == kennel_role) {
            tracing::debug!("Stale kenneling detected! {kenneling:?}");

            let kenneled_at = kenneling.kenneled_at;
            let now = chrono::Utc::now();

            let dur_served = now - kenneled_at;
            let dur_served = Duration::from_secs(dur_served.num_seconds() as u64)
                + Duration::from_micros(dur_served.subsec_micros() as u64);

            let time_served =
                PgInterval::try_from(dur_served).expect("Duration served got constructed wrong!");

            sqlx::query!(
                r#"
                    UPDATE kennelings
                    SET
                        kennel_length = $1
                    WHERE
                        id = $2
                        ;
                "#,
                time_served,
                kenneling_id,
            )
            .execute(pool)
            .await?;

            tracing::info!(
                "Kenneling ended early. Time served: {}",
                humantime::format_duration(dur_served)
            );

            self.edit_messages(http, pool, kenneling).await?;
            // TODO: Should set_activity, but with what context?'
        }

        Ok(())
    }

    /// Edits the messages for a given kenneling, deleting them if necessary, including from the database.
    pub async fn edit_messages(
        &self,
        http: &serenity::all::Http,
        pool: &PgPool,
        kenneling: &Kenneling,
    ) -> Result<()> {
        let Self {
            msg_announce_edit,
            kennel_msg_edit,
            ..
        } = self;

        let Kenneling {
            msg_announce,
            kennel_msg,
            ..
        } = kenneling;

        if let Some(msg) = msg_announce {
            let mut handle = http.get_message(msg.1, msg.0).await;

            if let Err(e) = handle {
                tracing::error!("announce {e:?}");
                return Err(e.into());
            }

            let mut handle = handle.unwrap();

            match msg_announce_edit {
                Some(edit) => handle.edit(http, EditMessage::new().content(edit)).await?,
                None => {
                    handle.delete(http).await?;

                    sqlx::query!(
                        r#"
                        DELETE FROM
                            sent_messages s
                        WHERE
                            s.message_id = $1
                        "#,
                        msg.1.get() as i64
                    )
                    .execute(pool)
                    .await?;
                }
            }
        }

        if let Some(msg) = kennel_msg {
            let mut handle = http.get_message(msg.1, msg.0).await;

            if let Err(e) = handle {
                tracing::error!("kennel {e:?}");
                return Err(e.into());
            }

            let mut handle = handle.unwrap();

            match kennel_msg_edit {
                Some(edit) => handle.edit(http, EditMessage::new().content(edit)).await?,
                None => {
                    handle.delete(http).await?;

                    sqlx::query!(
                        r#"
                        DELETE FROM
                            sent_messages s
                        WHERE
                            s.message_id = $1
                        "#,
                        msg.1.get() as i64
                    )
                    .execute(pool)
                    .await?;
                }
            }
        }

        Ok(())
    }
}
