# CLAUDE.md

## Project

A terminal typing trainer (in the spirit of monkeytype / keybr) that drills real
Linux sysadmin commands full of special characters: `| & ; > >> 2>&1 $() {} [] ^ ~ \ ' "`.
The user types a command shown on screen; the app tracks speed, accuracy and which
symbols they miss most, and serves more commands containing their weak symbols.

The developer is **learning Rust** with this project. Optimize for code he can read
and understand, not for cleverness.

## Stack

- Rust (stable), edition 2024 (the `cargo new` default)
- `ratatui` for UI, using its re-exported `ratatui::crossterm` (do not add crossterm separately)
- `color-eyre` for errors
- `serde` + `toml` (command corpus), `serde_json` (saved stats)
- `directories` for config/data paths
- `rand` for weighted command selection
- Dev: `insta` for snapshot tests of rendered UI via `ratatui::backend::TestBackend`

No async runtime. The app is local and single-threaded: one loop that polls for
key events with a short timeout and redraws on each tick. Do not add tokio.

Ask before adding any dependency not listed here. Add dependencies with `cargo add`,
never by typing version numbers into `Cargo.toml` (they go stale).

## Architecture

```
corpus/                 # TOML command lists, one file per topic, embedded with include_str!
src/
├── main.rs             # terminal init/restore, panic-safe, runs the loop
├── app.rs              # App state, Screen enum (Menu, Typing, Results), update(event)
├── event.rs            # poll keys + ticks into one Event enum
├── session.rs          # one typing round: target text, typed chars, start/end time
├── stats.rs            # WPM, accuracy, per-symbol error counts
├── corpus.rs           # parse embedded TOML, weighted pick by weak symbols
├── storage.rs          # load/save stats JSON in the platform data dir
├── theme.rs            # THE ONLY place colors are defined
└── ui/
    ├── mod.rs          # top-level layout, dispatches by Screen
    ├── menu.rs
    ├── typing.rs
    └── results.rs
```

Pattern: Elm-style. State lives in `App`. `update` changes state and never touches
the terminal. `ui::render` reads state and never mutates it. This keeps
`session.rs`, `stats.rs` and `corpus.rs` pure and unit-testable.

Keep it shallow: plain structs, enums and functions. No traits or generics unless
there are already two concrete implementations that need them.

## Corpus format

```toml
[[command]]
text = '''find /var/log -name '*.log' -mtime +7 -exec gzip {} \;'''
explain = "Compress log files older than 7 days"
tags = ["find", "exec"]
```

Always use TOML multi-line literal strings (`'''...'''`) for `text` so quotes and
backslashes are stored exactly as typed. `explain` is shown after each round.

## Rules

- **Never execute commands from `corpus/`.** They are display text only.
- Before finishing any task, run and pass:
  `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- No `unwrap()` / `expect()` outside tests; propagate with `?`.
- All styling comes from the functions in `theme.rs` (e.g. `theme::untyped()`,
  `theme::incorrect(c)`, `theme::cursor()`). Never construct a `Color` outside
  `theme.rs`. Accent 2 (magenta) is reserved for mistakes; never use it elsewhere.
- Build every bordered panel with `theme::panel_block(focused)` (rounded corners);
  don't construct `Block` with borders directly in `ui/`.
- Handle only `KeyEventKind::Press` (Windows also emits release events).
- Use `directories` for file paths; never hardcode `~/` or `/`-separated paths.
- Terminal must be restored on exit and on panic (use `ratatui::init` / `ratatui::restore`).

## Teaching rules (developer is learning Rust)

- When fixing a borrow-checker or lifetime error, explain the cause in 1–2 sentences.
- Do not add `.clone()`, `Rc`, `Arc` or `Mutex` just to silence the compiler.
  If one is genuinely the right fix, say why.
- Prefer explicit, readable code over iterator chains when a chain gets longer than
  ~3 adapters.
- When asked to "explain" or "review", do not rewrite the code unless asked.
