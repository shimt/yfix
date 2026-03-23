use super::OutputTarget;
use crate::error::OutputError;

pub struct OsClipboard;

impl OutputTarget for OsClipboard {
    fn name(&self) -> &'static str {
        "os-clipboard"
    }

    fn write(&self, text: &str) -> Result<(), OutputError> {
        let mut cb =
            arboard::Clipboard::new().map_err(|e| OutputError::Clipboard(e.to_string()))?;
        cb.set_text(text)
            .map_err(|e| OutputError::Clipboard(e.to_string()))?;
        Ok(())
    }
}
