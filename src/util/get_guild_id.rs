use serenity::all::GuildId;

use crate::{Context, Error};

pub trait GetGuildID {
    #[allow(async_fn_in_trait)]
    async fn require_guild(&self) -> Result<GuildId, Error>;
}

impl GetGuildID for Context<'_> {
    async fn require_guild(&self) -> Result<GuildId, Error> {
        let Some(guild_id) = self.guild_id() else {
            self.reply("This command can only be used in a server!")
                .await?;

            return Err("Server command used outside of server".into());
        };

        Ok(guild_id)
    }
}
