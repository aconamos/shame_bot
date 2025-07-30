use std::{num::ParseIntError, time::Duration};

use chrono::{DateTime, Utc};
use serenity::all::{
    CommandOptionType, CreateCommand, CreateCommandOption, GuildId, Permissions, UserId,
};
use sqlx::{FromRow, Row, postgres::PgRow};

use crate::util::pgint_dur::PgIntervalToDuration as _;

pub mod util {
    pub mod pgint_dur;
    pub mod stefan_traits;
}

// User data, which is stored and accessible in all command invocations
pub struct ShameBotData {
    pub pool: std::sync::Arc<sqlx::PgPool>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, ShameBotData, Error>;

/// Represents the fields available from a query to the `kennelings` table.
#[derive(Debug)]
pub struct KennelingRow {
    pub guild_id: String,
    pub kennel_length: sqlx::postgres::types::PgInterval,
    pub kenneled_at: sqlx::types::chrono::NaiveDateTime,
    pub kenneler: String,
    pub released_at: sqlx::types::chrono::NaiveDateTime,
    pub victim: String,
    pub id: i32,
}

pub struct Kenneling {
    pub guild_id: GuildId,
    pub kennel_length: Duration,
    pub kenneled_at: DateTime<Utc>,
    pub kenneler: UserId,
    pub released_at: DateTime<Utc>,
    pub victim: UserId,
    pub id: i32,
}

impl TryFrom<&KennelingRow> for Kenneling {
    type Error = Box<dyn std::error::Error>;

    fn try_from(row: &KennelingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            guild_id: string_to_id(&row.guild_id)?,
            kennel_length: row.kennel_length.as_duration(),
            kenneled_at: row.kenneled_at.and_utc(),
            kenneler: string_to_id(&row.kenneler)?,
            released_at: row.kenneled_at.and_utc(),
            victim: string_to_id(&row.victim)?,
            id: row.id,
        })
    }
}

pub fn string_to_id<Id: From<u64>>(string: &str) -> Result<Id, ParseIntError> {
    Ok(string.parse::<u64>()?.into())
}

/// Represents the fields available from a query to the `servers` table.
#[derive(Debug)]
pub struct ServerRow {
    pub guild_id: String,
    pub command_name: String,
    pub announcement_message: String,
    pub release_message: String,
    pub role_id: String,
    pub kennel_channel: String,
    pub kennel_message: String,
}

/// Returns a [`CreateCommand`] that represents the general kennel command (per guild) object for the Discord API.
pub fn get_kennel_command_struct(command: &str) -> CreateCommand {
    CreateCommand::new(command)
        .description("Punish a user!")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "User to be punished")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "time",
                "How long to punish the user",
            )
            .required(true),
        )
        .default_member_permissions(Permissions::MODERATE_MEMBERS)
}
