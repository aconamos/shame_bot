use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption, Permissions};

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
    pub kenneled_at: sqlx::types::time::PrimitiveDateTime,
    pub kenneler: String,
    pub released_at: sqlx::types::time::PrimitiveDateTime,
    pub victim: String,
    pub id: i32,
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
