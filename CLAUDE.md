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
    mod.rs             OutputTarget trait, Environment (SSH detect via process tree), auto_targets()
    stdout.rs          Write to stdout
    os_clipboard.rs    arboard clipboard
    tmux_buffer.rs     tmux load-buffer
    screen_buffer.rs   screen writebuf
    osc52.rs           OSC 52 (raw / tmux-passthrough / screen-passthrough)
tests/
  integration.rs       CLI integration tests (5 tests)
docs/
  help-ai.md           AI integration guide (included via include_str!, no runtime paths)
```

## Key design decisions

- **Transformer pipeline order**: strip_ansi → strip_line_numbers → join_wrapped → dedent → strip_trailing → compress_blank → strip_prompt
- **SSH detection in tmux**: walks tmux client process tree (`ps -o comm= / ppid=`) looking for `sshd` or `sshd-*` — avoids stale session env vars
- **JoinWrapped**: strict threshold (`wrap_width - 2`) to start, relaxed (`wrap_width / 2`) for continuations. Space insertion uses CJK character detection (Unicode block ranges), not display width
- **Debug mode**: toggled by file existence (`{config_dir}/yfix/debug`), logs to `debug.log`, clipboard stays clean
- **`--help-ai`**: static markdown via `include_str!` + runtime paths appended in `print_help_ai()`

# Conventions

- Language: Rust 2021
- Code style: `cargo fmt` (default rustfmt)
- Documentation language: English in code, Japanese in user-facing docs/help
- Naming: snake_case for functions/modules, PascalCase for types/traits
- Tests: inline `#[cfg(test)] mod tests` in each module
- New transformer: implement `Transformer` trait, add to `transformer/mod.rs` and `Processor::from_config()`
- Diagnostics: override `transform_with_diagnostics()` only for transformers that can generate warnings

# Communication conventions

- [Conventional Comments](https://conventionalcomments.org/) in Claude responses and PR reviews
- Labels: `suggestion:`, `issue:`, `praise:`, `nitpick:`, `question:`, `todo:`, `note:`
- Decorations: `(non-blocking)`, `(blocking)`, `(if-minor)` as needed

# Commit conventions

- Style: [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, `test:`, `refactor:`)
- Branch: `feat/yfix-implementation` (current development), `main` (stable)
- Fix commits: squash into the relevant feat commit before merging
- Use `git commit --fixup <SHA>` + `GIT_SEQUENCE_EDITOR=: git rebase --autosquash <base>` for non-interactive squash

## Strategy: Rebase + Fixup (worktree内の開発)

開発中は細かくコミットし、PR前に `fixup` + `autosquash` で論理単位に整理する。

### Rules

1. **1コミット = 1目的**: feat, fix, refactor, chore, test を混ぜない
2. **amend/fixup する前に必ず `git diff --cached --stat` で確認**: 変更されるファイルが全て同じ目的か検証する。少しでも異なる目的の変更が含まれていたら、別コミットに分離する
3. **「ついでに」の修正は別コミット**: 本来の作業と異なる修正（typo 修正、無関係なリファクタ等）は必ず別コミットにする。ただし pre-commit で実行する `cargo fmt` / `cargo clippy --fix` の変更は本来の作業に含めてよい（同じコミットにステージする）
4. **amend は直前コミットと完全に同じ目的の場合のみ**: 迷ったら新しいコミットを作る。amend で混入させると `git reset HEAD~1` での分離が必要になり手間が増える
5. **PR前の整理**: `git rebase --autosquash` で fixup コミットを統合し、各コミットが独立してビルド・テスト通過することを確認する
6. **ドキュメントを後回しにしない**: 機能追加・変更時は `docs/help-ai.md` と `CLAUDE.md` の該当箇所も必ず更新する。`docs/help-ai.md` は `include_str!` でバイナリに埋め込まれるためソースの一部 — ソースコードと同じコミットに含める。CLAUDE.md は独立コミット（後で1つにまとめやすいよう）
7. **コミット後に TODO.md をチェック**: コミットした変更で TODO の項目が解決していれば `- [x]` に更新する。TODO.md の更新は独立コミットにする
8. **問題を見つけたら TODO に提案**: 実装・ドキュメントの問題や改善点に気づいたら、ユーザーに確認の上 TODO.md に追加する

# Development workflow

- **After Write/Edit**: run `cargo fmt` immediately to keep formatting consistent
- **After implementation**: run `cargo check` to catch compile errors early
- **Clippy findings**: fix manually, do not use `cargo clippy --fix` (may introduce unintended changes)

# Pre-commit

```sh
cargo test                                    # 1. correctness
cargo clippy --tests -- -D warnings           # 2. lint (detect only, no --fix)
cargo fmt --check                             # 3. verify formatting (should be no-op)
```

# Feedback process

Debug/feedback の詳細（ワークフロー、ログフォーマット、分析コマンド、warnings、known limitations）は `docs/help-ai.md` を参照。

# Gotchas

- **Error output policy**: stderr only when `isatty(stderr)`. Never print to stdout/stderr unconditionally — AI agents use `2>&1` which pollutes pipelines. Use `maybe_eprintln()` in main.rs, log errors to debug.log via `log_error()`.
- **`anstream` StripStr API**: use `StripStr::new()` + `strip_next()`, NOT `StripStr::from()`
- **tmux `display-message` arg order**: `-t <pane>` must come BEFORE `-p <format>`, otherwise "too many arguments" error
- **Debug mode and tests**: `YFIX_DEBUG_OVERRIDE=off|on` overrides the debug flag file. Integration tests set `off` to avoid interference. User can set `on` to force debug without the flag file
- **`include_str!` and `{}`**: `println!(include_str!(...))` fails if the file contains `{foo}` — use `print!("{}", include_str!(...))` instead
- **`cli.text` partial move**: `resolve_input(cli.text)` moves `text` out of `cli`. Use `.clone()` if `cli` is referenced later
