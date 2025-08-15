use crate::Context;
use anyhow::{Result, anyhow};
use serenity::all::GuildId;

pub trait GetGuildID {
    #[allow(async_fn_in_trait)]
    async fn require_guild(&self) -> Result<GuildId>;
}

impl GetGuildID for Context<'_> {
    async fn require_guild(&self) -> Result<GuildId> {
        let Some(guild_id) = self.guild_id() else {
            self.reply("This command can only be used in a server!")
                .await?;

            return Err(anyhow!("Server command called outside of server!"));
        };

        Ok(guild_id)
    }
}
