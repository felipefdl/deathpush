use chrono::{DateTime, Utc};

const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;
const MONTH: i64 = 30 * DAY;
const YEAR: i64 = 365 * DAY;

fn plural(value: i64, unit: &str) -> String {
  if value == 1 {
    format!("1 {unit} ago")
  } else {
    format!("{value} {unit}s ago")
  }
}

/// `just now`, `3 minutes ago`, `2 weeks ago`, matching the shipped app. Unparseable input is returned as is.
pub fn relative_time(iso: &str, now: DateTime<Utc>) -> String {
  let Ok(date) = DateTime::parse_from_rfc3339(iso) else {
    return iso.to_string();
  };
  let secs = now.signed_duration_since(date.with_timezone(&Utc)).num_seconds();
  if secs < MINUTE {
    "just now".to_string()
  } else if secs < HOUR {
    plural(secs / MINUTE, "minute")
  } else if secs < DAY {
    plural(secs / HOUR, "hour")
  } else if secs < WEEK {
    plural(secs / DAY, "day")
  } else if secs < MONTH {
    plural(secs / WEEK, "week")
  } else if secs < YEAR {
    plural(secs / MONTH, "month")
  } else {
    plural(secs / YEAR, "year")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::TimeZone;

  fn at(secs_ago: i64) -> (String, DateTime<Utc>) {
    let now = Utc.with_ymd_and_hms(2026, 9, 3, 12, 0, 0).unwrap();
    ((now - chrono::Duration::seconds(secs_ago)).to_rfc3339(), now)
  }

  #[test]
  fn buckets_match_the_old_formatter() {
    for (secs, expected) in [
      (5, "just now"),
      (-5, "just now"),
      (60, "1 minute ago"),
      (125, "2 minutes ago"),
      (3600, "1 hour ago"),
      (2 * DAY, "2 days ago"),
      (WEEK, "1 week ago"),
      (MONTH * 2, "2 months ago"),
      (YEAR, "1 year ago"),
    ] {
      let (iso, now) = at(secs);
      assert_eq!(relative_time(&iso, now), expected, "{secs}");
    }
  }

  #[test]
  fn unparseable_input_is_returned() {
    assert_eq!(relative_time("yesterday", Utc::now()), "yesterday");
  }
}
