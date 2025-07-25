use std::time::Duration;

use sqlx::postgres::types::PgInterval;

pub trait PgIntervalToDuration {
    fn as_duration(&self) -> Duration;
}

impl PgIntervalToDuration for PgInterval {
    fn as_duration(&self) -> Duration {
        Duration::from_micros(self.microseconds as u64)
            + Duration::from_secs(self.days as u64 * 24 * 60 * 60)
            + Duration::from_secs(self.months as u64 * 30 * 24 * 60 * 60)
    }
}
