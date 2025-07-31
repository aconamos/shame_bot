use std::time::Duration;

use serenity::all::{Http, RoleId};
use shame_bot::{string_to_id, types::*};
use sqlx::{PgPool, postgres::types::PgInterval};

pub async fn check(
    http: &Http,
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>> {
    let active_kennelings: Vec<Kenneling> = sqlx::query_as!(
        KennelingRow,
        r#"
        SELECT *
        FROM kennelings
        WHERE
            released_at > CURRENT_TIMESTAMP
            ;
        "#
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|kr| kr.try_into().expect("malformed data inserted"))
    .collect();

    for kenneling in active_kennelings {
        let kennel_role = sqlx::query!(
            r#"
        SELECT role_id
        FROM servers
        WHERE
            guild_id = $1
            ;
        "#,
            kenneling.guild_id.to_string(),
        )
        .fetch_one(pool)
        .await?;

        let kennel_role: RoleId = string_to_id(&kennel_role.role_id)?;

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
    let Kenneling {
        victim_id: victim,
        guild_id,
        ..
    } = kenneling;

    let guild = http.get_guild(guild_id).await?;
    let victim = guild.member(http, victim).await?;

    if !victim.roles.iter().any(|role| role == &kennel_role) {
        tracing::info!("Stale kenneling detected! {kenneling:?}");

        let kenneled_at = kenneling.kenneled_at;
        let now = chrono::Utc::now();

        let dur_served = now - kenneled_at;
        let dur_served = Duration::from_secs(dur_served.num_seconds() as u64)
            + Duration::from_micros(dur_served.subsec_micros() as u64);

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

        tracing::info!(
            "Kenneling ended early. Time served: {}",
            humantime::format_duration(dur_served)
        );

        // TODO: Should set_activity, but with what context?'
    }

    Ok(())
}
