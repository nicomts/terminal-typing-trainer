//! Small math for animations: how far along something is, and easing.
//!
//! Every function takes the times it needs as arguments and never reads the
//! clock, so `ui::render` stays a pure function of `App` and tests can check
//! any moment of an animation by passing a fixed time.

use std::time::{Duration, Instant};

/// How far an animation that started at `since` and lasts `duration` has got
/// at `now`: 0.0 at the start, 1.0 once it's over (and stays 1.0 after).
pub fn progress(since: Instant, now: Instant, duration: Duration) -> f64 {
    if duration.is_zero() {
        return 1.0;
    }
    // `saturating_duration_since` gives zero instead of panicking if `now`
    // is somehow before `since`.
    let elapsed = now.saturating_duration_since(since);
    (elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0)
}

/// Turns linear progress `t` (0.0 to 1.0) into motion that starts fast and
/// slows down at the end, which looks more natural than constant speed.
pub fn ease_out(t: f64) -> f64 {
    let rest = 1.0 - t.clamp(0.0, 1.0);
    1.0 - rest * rest * rest
}

/// Position within a repeating cycle of length `period`: 0.0 at the start of
/// each cycle, rising towards 1.0, then back to 0.0. Used for blinking and
/// for the logo shimmer.
pub fn phase(since: Instant, now: Instant, period: Duration) -> f64 {
    if period.is_zero() {
        return 0.0;
    }
    let elapsed = now.saturating_duration_since(since).as_secs_f64();
    let period = period.as_secs_f64();
    // `%` on floats is the remainder, so this is "seconds into this cycle".
    (elapsed % period) / period
}

/// Rises from 0.0 to 1.0 and falls back to 0.0 once every `period`: a
/// steady pulse, for things that should gently draw the eye.
pub fn pulse(since: Instant, now: Instant, period: Duration) -> f64 {
    let p = phase(since, now, period);
    if p < 0.5 { p * 2.0 } else { 2.0 - p * 2.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn progress_goes_from_zero_to_one() {
        let start = Instant::now();
        assert_eq!(progress(start, start, SECOND), 0.0);
        assert_eq!(progress(start, start + SECOND / 2, SECOND), 0.5);
        assert_eq!(progress(start, start + SECOND, SECOND), 1.0);
    }

    #[test]
    fn progress_stays_at_one_after_the_end() {
        let start = Instant::now();
        assert_eq!(progress(start, start + SECOND * 5, SECOND), 1.0);
    }

    #[test]
    fn progress_before_the_start_is_zero() {
        let start = Instant::now() + SECOND;
        assert_eq!(progress(start, start - SECOND, SECOND), 0.0);
    }

    #[test]
    fn zero_duration_is_already_done() {
        let start = Instant::now();
        assert_eq!(progress(start, start, Duration::ZERO), 1.0);
    }

    #[test]
    fn ease_out_keeps_the_ends_and_runs_ahead_in_between() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.5) > 0.5);
    }

    #[test]
    fn phase_repeats_every_period() {
        let start = Instant::now();
        assert_eq!(phase(start, start, SECOND), 0.0);
        assert_eq!(phase(start, start + SECOND / 4, SECOND), 0.25);
        assert_eq!(phase(start, start + SECOND * 3 + SECOND / 4, SECOND), 0.25);
    }

    #[test]
    fn pulse_rises_then_falls() {
        let start = Instant::now();
        assert_eq!(pulse(start, start, SECOND), 0.0);
        assert_eq!(pulse(start, start + SECOND / 2, SECOND), 1.0);
        assert_eq!(pulse(start, start + SECOND / 4, SECOND), 0.5);
        assert_eq!(pulse(start, start + SECOND * 3 / 4, SECOND), 0.5);
    }
}
