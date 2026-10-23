# Still screenshots for the README: menu, results and help.
# Run from the repository root:  vhs docs/screens.tape
#
# Uses the demo stats in docs/demo/ and a fixed seed, so the same commands
# come up every time. Your own stats are never read or written: HOME points
# at a temporary folder while it runs.

Output "/tmp/terminal-typing-trainer-screens.gif"

Set Shell zsh
# JetBrains Mono with only its Bold face installed, so all text is bold,
# matching the web version. See "Development" in the README.
Set FontFamily "JetBrains Mono"
Set FontSize 20
Set Width 1280
Set Height 640
Set Padding 24
Set CursorBlink false
Set TypingSpeed 190ms
Set Theme { "name": "Project DL", "background": "#0C0C10", "foreground": "#EAEAF2", "cursor": "#00D4FF", "selection": "#28283A", "black": "#0C0C10", "brightBlack": "#3A3A52", "red": "#FF2EA3", "brightRed": "#FF2EA3", "green": "#00D4FF", "brightGreen": "#00D4FF", "yellow": "#EAEAF2", "brightYellow": "#EAEAF2", "blue": "#00D4FF", "brightBlue": "#00D4FF", "magenta": "#FF2EA3", "brightMagenta": "#FF2EA3", "cyan": "#00D4FF", "brightCyan": "#00D4FF", "white": "#EAEAF2", "brightWhite": "#FFFFFF" }

Hide
Type@0ms "cargo build --release -q && REPO=$PWD && export HOME=$(mktemp -d)" Enter
Type@0ms `DATA="$HOME/Library/Application Support/terminal-typing-trainer" && mkdir -p "$DATA"` Enter
Type@0ms `cp "$REPO/docs/demo/stats.json" "$DATA/" && clear` Enter
Sleep 15s
Type@0ms "TYPE_TRAINER_SEED=1 $REPO/target/release/terminal-typing-trainer" Enter
Sleep 2s
Show
Sleep 2s

# Menu, with "services" selected.
Down 7
Sleep 1s
Screenshot "docs/menu.png"

# A round with two mistakes: '{' for '[' and '7' for '&'.
Enter
Sleep 500ms
Type "0 3 * * 1-5 { -x ~/backup.sh ] 7& ~/backup.sh"
Sleep 2s
Screenshot "docs/results.png"

Type "?"
Sleep 1s
Screenshot "docs/help.png"

Hide
Type "q"
Escape
Type "q"
