use crate::config::Config;
use crate::multiplexer::Multiplexer;
use std::process::Command;

#[derive(Debug)]
pub enum WidthSource {
    CliFlag,
    Columns,
    TmuxPane,
    Ioctl,
    TputCols,
    ConfigFallback,
}

pub fn resolve_width(cli_width: Option<usize>, config: &Config) -> (usize, WidthSource) {
    if let Some(w) = cli_width {
        return (w, WidthSource::CliFlag);
    }

    if let Ok(s) = std::env::var("COLUMNS") {
        if let Ok(w) = s.trim().parse::<usize>() {
            return (w, WidthSource::Columns);
        }
    }

    if std::env::var("TMUX").is_ok() {
        if let Ok(w) = Multiplexer::Tmux.get_width() {
            return (w, WidthSource::TmuxPane);
        }
    }

    if let Some(w) = ioctl_width() {
        return (w, WidthSource::Ioctl);
    }

    if let Ok(out) = Command::new("tput").arg("cols").output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(w) = s.trim().parse::<usize>() {
                return (w, WidthSource::TputCols);
            }
        }
    }

    (config.fallback_width, WidthSource::ConfigFallback)
}

fn ioctl_width() -> Option<usize> {
    terminal_size::terminal_size().map(|(w, _)| w.0 as usize)
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
    use crate::config::Config;

    #[test]
    fn cli_flag_takes_priority() {
        let config = Config::default();
        let (w, src) = resolve_width(Some(132), &config);
        assert_eq!(w, 132);
        assert!(matches!(src, WidthSource::CliFlag));
    }

    #[test]
    fn falls_back_to_config() {
        let config = Config {
            fallback_width: 120,
            ..Config::default()
        };
        let (w, _) = resolve_width(None, &config);
        assert!(w > 0);
    }
}
