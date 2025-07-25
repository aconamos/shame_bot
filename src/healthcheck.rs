use std::time::Duration;

use serenity::all::{GuildId, Http, RoleId, UserId};
use sqlx::{PgPool, postgres::types::PgInterval, types::time::PrimitiveDateTime};
use tracing::{debug, error, info, trace, warn};

// TODO: find a better spot for this struct
#[derive(Debug)]
#[allow(dead_code)]
pub struct Kenneling {
    pub guild_id: String,
    pub kennel_length: PgInterval,
    pub kenneled_at: PrimitiveDateTime,
    pub kenneler: String,
    pub released_at: PrimitiveDateTime,
    pub victim: String,
    pub id: i32,
}

pub async fn check(
    http: &Http,
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>> {
    let active_kennelings = sqlx::query_as!(
        Kenneling,
        r#"
        SELECT *
        FROM kennelings
        WHERE
            released_at > CURRENT_TIMESTAMP
            ;
        "#
    )
    .fetch_all(pool)
    .await?;

    for kenneling in active_kennelings {
        let kennel_role = sqlx::query!(
            r#"
        SELECT role_id
        FROM servers
        WHERE
            guild_id = $1
            ;
        "#,
            kenneling.guild_id,
        )
        .fetch_one(pool)
        .await?;

        let kennel_role = RoleId::from(kennel_role.role_id.parse::<u64>()?);

        validate_kenneling(http, pool, kenneling, kennel_role).await?;
    }

    Ok(())
}

async fn validate_kenneling(
    http: &Http,
    pool: &PgPool,
    kenneling: Kenneling,
    kennel_role: RoleId,
) -> Result<(), Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>> {
    let victim = UserId::from(kenneling.victim.parse::<u64>()?);
    let guild_id = GuildId::from(kenneling.guild_id.parse::<u64>()?);
    let _kenneler = UserId::from(kenneling.kenneler.parse::<u64>()?);

    let guild = http.get_guild(guild_id).await?;
    let victim = guild.member(http, victim).await?;

    if !victim.roles.iter().any(|role| role == &kennel_role) {
        info!("Stale kenneling detected! {kenneling:?}");

        let t = kenneling.kenneled_at.as_utc().unix_timestamp();
        let now = chrono::Utc::now().timestamp();

        let dur_served = Duration::from_secs((now - t) as u64);

        let time_served = PgInterval::try_from(dur_served)?;

        sqlx::query!(
            r#"
                UPDATE kennelings
                SET
                    kennel_length = $1
                WHERE
                    id = $2
                    ;
            "#,
            time_served,
            kenneling.id,
        )
        .execute(pool)
        .await?;

        info!(
            "Kenneling ended early. Time served: {}",
            humantime::format_duration(dur_served)
        );
    }

    Ok(())
}
