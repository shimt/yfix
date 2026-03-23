use super::Transformer;
use crate::error::TransformerError;

pub struct CompressBlank;

impl Transformer for CompressBlank {
    fn name(&self) -> &'static str {
        "compress_blank"
    }

    fn transform(&self, text: &str) -> Result<String, TransformerError> {
        let lines: Vec<&str> = text.lines().collect();
        let mut result: Vec<&str> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].trim().is_empty() {
                let start = i;
                while i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
                let run = i - start;
                let keep = if run >= 3 { 1 } else { run };
                result.extend(std::iter::repeat_n("", keep));
            } else {
                result.push(lines[i]);
                i += 1;
            }
        }

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
    fn compresses_three_blank_lines_to_one() {
        let t = CompressBlank;
        let input = "a\n\n\n\nb";
        assert_eq!(t.transform(input).unwrap(), "a\n\nb");
    }

    #[test]
    fn compresses_four_blank_lines_to_one() {
        let t = CompressBlank;
        let input = "a\n\n\n\n\nb";
        assert_eq!(t.transform(input).unwrap(), "a\n\nb");
    }

    #[test]
    fn keeps_two_blank_lines_unchanged() {
        let t = CompressBlank;
        let input = "a\n\n\nb";
        assert_eq!(t.transform(input).unwrap(), "a\n\n\nb");
    }

    #[test]
    fn keeps_one_blank_line_unchanged() {
        let t = CompressBlank;
        let input = "a\n\nb";
        assert_eq!(t.transform(input).unwrap(), "a\n\nb");
    }
}
