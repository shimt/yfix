use anstream::adapter::StripStr;

use super::Transformer;
use crate::error::TransformerError;

pub struct StripAnsi;

impl Transformer for StripAnsi {
    fn name(&self) -> &'static str {
        "strip_ansi"
    }

    fn transform(&self, text: &str) -> Result<String, TransformerError> {
        let mut stripper = StripStr::new();
        let stripped: String = stripper.strip_next(text).collect();
        Ok(stripped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    #[test]
    fn strips_color_codes() {
        let t = StripAnsi;
        let input = "\x1b[31mHello\x1b[0m World";
        assert_eq!(t.transform(input).unwrap(), "Hello World");
    }

    #[test]
    fn leaves_plain_text_unchanged() {
        let t = StripAnsi;
        let input = "hello world";
        assert_eq!(t.transform(input).unwrap(), "hello world");
    }

    #[test]
    fn strips_cursor_movement() {
        let t = StripAnsi;
        let input = "\x1b[2K\x1b[1Ahello";
        assert_eq!(t.transform(input).unwrap(), "hello");
    }
}
