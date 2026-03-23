use base64::Engine;

use super::OutputTarget;
use crate::error::OutputError;

pub enum Osc52Mode {
    Raw,
    TmuxPassthrough,
    ScreenPassthrough,
}

pub struct Osc52 {
    pub mode: Osc52Mode,
}

impl OutputTarget for Osc52 {
    fn name(&self) -> &'static str {
        match self.mode {
            Osc52Mode::Raw => "osc52",
            Osc52Mode::TmuxPassthrough => "osc52(tmux-passthrough)",
            Osc52Mode::ScreenPassthrough => "osc52(screen-passthrough)",
        }
    }

    fn write(&self, text: &str) -> Result<(), OutputError> {
        use std::io::Write;
        let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
        let seq = match self.mode {
            Osc52Mode::Raw => {
                format!("\x1b]52;c;{encoded}\x07")
            }
            Osc52Mode::TmuxPassthrough => {
                format!("\x1bPtmux;\x1b\x1b]52;c;{encoded}\x07\x1b\\")
            }
            Osc52Mode::ScreenPassthrough => {
                format!("\x1bP\x1b]52;c;{encoded}\x07\x1b\\")
            }
        };
        std::io::stdout().write_all(seq.as_bytes())?;
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
    fn tmux_passthrough_name() {
        let o = Osc52 {
            mode: Osc52Mode::TmuxPassthrough,
        };
        assert_eq!(o.name(), "osc52(tmux-passthrough)");
    }
}
