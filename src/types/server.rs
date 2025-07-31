use serenity::all::{ChannelId, GuildId, RoleId};

use crate::string_to_id;

/// Represents the fields available from a query to the `servers` table.
#[derive(Debug)]
pub struct ServerRow {
    pub guild_id: String,
    pub command_name: String,
    pub announcement_message: String,
    pub release_message: String,
    pub role_id: String,
    pub kennel_channel: Option<String>,
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
    pub kennel_channel: Option<ChannelId>,
    pub kennel_message: String,
}

impl TryFrom<ServerRow> for Server {
    type Error = std::num::ParseIntError;

    fn try_from(row: ServerRow) -> Result<Self, Self::Error> {
        Ok(Self {
            guild_id: string_to_id(&row.guild_id)?,
            role_id: string_to_id(&row.role_id)?,
            kennel_channel: row
                .kennel_channel
                .map(|kc| string_to_id::<ChannelId>(&kc).expect("Malformed channel id")),
            command_name: row.command_name,
            announcement_message: row.announcement_message,
            release_message: row.release_message,
            kennel_message: row.kennel_message,
        })
    }
}
