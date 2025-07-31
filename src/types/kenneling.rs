use serenity::all::UserId;

use chrono::Utc;

use chrono::DateTime;

use std::time::Duration;

use serenity::all::GuildId;

use crate::Context;
use crate::Error;
use crate::get_formatted_message;
use crate::string_to_id;
use crate::types::server::Server;
use crate::types::server::ServerRow;
use crate::util::pgint_dur::PgIntervalToDuration as _;
use crate::util::stefan_traits::GetRelativeTimestamp as _;

/// Represents the fields available from a query to the `kennelings` table.
#[derive(Debug)]
pub struct KennelingRow {
    pub guild_id: String,
    pub kennel_length: sqlx::postgres::types::PgInterval,
    pub kenneled_at: sqlx::types::chrono::NaiveDateTime,
    pub author_id: String,
    pub released_at: sqlx::types::chrono::NaiveDateTime,
    pub victim_id: String,
    pub id: Option<i32>,
}

/// Information about a given Kenneling from the database.
#[derive(Debug)]
pub struct Kenneling {
    pub guild_id: GuildId,
    pub kennel_length: Duration,
    pub kenneled_at: DateTime<Utc>,
    pub author_id: UserId,
    pub released_at: DateTime<Utc>,
    pub victim_id: UserId,
    pub id: Option<i32>,
}

impl TryFrom<&KennelingRow> for Kenneling {
    type Error = std::num::ParseIntError;

    fn try_from(row: &KennelingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            guild_id: string_to_id(&row.guild_id)?,
            kennel_length: row.kennel_length.as_duration(),
            kenneled_at: row.kenneled_at.and_utc(),
            author_id: string_to_id(&row.author_id)?,
            released_at: row.kenneled_at.and_utc(),
            victim_id: string_to_id(&row.victim_id)?,
            id: row.id,
        })
    }
}

impl TryFrom<&Kenneling> for KennelingRow {
    type Error = Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>;

    fn try_from(row: &Kenneling) -> Result<Self, Self::Error> {
        Ok(KennelingRow {
            guild_id: row.guild_id.to_string(),
            kennel_length: row.kennel_length.try_into()?,
            kenneled_at: row.kenneled_at.naive_utc(),
            author_id: row.author_id.to_string(),
            released_at: row.released_at.naive_utc(),
            victim_id: row.victim_id.to_string(),
            id: None,
        })
    }
}

impl Kenneling {
    /// Applies the roles for a given Kenneling, sends a message, and returns a handle to the announcement message.
    ///
    /// If `ctx` is [`None`], applies the roles, but does not send a message.
    ///
    /// This version exists primarily so that if you already fetched a Server from the database, you don't have to
    /// duplicate that query (assuming it's current).
    pub async fn apply_kennel<'a>(
        &self,
        http: &serenity::all::Http,
        server: &Server,
        ctx: Option<&Context<'a>>,
    ) -> Result<Option<poise::ReplyHandle<'a>>, Error> {
        let Kenneling {
            guild_id,
            kennel_length,
            kenneled_at: _kenneled_at,
            author_id: kenneler_id,
            released_at,
            victim_id,
            id: _id,
        } = self;

        let kenneler = http.get_member(*guild_id, *kenneler_id).await?;
        let victim = http.get_member(*guild_id, *victim_id).await?;

        http.add_member_role(*guild_id, (&victim).into(), server.role_id, None)
            .await?;

        tracing::info!(
            "{} kenneled user {} for {}!",
            kenneler.display_name(),
            victim.display_name(),
            humantime::format_duration(*kennel_length)
        );

        let mut reply_handle: Option<poise::ReplyHandle<'a>> = None;

        if let Some(ctx) = ctx {
            let announcement_msg = get_formatted_message(
                &server.announcement_message,
                victim_id,
                kenneler_id,
                &humantime::format_duration(*kennel_length).to_string(),
                &released_at.discord_relative_timestamp(),
            );

            reply_handle = Some(ctx.reply(&announcement_msg).await?);

            // This is kinda dumb. // TODO: put this in a better spot
            if let Some(kennel_channel) = server.kennel_channel {
                if kennel_channel != ctx.channel_id() {
                    let kennel_announcement_msg = get_formatted_message(
                        &server.kennel_message,
                        victim_id,
                        kenneler_id,
                        &humantime::format_duration(*kennel_length).to_string(),
                        &released_at.discord_relative_timestamp(),
                    );

                    match kennel_channel
                        .send_message(
                            http,
                            serenity::all::CreateMessage::new().content(kennel_announcement_msg),
                        )
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => tracing::error!("Couldn't send kennel message! {e:?}"),
                    }
                }
            }
        }

        Ok(reply_handle)
    }

    /// Applies the roles for a kennel, fetching the server info from the database.
    pub async fn apply_kennel_from_db(
        &self,
        http: &serenity::all::Http,
        pool: &sqlx::PgPool,
    ) -> Result<(), Error> {
        let server: Server = sqlx::query_as!(
            ServerRow,
            r#"
            SELECT * FROM servers
            WHERE
                guild_id = $1
                ;
            "#,
            &self.guild_id.to_string(),
        )
        .fetch_one(pool)
        .await?
        .try_into()?;

        self.apply_kennel(http, &server, None).await.map(|_| ())
    }

    /// Removes the roles for a Kenneling, and edits the reply if the handle is present.
    pub async fn unapply_kennel<'a>(
        &self,
        http: &serenity::all::Http,
        pool: &sqlx::PgPool,
        send_in_channel: bool,
        reply_handle: Option<&poise::ReplyHandle<'a>>,
        ctx: Option<&Context<'a>>,
    ) -> Result<(), Error> {
        let Kenneling {
            guild_id,
            kennel_length,
            kenneled_at: _kenneled_at,
            author_id: kenneler_id,
            released_at,
            victim_id,
            id: _id,
        } = self;

        let server: Server = sqlx::query_as!(
            ServerRow,
            r#"
            SELECT * FROM servers
            WHERE
                guild_id = $1
                ;
            "#,
            &self.guild_id.to_string(),
        )
        .fetch_one(pool)
        .await?
        .try_into()?;

        let victim = http.get_member(*guild_id, *victim_id).await?;

        victim.remove_role(http, server.role_id).await?;

        tracing::info!("Unkenneled {}", victim.display_name());

        if let Some(reply_handle) = reply_handle
            && let Some(ctx) = ctx
        {
            let edit_msg = get_formatted_message(
                &server.release_message,
                victim_id,
                kenneler_id,
                &humantime::format_duration(*kennel_length).to_string(),
                &released_at.discord_relative_timestamp(),
            );

            reply_handle
                .edit(*ctx, poise::CreateReply::default().content(edit_msg))
                .await?;

            tracing::trace!("Reply edited!");
        }

        if let Some(kennel_channel) = server.kennel_channel
            && send_in_channel
        {
            let release_message = get_formatted_message(
                &server.release_message,
                victim_id,
                kenneler_id,
                &humantime::format_duration(*kennel_length).to_string(),
                &released_at.discord_relative_timestamp(),
            );

            kennel_channel
                .send_message(
                    http,
                    serenity::all::CreateMessage::new().content(release_message),
                )
                .await?;
            tracing::trace!("Sent release message in kennel channel!");
        }

        Ok(())
    }
}

impl KennelingRow {
    /// Inserts this Row into the database, assuming that it is current (ignores the timestamp value of the Row).
    ///
    /// Returns the ID of the inserted kenneling.
    pub async fn assume_current_and_insert(&self, pool: &sqlx::PgPool) -> Result<i32, Error> {
        tracing::trace!("Inserting a new Kenneling...");

        let id = sqlx::query!(
            r#"
            INSERT INTO kennelings
                (guild_id, victim_id, author_id, kennel_length)
            VALUES
                ($1, $2, $3, $4)
            RETURNING
                id
                ;
            "#,
            self.guild_id,
            self.victim_id,
            self.author_id,
            self.kennel_length,
        )
        .fetch_one(pool)
        .await?
        .id;

        tracing::trace!("New Kenneling inserted! Id: {id}");

        Ok(id)
    }
}
