use std::{num::ParseIntError, time::Duration};

use chrono::{DateTime, Utc};
use serenity::all::{
    ChannelId, CommandOptionType, CreateCommand, CreateCommandOption, GuildId, Permissions, RoleId,
    UserId,
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

/// Information about a given Kenneling from the database.
#[derive(Debug)]
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

/// Information about a given Server from the database.
#[derive(Debug)]
pub struct Server {
    pub guild_id: GuildId,
    pub command_name: String,
    pub announcement_message: String,
    pub release_message: String,
    pub role_id: RoleId,
    pub kennel_channel: ChannelId,
    pub kennel_message: String,
}

impl TryFrom<ServerRow> for Server {
    type Error = Box<dyn std::error::Error>;

    fn try_from(row: ServerRow) -> Result<Self, Self::Error> {
        Ok(Self {
            guild_id: string_to_id(&row.guild_id)?,
            role_id: string_to_id(&row.role_id)?,
            kennel_channel: string_to_id(&row.kennel_channel)?,
            command_name: row.command_name,
            announcement_message: row.announcement_message,
            release_message: row.release_message,
            kennel_message: row.kennel_message,
        })
    }
}

/// Helper to parse a string into a u64 and then turn it into something.
pub fn string_to_id<Id: From<u64>>(string: &str) -> Result<Id, ParseIntError> {
    Ok(string.parse::<u64>()?.into())
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
