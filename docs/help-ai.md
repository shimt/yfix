# yfix - clean and copy terminal text

> This guide is for AI agents (Claude Code, Gemini, etc.) to learn how to use yfix.
> Environment-dependent paths are shown in the "Runtime info" section at the end of `yfix --help-ai` output.

## What it does

Cleans terminal-copied text: removes line-wrap artifacts, line numbers, ANSI escapes, normalizes indentation. Writes result to OS clipboard, tmux/screen buffer, OSC 52, or stdout. Works standalone, inside tmux/screen/byobu, over SSH, or via mosh.

## CLI

```
yfix [OPTIONS] [TEXT]

Options:
  --output <targets>   stdout, os-clipboard, clipboard, wsl-clipboard, tmux, screen, osc52 (comma-separated)
  --width <N>          terminal width for unwrap (use #{pane_width} in tmux)
  --config <path>      override config file path
  --show-terminal      print detection results to stderr
  --help-ai            print this guide (markdown)
  --oops [comment]     flag last debug log entry as problematic
```

## How to use

### Pipe text through yfix
```sh
pbpaste | yfix                          # clean and write back to OS clipboard
pbpaste | yfix --output stdout          # output to stdout
echo "text" | yfix                      # clean and copy
```

### tmux copy-mode integration
```tmux
bind-key -T copy-mode-vi y \
  send-keys -X copy-pipe-and-cancel "yfix --width #{pane_width}"

bind-key -T copy-mode MouseDragEnd1Pane \
  send-keys -X copy-pipe-and-cancel "yfix --width #{pane_width}"
```

## Error handling

- **stderr is a TTY**: errors printed to stderr
- **stderr is not a TTY**: silent (safe for pipes, `2>&1`, AI agents)
- **Exit codes**: 0=success, 1=unrecoverable error, 2=partial output failure

## Environment variables

| Variable | Purpose |
|---|---|
| TMUX | detect tmux session |
| TMUX_PANE | pane ID for width query |
| STY | detect screen session |
| SSH_CLIENT | detect SSH session |
| SSH_TTY | detect SSH session |
| COLUMNS | terminal width fallback |
| YFIX_DEBUG_OVERRIDE | `on`/`off` to override debug flag file |

## Output auto-detection

Run `yfix --show-terminal` to see resolved targets for your environment.

| Environment | Output Targets |
|---|---|
| tmux + local (`set-clipboard=on`) | tmux-buffer + os-clipboard |
| tmux + local (`set-clipboard=external`) | tmux-buffer + os-clipboard + osc52(tmux-client-tty) |
| tmux + local (`set-clipboard=off`) | tmux-buffer |
| tmux + SSH/mosh (`set-clipboard=on` or `external`) | tmux-buffer + osc52(tmux-client-tty) |
| tmux + SSH/mosh (`set-clipboard=off`) | tmux-buffer |
| screen + local | screen-buffer + os-clipboard |
| screen + SSH | screen-buffer + osc52(screen-passthrough) |
| standalone + local | os-clipboard |
| standalone + SSH/mosh | osc52 |
| WSL standalone | wsl-clipboard (clip.exe) |
| WSL + tmux | tmux-buffer + wsl-clipboard |

WSL detected via `/proc/version` containing "microsoft".

## Debug / feedback mode

Toggle by creating/removing the debug flag file shown in Runtime info (typically `<app_config_dir>/debug`).

When enabled:
- Clipboard output stays clean
- Each run appends a JSONL entry to `debug.log` with: `id`, `timestamp`, `version`, `type` (`trace`/`error`), `width`, `input` (raw text), `trace`, `warnings`
- `yfix --oops ["comment"]` flags the last entry as problematic

### Log analysis (jq)

```sh
jq 'select(.flagged)' debug.log                  # flagged entries
jq 'select(.warnings | length > 0)' debug.log    # entries with warnings
jq 'select(.type == "error")' debug.log           # errors
tail -1 debug.log | jq .                           # last entry

# Re-process flagged entries with current code
current=$(yfix --help-ai 2>/dev/null | grep Version | awk '{print $NF}')
jq -r --arg v "$current" \
  'select(.version != $v and (.flagged or (.warnings | length > 0))) | @json' \
  debug.log | while read -r entry; do
    w=$(echo "$entry" | jq -r '.width')
    echo "$entry" | jq -r '.input' | yfix --width "$w" --output stdout
done
```

### Auto-detected warnings

| Warning | Condition |
|---|---|
| `line_numbers_borderline` | Match rate 50-70% (near strip threshold) |
| `line_numbers_partial_gutter` | Non-matched lines retain gutter-width indent |
| `join_near_miss` | Line width 70-97% of wrap_width, not joined |
| `join_relaxed_used` | Join used relaxed threshold (potential false positive) |

### Known limitations

- **Application word-wrap**: Word-boundary wrapping (e.g., Claude Code) produces lines much shorter than pane width — `join_wrapped` cannot detect these.
- **Line number gutter + wrap**: After `strip_line_numbers` removes prefixes, remaining lines may be too short for `join_wrapped` threshold.
