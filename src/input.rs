use crate::config::Config;
use crate::multiplexer::Multiplexer;

#[derive(Debug)]
pub enum WidthSource {
    CliFlag,
    Columns,
    TmuxPane,
    Ioctl,
    TputCols,
    ConfigFallback,
}

impl std::fmt::Display for WidthSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WidthSource::CliFlag => write!(f, "CliFlag"),
            WidthSource::Columns => write!(f, "Columns"),
            WidthSource::TmuxPane => write!(f, "TmuxPane"),
            WidthSource::Ioctl => write!(f, "Ioctl"),
            WidthSource::TputCols => write!(f, "TputCols"),
            WidthSource::ConfigFallback => write!(f, "ConfigFallback"),
        }
    }
}

/// Pure width resolution logic — no side effects.
/// Candidates are checked in priority order; first Some wins.
pub fn resolve_width_from(
    cli_width: Option<usize>,
    columns: Option<usize>,
    tmux_width: Option<usize>,
    terminal_width: Option<usize>,
    tput_width: Option<usize>,
    fallback_width: usize,
) -> (usize, WidthSource) {
    if let Some(w) = cli_width {
        return (w, WidthSource::CliFlag);
    }
    if let Some(w) = columns {
        return (w, WidthSource::Columns);
    }
    if let Some(w) = tmux_width {
        return (w, WidthSource::TmuxPane);
    }
    if let Some(w) = terminal_width {
        return (w, WidthSource::Ioctl);
    }
    if let Some(w) = tput_width {
        return (w, WidthSource::TputCols);
    }
    (fallback_width, WidthSource::ConfigFallback)
}

/// Resolve width by probing the environment.
pub fn resolve_width(cli_width: Option<usize>, config: &Config) -> (usize, WidthSource) {
    let columns = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok());

    let tmux_width = if std::env::var("TMUX").is_ok() {
        Multiplexer::Tmux.get_width().ok()
    } else {
        None
    };

    let terminal_width = ioctl_width();

    #[cfg(unix)]
    let tput = tput_width_cmd();
    #[cfg(not(unix))]
    let tput: Option<usize> = None;

    resolve_width_from(
        cli_width,
        columns,
        tmux_width,
        terminal_width,
        tput,
        config.fallback_width,
    )
}

fn ioctl_width() -> Option<usize> {
    terminal_size::terminal_size().map(|(w, _)| w.0 as usize)
}

#[cfg(unix)]
fn tput_width_cmd() -> Option<usize> {
    use std::process::Command;
    Command::new("tput")
        .arg("cols")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|s| s.trim().parse::<usize>().ok())
}

pub fn resolve_input(cli_text: Option<String>) -> anyhow::Result<String> {
    if let Some(text) = cli_text {
        return Ok(text);
    }

    use std::io::Read;
    let stdin = std::io::stdin();
    if !is_tty() {
        let mut buf = String::new();
        stdin.lock().read_to_string(&mut buf)?;
        return Ok(buf);
    }

    match Multiplexer::detect() {
        Some(mux) => Ok(mux.read_buffer()?),
        None => anyhow::bail!("No input: provide text, pipe stdin, or run inside tmux/screen"),
    }
}

fn is_tty() -> bool {
    use is_terminal::IsTerminal;
    std::io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_flag_takes_priority() {
        let (w, src) = resolve_width_from(Some(132), Some(80), Some(90), Some(100), Some(110), 80);
        assert_eq!(w, 132);
        assert!(matches!(src, WidthSource::CliFlag));
    }

    #[test]
    fn columns_second_priority() {
        let (w, src) = resolve_width_from(None, Some(99), Some(90), Some(100), Some(110), 80);
        assert_eq!(w, 99);
        assert!(matches!(src, WidthSource::Columns));
    }

    #[test]
    fn tmux_third_priority() {
        let (w, src) = resolve_width_from(None, None, Some(90), Some(100), Some(110), 80);
        assert_eq!(w, 90);
        assert!(matches!(src, WidthSource::TmuxPane));
    }

    #[test]
    fn ioctl_fourth_priority() {
        let (w, src) = resolve_width_from(None, None, None, Some(100), Some(110), 80);
        assert_eq!(w, 100);
        assert!(matches!(src, WidthSource::Ioctl));
    }

    #[test]
    fn tput_fifth_priority() {
        let (w, src) = resolve_width_from(None, None, None, None, Some(110), 80);
        assert_eq!(w, 110);
        assert!(matches!(src, WidthSource::TputCols));
    }

    #[test]
    fn fallback_last() {
        let (w, src) = resolve_width_from(None, None, None, None, None, 80);
        assert_eq!(w, 80);
        assert!(matches!(src, WidthSource::ConfigFallback));
    }
}
