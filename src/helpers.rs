// Copyright 2022 Martin Pool

//! Helpful functions for drawing progress bars.

use std::time::{Duration, Instant};

fn duration_brief(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 120 {
        format!("{} min", secs / 60)
    } else {
        format!("{} sec", secs)
    }
}

/// Estimate by linear extrapolation the time remaining for a task with a given
/// start time, number of completed items and number of total items.
///
/// The result is in the format "33 sec" or "12 min". This format may change in
/// future releases before 1.0.
///
/// If the remaining time is not estimatable, returns None.
pub fn estimate_remaining(start: &Instant, done: usize, total: usize) -> Option<String> {
    let elapsed = start.elapsed();
    if total == 0 || done == 0 || elapsed.is_zero() || done > total {
        None
    } else {
        let done = done as f64;
        let total = total as f64;
        let estimate = Duration::from_secs_f64(elapsed.as_secs_f64() * (total / done - 1.0));
        Some(duration_brief(estimate))
    }
}

/// Return a string representation of the percentage of work completed.
///
/// ```
/// use nutmeg::percent_done;
/// assert_eq!(percent_done(6, 12), "50.0%");
/// assert_eq!(percent_done(0, 0), "??%");
/// ```
pub fn percent_done(done: usize, total: usize) -> String {
    if total == 0 || done > total {
        "??%".into()
    } else {
        format!("{:.1}%", done as f64 * 100.0 / total as f64)
    }
}
