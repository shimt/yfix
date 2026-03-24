use regex::Regex;
use std::sync::OnceLock;

use super::{TransformDiagnostics, Transformer, Warning};
use crate::error::TransformerError;

pub struct StripLineNumbers;

fn line_number_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*\d+(\s|$)").unwrap())
}

struct StripResult {
    text: String,
    match_rate_pct: u32,
    gutter_width: Option<usize>,
    partial_gutter_lines: usize,
}

fn strip_inner(text: &str) -> StripResult {
    let re = line_number_re();
    let lines: Vec<&str> = text.lines().collect();

    let non_empty: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();

    if non_empty.is_empty() {
        return StripResult {
            text: text.to_string(),
            match_rate_pct: 0,
            gutter_width: None,
            partial_gutter_lines: 0,
        };
    }

    let matching = non_empty.iter().filter(|l| re.is_match(l)).count();
    let match_rate_pct = (matching * 100 / non_empty.len()) as u32;

    if matching * 2 < non_empty.len() {
        return StripResult {
            text: text.to_string(),
            match_rate_pct,
            gutter_width: None,
            partial_gutter_lines: 0,
        };
    }

    // Calculate gutter width from matching lines (max match end position)
    let gutter_width = non_empty
        .iter()
        .filter_map(|l| re.find(l).map(|m| m.end()))
        .max();

    let result: Vec<&str> = lines
        .iter()
        .copied()
        .map(|line| {
            if re.is_match(line) {
                let m = re.find(line).unwrap();
                &line[m.end()..]
            } else {
                line
            }
        })
        .collect();

    // Count non-matching lines that have gutter-width leading spaces
    let partial_gutter_lines = if let Some(gw) = gutter_width {
        lines
            .iter()
            .filter(|l| {
                !l.trim().is_empty()
                    && !re.is_match(l)
                    && l.as_bytes().iter().take_while(|&&b| b == b' ').count() >= gw
            })
            .count()
    } else {
        0
    };

    let text = result.join("\n") + if text.ends_with('\n') { "\n" } else { "" };

    StripResult {
        text,
        match_rate_pct,
        gutter_width,
        partial_gutter_lines,
    }
}

impl Transformer for StripLineNumbers {
    fn name(&self) -> &'static str {
        "strip_line_numbers"
    }

    fn transform(&self, text: &str) -> Result<String, TransformerError> {
        Ok(strip_inner(text).text)
    }

    fn transform_with_diagnostics(
        &self,
        text: &str,
    ) -> Result<(String, TransformDiagnostics), TransformerError> {
        let result = strip_inner(text);
        let mut warnings = Vec::new();

        if result.match_rate_pct >= 50 && result.match_rate_pct <= 70 {
            warnings.push(Warning::LineNumbersBorderline {
                match_rate_pct: result.match_rate_pct,
            });
        }

        if result.partial_gutter_lines > 0 {
            if let Some(gw) = result.gutter_width {
                warnings.push(Warning::LineNumbersPartialGutter {
                    gutter_width: gw,
                    affected_lines: result.partial_gutter_lines,
                });
            }
        }

        Ok((result.text, TransformDiagnostics { warnings }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    fn make() -> StripLineNumbers {
        StripLineNumbers
    }

    #[test]
    fn strips_when_majority_match() {
        let t = make();
        let input = "      154 ### tmux設定\n      155\n      156 ```tmux\n      157 # ~/.tmux.conf\n      158 set -g allow-passthrough on\n      159 ```\n      160\n";
        let result = t.transform(input).unwrap();
        assert!(result.contains("### tmux設定"));
        assert!(!result.contains("154"));
        assert!(!result.contains("158"));
    }

    #[test]
    fn noop_when_minority_match() {
        let t = make();
        let input = "  1 hello\nworld\nfoo\nbar";
        let result = t.transform(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn keeps_non_matching_lines_as_is() {
        let t = make();
        let input = "  1 hello\n  2 world\n  3 foo\nno number here";
        let result = t.transform(input).unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("no number here"));
        assert!(!result.contains("  1 "));
    }

    #[test]
    fn empty_lines_ignored_in_threshold() {
        let t = make();
        let input = "  1 hello\n\n  2 world\n\n  3 foo\n";
        let result = t.transform(input).unwrap();
        assert!(!result.contains("  1 "));
        assert!(result.contains("hello"));
    }

    #[test]
    fn does_not_strip_git_hashes() {
        let t = make();
        let input = "00fe40a feat: add debug mode\n6677c2a feat: implement width\n771121f feat: implement OutputTarget\na82a732 feat: add Config";
        let result = t.transform(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn diagnostics_borderline() {
        let t = make();
        // 3 of 5 non-empty match (60%)
        let input = "  1 hello\n  2 world\n  3 foo\nno number\nalso no number";
        let (_, diag) = t.transform_with_diagnostics(input).unwrap();
        assert!(diag
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::LineNumbersBorderline { .. })));
    }

    #[test]
    fn diagnostics_partial_gutter() {
        let t = make();
        // Line with gutter-width spaces but no number (continuation line)
        let input = "      94 assert!(true);\n      95 assert!(false);\n         continuation line";
        let (_, diag) = t.transform_with_diagnostics(input).unwrap();
        assert!(diag
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::LineNumbersPartialGutter { .. })));
    }
}
