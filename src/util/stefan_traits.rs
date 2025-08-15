use anyhow::Result;
use poise::{Context, CreateReply};

// https://github.com/coravacav/uofu-cs-discord-bot/blob/0513983163f0563a26709004dccb954948dffb2a/bot-lib/src/utils.rs
pub trait GetRelativeTimestamp {
    fn discord_relative_timestamp(&self) -> String;
}

impl GetRelativeTimestamp for chrono::DateTime<chrono::Utc> {
    fn discord_relative_timestamp(&self) -> String {
        format!("<t:{}:R>", self.timestamp())
    }
}

pub trait SendReplyEphemeral {
    #[allow(async_fn_in_trait)]
    async fn reply_ephemeral(&self, content: impl Into<String>) -> Result<()>;
}

impl<A, B> SendReplyEphemeral for Context<'_, A, B> {
    async fn reply_ephemeral(&self, content: impl Into<String>) -> Result<()> {
        let reply = CreateReply::default()
            .reply(true)
            .ephemeral(true)
            .content(content);

        self.send(reply).await?;

        Ok(())
    }
}
