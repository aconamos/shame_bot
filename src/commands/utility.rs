use humantime::format_duration;
use shame_bot::Context;

use crate::{ShameBotData, Error, util::pgint_dur::PgIntervalToDuration};

/// Tells you the total time kenneled in case you can't read the status
#[poise::command(slash_command)]
pub async fn time_kenneled(ctx: Context<'_>) -> Result<(), Error> {
    let ShameBotData { pool } = ctx.data();
    let pool = pool.as_ref();

    match sqlx::query!(
        r#"
        SELECT SUM(kennel_length)
        FROM kennelings
        WHERE 
            NOT guild_id = '849505364764524565'
            ;
        "#
    )
    .fetch_one(pool)
    .await
    {
        Ok(res) => {
            if let Some(sum) = res.sum {
                ctx.reply(format!(
                    "Kenneled users for {}",
                    format_duration(sum.as_duration())
                ))
                .await?;
            }
        }
        Err(_) => {
            ctx.reply("Database error???").await?;
        }
    }

    Ok(())
}
