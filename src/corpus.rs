//! The command corpus: TOML files in `corpus/`, embedded into the binary.
//!
//! The commands are display text only. Nothing here ever runs them.

use color_eyre::Result;
use color_eyre::eyre::{WrapErr, bail};
use rand::RngExt;
use rand::rngs::StdRng;
use serde::Deserialize;

/// Longest allowed `explain`. It gets one row of the Results panel, so it
/// must fit in the narrowest panel: 60 columns minus the 2 side borders.
pub const MAX_EXPLAIN_CHARS: usize = 58;

/// Every corpus file, embedded at compile time. The name is only used in
/// error messages.
const FILES: [(&str, &str); 3] = [
    ("files.toml", include_str!("../corpus/files.toml")),
    ("text.toml", include_str!("../corpus/text.toml")),
    ("shell.toml", include_str!("../corpus/shell.toml")),
];

#[derive(Debug, Deserialize)]
pub struct Command {
    pub text: String,
    pub explain: String,
    pub tags: Vec<String>,
}

/// The shape of one corpus file: a list of `[[command]]` tables.
#[derive(Debug, Deserialize)]
struct CorpusFile {
    command: Vec<Command>,
}

/// Parses and checks every embedded corpus file.
pub fn load() -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    for (name, contents) in FILES {
        let file: CorpusFile = toml::from_str(contents)
            .wrap_err_with(|| format!("corpus file {name} is not valid"))?;
        for command in file.command {
            check(&command)
                .wrap_err_with(|| format!("corpus file {name}: bad command {:?}", command.text))?;
            commands.push(command);
        }
    }
    if commands.is_empty() {
        bail!("the corpus has no commands");
    }
    Ok(commands)
}

/// Rejects commands that can't be typed or don't fit on screen.
fn check(command: &Command) -> Result<()> {
    if command.text.is_empty() {
        bail!("text is empty");
    }
    // Newlines and tabs can't be typed as text in the typing screen.
    if command.text.chars().any(|c| c.is_control()) {
        bail!("text contains a newline, tab or other control character");
    }
    // Easy to get by accident when a ''' string spans several lines.
    if command.text.trim() != command.text {
        bail!("text starts or ends with whitespace");
    }
    if command.explain.is_empty() {
        bail!("explain is empty");
    }
    if command.explain.chars().count() > MAX_EXPLAIN_CHARS {
        bail!("explain is longer than {MAX_EXPLAIN_CHARS} characters");
    }
    if command.tags.is_empty() {
        bail!("tags are empty");
    }
    Ok(())
}

/// A random index into a list of `len` commands. `len` must be at least 1.
pub fn pick(len: usize, rng: &mut StdRng) -> usize {
    rng.random_range(0..len)
}

/// A random index other than `current`, so the same command never comes
/// twice in a row. With a single command there is no choice: returns 0.
pub fn pick_next(len: usize, current: usize, rng: &mut StdRng) -> usize {
    if len < 2 {
        return 0;
    }
    // Draw from the other `len - 1` positions, then step over `current`.
    let index = rng.random_range(0..len - 1);
    if index >= current { index + 1 } else { index }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn command(text: &str, explain: &str) -> Command {
        Command {
            text: text.to_string(),
            explain: explain.to_string(),
            tags: vec!["test".to_string()],
        }
    }

    #[test]
    fn embedded_corpus_loads_and_passes_checks() {
        // If this fails, the error names the file and the command.
        let commands = load().unwrap();
        assert!(commands.len() >= 30);
    }

    #[test]
    fn literal_strings_keep_quotes_and_backslashes() {
        let commands = load().unwrap();
        let texts: Vec<&str> = commands.iter().map(|c| c.text.as_str()).collect();
        assert!(texts.contains(&r#"find /var/log -name '*.log' -mtime +7 -exec gzip {} \;"#));
        // Written as `...'''' ` in the TOML: a quote right before the closing '''.
        assert!(texts.contains(&r#"ps aux | grep '[n]ginx' | awk '{print $2}'"#));
    }

    #[test]
    fn rejects_text_with_newline() {
        assert!(check(&command("ls\n-l", "List")).is_err());
    }

    #[test]
    fn rejects_text_with_trailing_space() {
        assert!(check(&command("ls -l ", "List")).is_err());
    }

    #[test]
    fn rejects_long_explain() {
        let explain = "x".repeat(MAX_EXPLAIN_CHARS + 1);
        assert!(check(&command("ls", &explain)).is_err());
    }

    #[test]
    fn rejects_missing_tags() {
        let mut no_tags = command("ls", "List");
        no_tags.tags.clear();
        assert!(check(&no_tags).is_err());
    }

    #[test]
    fn reports_which_field_is_missing() {
        let error = toml::from_str::<CorpusFile>("[[command]]\ntext = '''ls'''\n").unwrap_err();
        assert!(error.to_string().contains("explain"));
    }

    #[test]
    fn pick_next_never_repeats() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut current = 0;
        for _ in 0..200 {
            let next = pick_next(3, current, &mut rng);
            assert_ne!(next, current);
            assert!(next < 3);
            current = next;
        }
    }

    #[test]
    fn pick_next_reaches_every_other_command() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut seen = [false; 4];
        for _ in 0..200 {
            seen[pick_next(4, 0, &mut rng)] = true;
        }
        assert_eq!(seen, [false, true, true, true]);
    }

    #[test]
    fn pick_next_with_one_command_returns_it() {
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(pick_next(1, 0, &mut rng), 0);
    }
}
