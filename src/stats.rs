//! Numbers about a finished round: speed, accuracy and missed symbols.
//!
//! Pure functions over a `Session`, no terminal and no clock.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::time::Duration;

use crate::session::Session;

/// The usual typing-test convention: 5 characters make one "word".
const CHARS_PER_WORD: f64 = 5.0;

#[derive(Debug, PartialEq)]
pub struct RoundStats {
    /// Words per minute, counting only characters that are correct at the end.
    pub wpm: f64,
    /// Percent of keystrokes that were right, corrected mistakes included.
    pub accuracy: f64,
    /// Each missed symbol with how often it was missed, most missed first.
    pub missed: Vec<(char, usize)>,
}

pub fn round_stats(session: &Session) -> RoundStats {
    let elapsed = session.elapsed().unwrap_or(Duration::ZERO);
    RoundStats {
        wpm: wpm(correct_chars(session), elapsed),
        accuracy: accuracy(session.keystrokes(), session.mistakes().len()),
        missed: missed_symbols(session),
    }
}

/// Characters that match the target at the end of the round.
fn correct_chars(session: &Session) -> usize {
    let target = session.target();
    let mut count = 0;
    for (i, &typed) in session.typed().iter().enumerate() {
        if typed == target[i] {
            count += 1;
        }
    }
    count
}

fn wpm(correct_chars: usize, elapsed: Duration) -> f64 {
    // A one-character round, or one typed in a single instant, has no speed.
    if elapsed.is_zero() {
        return 0.0;
    }
    let words = correct_chars as f64 / CHARS_PER_WORD;
    let minutes = elapsed.as_secs_f64() / 60.0;
    words / minutes
}

fn accuracy(keystrokes: usize, mistakes: usize) -> f64 {
    if keystrokes == 0 {
        return 100.0;
    }
    let right = keystrokes.saturating_sub(mistakes);
    right as f64 / keystrokes as f64 * 100.0
}

/// Counts mistakes per expected symbol. Only ASCII punctuation is counted
/// (`| & ; > $ { } ' "` and friends): letters and spaces are not what this
/// trainer is about.
fn missed_symbols(session: &Session) -> Vec<(char, usize)> {
    // A BTreeMap iterates in key order, unlike a HashMap, so symbols missed
    // equally often always come out in the same order.
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    for &position in session.mistakes() {
        let expected = session.target()[position];
        if expected.is_ascii_punctuation() {
            *counts.entry(expected).or_insert(0) += 1;
        }
    }

    let mut missed: Vec<(char, usize)> = counts.into_iter().collect();
    // Most missed first: `Reverse` flips the order so big counts come first.
    // The sort is stable, so ties keep their char order from the BTreeMap.
    missed.sort_by_key(|&(_, count)| Reverse(count));
    missed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// A session over `target` after typing `typed`, one character every `step`.
    fn finished_session(target: &str, typed: &str, step: Duration) -> Session {
        let mut session = Session::new(target);
        let mut at = Instant::now();
        for c in typed.chars() {
            session.type_char(c, at);
            at += step;
        }
        session
    }

    #[test]
    fn fifty_chars_in_a_minute_is_ten_wpm() {
        assert_eq!(wpm(50, Duration::from_secs(60)), 10.0);
    }

    #[test]
    fn no_time_means_no_speed() {
        assert_eq!(wpm(5, Duration::ZERO), 0.0);
    }

    #[test]
    fn accuracy_counts_mistakes_against_keystrokes() {
        assert_eq!(accuracy(20, 5), 75.0);
    }

    #[test]
    fn nothing_typed_is_full_accuracy() {
        assert_eq!(accuracy(0, 0), 100.0);
    }

    #[test]
    fn wpm_ignores_wrong_chars() {
        // 11 keystrokes 1 s apart = 10 s. "abcde" correct + "fghij" with
        // two wrong = 9 correct chars = 1.8 words in 1/6 minute.
        let session = finished_session("abcde fghij", "abcde fgXiX", Duration::from_secs(1));
        let stats = round_stats(&session);
        assert!((stats.wpm - 10.8).abs() < 1e-9);
    }

    #[test]
    fn letters_are_not_listed_as_missed_symbols() {
        let session = finished_session("ab|", "xy|", Duration::from_secs(1));
        assert!(round_stats(&session).missed.is_empty());
    }

    #[test]
    fn missed_symbols_are_sorted_by_count() {
        let session = finished_session("{ ' ' }", "x x x x", Duration::from_secs(1));
        assert_eq!(
            round_stats(&session).missed,
            vec![('\'', 2), ('{', 1), ('}', 1)]
        );
    }
}
