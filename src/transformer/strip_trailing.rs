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
}
