use actix_web::rt::time::{Interval, interval};
use std::time::Duration;

pub fn var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

pub fn interval_secs(key: &str, default: u64) -> Interval {
    let secs = var(key)
        .and_then(|secs| secs.parse().ok())
        .filter(|&secs| secs > 0)
        .unwrap_or(default);
    interval(Duration::from_secs(secs))
}
