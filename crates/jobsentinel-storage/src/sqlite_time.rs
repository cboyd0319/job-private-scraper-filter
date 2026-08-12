use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};

pub(crate) fn parse_sqlite_datetime(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Ok(datetime.with_timezone(&Utc));
    }

    for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(datetime) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(DateTime::from_naive_utc_and_offset(datetime, Utc));
        }
    }

    Err(anyhow!("failed to parse SQLite datetime: {value}"))
}

#[cfg(test)]
mod tests {
    use super::parse_sqlite_datetime;

    #[test]
    fn parses_supported_sqlite_datetime_formats() {
        for value in [
            "2026-01-15T12:34:56Z",
            "2026-01-15 12:34:56",
            "2026-01-15 12:34:56.123",
            "2026-01-15T12:34:56",
        ] {
            let parsed = parse_sqlite_datetime(value).expect("supported datetime format");
            assert!(parsed.to_rfc3339().starts_with("2026-01-15T12:34:56"));
        }
    }

    #[test]
    fn rejects_malformed_sqlite_datetimes() {
        for value in ["", "invalid", "2026-13-45"] {
            assert!(parse_sqlite_datetime(value).is_err(), "value: {value}");
        }
    }
}
