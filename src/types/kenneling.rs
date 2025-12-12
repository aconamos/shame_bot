use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::Utc;
use serenity::all::GuildId;
use serenity::all::MessageId;
use serenity::all::UserId;
use sqlx::Executor;
use std::time::Duration;

use crate::Context;
use crate::get_formatted_message;
use crate::string_to_id;
use crate::util::pgint_dur::PgIntervalToDuration as _;
use crate::util::stefan_traits::GetRelativeTimestamp as _;

/// Represents the fields available from a query to the `kennelings` table.
#[derive(Debug)]
pub struct KennelingRow {
    pub id: i32,
    pub kennel_id: i32,
    pub guild_id: i64,
    pub author_id: i64,
    pub victim_id: i64,
    pub kenneled_at: sqlx::types::chrono::NaiveDateTime,
    pub kennel_length: sqlx::postgres::types::PgInterval,
    pub released_at: sqlx::types::chrono::NaiveDateTime,
    pub msg_announce_id: Option<i64>,
    pub kennel_msg_id: Option<i64>,
}

/// Information about a given Kenneling from the database.
#[derive(Debug)]
pub struct Kenneling {
    pub id: i32,
    pub kennel_id: i32,
    pub guild_id: GuildId,
    pub author_id: UserId,
    pub victim_id: UserId,
    pub kenneled_at: DateTime<Utc>,
    pub kennel_length: Duration,
    pub released_at: DateTime<Utc>,
    pub msg_announce_id: Option<MessageId>,
    pub kennel_msg_id: Option<MessageId>,
}

impl From<&KennelingRow> for Kenneling {
    fn from(row: &KennelingRow) -> Self {
        Self {
            id: row.id,
            kennel_id: row.kennel_id,
            guild_id: GuildId::new(row.guild_id as u64),
            author_id: UserId::new(row.author_id as u64),
            victim_id: UserId::new(row.victim_id as u64),
            kenneled_at: row.kenneled_at.and_utc(),
            kennel_length: row.kennel_length.as_duration(),
            released_at: row.released_at.and_utc(),
            msg_announce_id: row.msg_announce_id.map(|id| MessageId::new(id as u64)),
            kennel_msg_id: row.kennel_msg_id.map(|id| MessageId::new(id as u64)),
        }
    }
}

impl TryFrom<&Kenneling> for KennelingRow {
    type Error = anyhow::Error;

    fn try_from(row: &Kenneling) -> Result<Self, Self::Error> {
        Ok(KennelingRow {
            id: row.id,
            kennel_id: row.kennel_id,
            guild_id: row.guild_id.get() as i64,
            author_id: row.author_id.get() as i64,
            victim_id: row.victim_id.get() as i64,
            kenneled_at: row.kenneled_at.naive_utc(),
            kennel_length: row
                .kennel_length
                .try_into()
                .map_err(|_| anyhow!("Couldn't convert length into PgInterval"))?,
            released_at: row.released_at.naive_utc(),
            msg_announce_id: row.msg_announce_id.map(|id| id.get() as i64),
            kennel_msg_id: row.kennel_msg_id.map(|id| id.get() as i64),
        })
    }
}
