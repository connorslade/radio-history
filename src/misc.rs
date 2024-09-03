use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, NaiveDateTime};

pub fn date_time() -> NaiveDateTime {
    DateTime::from_timestamp(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        0,
    )
    .unwrap()
    .naive_local()
}
