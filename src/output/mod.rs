pub mod os_clipboard;
pub mod osc52;
pub mod screen_buffer;
pub mod stdout;
pub mod tmux_buffer;

use std::process::Command;

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
            return has_sshd_ancestor(client_pid);
        }
    }

    false
}

fn tmux_client_pid() -> Option<u32> {
    Command::new("tmux")
        .args(["display-message", "-p", "#{client_pid}"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok()?.trim().parse::<u32>().ok())
}

fn has_sshd_ancestor(start_pid: u32) -> bool {
    let mut pid = start_pid;
    while pid > 1 {
        if let Some(name) = process_name(pid) {
            if name == "sshd" || name.starts_with("sshd-") {
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

fn process_name(pid: u32) -> Option<String> {
    Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parent_pid(pid: u32) -> Option<u32> {
    Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
}

pub trait OutputTarget {
    fn write(&self, text: &str) -> Result<(), OutputError>;
    fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub struct Environment {
    pub multiplexer: Option<Multiplexer>,
    pub is_ssh: bool,
    pub set_clipboard: String,
}

impl Environment {
    pub fn detect() -> Self {
        let multiplexer = Multiplexer::detect();
        let is_ssh = detect_ssh(&multiplexer);
        let set_clipboard = if matches!(multiplexer, Some(Multiplexer::Tmux)) {
            Multiplexer::tmux_set_clipboard()
        } else {
            "on".to_string()
        };
        Self {
            multiplexer,
            is_ssh,
            set_clipboard,
        }
    }

    pub fn auto_targets(&self) -> Vec<Box<dyn OutputTarget>> {
        use osc52::{Osc52, Osc52Mode};
        let mut targets: Vec<Box<dyn OutputTarget>> = Vec::new();

        match (&self.multiplexer, self.is_ssh, self.set_clipboard.as_str()) {
            (Some(Multiplexer::Tmux), false, "on") => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
                targets.push(Box::new(os_clipboard::OsClipboard));
            }
            (Some(Multiplexer::Tmux), false, "external") => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
                targets.push(Box::new(os_clipboard::OsClipboard));
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::TmuxPassthrough,
                }));
            }
            (Some(Multiplexer::Tmux), false, _) => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
            }
            (Some(Multiplexer::Tmux), true, "on" | "external") => {
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::TmuxPassthrough,
                }));
            }
            (Some(Multiplexer::Tmux), true, _) => {
                // set-clipboard=off: OSC 52 not forwarded
                targets.push(Box::new(tmux_buffer::TmuxBuffer));
            }
            (Some(Multiplexer::Screen), false, _) => {
                targets.push(Box::new(screen_buffer::ScreenBuffer));
                targets.push(Box::new(os_clipboard::OsClipboard));
            }
            (Some(Multiplexer::Screen), true, _) => {
                targets.push(Box::new(screen_buffer::ScreenBuffer));
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::ScreenPassthrough,
                }));
            }
            (None, false, _) => {
                targets.push(Box::new(os_clipboard::OsClipboard));
            }
            (None, true, _) => {
                targets.push(Box::new(Osc52 {
                    mode: Osc52Mode::Raw,
                }));
            }
        }
        targets
    }
}
