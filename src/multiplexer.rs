use crate::error::MultiplexerError;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum Multiplexer {
    Tmux,
    Screen,
}

impl Multiplexer {
    /// Pure detection logic — testable without env var side effects.
    pub fn detect_from(tmux_set: bool, sty_set: bool) -> Option<Self> {
        if tmux_set {
            return Some(Multiplexer::Tmux);
        }
        if sty_set {
            return Some(Multiplexer::Screen);
        }
        None
    }

    /// Detect from environment variables.
    pub fn detect() -> Option<Self> {
        Self::detect_from(std::env::var("TMUX").is_ok(), std::env::var("STY").is_ok())
    }

    pub fn get_width(&self) -> Result<usize, MultiplexerError> {
        match self {
            Multiplexer::Tmux => {
                let pane = std::env::var("TMUX_PANE").unwrap_or_default();
                let mut cmd = Command::new("tmux");
                cmd.arg("display-message");
                if !pane.is_empty() {
                    cmd.args(["-t", &pane]);
                }
                cmd.args(["-p", "#{pane_width}"]);
                let out = cmd.output()?;
                let s = String::from_utf8(out.stdout)?;
                s.trim()
                    .parse::<usize>()
                    .map_err(|_| MultiplexerError::CommandFailed("tmux width parse failed".into()))
            }
            Multiplexer::Screen => {
                #[cfg(unix)]
                {
                    let out = Command::new("tput").arg("cols").output()?;
                    let s = String::from_utf8(out.stdout)?;
                    s.trim().parse::<usize>().map_err(|_| {
                        MultiplexerError::CommandFailed("tput cols parse failed".into())
                    })
                }
                #[cfg(not(unix))]
                Err(MultiplexerError::CommandFailed(
                    "screen is not supported on this platform".into(),
                ))
            }
        }
    }

    pub fn read_buffer(&self) -> Result<String, MultiplexerError> {
        match self {
            Multiplexer::Tmux => {
                let out = Command::new("tmux").args(["save-buffer", "-"]).output()?;
                if !out.status.success() {
                    return Err(MultiplexerError::CommandFailed(
                        "tmux save-buffer failed".into(),
                    ));
                }
                Ok(String::from_utf8(out.stdout)?)
            }
            Multiplexer::Screen => {
                let sty = std::env::var("STY").map_err(|_| MultiplexerError::NotInSession)?;
                let tmp = tempfile::NamedTempFile::new()?;
                let tmp_path = tmp
                    .path()
                    .to_str()
                    .ok_or_else(|| {
                        MultiplexerError::CommandFailed(
                            "tmpfile path contains invalid UTF-8".into(),
                        )
                    })?
                    .to_string();
                let status = Command::new("screen")
                    .args(["-S", &sty, "-X", "readbuf", &tmp_path])
                    .status()?;
                if !status.success() {
                    return Err(MultiplexerError::CommandFailed(
                        "screen readbuf failed".into(),
                    ));
                }
                let content = std::fs::read_to_string(tmp.path())?;
                drop(tmp);
                Ok(content)
            }
        }
    }

    pub fn load_buffer(&self, text: &str) -> Result<(), MultiplexerError> {
        match self {
            Multiplexer::Tmux => {
                use std::io::Write;
                let mut child = Command::new("tmux")
                    .args(["load-buffer", "-"])
                    .stdin(std::process::Stdio::piped())
                    .spawn()?;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()?;
                Ok(())
            }
            Multiplexer::Screen => {
                let sty = std::env::var("STY").map_err(|_| MultiplexerError::NotInSession)?;
                let tmp = tempfile::NamedTempFile::new()?;
                std::fs::write(tmp.path(), text)?;
                let tmp_path = tmp
                    .path()
                    .to_str()
                    .ok_or_else(|| {
                        MultiplexerError::CommandFailed(
                            "tmpfile path contains invalid UTF-8".into(),
                        )
                    })?
                    .to_string();
                Command::new("screen")
                    .args(["-S", &sty, "-X", "writebuf", &tmp_path])
                    .status()?;
                Ok(())
            }
        }
    }

    pub fn tmux_set_clipboard() -> String {
        Command::new("tmux")
            .args(["show-option", "-gv", "set-clipboard"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "on".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_tmux_when_tmux_set() {
        assert_eq!(
            Multiplexer::detect_from(true, false),
            Some(Multiplexer::Tmux)
        );
    }

    #[test]
    fn detect_screen_when_sty_set() {
        assert_eq!(
            Multiplexer::detect_from(false, true),
            Some(Multiplexer::Screen)
        );
    }

    #[test]
    fn detect_tmux_takes_priority_over_screen() {
        assert_eq!(
            Multiplexer::detect_from(true, true),
            Some(Multiplexer::Tmux)
        );
    }

    #[test]
    fn detect_none_when_neither_set() {
        assert_eq!(Multiplexer::detect_from(false, false), None);
    }
}
