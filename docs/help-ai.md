# yfix - clean and copy terminal text

## Purpose
Cleans up text copied from terminals: removes line-wrap artifacts,
line numbers, ANSI escapes, and normalizes indentation.
Writes result to OS clipboard, tmux/screen buffer, OSC 52, or stdout.
Works standalone, inside tmux/screen/byobu, or over SSH.

## CLI
```
yfix [OPTIONS] [TEXT]

Options:
  --output <targets>   stdout, os-clipboard, tmux, osc52 (comma-separated)
  --auto               auto-detect output targets (default)
  --width <N>          terminal width for unwrap (use #{pane_width} in tmux)
  --config <path>      override config file path
  --show-terminal      print detection results to stderr
  --help-ai            print this AI integration guide (markdown)
  --oops [comment]     flag last debug log entry as problematic
```

## Error Handling
- **stderr is a TTY**: errors printed to stderr (interactive use)
- **stderr is not a TTY**: silent (safe for pipes, `2>&1`, AI agents)
- **debug mode ON**: errors logged to `debug.log` as `"type":"error"` entries
- **exit codes**: 0=success, 1=unrecoverable error, 2=partial output failure

## Usage Examples

### Pipe from clipboard
```sh
pbpaste | yfix                          # clean and write back to OS clipboard
pbpaste | yfix --output stdout          # output to stdout
pbpaste | yfix --output osc52           # send via OSC 52
```

### tmux copy-mode integration
```tmux
bind-key -T copy-mode-vi y \
  send-keys -X copy-pipe-and-cancel "yfix --width #{pane_width}"

bind-key -T copy-mode MouseDragEnd1Pane \
  send-keys -X copy-pipe-and-cancel "yfix --width #{pane_width}"
```

## Environment Variables Used for Detection
| Variable      | Purpose                    |
|---------------|----------------------------|
| TMUX          | detect tmux session        |
| TMUX_PANE     | pane ID for width query    |
| STY           | detect screen session      |
| BYOBU_BACKEND | detect byobu backend       |
| SSH_CLIENT    | detect SSH session         |
| SSH_TTY       | detect SSH session         |
| TERM_PROGRAM  | detect terminal type       |
| COLUMNS       | terminal width fallback    |
| YFIX_DEBUG_OVERRIDE | `on`/`off` to override debug flag file |

## Auto-detection Matrix
tmux output targets vary by the `set-clipboard` option. Run `yfix --show-terminal` to see resolved targets for your environment.

| Environment                          | Output Targets                                       |
|-------------------------------------|------------------------------------------------------|
| tmux + local (`set-clipboard=on`)   | tmux-buffer + os-clipboard                           |
| tmux + local (`set-clipboard=ext`)  | tmux-buffer + os-clipboard + osc52(tmux-client-tty) |
| tmux + local (`set-clipboard=off`)  | tmux-buffer                                          |
| tmux + SSH (`set-clipboard=on`)     | tmux-buffer + osc52(tmux-client-tty)                |
| tmux + SSH (`set-clipboard=ext`)    | tmux-buffer + osc52(tmux-client-tty)                |
| tmux + SSH (`set-clipboard=off`)    | tmux-buffer                                          |
| screen + local                      | screen-buffer + os-clipboard                         |
| screen + SSH                        | screen-buffer + osc52(screen-passthrough)            |
| standalone + local                  | os-clipboard                                         |
| standalone + SSH                    | osc52                                                |
| WSL standalone                      | wsl-clipboard (clip.exe)                             |
| WSL + tmux (local)                  | tmux-buffer + wsl-clipboard                          |

Note: WSL is detected via `/proc/version` containing "microsoft". `os-clipboard` is replaced by `wsl-clipboard` (uses `clip.exe`) since X11/Wayland is typically unavailable in WSL.

## Config File
Location: {config_dir}/yfix/config.yaml (via `directories` crate)

## Debug / Feedback Mode

### Toggle
Create `{config_dir}/yfix/debug` to enable. Remove to disable.

```sh
# Enable (macOS)
touch ~/Library/Application\ Support/yfix/debug

# Enable (Linux)
touch ~/.config/yfix/debug

# Disable
rm ~/Library/Application\ Support/yfix/debug  # macOS
rm ~/.config/yfix/debug                        # Linux
```

### Behavior when enabled
- **Clipboard output stays clean** (no debug info mixed in)
- **Log file** (`{config_dir}/yfix/debug.log`) is appended on each run in **JSONL format** (one JSON object per line)
- Two entry types distinguished by `"type"` field:
  - `"trace"`: normal operation — transformer pipeline trace with per-line widths and warnings
  - `"error"`: unrecoverable error — error message with environment context
- Common fields: `id`, `timestamp`, `version`, `type`, `width`, `width_source`, `is_ssh`
- Trace-only fields: `output_targets`, `input` (raw input text), `trace`, `warnings`
- Error-only field: `error`
- Feedback fields: `flagged`, `flagged_comment` (set by `--oops`)

### Automatic warnings
The log flags suspicious patterns for review:

| Warning (`type` field) | Condition |
|---|---|
| `line_numbers_borderline` | Line number match rate 50-70% (near threshold) |
| `line_numbers_partial_gutter` | After stripping, non-matched lines retain gutter-width indent |
| `join_near_miss` | Line width is 70-97% of wrap_width but was not joined |
| `join_relaxed_used` | Join used relaxed threshold (potential false positive) |

### Flagging bad results: `--oops`

```sh
yfix --oops              # mark last log entry as problematic
yfix --oops "comment"    # mark with a description
```

When `--oops` is run, the most recent entry in `debug.log` gets `"flagged": true`
with an optional `flagged_comment`. No tmux config change needed — run it manually
after noticing a bad result.

### Analyzing the log (JSONL + jq)

```sh
# Show all flagged entries
jq 'select(.flagged)' debug.log

# Show entries with warnings
jq 'select(.warnings | length > 0)' debug.log

# Warning type frequency
jq -s '[.[].warnings[].type] | group_by(.) | map({type: .[0], count: length})' debug.log

# Flagged entries with comments
jq 'select(.flagged) | {id, flagged_comment, warnings}' debug.log

# Filter by width source
jq 'select(.width_source == "TmuxPane")' debug.log

# Error entries
jq 'select(.type == "error")' debug.log

# Trace entries only (exclude errors)
jq 'select(.type == "trace")' debug.log

# Pretty-print last entry
tail -1 debug.log | jq .

# Re-process flagged/warned entries from older versions with current code
current=$(yfix --help-ai 2>/dev/null | grep Version | awk '{print $NF}')
jq -r --arg v "$current" \
  'select(.version != $v and (.flagged or (.warnings | length > 0))) | @json' \
  debug.log | while read -r entry; do
    w=$(echo "$entry" | jq -r '.width')
    echo "$entry" | jq -r '.input' | yfix --width "$w" --output stdout
done
```

### Known limitations
- **Application word-wrap**: When an application (e.g., Claude Code) wraps text at word
  boundaries rather than column boundaries, `join_wrapped` cannot detect it because line
  widths are significantly shorter than pane width. Terminal hard-wraps (exact column) are
  handled correctly.
- **Line number gutter + terminal wrap**: After `strip_line_numbers` removes the number
  prefix, the remaining line may be too short for `join_wrapped` to detect as wrapped.
  Continuation lines retain the original gutter indent as leading spaces.
