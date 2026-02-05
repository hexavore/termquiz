use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};

#[derive(Debug, Clone)]
pub enum TimerEvent {
    Tick(i64),
    TwoMinuteWarning,
    TimeExpired,
}

pub fn spawn_timer(
    end_time: DateTime<FixedOffset>,
) -> mpsc::Receiver<TimerEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut warned_two_min = false;
        let expired = false;

        loop {
            let now = Utc::now();
            let remaining = end_time.signed_duration_since(now);
            let secs = remaining.num_seconds();

            if secs <= 0 && !expired {
                let _ = tx.send(TimerEvent::TimeExpired);
                break;
            }

            if secs <= 120 && secs > 0 && !warned_two_min {
                warned_two_min = true;
                let _ = tx.send(TimerEvent::TwoMinuteWarning);
            }

            if tx.send(TimerEvent::Tick(secs)).is_err() {
                break;
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    rx
}

pub fn format_duration(total_secs: i64) -> String {
    if total_secs <= 0 {
        return "0h 0m 0s".to_string();
    }
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}

pub fn time_until_start(
    start: &DateTime<FixedOffset>,
) -> i64 {
    let now = Utc::now();
    let remaining = start.signed_duration_since(now);
    remaining.num_seconds()
}

pub fn format_wait_duration(total_secs: i64) -> String {
    if total_secs <= 0 {
        return "now".to_string();
    }
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
