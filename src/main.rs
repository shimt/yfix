use anyhow::Context;
use clap::Parser;
use std::os::unix::io::AsFd;
use std::path::PathBuf;
use std::process;

use yfix::{
    config::{self, Config},
    debug_log,
    input::{resolve_input, resolve_width, WidthSource},
    multiplexer::Multiplexer,
    output::{
        os_clipboard::OsClipboard,
        osc52::{Osc52, Osc52Mode},
        screen_buffer::ScreenBuffer,
        stdout::Stdout,
        tmux_buffer::TmuxBuffer,
        Environment, OutputTarget,
    },
    processor::Processor,
};

#[derive(Parser, Debug)]
#[command(name = "yfix", about = "Clean and copy terminal text")]
struct Cli {
    /// Text to clean (highest priority input source)
    text: Option<String>,

    /// Output targets: stdout, os-clipboard, tmux, osc52 (comma-separated)
    #[arg(long)]
    output: Option<String>,

    /// Auto-detect output targets (default behavior, ignored if --output is set)
    #[arg(long)]
    auto: bool,

    /// Terminal pane width for unwrap (e.g., pass #{pane_width} from tmux)
    #[arg(long)]
    width: Option<usize>,

    /// Override config file path
    #[arg(long)]
    config: Option<PathBuf>,

    /// Print environment detection results to stderr
    #[arg(long)]
    show_terminal: bool,

    /// Print AI integration guide in Markdown to stdout
    #[arg(long)]
    help_ai: bool,

    /// Flag last debug log entry as problematic (optionally with comment)
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    oops: Option<String>,
}

/// Whether stderr is a TTY (safe to print errors)
fn stderr_is_tty() -> bool {
    rustix::termios::isatty(std::io::stderr().as_fd())
}

/// Print error to stderr only if it's a TTY
fn maybe_eprintln(msg: &str) {
    if stderr_is_tty() {
        eprintln!("{msg}");
    }
}

/// Log error to debug.log if debug mode is enabled
fn log_error(
    debug: bool,
    version: &str,
    width: usize,
    width_source: &str,
    is_ssh: bool,
    error_msg: &str,
) {
    if !debug {
        return;
    }
    if let Some(log_path) = debug_log::debug_log_path() {
        let entry = debug_log::build_error_entry(
            version,
            width,
            width_source.to_string(),
            is_ssh,
            error_msg,
            &log_path,
        );
        let _ = debug_log::write_entry(&log_path, &entry);
    }
}

fn main() {
    let exit_code = match run() {
        Ok(code) => code,
        Err(e) => {
            maybe_eprintln(&format!("yfix: error: {e:#}"));
            1
        }
    };
    if exit_code != 0 {
        process::exit(exit_code);
    }
}

fn run() -> anyhow::Result<i32> {
    let cli = Cli::parse();

    if cli.help_ai {
        print_help_ai();
        return Ok(0);
    }

    // --oops: always interactive (user runs manually), stderr OK
    if let Some(ref comment) = cli.oops {
        let log_path = debug_log::debug_log_path()
            .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
        let comment = if comment.is_empty() {
            None
        } else {
            Some(comment.as_str())
        };
        debug_log::flag_last_entry(&log_path, comment)?;
        return Ok(0);
    }

    let config = Config::load(cli.config.as_ref()).context("failed to load config")?;

    let (wrap_width, width_source) = resolve_width(cli.width, &config);

    let env = Environment::detect();

    // --show-terminal: explicitly requested, stderr OK
    if cli.show_terminal {
        print_show_terminal(&env, wrap_width, &width_source);
        if cli.text.is_none() {
            return Ok(0);
        }
    }

    let debug = match std::env::var("YFIX_DEBUG_OVERRIDE").as_deref() {
        Ok("on") => true,
        Ok("off") => false,
        _ => config::debug_flag_path()
            .map(|p| p.exists())
            .unwrap_or(false),
    };

    let width_source_str = format!("{:?}", width_source);

    let input = match resolve_input(cli.text.clone()) {
        Ok(input) => input,
        Err(e) => {
            let msg = format!("failed to read input: {e:#}");
            maybe_eprintln(&format!("yfix: {msg}"));
            log_error(
                debug,
                env!("YFIX_VERSION"),
                wrap_width,
                &width_source_str,
                env.is_ssh,
                &msg,
            );
            return Ok(1);
        }
    };

    if input.trim().is_empty() {
        let targets = resolve_targets(&cli, &env);
        return Ok(write_to_targets(
            &targets,
            "",
            debug,
            wrap_width,
            &width_source_str,
            env.is_ssh,
        ));
    }

    let processor = Processor::from_config(&config, wrap_width);

    let output_text = if debug {
        let result = match processor.process_with_trace(&input) {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("transform failed: {e}");
                maybe_eprintln(&format!("yfix: {msg}"));
                log_error(
                    debug,
                    env!("YFIX_VERSION"),
                    wrap_width,
                    &width_source_str,
                    env.is_ssh,
                    &msg,
                );
                return Ok(1);
            }
        };

        if let Some(log_path) = debug_log::debug_log_path() {
            let target_names: Vec<String> = resolve_targets(&cli, &env)
                .iter()
                .map(|t| t.name().to_string())
                .collect();
            let entry = debug_log::build_trace_entry(
                env!("YFIX_VERSION"),
                wrap_width,
                width_source_str.clone(),
                env.is_ssh,
                target_names,
                &input,
                result.trace,
                result.warnings,
                &log_path,
            );
            let _ = debug_log::write_entry(&log_path, &entry);
        }

        result.text
    } else {
        match processor.process(&input) {
            Ok(text) => text,
            Err(e) => {
                maybe_eprintln(&format!("yfix: transform failed: {e}"));
                return Ok(1);
            }
        }
    };

    let targets = resolve_targets(&cli, &env);
    let exit_code = write_to_targets(
        &targets,
        &output_text,
        debug,
        wrap_width,
        &width_source_str,
        env.is_ssh,
    );
    Ok(exit_code)
}

fn resolve_targets(cli: &Cli, env: &Environment) -> Vec<Box<dyn OutputTarget>> {
    if let Some(ref spec) = cli.output {
        parse_output_spec(spec, env)
    } else {
        env.auto_targets()
    }
}

fn parse_output_spec(spec: &str, env: &Environment) -> Vec<Box<dyn OutputTarget>> {
    use yfix::multiplexer::Multiplexer as Mux;
    let mut targets: Vec<Box<dyn OutputTarget>> = Vec::new();
    for part in spec.split(',') {
        match part.trim() {
            "stdout" => targets.push(Box::new(Stdout)),
            "os-clipboard" => targets.push(Box::new(OsClipboard)),
            "tmux" => targets.push(Box::new(TmuxBuffer)),
            "screen" => targets.push(Box::new(ScreenBuffer)),
            "osc52" => {
                let mode = match env.multiplexer {
                    Some(Mux::Tmux) => Osc52Mode::TmuxPassthrough,
                    Some(Mux::Screen) => Osc52Mode::ScreenPassthrough,
                    None => Osc52Mode::Raw,
                };
                targets.push(Box::new(Osc52 { mode }));
            }
            other => maybe_eprintln(&format!("yfix: unknown output target '{other}', skipping")),
        }
    }
    targets
}

fn write_to_targets(
    targets: &[Box<dyn OutputTarget>],
    text: &str,
    debug: bool,
    width: usize,
    width_source: &str,
    is_ssh: bool,
) -> i32 {
    let mut had_error = false;
    for target in targets {
        if let Err(e) = target.write(text) {
            let msg = format!("failed to write to {}: {e}", target.name());
            maybe_eprintln(&format!("yfix: {msg}"));
            log_error(
                debug,
                env!("YFIX_VERSION"),
                width,
                width_source,
                is_ssh,
                &msg,
            );
            had_error = true;
        }
    }
    if had_error {
        2
    } else {
        0
    }
}

fn print_show_terminal(env: &Environment, wrap_width: usize, width_source: &WidthSource) {
    let tmux = std::env::var("TMUX").unwrap_or_else(|_| "(not set)".into());
    let tmux_pane = std::env::var("TMUX_PANE").unwrap_or_else(|_| "(not set)".into());
    let sty = std::env::var("STY").unwrap_or_else(|_| "(not set)".into());
    let byobu = std::env::var("BYOBU_BACKEND").unwrap_or_else(|_| "(not set)".into());
    let ssh_client = std::env::var("SSH_CLIENT").unwrap_or_else(|_| "(not set)".into());
    let ssh_tty = std::env::var("SSH_TTY").unwrap_or_else(|_| "(not set)".into());
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_else(|_| "(not set)".into());
    let columns = std::env::var("COLUMNS").unwrap_or_else(|_| "(not set)".into());

    eprintln!("[yfix] env:");
    eprintln!("  TMUX          = {tmux}");
    eprintln!("  TMUX_PANE     = {tmux_pane}");
    eprintln!("  STY           = {sty}");
    eprintln!("  BYOBU_BACKEND = {byobu}");
    eprintln!("  SSH_CLIENT    = {ssh_client}");
    eprintln!("  SSH_TTY       = {ssh_tty}");
    eprintln!("  TERM_PROGRAM  = {term_program}");
    eprintln!("  COLUMNS       = {columns}");

    if matches!(env.multiplexer, Some(Multiplexer::Tmux)) {
        let client_pid = std::process::Command::new("tmux")
            .args(["display-message", "-p", "#{client_pid}"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "?".into());
        eprintln!("[yfix] tmux: set-clipboard={}", env.set_clipboard);
        eprintln!("[yfix] tmux: client_pid={client_pid}");
    }

    eprintln!("[yfix] ssh: {}", env.is_ssh);

    let src_label = match width_source {
        WidthSource::CliFlag => "from --width",
        WidthSource::Columns => "from $COLUMNS",
        WidthSource::TmuxPane => "from tmux pane",
        WidthSource::Ioctl => "from terminal ioctl",
        WidthSource::TputCols => "from tput cols",
        WidthSource::ConfigFallback => "from config fallback_width",
    };
    eprintln!("[yfix] width: {wrap_width} ({src_label})");

    let targets = env.auto_targets();
    let names: Vec<&str> = targets.iter().map(|t| t.name()).collect();
    eprintln!("[yfix] output: {}", names.join(", "));
}

fn print_help_ai() {
    print!("{}", include_str!("../docs/help-ai.md"));

    println!("\n## Runtime info (this system)");
    println!("- Version: {}", env!("YFIX_VERSION"));
    let config_path = config::default_config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown)".into());
    let debug_flag = config::debug_flag_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown)".into());
    let debug_log = debug_log::debug_log_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown)".into());
    println!("- Config: {config_path}");
    println!("- Debug flag: {debug_flag}");
    println!("- Debug log: {debug_log}");
}
