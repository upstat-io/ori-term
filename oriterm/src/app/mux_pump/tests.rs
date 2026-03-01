//! Unit tests for mux pump helpers.

use std::time::Duration;

use super::format_duration_body;

#[test]
fn format_seconds_only() {
    assert_eq!(
        format_duration_body(Duration::from_secs(12)),
        "Completed in 12s"
    );
}

#[test]
fn format_minutes_and_seconds() {
    assert_eq!(
        format_duration_body(Duration::from_secs(150)),
        "Completed in 2m 30s"
    );
}

#[test]
fn format_hours_and_minutes() {
    assert_eq!(
        format_duration_body(Duration::from_secs(3900)),
        "Completed in 1h 5m"
    );
}

#[test]
fn format_exactly_one_minute() {
    assert_eq!(
        format_duration_body(Duration::from_secs(60)),
        "Completed in 1m 0s"
    );
}

#[test]
fn format_exactly_one_hour() {
    assert_eq!(
        format_duration_body(Duration::from_secs(3600)),
        "Completed in 1h 0m"
    );
}

#[test]
fn format_zero_seconds() {
    assert_eq!(format_duration_body(Duration::ZERO), "Completed in 0s");
}
