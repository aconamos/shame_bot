use anyhow::Result;
use serenity::all::Http;
use shame_bot::types::*;
use sqlx::PgPool;

pub async fn check(
    http: &Http,
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>> {
    let kennels: Vec<Kennel> = sqlx::query_as!(
        KennelRow,
        r#"
        SELECT *
        FROM kennels
            ;
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|kr| kr.into())
    .collect();

    for kennel in kennels {
        let kennelings = kennel.get_current_kennelings(pool).await?;

        for kenneling in kennelings {
            if let Err(err) = kennel.validate_kenneling(http, pool, &kenneling).await {
                tracing::error!("Error validating kenneling {:?}, error: {err:?}", kenneling);
            }
        }
    }

    Ok(())
}
