use std::io::Write;
use std::process::{Command, Stdio};

use super::OutputTarget;
use crate::error::OutputError;

pub struct WslClipboard;

impl OutputTarget for WslClipboard {
    fn name(&self) -> &'static str {
        "wsl-clipboard"
    }

    fn write(&self, text: &str) -> Result<(), OutputError> {
        let mut child = Command::new("clip.exe").stdin(Stdio::piped()).spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;
        Ok(())
    }
}
