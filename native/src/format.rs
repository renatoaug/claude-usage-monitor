// How numbers and times read on the widget.

/// "13m", "2h 13m", past a day "6d 16h".
pub fn countdown(ms: i64) -> String {
    let m = ms.max(0) / 60_000;
    let (d, h, m) = (m / 1440, m % 1440 / 60, m % 60);
    match (d, h) {
        (0, 0) => format!("{m}m"),
        (0, _) => format!("{h}h {m}m"),
        _ => format!("{d}d {h}h"),
    }
}

/// Now, as the widget reads clocks. Snapshots pin it so their text never moves.
fn now() -> chrono::DateTime<chrono::FixedOffset> {
    #[cfg(test)]
    if let Some(t) = FROZEN.get() {
        return *t;
    }
    chrono::Local::now().fixed_offset()
}

#[cfg(test)]
pub static FROZEN: std::sync::OnceLock<chrono::DateTime<chrono::FixedOffset>> = std::sync::OnceLock::new();

/// The wall-clock time a reset lands on: "4:40 PM", "Mon 11:37 AM" past a
/// day, "Nov 12" a week or more out.
pub fn clock(ms: i64) -> String {
    let at = now() + chrono::Duration::milliseconds(ms.max(0));
    if ms >= 6 * 86_400_000 {
        at.format("%b %-d").to_string()
    } else if ms >= 86_400_000 {
        at.format("%a %-I:%M %p").to_string()
    } else {
        at.format("%-I:%M %p").to_string()
    }
}

/// "2h 13m (12:50 PM)": the countdown and the clock time it lands on.
pub fn reset_in(ms: i64) -> String {
    format!("{} ({})", countdown(ms), clock(ms))
}

/// "37.8M", "1.12B".
pub fn tokens(n: u64) -> String {
    match n {
        n if n >= 1_000_000_000 => format!("{:.2}B", n as f64 / 1e9),
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1e6),
        n if n >= 1_000 => format!("{:.1}k", n as f64 / 1e3),
        n => n.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdowns_and_token_counts_read_short() {
        assert_eq!(countdown(13 * 60_000 + 59_000), "13m");
        assert_eq!(countdown((2 * 60 + 13) * 60_000), "2h 13m");
        assert_eq!(countdown((25 * 60) * 60_000), "1d 1h");
        assert!(reset_in(2 * 3_600_000).starts_with("2h 0m ("));
        assert_eq!(tokens(37_800_000), "37.8M");
        assert_eq!(tokens(1_120_000_000), "1.12B");
        assert_eq!(tokens(950), "950");
    }
}
