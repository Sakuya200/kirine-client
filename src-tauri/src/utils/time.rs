use time::{macros::format_description, OffsetDateTime, UtcOffset};

use crate::Result;
use anyhow::Context;

pub(crate) fn now_string() -> Result<String> {
    Ok(current_local_time()
        .format(&format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .context("failed to format current local timestamp")?)
}

pub(crate) fn current_local_time() -> OffsetDateTime {
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    OffsetDateTime::now_utc().to_offset(local_offset)
}

pub(crate) fn generate_unique_token(prefix: &str) -> String {
    format!(
        "{}-{}",
        prefix,
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    )
}