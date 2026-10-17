//! The command corpus: TOML files in `corpus/`, embedded into the binary.
//!
//! The commands are display text only. Nothing here ever runs them.

use std::collections::BTreeSet;

use color_eyre::Result;
use color_eyre::eyre::{WrapErr, bail};
use rand::RngExt;
use rand::rngs::StdRng;
use serde::Deserialize;

use crate::stats::{self, Profile};

/// Longest allowed `explain`. It gets one row of the Results panel, so it
/// must fit in the narrowest panel: 60 columns minus the 2 side borders.
pub const MAX_EXPLAIN_CHARS: usize = 58;

/// What a command practises. Each topic is one corpus file.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    // `Default` is only needed by `#[serde(skip)]` below; `load` always
    // replaces it with the real topic.
    #[default]
    Files,
    Text,
    Shell,
}

impl Topic {
    /// Every topic, in the order the menu lists them.
    pub const ALL: [Topic; 3] = [Topic::Files, Topic::Text, Topic::Shell];

    pub fn name(self) -> &'static str {
        match self {
            Topic::Files => "files",
            Topic::Text => "text",
            Topic::Shell => "shell",
        }
    }
}

/// Every corpus file, embedded at compile time, with the topic of its
/// commands. The file name is only used in error messages.
const FILES: [(Topic, &str, &str); 3] = [
    (
        Topic::Files,
        "files.toml",
        include_str!("../corpus/files.toml"),
    ),
    (
        Topic::Text,
        "text.toml",
        include_str!("../corpus/text.toml"),
    ),
    (
        Topic::Shell,
        "shell.toml",
        include_str!("../corpus/shell.toml"),
    ),
];

#[derive(Debug, Deserialize)]
pub struct Command {
    pub text: String,
    pub explain: String,
    pub tags: Vec<String>,
    /// Not written in the TOML: it comes from which file the command is in.
    /// `skip` makes serde fill it with `Topic::default()`.
    #[serde(skip)]
    pub topic: Topic,
}

/// The shape of one corpus file: a list of `[[command]]` tables.
#[derive(Debug, Deserialize)]
struct CorpusFile {
    command: Vec<Command>,
}

/// Parses and checks every embedded corpus file.
pub fn load() -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    for (topic, name, contents) in FILES {
        let file: CorpusFile = toml::from_str(contents)
            .wrap_err_with(|| format!("corpus file {name} is not valid"))?;
        for mut command in file.command {
            command.topic = topic;
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

/// How likely `command` is to be picked: 1, plus the weakness of each
/// distinct symbol in it. The base 1 means every command can still come up;
/// commands full of symbols the user misses come up more often.
pub fn command_weight(command: &Command, profile: &Profile) -> f64 {
    // A set, so `{} {}` counts '{' once rather than twice.
    let mut symbols = BTreeSet::new();
    for c in command.text.chars() {
        if stats::is_symbol(c) {
            symbols.insert(c);
        }
    }

    let mut weight = 1.0;
    for symbol in symbols {
        weight += profile.weakness(symbol);
    }
    weight
}

/// Picks a command index at random, favouring commands with weak symbols.
///
/// Only commands in `topic` are picked; `None` means any topic. `current`,
/// if given, is not picked, so a command never comes twice in a row, unless
/// it is the only command in the topic.
pub fn pick_weighted(
    commands: &[Command],
    profile: &Profile,
    topic: Option<Topic>,
    current: Option<usize>,
    rng: &mut StdRng,
) -> usize {
    // Commands that can't be picked get weight 0.
    let mut weights = Vec::with_capacity(commands.len());
    let mut first_in_topic = None;
    for (i, command) in commands.iter().enumerate() {
        let in_topic = topic.is_none() || topic == Some(command.topic);
        if in_topic && first_in_topic.is_none() {
            first_in_topic = Some(i);
        }
        if in_topic && Some(i) != current {
            weights.push(command_weight(command, profile));
        } else {
            weights.push(0.0);
        }
    }

    // Every pickable command weighs at least 1, so a total of 0 means there
    // is nothing to choose from: the topic holds only `current`.
    let total: f64 = weights.iter().sum();
    if total == 0.0 {
        return first_in_topic.unwrap_or(0);
    }

    // Think of the weights as lengths laid end to end: draw a point on the
    // whole line and find which command's stretch it lands in.
    let mut point = rng.random_range(0.0..total);
    let mut last_candidate = 0;
    for (i, &weight) in weights.iter().enumerate() {
        if weight == 0.0 {
            continue;
        }
        if point < weight {
            return i;
        }
        point -= weight;
        last_candidate = i;
    }
    // Only reached if float rounding left `point` a hair past the end.
    last_candidate
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
            topic: Topic::Files,
        }
    }

    #[test]
    fn embedded_corpus_loads_and_passes_checks() {
        // If this fails, the error names the file and the command.
        let commands = load().unwrap();
        assert!(commands.len() >= 30);
    }

    #[test]
    fn every_topic_has_commands() {
        // The menu offers every topic, so none may be empty.
        let commands = load().unwrap();
        for topic in Topic::ALL {
            assert!(commands.iter().any(|c| c.topic == topic), "{topic:?}");
        }
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

    fn commands(texts: &[&str]) -> Vec<Command> {
        let mut list = Vec::new();
        for text in texts {
            list.push(command(text, "Explain"));
        }
        list
    }

    #[test]
    fn command_without_symbols_has_base_weight() {
        assert_eq!(
            command_weight(&command("ls", "List"), &Profile::default()),
            1.0
        );
    }

    #[test]
    fn each_distinct_symbol_adds_its_weakness() {
        // '{' and '}' are new symbols (0.5 each); the repeated pair counts once.
        let weight = command_weight(&command("{} {}", "Braces"), &Profile::default());
        assert_eq!(weight, 2.0);
    }

    #[test]
    fn pick_weighted_never_repeats() {
        let list = commands(&["ls", "pwd", "a | b"]);
        let profile = Profile::default();
        let mut rng = StdRng::seed_from_u64(1);
        let mut current = 0;
        for _ in 0..200 {
            let next = pick_weighted(&list, &profile, None, Some(current), &mut rng);
            assert_ne!(next, current);
            assert!(next < 3);
            current = next;
        }
    }

    #[test]
    fn pick_weighted_reaches_every_other_command() {
        let list = commands(&["ls", "pwd", "a | b", "a & b"]);
        let mut rng = StdRng::seed_from_u64(1);
        let mut seen = [false; 4];
        for _ in 0..200 {
            seen[pick_weighted(&list, &Profile::default(), None, Some(0), &mut rng)] = true;
        }
        assert_eq!(seen, [false, true, true, true]);
    }

    #[test]
    fn weak_symbols_are_picked_more_often() {
        let list = commands(&["ls", "a | b"]);
        let mut profile = Profile::default();
        profile.symbols.insert(
            '|',
            crate::stats::SymbolRecord {
                seen: 10,
                missed: 9,
            },
        );
        // Weights: "ls" 1.0, "a | b" 1 + 10/12 = 1.83, so about 65% of picks.
        let mut rng = StdRng::seed_from_u64(1);
        let mut pipes = 0;
        for _ in 0..2000 {
            if pick_weighted(&list, &profile, None, None, &mut rng) == 1 {
                pipes += 1;
            }
        }
        assert!((1200..1400).contains(&pipes), "picked {pipes} of 2000");
    }

    #[test]
    fn pick_weighted_with_one_command_returns_it() {
        let mut rng = StdRng::seed_from_u64(1);
        let list = commands(&["ls"]);
        assert_eq!(
            pick_weighted(&list, &Profile::default(), None, Some(0), &mut rng),
            0
        );
    }

    #[test]
    fn pick_weighted_stays_in_the_topic() {
        let mut list = commands(&["ls", "a | b", "a & b", "pwd"]);
        list[1].topic = Topic::Shell;
        list[2].topic = Topic::Shell;
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..200 {
            let picked = pick_weighted(
                &list,
                &Profile::default(),
                Some(Topic::Shell),
                None,
                &mut rng,
            );
            assert_eq!(list[picked].topic, Topic::Shell);
        }
    }

    #[test]
    fn pick_weighted_repeats_the_only_command_in_a_topic() {
        let mut list = commands(&["ls", "a | b", "pwd"]);
        list[1].topic = Topic::Shell;
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(
            pick_weighted(
                &list,
                &Profile::default(),
                Some(Topic::Shell),
                Some(1),
                &mut rng
            ),
            1
        );
    }
}
