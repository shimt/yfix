use super::Transformer;
use crate::error::TransformerError;

pub struct Dedent;

impl Transformer for Dedent {
    fn name(&self) -> &'static str {
        "dedent"
    }

    fn transform(&self, text: &str) -> Result<String, TransformerError> {
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() <= 1 {
            return Ok(text.to_string());
        }

        let min_indent = lines[1..]
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        if min_indent == 0 {
            return Ok(text.to_string());
        }

        let mut out = Vec::new();

        // Dedent line 1 too if it has at least min_indent leading spaces
        let first_indent = lines[0].len() - lines[0].trim_start().len();
        if first_indent >= min_indent {
            out.push(lines[0][min_indent..].to_string());
        } else {
            out.push(lines[0].to_string());
        }

        for line in &lines[1..] {
            if line.trim().is_empty() {
                out.push(String::new());
            } else {
                out.push(line[min_indent..].to_string());
            }
        }

        let mut result = out.join("\n");
        if text.ends_with('\n') {
            result.push('\n');
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    fn t() -> Dedent {
        Dedent
    }

    #[test]
    fn line1_without_indent_is_untouched() {
        let input = "first line\n  second\n  third";
        let result = t().transform(input).unwrap();
        assert_eq!(result, "first line\nsecond\nthird");
    }

    #[test]
    fn line1_with_same_indent_is_also_dedented() {
        // All lines have 2-space indent → all dedented
        let input = "  TMUX          = foo\n  TMUX_PANE     = bar\n  STY           = baz";
        let result = t().transform(input).unwrap();
        assert_eq!(
            result,
            "TMUX          = foo\nTMUX_PANE     = bar\nSTY           = baz"
        );
    }

    #[test]
    fn first_line_without_indent_is_untouched() {
        let input = "// comment\n  libc::mmap(\n      ptr,\n  )";
        let result = t().transform(input).unwrap();
        assert_eq!(result, "// comment\nlibc::mmap(\n    ptr,\n)");
    }

    #[test]
    fn empty_first_line_treated_as_line_1() {
        let input = "\n  hello\n  world";
        let result = t().transform(input).unwrap();
        assert_eq!(result, "\nhello\nworld");
    }

    #[test]
    fn preserves_relative_indent() {
        // min_indent of lines 2+ is 0 (the "}" line), so no dedent occurs
        let input = "fn foo() {\n    let x = 1;\n        let y = 2;\n}";
        let result = t().transform(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn dedents_when_all_lines_indented() {
        // line1 has 0 indent, lines 2+ min_indent=4 → line1 untouched
        let input = "fn foo() {\n    let x = 1;\n        let y = 2;\n    }";
        let result = t().transform(input).unwrap();
        assert_eq!(result, "fn foo() {\nlet x = 1;\n    let y = 2;\n}");
    }

    #[test]
    fn line1_with_more_indent_is_also_dedented() {
        // line1 has 4 spaces, lines 2+ min_indent=2 → line1 dedented by 2
        let input = "    header\n  body\n  footer";
        let result = t().transform(input).unwrap();
        assert_eq!(result, "  header\nbody\nfooter");
    }

    #[test]
    fn single_line_unchanged() {
        let input = "  hello";
        let result = t().transform(input).unwrap();
        assert_eq!(result, "  hello");
    }
}
