//! One typing round: the text to type, what the user has typed so far,
//! when they started and finished, and every mistake they made.
//!
//! Pure data, no terminal code, so everything here is easy to unit test.

use std::time::{Duration, Instant};

/// Both texts are stored as `Vec<char>` rather than `String` so that
/// "the i-th character" is a plain index. A `String` is indexed by byte,
/// which only matches characters for ASCII text.
pub struct Session {
    target: Vec<char>,
    typed: Vec<char>,
    /// Every character key pressed, including ones later erased.
    keystrokes: usize,
    /// Position in `target` of every wrong keystroke. Backspace does not
    /// remove entries: a corrected mistake still counts against accuracy.
    mistakes: Vec<usize>,
    /// Time of the first keystroke.
    started_at: Option<Instant>,
    /// Time of the keystroke that completed the target.
    finished_at: Option<Instant>,
}

impl Session {
    pub fn new(target: &str) -> Self {
        Session {
            target: target.chars().collect(),
            typed: Vec::new(),
            keystrokes: 0,
            mistakes: Vec::new(),
            started_at: None,
            finished_at: None,
        }
    }

    /// Records one character typed at time `at`. Ignored once the whole
    /// target has been typed.
    pub fn type_char(&mut self, c: char, at: Instant) {
        if self.is_finished() {
            return;
        }
        if self.started_at.is_none() {
            self.started_at = Some(at);
        }

        let position = self.typed.len();
        if c != self.target[position] {
            self.mistakes.push(position);
        }
        self.keystrokes += 1;
        self.typed.push(c);

        if self.is_finished() {
            self.finished_at = Some(at);
        }
    }

    /// Removes the last typed character, if there is one. A finished round
    /// is final, so this does nothing after the last character.
    pub fn backspace(&mut self) {
        if !self.is_finished() {
            self.typed.pop();
        }
    }

    pub fn is_finished(&self) -> bool {
        self.typed.len() == self.target.len()
    }

    /// Time from the first to the last keystroke, once the round is finished.
    pub fn elapsed(&self) -> Option<Duration> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            _ => None,
        }
    }

    pub fn target(&self) -> &[char] {
        &self.target
    }

    pub fn typed(&self) -> &[char] {
        &self.typed
    }

    pub fn keystrokes(&self) -> usize {
        self.keystrokes
    }

    pub fn mistakes(&self) -> &[usize] {
        &self.mistakes
    }

    /// One flag per target position: was it ever typed wrong this round?
    /// Several mistakes at the same position still give a single `true`.
    pub fn missed_positions(&self) -> Vec<bool> {
        let mut missed = vec![false; self.target.len()];
        for &position in &self.mistakes {
            missed[position] = true;
        }
        missed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Types `text`, one character per second starting at `t0`.
    fn type_text(session: &mut Session, text: &str, t0: Instant) {
        let mut at = t0;
        for c in text.chars() {
            session.type_char(c, at);
            at += Duration::from_secs(1);
        }
    }

    #[test]
    fn records_correct_and_incorrect_chars() {
        let mut session = Session::new("ls -l");
        type_text(&mut session, "lx", Instant::now());
        assert_eq!(session.typed(), &['l', 'x']);
        assert_eq!(session.mistakes(), &[1]);
    }

    #[test]
    fn backspace_on_empty_input_does_nothing() {
        let mut session = Session::new("ls");
        session.backspace();
        assert!(session.typed().is_empty());
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut session = Session::new("ls -l");
        type_text(&mut session, "lx", Instant::now());
        session.backspace();
        assert_eq!(session.typed(), &['l']);
    }

    #[test]
    fn corrected_mistake_still_counts() {
        let mut session = Session::new("ls -l");
        type_text(&mut session, "lx", Instant::now());
        session.backspace();
        type_text(&mut session, "s", Instant::now());
        assert_eq!(session.typed(), &['l', 's']);
        assert_eq!(session.mistakes(), &[1]);
        assert_eq!(session.keystrokes(), 3);
    }

    #[test]
    fn finishes_when_every_char_is_typed() {
        let mut session = Session::new("ls");
        type_text(&mut session, "l", Instant::now());
        assert!(!session.is_finished());
        type_text(&mut session, "s", Instant::now());
        assert!(session.is_finished());
    }

    #[test]
    fn input_after_finish_is_ignored() {
        let mut session = Session::new("ls");
        type_text(&mut session, "ls!", Instant::now());
        assert_eq!(session.typed(), &['l', 's']);
        assert_eq!(session.keystrokes(), 2);
    }

    #[test]
    fn backspace_after_finish_is_ignored() {
        let mut session = Session::new("ls");
        type_text(&mut session, "ls", Instant::now());
        session.backspace();
        assert!(session.is_finished());
    }

    #[test]
    fn elapsed_runs_from_first_to_last_keystroke() {
        let mut session = Session::new("abc");
        // Keystrokes at t0, t0+1s, t0+2s.
        type_text(&mut session, "abc", Instant::now());
        assert_eq!(session.elapsed(), Some(Duration::from_secs(2)));
    }

    #[test]
    fn no_elapsed_time_before_finishing() {
        let mut session = Session::new("abc");
        type_text(&mut session, "ab", Instant::now());
        assert_eq!(session.elapsed(), None);
    }
}
