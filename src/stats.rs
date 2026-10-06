//! Numbers about typing: one round's speed, accuracy and missed symbols,
//! and the `Profile` that adds up every round so far.
//!
//! Pure functions and data, no terminal, no clock, no files.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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

/// Whether `c` is a symbol this trainer tracks: ASCII punctuation
/// (`| & ; > $ { } ' "` and friends). Letters and spaces are not tracked.
pub fn is_symbol(c: char) -> bool {
    c.is_ascii_punctuation()
}

/// Counts mistakes per expected symbol, for this round only.
fn missed_symbols(session: &Session) -> Vec<(char, usize)> {
    // A BTreeMap iterates in key order, unlike a HashMap, so symbols missed
    // equally often always come out in the same order.
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    for &position in session.mistakes() {
        let expected = session.target()[position];
        if is_symbol(expected) {
            *counts.entry(expected).or_insert(0) += 1;
        }
    }

    let mut missed: Vec<(char, usize)> = counts.into_iter().collect();
    // Most missed first: `Reverse` flips the order so big counts come first.
    // The sort is stable, so ties keep their char order from the BTreeMap.
    missed.sort_by_key(|&(_, count)| Reverse(count));
    missed
}

/// Everything remembered between runs. Saved as JSON by `storage.rs`.
///
/// `#[serde(default)]` fills in any field missing from the file, so a file
/// written by an older version, before a field existed, still loads.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub rounds: u32,
    pub best_wpm: f64,
    /// Per symbol. A BTreeMap keeps the JSON file in a stable order.
    pub symbols: BTreeMap<char, SymbolRecord>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SymbolRecord {
    /// Times the symbol appeared in a finished command.
    pub seen: u32,
    /// Of those, how many were typed wrong at least once.
    pub missed: u32,
}

impl SymbolRecord {
    /// Plain share of misses, 0.0 to 1.0. Shown to the user.
    pub fn miss_rate(&self) -> f64 {
        if self.seen == 0 {
            return 0.0;
        }
        f64::from(self.missed) / f64::from(self.seen)
    }
}

impl Profile {
    /// Adds a finished round to the totals.
    pub fn record_round(&mut self, session: &Session, stats: &RoundStats) {
        self.rounds += 1;
        if stats.wpm > self.best_wpm {
            self.best_wpm = stats.wpm;
        }

        let missed = session.missed_positions();
        for (i, &c) in session.target().iter().enumerate() {
            if !is_symbol(c) {
                continue;
            }
            // `or_default` inserts a zeroed record the first time we see `c`.
            let record = self.symbols.entry(c).or_default();
            record.seen += 1;
            if missed[i] {
                record.missed += 1;
            }
        }
    }

    /// How weak the user is at `symbol`, 0.0 to 1.0. Used to pick commands.
    ///
    /// This is the miss rate with one imaginary miss and one imaginary hit
    /// added ("smoothing"). A symbol never typed scores 0.5, so it still gets
    /// practised, and a single early miss doesn't jump straight to 100%.
    pub fn weakness(&self, symbol: char) -> f64 {
        match self.symbols.get(&symbol) {
            Some(record) => (f64::from(record.missed) + 1.0) / (f64::from(record.seen) + 2.0),
            None => 0.5,
        }
    }

    /// Up to `n` symbols the user has missed, weakest first, each with its
    /// plain miss rate for display. Ordered by `weakness`, not miss rate:
    /// 9 misses out of 10 (90%) ranks above 1 out of 1 (100%), because ten
    /// tries say more than one.
    pub fn weakest(&self, n: usize) -> Vec<(char, f64)> {
        let mut missed: Vec<char> = Vec::new();
        for (&symbol, record) in &self.symbols {
            if record.missed > 0 {
                missed.push(symbol);
            }
        }
        // f64 has no total order (because of NaN), so `sort_by_key` can't be
        // used; `total_cmp` gives one. Arguments swapped: weakest first.
        missed.sort_by(|a, b| self.weakness(*b).total_cmp(&self.weakness(*a)));
        missed.truncate(n);

        let mut result = Vec::with_capacity(missed.len());
        for symbol in missed {
            result.push((symbol, self.symbols[&symbol].miss_rate()));
        }
        result
    }
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

    fn record(profile: &mut Profile, target: &str, typed: &str) {
        let session = finished_session(target, typed, Duration::from_secs(1));
        profile.record_round(&session, &round_stats(&session));
    }

    #[test]
    fn record_round_counts_seen_and_missed_symbols() {
        let mut profile = Profile::default();
        record(&mut profile, "a | b | c", "a x b | c");
        assert_eq!(profile.rounds, 1);
        assert_eq!(profile.symbols[&'|'], SymbolRecord { seen: 2, missed: 1 });
        // Letters and spaces are not tracked.
        assert_eq!(profile.symbols.len(), 1);
    }

    #[test]
    fn repeated_mistakes_at_one_position_count_once() {
        let mut session = Session::new("a|b");
        let at = Instant::now();
        session.type_char('a', at);
        session.type_char('x', at);
        session.backspace();
        session.type_char('y', at);
        session.backspace();
        session.type_char('|', at);
        session.type_char('b', at);

        let mut profile = Profile::default();
        profile.record_round(&session, &round_stats(&session));
        assert_eq!(profile.symbols[&'|'], SymbolRecord { seen: 1, missed: 1 });
    }

    #[test]
    fn best_wpm_only_goes_up() {
        let mut profile = Profile::default();
        let fast = finished_session("abcdef", "abcdef", Duration::from_millis(100));
        let slow = finished_session("abcdef", "abcdef", Duration::from_secs(1));
        profile.record_round(&fast, &round_stats(&fast));
        let best = profile.best_wpm;
        profile.record_round(&slow, &round_stats(&slow));
        assert_eq!(profile.best_wpm, best);
        assert_eq!(profile.rounds, 2);
    }

    #[test]
    fn unseen_symbol_has_medium_weakness() {
        assert_eq!(Profile::default().weakness('|'), 0.5);
    }

    #[test]
    fn more_misses_mean_more_weakness() {
        let mut profile = Profile::default();
        profile.symbols.insert(
            '|',
            SymbolRecord {
                seen: 10,
                missed: 1,
            },
        );
        profile.symbols.insert(
            '{',
            SymbolRecord {
                seen: 10,
                missed: 8,
            },
        );
        assert!(profile.weakness('{') > profile.weakness('|'));
        assert!(profile.weakness('|') < 0.5);
    }

    #[test]
    fn weakest_ranks_by_weakness_and_skips_never_missed() {
        let mut profile = Profile::default();
        profile
            .symbols
            .insert('|', SymbolRecord { seen: 1, missed: 1 });
        profile.symbols.insert(
            '{',
            SymbolRecord {
                seen: 10,
                missed: 9,
            },
        );
        profile.symbols.insert(
            ';',
            SymbolRecord {
                seen: 10,
                missed: 0,
            },
        );
        // Miss rates are '|' 100%, '{' 90%, but weakness is '|' 2/3 = 0.67
        // and '{' 10/12 = 0.83, so '{' comes first. ';' was never missed.
        assert_eq!(profile.weakest(5), vec![('{', 0.9), ('|', 1.0)]);
        assert_eq!(profile.weakest(1).len(), 1);
    }
}
