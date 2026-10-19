# The typing GIF for the README: one round with a fixed typo, then results.
# Run from the repository root:  vhs docs/typing.tape
#
# Uses the demo stats in docs/demo/ and a fixed seed, so the same command
# comes up every time. Your own stats are never read or written: HOME points
# at a temporary folder while it runs.

Output "docs/typing.gif"

Set Shell zsh
Set FontSize 20
Set Width 1280
Set Height 320
Set Padding 24
Set CursorBlink false
Set TypingSpeed 170ms
Set Theme { "name": "Project DL", "background": "#0C0C10", "foreground": "#EAEAF2", "cursor": "#00D4FF", "selection": "#28283A", "black": "#0C0C10", "brightBlack": "#3A3A52", "red": "#FF2EA3", "brightRed": "#FF2EA3", "green": "#00D4FF", "brightGreen": "#00D4FF", "yellow": "#EAEAF2", "brightYellow": "#EAEAF2", "blue": "#00D4FF", "brightBlue": "#00D4FF", "magenta": "#FF2EA3", "brightMagenta": "#FF2EA3", "cyan": "#00D4FF", "brightCyan": "#00D4FF", "white": "#EAEAF2", "brightWhite": "#FFFFFF" }

Hide
Type@0ms "cargo build --release -q && REPO=$PWD && export HOME=$(mktemp -d)" Enter
Type@0ms `DATA="$HOME/Library/Application Support/terminal-typing-trainer" && mkdir -p "$DATA"` Enter
Type@0ms `cp "$REPO/docs/demo/stats.json" "$DATA/" && clear` Enter
Sleep 15s
Type@0ms "TYPE_TRAINER_SEED=2 $REPO/target/release/terminal-typing-trainer" Enter
Sleep 2s
# Start a round in "shell".
Down 3
Enter
Sleep 1s
Show

Sleep 800ms
Type "./backup.sh >> /var/log/backup.log 2>"
# A typo where '&' belongs, then the fix.
Type "1"
Sleep 600ms
Backspace
Sleep 300ms
Type "&1"
Sleep 3500ms

Hide
Escape
Type "q"
