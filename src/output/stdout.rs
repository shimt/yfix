use super::OutputTarget;
use crate::error::OutputError;

pub struct Stdout;

impl OutputTarget for Stdout {
    fn name(&self) -> &'static str {
        "stdout"
    }

    fn write(&self, text: &str) -> Result<(), OutputError> {
        use std::io::Write;
        std::io::stdout().write_all(text.as_bytes())?;
        Ok(())
    }
}
