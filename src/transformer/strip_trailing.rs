use super::Transformer;
use crate::error::TransformerError;

pub struct StripTrailing;

impl Transformer for StripTrailing {
    fn name(&self) -> &'static str {
        "strip_trailing"
    }

    fn transform(&self, text: &str) -> Result<String, TransformerError> {
        let mut result: String = text
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        while result.ends_with("\n\n") {
            result.pop();
        }
        if result.ends_with('\n') {
            result.pop();
        }

        // Single non-empty line: also trim leading whitespace (copy-mode drag offset)
        if !result.contains('\n') {
            result = result.trim_start().to_string();
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    #[test]
    fn removes_trailing_spaces() {
        let t = StripTrailing;
        assert_eq!(t.transform("hello   \nworld  ").unwrap(), "hello\nworld");
    }

    #[test]
    fn removes_trailing_blank_lines() {
        let t = StripTrailing;
        assert_eq!(t.transform("hello\n\n\n").unwrap(), "hello");
    }

    #[test]
    fn single_line_trims_leading() {
        let t = StripTrailing;
        assert_eq!(t.transform(" hello").unwrap(), "hello");
    }

    #[test]
    fn single_line_with_trailing_newline_trims_leading() {
        let t = StripTrailing;
        assert_eq!(t.transform(" hello\n").unwrap(), "hello");
    }

    #[test]
    fn multi_line_preserves_leading() {
        let t = StripTrailing;
        assert_eq!(t.transform(" a\n b").unwrap(), " a\n b");
    }
}
