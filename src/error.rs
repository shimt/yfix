#[derive(Debug, thiserror::Error)]
pub enum TransformerError {
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("multiplexer error: {0}")]
    Multiplexer(#[from] MultiplexerError),
}

#[derive(Debug, thiserror::Error)]
pub enum MultiplexerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("not in a multiplexer session")]
    NotInSession,
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}
