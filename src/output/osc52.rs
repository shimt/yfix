use std::fs;
use std::io::Write;
use std::process::Command;

use base64::Engine;

use super::OutputTarget;
use crate::error::OutputError;

pub enum Osc52Mode {
    Raw,
    TmuxClientTty,
    ScreenPassthrough,
}

pub struct Osc52 {
    pub mode: Osc52Mode,
}

fn raw_osc52(encoded: &str) -> String {
    format!("\x1b]52;c;{encoded}\x07")
}

fn tmux_client_tty() -> Result<String, OutputError> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{client_tty}"])
        .output()
        .map_err(OutputError::Io)?;
    let tty = String::from_utf8(output.stdout)
        .map_err(|_| OutputError::Clipboard("failed to parse client_tty".into()))?;
    let tty = tty.trim().to_string();
    if tty.is_empty() {
        return Err(OutputError::Clipboard("tmux client_tty is empty".into()));
    }
    Ok(tty)
}

impl OutputTarget for Osc52 {
    fn name(&self) -> &'static str {
        match self.mode {
            Osc52Mode::Raw => "osc52",
            Osc52Mode::TmuxClientTty => "osc52(tmux-client-tty)",
            Osc52Mode::ScreenPassthrough => "osc52(screen-passthrough)",
        }
    }

    fn write(&self, text: &str) -> Result<(), OutputError> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
        let seq = raw_osc52(&encoded);

        match self.mode {
            Osc52Mode::Raw => {
                std::io::stdout().write_all(seq.as_bytes())?;
            }
            Osc52Mode::TmuxClientTty => {
                let tty = tmux_client_tty()?;
                let mut file = fs::OpenOptions::new().write(true).open(&tty)?;
                file.write_all(seq.as_bytes())?;
            }
            Osc52Mode::ScreenPassthrough => {
                let seq = format!("\x1bP\x1b]52;c;{encoded}\x07\x1b\\");
                std::io::stdout().write_all(seq.as_bytes())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_format_is_correct() {
        let o = Osc52 {
            mode: Osc52Mode::Raw,
        };
        assert_eq!(o.name(), "osc52");
    }

    #[test]
    fn tmux_client_tty_name() {
        let o = Osc52 {
            mode: Osc52Mode::TmuxClientTty,
        };
        assert_eq!(o.name(), "osc52(tmux-client-tty)");
    }
}
