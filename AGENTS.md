# yfix — Agent Instructions

Shared rules for all AI agents working on this project. Tool-specific instructions are in:
- Claude Code: `CLAUDE.md`
- GitHub Copilot: `.github/copilot-instructions.md`

# Target environments

- **OS**: macOS, Linux, Windows (WSL), Windows (native)
- **Multiplexer**: tmux, screen, standalone (no multiplexer)
- **Remote access**: SSH, mosh
- **Shell**: bash, zsh, fish

# Commands

```sh
cargo build                          # build
cargo test                           # all tests (unit + integration)
cargo test <filter>                  # run specific tests (e.g., join_wrapped, strip_line_numbers)
cargo test --test integration        # integration tests only
cargo install --path .               # install to ~/.cargo/bin/yfix
cargo build --release                # release build
```

# Architecture

CLI tool that cleans terminal-copied text and writes to clipboard/tmux/osc52.

```
src/
  main.rs              CLI entry point (clap), --oops, --show-terminal, --help-ai
  lib.rs               pub mod re-exports
  config.rs            Config struct, YAML loading, debug_flag_path()
  error.rs             TransformerError, OutputError, MultiplexerError
  input.rs             resolve_width() (--width > $COLUMNS > tmux > ioctl > tput > config),
                       resolve_input() (cli arg > stdin > mux buffer)
  processor.rs         Processor: applies transformers in order; ProcessResult with trace+warnings
  multiplexer.rs       tmux/screen: width query, buffer read/write
  debug_log.rs         Log file I/O, --oops flag support, LogEntry, sequential IDs
  transformer/
    mod.rs             Transformer trait, Warning enum, TransformDiagnostics
    strip_ansi.rs      Strip ANSI escapes (anstream)
    strip_line_numbers.rs  Strip line numbers (>50% threshold, regex)
    join_wrapped.rs    Join terminal-wrapped lines (width threshold + relaxed continuation)
    dedent.rs          Remove common indent (line 1 included if >= min_indent)
    strip_trailing.rs  Remove trailing whitespace + blank lines
    compress_blank.rs  Compress 3+ blank lines to 1
    strip_prompt.rs    Remove bare prompt lines (❯ $ % > >>>)
  output/
    mod.rs             OutputTarget trait, Environment (SSH/mosh detect via process tree, WSL detect), auto_targets()
    stdout.rs          Write to stdout
    os_clipboard.rs    arboard clipboard
    wsl_clipboard.rs   WSL clipboard via clip.exe
    tmux_buffer.rs     tmux load-buffer
    screen_buffer.rs   screen writebuf
    osc52.rs           OSC 52 (raw / tmux-client-tty / screen-passthrough)
tests/
  integration.rs       CLI integration tests (5 tests)
docs/
  help-ai.md           AI integration guide (included via include_str!, no runtime paths)
```

## Key design decisions

- **Transformer pipeline order**: strip_ansi → strip_line_numbers → join_wrapped → dedent → strip_trailing → compress_blank → strip_prompt
- **SSH/mosh detection in tmux**: walks tmux client process tree (`ps -o comm= / ppid=`) looking for `sshd`, `sshd-*`, or `mosh-server` — avoids stale session env vars
- **OSC 52 in tmux**: writes raw OSC 52 directly to client TTY (`#{client_tty}`), not DCS passthrough — mosh filters DCS but passes raw OSC 52
- **WSL detection**: `/proc/version` contains "microsoft" → uses `clip.exe` instead of arboard
- **JoinWrapped**: strict threshold (`wrap_width - 2`) to start, relaxed (`wrap_width / 2`) for continuations. Space insertion uses CJK character detection (Unicode block ranges), not display width
- **Debug mode**: toggled by file existence (`{config_dir}/yfix/debug`), logs to `debug.log`, clipboard stays clean
- **`--help-ai`**: static markdown via `include_str!` + runtime paths appended in `print_help_ai()`

# Conventions

- Language: Rust 2021
- Code style: `cargo fmt` (default rustfmt)
- Documentation language: English (source, docs, comments, commit messages)
- Tests: inline `#[cfg(test)] mod tests` in each module
- New transformer: implement `Transformer` trait, add to `transformer/mod.rs` and `Processor::from_config()`
- Diagnostics: override `transform_with_diagnostics()` only for transformers that can generate warnings
- Knowledge placement: environment-independent items in AGENTS.md / docs/help-ai.md, environment-dependent items (build setup, test hosts, local paths) in project memory
- `docs/help-ai.md`: user-facing guide for AI agents (Claude Code, Gemini, etc.) to learn how to use yfix. Accessed via `yfix --help-ai`. Write for an AI reader, not a human developer.

# Communication conventions

- [Conventional Comments](https://conventionalcomments.org/) in Claude responses and PR reviews
- Labels: `suggestion:`, `issue:`, `praise:`, `nitpick:`, `question:`, `todo:`, `note:`
- Decorations: `(non-blocking)`, `(blocking)`, `(if-minor)` as needed

# Commit conventions

- Style: [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, `test:`, `refactor:`)
- Versioning: [Semantic Versioning](https://semver.org/) (post v1.0.0)
- Branching: [GitHub Flow](https://docs.github.com/en/get-started/using-github/github-flow) (post-release)
- License: MIT OR Apache-2.0
- Fix commits: squash into the relevant feat commit before merging
- Use `git commit --fixup <SHA>` + `GIT_SEQUENCE_EDITOR=: git rebase --autosquash <base>` for non-interactive squash
- **PR title and body must be written in English**

## Strategy: Rebase + Fixup

### Rules

1. **Before amend/fixup, always run `git diff --cached --stat`**: verify all staged files serve the same purpose. If not, split into separate commits
2. **Unrelated changes get their own commit**: typo fixes, unrelated refactors, etc. Exception: `cargo fmt` output goes with the source commit
3. **When in doubt, create a new commit** over amend — splitting later is harder than squashing
4. **Docs alongside code, but separate commits**: `docs/help-ai.md` is source (include_str!) — goes with the source commit. `CLAUDE.md` gets independent commits
5. **After committing, check TODO.md** (see project memory for location): close resolved items. Propose new items to user when issues are found
6. **PR cleanup**: `git rebase --autosquash` to consolidate, verify each commit builds and tests independently

# Development workflow

- **After Write/Edit**: run `cargo fmt` immediately to keep formatting consistent
- **After implementation**: run `cargo check` to catch compile errors early
- **Clippy findings**: fix manually, do not use `cargo clippy --fix` (may introduce unintended changes)

## Bump version checklist

When bumping the version, update **all** of the following:

1. `Cargo.toml` — `version = "x.y.z"`
2. `cargo build` (or `cargo check`) — updates `Cargo.lock`
3. `Formula/yfix.rb` — `version "x.y.z"` and sha256 hashes

> **Note**: sha256 hashes in `Formula/yfix.rb` are auto-updated by the `update-formula` job in `release.yml` after each tagged release. Manual update is only needed when testing the formula before tagging.

# Pre-commit

```sh
cargo test                                    # 1. correctness
cargo clippy --tests -- -D warnings           # 2. lint (detect only, no --fix)
cargo fmt --check                             # 3. verify formatting (should be no-op)
brew style Formula/yfix.rb                    # 4. formula style (macOS only, skip if brew unavailable)
```

## Cross-build verification

Run when changes touch `#[cfg]` guards, platform-specific code, `Cargo.toml` dependencies, or output/input modules. Skip for transformer-only or docs-only changes.

```sh
# Only if `cross` is installed (check: which cross)
cross build --target x86_64-unknown-linux-gnu 2>&1 | tail -3
cross build --target x86_64-pc-windows-gnu 2>&1 | tail -3
```

Targets: Linux x86_64, Windows x86_64. Verify zero warnings.

# Feedback process

See `docs/help-ai.md` for details (workflow, log format, analysis commands, warnings, known limitations).

# Gotchas

- **Error output policy**: stderr only when `isatty(stderr)`. Never print to stdout/stderr unconditionally — AI agents use `2>&1` which pollutes pipelines. Use `maybe_eprintln()` in main.rs, log errors to debug.log via `log_error()`.
- **mosh + tmux OSC 52**: DCS passthrough does not work through mosh. yfix writes raw OSC 52 directly to tmux client TTY to bypass this.
- **`anstream` StripStr API**: use `StripStr::new()` + `strip_next()`, NOT `StripStr::from()`
- **tmux `display-message` arg order**: `-t <pane>` must come BEFORE `-p <format>`, otherwise "too many arguments" error
- **Debug mode and tests**: `YFIX_DEBUG_OVERRIDE=off|on` overrides the debug flag file. Integration tests set `off` to avoid interference
- **`include_str!` and `{}`**: `println!(include_str!(...))` fails if the file contains `{foo}` — use `print!("{}", include_str!(...))` instead
- **`cli.text` partial move**: `resolve_input(cli.text)` moves `text` out of `cli`. Use `.clone()` if `cli` is referenced later
