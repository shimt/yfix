use regex::Regex;
use std::sync::OnceLock;

use super::Transformer;
use crate::error::TransformerError;

pub struct StripPrompt;

fn prompt_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(?:❯|[$%>]|>>>)\s*$").unwrap())
}

impl Transformer for StripPrompt {
    fn name(&self) -> &'static str {
        "strip_prompt"
    }

    fn transform(&self, text: &str) -> Result<String, TransformerError> {
        let re = prompt_re();
        let lines: Vec<&str> = text.lines().collect();
        let result: Vec<&str> = lines.iter().filter(|l| !re.is_match(l)).copied().collect();

        let mut out = result.join("\n");
        if text.ends_with('\n') {
            out.push('\n');
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    #[test]
    fn removes_fish_prompt() {
        let t = StripPrompt;
        let input = "some output\n❯\nmore output";
        assert_eq!(t.transform(input).unwrap(), "some output\nmore output");
    }

    #[test]
    fn removes_dollar_prompt() {
        let t = StripPrompt;
        let input = "$ \nresult";
        assert_eq!(t.transform(input).unwrap(), "result");
    }

    #[test]
    fn keeps_line_with_content_after_prompt() {
        let t = StripPrompt;
        let input = "$ ls -la\nresult";
        assert_eq!(t.transform(input).unwrap(), "$ ls -la\nresult");
    }

    #[test]
    fn does_not_remove_markdown_heading() {
        let t = StripPrompt;
        let input = "# This is a heading\ncontent";
        assert_eq!(t.transform(input).unwrap(), "# This is a heading\ncontent");
    }

    #[test]
    fn removes_python_repl_prompt() {
        let t = StripPrompt;
        let input = ">>>\nresult";
        assert_eq!(t.transform(input).unwrap(), "result");
    }
}
