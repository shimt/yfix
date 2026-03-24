pub mod os_clipboard;
pub mod osc52;
pub mod screen_buffer;
pub mod stdout;
pub mod tmux_buffer;
pub mod wsl_clipboard;

use crate::error::OutputError;
use crate::multiplexer::Multiplexer;

/// Detect SSH session.
/// 1. Check process env vars (SSH_CLIENT, SSH_TTY) — works outside multiplexers
/// 2. Inside tmux: get the current client PID and walk the process tree looking
///    for sshd. This correctly handles attach/detach: only the current client's
///    ancestry matters, not stale session environment variables.
fn detect_ssh(multiplexer: &Option<Multiplexer>) -> bool {
    if std::env::var("SSH_CLIENT").is_ok() || std::env::var("SSH_TTY").is_ok() {
        return true;
    }

    if matches!(multiplexer, Some(Multiplexer::Tmux)) {
        if let Some(client_pid) = tmux_client_pid() {
            return has_remote_ancestor(client_pid);
        }
    } else {
        // Outside tmux: walk the current process tree to detect sshd/mosh-server.
        // Note: for detached screen sessions, the ancestry traces to PID 1,
        // so mosh-server won't be found here. SSH_CLIENT/SSH_TTY above covers
        // the SSH case; standalone mosh inside detached screen is undetectable.
        if has_remote_ancestor(std::process::id()) {
            return true;
        }
    }

    false
}

fn tmux_client_pid() -> Option<u32> {
    use std::process::Command;
    Command::new("tmux")
        .args(["display-message", "-p", "#{client_pid}"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok()?.trim().parse::<u32>().ok())
}

/// Check if any ancestor process is a remote access daemon (sshd, mosh-server).
fn has_remote_ancestor(start_pid: u32) -> bool {
    let mut pid = start_pid;
    while pid > 1 {
        if let Some(name) = process_name(pid) {
            if name == "sshd" || name.starts_with("sshd-") || name.contains("mosh-server") {
                return true;
            }
        }
        match parent_pid(pid) {
            Some(ppid) if ppid != pid => pid = ppid,
            _ => break,
        }
    }
    false
}

#[cfg(unix)]
fn process_name(pid: u32) -> Option<String> {
    use std::process::Command;
    Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(not(unix))]
fn process_name(_pid: u32) -> Option<String> {
    None
}

#[cfg(unix)]
fn parent_pid(pid: u32) -> Option<u32> {
    use std::process::Command;
    Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
}

#[cfg(not(unix))]
fn parent_pid(_pid: u32) -> Option<u32> {
    None
}

pub trait OutputTarget {
    fn write(&self, text: &str) -> Result<(), OutputError>;
    fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub struct Environment {
    pub multiplexer: Option<Multiplexer>,
    pub is_ssh: bool,
    pub is_wsl: bool,
    pub set_clipboard: String,
}

fn detect_wsl() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/version")
            .map(|s| s.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

impl Environment {
    pub fn detect() -> Self {
        let multiplexer = Multiplexer::detect();
        let is_ssh = detect_ssh(&multiplexer);
        let is_wsl = detect_wsl();
        let set_clipboard = if matches!(multiplexer, Some(Multiplexer::Tmux)) {
            Multiplexer::tmux_set_clipboard()
        } else {
            "on".to_string()
        };
        Self {
            multiplexer,
            is_ssh,
            is_wsl,
            set_clipboard,
        }
    }

    /// Select the appropriate clipboard target for the platform.
    fn clipboard_target(&self) -> Box<dyn OutputTarget> {
        if self.is_wsl {
            Box::new(wsl_clipboard::WslClipboard)
        } else {
            Box::new(os_clipboard::OsClipboard)
        }
    }

    pub fn auto_targets(&self) -> Vec<Box<dyn OutputTarget>> {
        use osc52::{Osc52, Osc52Mode};
        let mut targets: Vec<Box<dyn OutputTarget>> = Vec::new();

        match (&self.multiplexer, self.is_ssh, self.set_clipboard.as_str()) {
            (Some(Multiplexer::Tmux), false, "on") => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
                targets.push(self.clipboard_target());
            }
            (Some(Multiplexer::Tmux), false, "external") => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
                targets.push(self.clipboard_target());
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::TmuxClientTty,
                }));
            }
            (Some(Multiplexer::Tmux), false, _) => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
            }
            (Some(Multiplexer::Tmux), true, "on" | "external") => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::TmuxClientTty,
                }));
            }
            (Some(Multiplexer::Tmux), true, _) => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
            }
            (Some(Multiplexer::Screen), false, _) => {
                targets.push(Box::new(screen_buffer::ScreenBuffer));
                targets.push(self.clipboard_target());
            }
            (Some(Multiplexer::Screen), true, _) => {
                targets.push(Box::new(screen_buffer::ScreenBuffer));
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::ScreenPassthrough,
                }));
            }
            (None, false, _) => {
                targets.push(self.clipboard_target());
            }
            (None, true, _) => {
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::Raw,
                }));
            }
        }
        targets
    }

    /// Parse a comma-separated output spec into targets.
    /// Returns (targets, unknown_names).
    pub fn parse_output_spec(&self, spec: &str) -> (Vec<Box<dyn OutputTarget>>, Vec<String>) {
        let mut targets: Vec<Box<dyn OutputTarget>> = Vec::new();
        let mut unknown = Vec::new();
        for part in spec.split(',') {
            match part.trim() {
                "stdout" => targets.push(Box::new(stdout::Stdout)),
                "os-clipboard" => targets.push(Box::new(os_clipboard::OsClipboard)),
                "clipboard" => targets.push(self.clipboard_target()),
                "wsl-clipboard" => targets.push(Box::new(wsl_clipboard::WslClipboard)),
                "tmux" => targets.push(Box::new(tmux_buffer::TmuxBuffer)),
                "screen" => targets.push(Box::new(screen_buffer::ScreenBuffer)),
                "osc52" => {
                    let mode = match self.multiplexer {
                        Some(Multiplexer::Tmux) => osc52::Osc52Mode::TmuxClientTty,
                        Some(Multiplexer::Screen) => osc52::Osc52Mode::ScreenPassthrough,
                        None => osc52::Osc52Mode::Raw,
                    };
                    targets.push(Box::new(osc52::Osc52 { mode }));
                }
                other => unknown.push(other.to_string()),
            }
        }
        (targets, unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_names(env: &Environment) -> Vec<&'static str> {
        env.auto_targets().iter().map(|t| t.name()).collect()
    }

    fn env(mux: Option<Multiplexer>, ssh: bool, wsl: bool, clipboard: &str) -> Environment {
        Environment {
            multiplexer: mux,
            is_ssh: ssh,
            is_wsl: wsl,
            set_clipboard: clipboard.to_string(),
        }
    }

    #[test]
    fn tmux_local_on() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Tmux), false, false, "on")),
            vec!["tmux-buffer", "os-clipboard"]
        );
    }

    #[test]
    fn tmux_local_external() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Tmux), false, false, "external")),
            vec!["tmux-buffer", "os-clipboard", "osc52(tmux-client-tty)"]
        );
    }

    #[test]
    fn tmux_local_off() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Tmux), false, false, "off")),
            vec!["tmux-buffer"]
        );
    }

    #[test]
    fn tmux_ssh_on() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Tmux), true, false, "on")),
            vec!["tmux-buffer", "osc52(tmux-client-tty)"]
        );
    }

    #[test]
    fn tmux_ssh_off() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Tmux), true, false, "off")),
            vec!["tmux-buffer"]
        );
    }

    #[test]
    fn screen_local() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Screen), false, false, "on")),
            vec!["screen-buffer", "os-clipboard"]
        );
    }

    #[test]
    fn screen_ssh() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Screen), true, false, "on")),
            vec!["screen-buffer", "osc52(screen-passthrough)"]
        );
    }

    #[test]
    fn standalone_local() {
        assert_eq!(
            target_names(&env(None, false, false, "on")),
            vec!["os-clipboard"]
        );
    }

    #[test]
    fn standalone_ssh() {
        assert_eq!(target_names(&env(None, true, false, "on")), vec!["osc52"]);
    }

    #[test]
    fn wsl_standalone() {
        assert_eq!(
            target_names(&env(None, false, true, "on")),
            vec!["wsl-clipboard"]
        );
    }

    #[test]
    fn wsl_tmux_local() {
        assert_eq!(
            target_names(&env(Some(Multiplexer::Tmux), false, true, "on")),
            vec!["tmux-buffer", "wsl-clipboard"]
        );
    }

    #[test]
    fn parse_spec_stdout() {
        let e = env(None, false, false, "on");
        let (targets, unknown) = e.parse_output_spec("stdout");
        assert_eq!(
            targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
            vec!["stdout"]
        );
        assert!(unknown.is_empty());
    }

    #[test]
    fn parse_spec_multiple() {
        let e = env(None, false, false, "on");
        let (targets, unknown) = e.parse_output_spec("stdout,os-clipboard");
        assert_eq!(
            targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
            vec!["stdout", "os-clipboard"]
        );
        assert!(unknown.is_empty());
    }

    #[test]
    fn parse_spec_unknown() {
        let e = env(None, false, false, "on");
        let (targets, unknown) = e.parse_output_spec("stdout,bogus");
        assert_eq!(
            targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
            vec!["stdout"]
        );
        assert_eq!(unknown, vec!["bogus"]);
    }

    #[test]
    fn parse_spec_osc52_tmux() {
        let e = env(Some(Multiplexer::Tmux), false, false, "on");
        let (targets, _) = e.parse_output_spec("osc52");
        assert_eq!(
            targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
            vec!["osc52(tmux-client-tty)"]
        );
    }

    #[test]
    fn parse_spec_osc52_raw() {
        let e = env(None, false, false, "on");
        let (targets, _) = e.parse_output_spec("osc52");
        assert_eq!(
            targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
            vec!["osc52"]
        );
    }

    #[test]
    fn parse_spec_clipboard_alias() {
        // clipboard resolves to wsl-clipboard on WSL
        let e = env(None, false, true, "on");
        let (targets, unknown) = e.parse_output_spec("clipboard");
        assert_eq!(
            targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
            vec!["wsl-clipboard"]
        );
        assert!(unknown.is_empty());
    }

    #[test]
    fn parse_spec_wsl_clipboard_explicit() {
        let e = env(None, false, false, "on");
        let (targets, unknown) = e.parse_output_spec("wsl-clipboard");
        assert_eq!(
            targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
            vec!["wsl-clipboard"]
        );
        assert!(unknown.is_empty());
    }
}
