# yfix

[![CI](https://github.com/shimt/yfix/actions/workflows/ci.yml/badge.svg)](https://github.com/shimt/yfix/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/shimt/yfix#license)

Clean and copy terminal text.

## Why

Terminal copy breaks text: line wraps, line numbers, ANSI escapes, extra indentation.
yfix fixes all of that in one pipe — paste-ready text lands in your clipboard.

Works standalone, inside tmux/screen, over SSH, and through mosh.

## Install

### Homebrew (macOS / Linux)

```sh
brew tap shimt/yfix https://github.com/shimt/yfix
brew install yfix
```

### From source

```sh
cargo install --git https://github.com/shimt/yfix.git
```

### From releases

Download a binary from [GitHub Releases](https://github.com/shimt/yfix/releases) and place it in your `PATH`.

Available targets: macOS aarch64, Linux x86_64, Linux aarch64, Windows x86_64.

## Usage

### Pipe

```sh
pbpaste | yfix                        # clean and write to clipboard
echo "messy text" | yfix              # same
pbpaste | yfix --output stdout        # output to stdout instead
```

### tmux copy-mode

Add to your tmux config:

```tmux
bind-key -T copy-mode-vi y \
  send-keys -X copy-pipe-and-cancel "yfix --width #{pane_width}"

bind-key -T copy-mode MouseDragEnd1Pane \
  send-keys -X copy-pipe-and-cancel "yfix --width #{pane_width}"
```

### Options

```
yfix [OPTIONS] [TEXT]

--output <targets>   stdout, os-clipboard, clipboard, wsl-clipboard, tmux, screen, osc52 (comma-separated)
--width <N>          terminal width for unwrap
--config <path>      override config file path
--version            print version and exit
--show-terminal      print environment detection to stderr
--help-ai            print AI integration guide (markdown)
--oops [comment]     flag last debug log entry
```

## What it cleans

| Transformer | What it does |
|---|---|
| strip_ansi | Remove ANSI escape sequences |
| strip_line_numbers | Remove leading line numbers (>50% threshold) |
| join_wrapped | Rejoin terminal-wrapped lines |
| dedent | Remove common indentation |
| strip_trailing | Remove trailing whitespace and blank lines |
| compress_blank | Compress 3+ blank lines to 1 |
| strip_prompt | Remove bare shell prompts (❯ $ % > >>>) |

## Supported environments

| OS | Status |
|---|---|
| macOS | Supported |
| Linux | Supported |
| Windows (native) | Supported |
| Windows (WSL) | Supported (via clip.exe) |

| Multiplexer | Minimum version |
|---|---|
| tmux | 2.x or later (3.3a+ recommended; older versions use `show-option` fallback for `set-clipboard`) |
| screen | Any |

### Input source & Output targets

yfix reads input in this priority order:

| Priority | Condition | Input source |
|---|---|---|
| 1 | CLI argument: `yfix "text"` | argument |
| 2 | stdin is a pipe | stdin |
| 3 | Inside tmux/screen (no pipe) | multiplexer paste buffer |

If none of the above apply, yfix exits with an error.

| Environment | tmux `set-clipboard` | Output targets |
|---|---|---|
| Local standalone | — | OS clipboard |
| Local + tmux | `on` (default) | tmux buffer + OS clipboard |
| Local + tmux | `external` | tmux buffer + OS clipboard + OSC 52 (client TTY) |
| Local + tmux | `off` | tmux buffer only |
| Local + screen | — | screen buffer + OS clipboard |
| SSH/mosh standalone | — | OSC 52 |
| SSH/mosh + tmux | `on` / `external` | tmux buffer + OSC 52 (client TTY) |
| SSH/mosh + tmux | `off` | tmux buffer only |
| SSH/mosh + screen | — | screen buffer + OSC 52 (screen passthrough) |
| WSL standalone | — | clip.exe |
| WSL + tmux | `on` (default) | tmux buffer + clip.exe |
| WSL + tmux | `external` | tmux buffer + clip.exe + OSC 52 (client TTY) |
| WSL + tmux | `off` | tmux buffer only |
| WSL + screen | — | screen buffer + clip.exe |

## Configuration

Config file location:

| OS | Path |
|---|---|
| Linux | `~/.config/yfix/config.yaml` |
| macOS | `~/Library/Application Support/yfix/config.yaml` |
| Windows | `%APPDATA%\yfix\config.yaml` |

```yaml
fallback_width: 80
transformers:
  strip_ansi: true
  strip_line_numbers: true
  join_wrapped: true
  dedent: true
  strip_trailing: true
  compress_blank: true
  strip_prompt: true
```

All transformers are enabled by default.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
