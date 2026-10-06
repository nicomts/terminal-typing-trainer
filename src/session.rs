//! One typing round: the text to type and what the user has typed so far.
//!
//! Pure data, no terminal code, so everything here is easy to unit test.

/// Both texts are stored as `Vec<char>` rather than `String` so that
/// "the i-th character" is a plain index. A `String` is indexed by byte,
/// which only matches characters for ASCII text.
pub struct Session {
    target: Vec<char>,
    typed: Vec<char>,
}

impl Session {
    pub fn new(target: &str) -> Self {
        Session {
            target: target.chars().collect(),
            typed: Vec::new(),
        }
    }

    /// Records one typed character. Ignored once the whole target has been typed.
    pub fn type_char(&mut self, c: char) {
        if !self.is_finished() {
            self.typed.push(c);
        }
    }

    /// Removes the last typed character, if there is one.
    pub fn backspace(&mut self) {
        self.typed.pop();
    }

    pub fn is_finished(&self) -> bool {
        self.typed.len() == self.target.len()
    }

    pub fn target(&self) -> &[char] {
        &self.target
    }

    pub fn typed(&self) -> &[char] {
        &self.typed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_correct_and_incorrect_chars() {
        let mut session = Session::new("ls -l");
        session.type_char('l');
        session.type_char('x');
        assert_eq!(session.typed(), &['l', 'x']);
        assert_eq!(session.target()[1], 's');
    }

    #[test]
    fn backspace_on_empty_input_does_nothing() {
        let mut session = Session::new("ls");
        session.backspace();
        assert!(session.typed().is_empty());
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut session = Session::new("ls");
        session.type_char('l');
        session.type_char('x');
        session.backspace();
        assert_eq!(session.typed(), &['l']);
    }

    #[test]
    fn finishes_when_every_char_is_typed() {
        let mut session = Session::new("ls");
        session.type_char('l');
        assert!(!session.is_finished());
        session.type_char('s');
        assert!(session.is_finished());
    }

    #[test]
    fn input_after_finish_is_ignored() {
        let mut session = Session::new("ls");
        session.type_char('l');
        session.type_char('s');
        session.type_char('!');
        assert_eq!(session.typed(), &['l', 's']);
    }
}
