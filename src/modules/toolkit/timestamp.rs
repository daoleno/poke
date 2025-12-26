//! Unix timestamp conversion

use super::ToolResult;
use crate::core::{Action, NotifyLevel};
use std::time::{SystemTime, UNIX_EPOCH};

/// Convert between unix timestamp and human readable date
pub fn timestamp(input: Option<String>) -> Action {
    let input = input.map(|s| s.trim().to_string());

    match input.as_deref() {
        None | Some("") | Some("now") => {
            // Show current time
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let result = ToolResult::new("Timestamp")
                .add("unix", now.to_string())
                .add("human", format_timestamp(now));
            result.into_action()
        }
        Some(s) => {
            // Try to parse as unix timestamp
            if let Ok(ts) = s.parse::<u64>() {
                let result = ToolResult::new("Timestamp")
                    .add("unix", ts.to_string())
                    .add("human", format_timestamp(ts));
                return result.into_action();
            }

            Action::Notify(format!("Cannot parse timestamp: {}", s), NotifyLevel::Error)
        }
    }
}

fn format_timestamp(ts: u64) -> String {
    // Simple formatting without external crates
    let secs_per_minute = 60u64;
    let secs_per_hour = 3600u64;
    let secs_per_day = 86400u64;

    // Days since epoch
    let days = ts / secs_per_day;
    let remaining = ts % secs_per_day;
    let hours = remaining / secs_per_hour;
    let remaining = remaining % secs_per_hour;
    let minutes = remaining / secs_per_minute;
    let seconds = remaining % secs_per_minute;

    // Approximate date calculation (not accounting for leap years precisely)
    let mut year = 1970u64;
    let mut remaining_days = days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let (month, day) = days_to_month_day(remaining_days as u32, is_leap_year(year));

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            year, month, day, hours, minutes, seconds)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_to_month_day(mut days: u32, leap: bool) -> (u32, u32) {
    let month_days = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for (i, &d) in month_days.iter().enumerate() {
        if days < d {
            return ((i + 1) as u32, days + 1);
        }
        days -= d;
    }
    (12, 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(format_timestamp(1704067200), "2024-01-01 00:00:00 UTC");
    }
}
