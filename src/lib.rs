use std::num::ParseIntError;

use serenity::all::{
    CommandOptionType, CreateCommand, CreateCommandOption, Permissions, UserId,
};


pub mod util {
    pub mod pgint_dur;
    pub mod stefan_traits;
}
pub mod types {
    pub mod kenneling;
    pub mod server;

    pub use kenneling::*;
    pub use server::*;
}

// User data, which is stored and accessible in all command invocations
pub struct ShameBotData {
    pub pool: std::sync::Arc<sqlx::PgPool>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, ShameBotData, Error>;

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

pub fn get_formatted_message(
    message: &str,
    victim_id: &UserId,
    author_id: &UserId,
    time: &str,
    return_time: &str,
) -> String {
    message
        .replace("$victim", format!("<@{victim_id}>").as_str())
        .replace("$kenneler", format!("<@{author_id}>").as_str())
        .replace("$time", time)
        .replace("$return", return_time)
}
