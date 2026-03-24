use super::OutputTarget;
use crate::error::OutputError;
use crate::multiplexer::Multiplexer;

pub struct TmuxBuffer;

impl OutputTarget for TmuxBuffer {
    fn name(&self) -> &'static str {
        "tmux-buffer"
    }

    fn write(&self, text: &str) -> Result<(), OutputError> {
        Multiplexer::Tmux.load_buffer(text)?;
        Ok(())
    }
}
