use super::OutputTarget;
use crate::error::OutputError;
use crate::multiplexer::Multiplexer;

pub struct ScreenBuffer;

impl OutputTarget for ScreenBuffer {
    fn name(&self) -> &'static str {
        "screen-buffer"
    }

    fn write(&self, text: &str) -> Result<(), OutputError> {
        Multiplexer::Screen.load_buffer(text)?;
        Ok(())
    }
}
