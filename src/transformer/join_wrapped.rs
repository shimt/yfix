use unicode_width::UnicodeWidthStr;

use super::{TransformDiagnostics, Transformer, Warning};
use crate::error::TransformerError;

/// Returns true if the character belongs to a CJK script where
/// inter-word spaces are not used (Han, Hiragana, Katakana, Hangul, etc.)
fn is_cjk(ch: Option<char>) -> bool {
    match ch {
        None => false,
        Some(c) => matches!(c,
            '\u{3000}'..='\u{303F}'   // CJK Symbols and Punctuation
            | '\u{3040}'..='\u{309F}' // Hiragana
            | '\u{30A0}'..='\u{30FF}' // Katakana
            | '\u{3400}'..='\u{4DBF}' // CJK Unified Ideographs Extension A
            | '\u{4E00}'..='\u{9FFF}' // CJK Unified Ideographs
            | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
            | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
            | '\u{FE30}'..='\u{FE4F}' // CJK Compatibility Forms
            | '\u{FF65}'..='\u{FF9F}' // Halfwidth Katakana
            | '\u{1100}'..='\u{11FF}' // Hangul Jamo
            | '\u{20000}'..='\u{2A6DF}' // CJK Unified Ideographs Extension B
            | '\u{2A700}'..='\u{2B73F}' // CJK Unified Ideographs Extension C
        ),
    }
}

/// Returns true if the line starts with a structural block prefix
/// (list item or blockquote) that should suppress join_wrapped merging.
fn starts_with_list_prefix(line: &str) -> bool {
    let trimmed = line.trim_start();
    // Markdown unordered list: - , * , +
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }
    // Blockquote: >
    if trimmed.starts_with("> ") {
        return true;
    }
    // Markdown ordered list: 1. , 2. , etc.
    let digits: &str = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
    if digits.starts_with(". ") && digits.len() < trimmed.len() {
        return true;
    }
    false
}

/// Returns true if the line contains any Box Drawing character (U+2500–U+257F).
fn contains_box_drawing(line: &str) -> bool {
    line.chars().any(|c| ('\u{2500}'..='\u{257F}').contains(&c))
}

pub struct JoinWrapped {
    pub wrap_width: usize,
    pub skip_table_lines: bool,
}

fn join_inner(
    text: &str,
    wrap_width: usize,
    skip_table_lines: bool,
    mut warnings: Option<&mut Vec<Warning>>,
) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let threshold = wrap_width.saturating_sub(2);
    let relaxed = wrap_width / 2;
    let near_miss_low = wrap_width * 70 / 100;
    let is_table = |line: &str| skip_table_lines && contains_box_drawing(line);

    while i < lines.len() {
        let line = lines[i];
        let width = UnicodeWidthStr::width(line);

        let next_is_non_empty = lines.get(i + 1).map(|l| !l.is_empty()).unwrap_or(false);
        let next_is_list = lines
            .get(i + 1)
            .map(|l| starts_with_list_prefix(l))
            .unwrap_or(false);

        let next_line = lines.get(i + 1).copied().unwrap_or("");
        if width >= threshold
            && next_is_non_empty
            && !next_is_list
            && !is_table(line)
            && !is_table(next_line)
        {
            let mut joined = line.to_string();
            let mut j = i + 1;
            loop {
                let cont = lines[j].trim_start();
                let needs_space = !joined.ends_with(' ')
                    && !cont.starts_with(' ')
                    && !cont.is_empty()
                    && !is_cjk(joined.chars().last())
                    && !is_cjk(cont.chars().next());
                if needs_space {
                    joined.push(' ');
                }
                joined.push_str(cont);
                j += 1;
                let seg_width = UnicodeWidthStr::width(lines[j - 1]);
                let next_exists = j < lines.len() && !lines[j].is_empty();
                let next_is_list = if j < lines.len() {
                    starts_with_list_prefix(lines[j])
                } else {
                    false
                };

                // Track relaxed threshold usage
                if let Some(ref mut w) = warnings {
                    if seg_width < threshold && seg_width >= relaxed {
                        w.push(Warning::JoinRelaxedUsed {
                            line_index: j - 1,
                            width: seg_width,
                        });
                    }
                }

                let next_is_table = if j < lines.len() {
                    is_table(lines[j])
                } else {
                    false
                };
                if seg_width >= relaxed && next_exists && !next_is_list && !next_is_table {
                    continue;
                }
                break;
            }
            result.push(joined);
            i = j;
        } else {
            // Track near misses
            if let Some(ref mut w) = warnings {
                if next_is_non_empty
                    && width >= near_miss_low
                    && width < threshold
                    && !is_table(line)
                    && !is_table(next_line)
                {
                    w.push(Warning::JoinNearMiss {
                        line_index: i,
                        width,
                        wrap_width,
                    });
                }
            }
            result.push(line.to_string());
            i += 1;
        }
    }

    let mut out = result.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

impl Transformer for JoinWrapped {
    fn name(&self) -> &'static str {
        "join_wrapped"
    }

    fn transform(&self, text: &str) -> Result<String, TransformerError> {
        Ok(join_inner(
            text,
            self.wrap_width,
            self.skip_table_lines,
            None,
        ))
    }

    fn transform_with_diagnostics(
        &self,
        text: &str,
    ) -> Result<(String, TransformDiagnostics), TransformerError> {
        let mut warnings = Vec::new();
        let result = join_inner(
            text,
            self.wrap_width,
            self.skip_table_lines,
            Some(&mut warnings),
        );
        Ok((result, TransformDiagnostics { warnings }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    #[test]
    fn joins_wrapped_ascii_line() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\ncontinuation";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890 continuation");
    }

    #[test]
    fn does_not_join_when_next_line_empty() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\n\nnext paragraph";
        let result = t.transform(input).unwrap();
        assert!(result.contains('\n'));
    }

    #[test]
    fn does_not_join_short_line() {
        let t = JoinWrapped {
            wrap_width: 80,
            skip_table_lines: true,
        };
        let input = "short line\nnext line";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "short line\nnext line");
    }

    #[test]
    fn joins_cjk_line_at_width_minus_one() {
        let t = JoinWrapped {
            wrap_width: 21,
            skip_table_lines: true,
        };
        let input = "あああああああああああ\ncontinuation";
        let result = t.transform(input).unwrap();
        assert!(result.starts_with("あああああああああああcontinuation"));
    }

    #[test]
    fn joins_with_tolerance_of_2() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "123456789012345678\ncontinuation";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "123456789012345678 continuation");
    }

    #[test]
    fn inserts_space_at_join_seam() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\n   continuation";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890 continuation");
    }

    #[test]
    fn no_extra_space_when_line_ends_with_space() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "1234567890123456789 \n   continuation";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "1234567890123456789 continuation");
    }

    #[test]
    fn no_space_between_cjk_chars() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "ああああああああああ\nいいいい";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "ああああああああああいいいい");
    }

    #[test]
    fn no_space_when_cjk_meets_ascii() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "ああああああああああ\ncontinuation";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "ああああああああああcontinuation");
    }

    #[test]
    fn no_space_when_ascii_meets_cjk() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\nああ";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890ああ");
    }

    #[test]
    fn joins_multiple_wraps() {
        let t = JoinWrapped {
            wrap_width: 10,
            skip_table_lines: true,
        };
        let input = "1234567890\nabcdefghij\nklmnopqrst\nend";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "1234567890 abcdefghij klmnopqrst end");
    }

    #[test]
    fn joins_multiple_wraps_then_stops() {
        let t = JoinWrapped {
            wrap_width: 10,
            skip_table_lines: true,
        };
        let input = "1234567890\nabcdefghij\nend\nshort";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "1234567890 abcdefghij end\nshort");
    }

    #[test]
    fn joins_word_wrapped_continuations() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\nword wrapped at\nboundary end";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890 word wrapped at boundary end");
    }

    #[test]
    fn stops_at_very_short_continuation() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\nword wrap\nshrt\nnext line";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890 word wrap\nshrt\nnext line");
    }

    #[test]
    fn diagnostics_near_miss() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        // Line width 15 = 75% of 20, threshold=18, near_miss_low=14
        let input = "123456789012345\nnext line";
        let (_, diag) = t.transform_with_diagnostics(input).unwrap();
        assert!(diag
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::JoinNearMiss { .. })));
    }

    #[test]
    fn does_not_join_list_items_unordered() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\n- item one\n- item two";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890\n- item one\n- item two");
    }

    #[test]
    fn does_not_join_list_items_ordered() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\n1. first item\n2. second item";
        let result = t.transform(input).unwrap();
        assert_eq!(
            result,
            "12345678901234567890\n1. first item\n2. second item"
        );
    }

    #[test]
    fn does_not_join_indented_list_items() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\n  - sub item one\n  - sub item two";
        let result = t.transform(input).unwrap();
        assert_eq!(
            result,
            "12345678901234567890\n  - sub item one\n  - sub item two"
        );
    }

    #[test]
    fn does_not_join_blockquote() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\n> quoted text\n> more quoted";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890\n> quoted text\n> more quoted");
    }

    #[test]
    fn breaks_continuation_at_list_item() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\nword wrapped at\n- list starts here";
        let result = t.transform(input).unwrap();
        assert_eq!(
            result,
            "12345678901234567890 word wrapped at\n- list starts here"
        );
    }

    #[test]
    fn long_list_item_joins_own_wrap() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "- abcdefghij1234567\ncontinuation text";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "- abcdefghij1234567 continuation text");
    }

    #[test]
    fn does_not_join_plus_list_items() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\n+ item one\n+ item two";
        let result = t.transform(input).unwrap();
        assert_eq!(result, "12345678901234567890\n+ item one\n+ item two");
    }

    #[test]
    fn diagnostics_relaxed_used() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        // First line hits threshold (20 >= 18), continuation uses relaxed (15 >= 10)
        let input = "12345678901234567890\nword wrapped at\nend";
        let (_, diag) = t.transform_with_diagnostics(input).unwrap();
        assert!(diag
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::JoinRelaxedUsed { .. })));
    }

    #[test]
    fn does_not_join_table_lines() {
        let t = JoinWrapped {
            wrap_width: 90,
            skip_table_lines: true,
        };
        let input = "  \u{250C}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{252C}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{252C}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}\n  \u{2502}  #  \u{2502} \u{30B3}\u{30DF}\u{30C3}\u{30C8} \u{2502}                              \u{5185}\u{5BB9}                               \u{2502}";
        let result = t.transform(input).unwrap();
        assert!(result.contains('\n'), "table lines should not be joined");
    }

    #[test]
    fn no_near_miss_warning_for_table_lines() {
        let t = JoinWrapped {
            wrap_width: 90,
            skip_table_lines: true,
        };
        // Table line at 86 chars wide = 95% of 90 (in near-miss range)
        let input = "  ┌─────┬──────────┬─────────────────────────────────────────────────────────────────┐\n  │ next │";
        let (_, diag) = t.transform_with_diagnostics(input).unwrap();
        assert!(
            !diag
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::JoinNearMiss { .. })),
            "table lines should not produce near-miss warnings"
        );
    }

    #[test]
    fn breaks_continuation_at_table_line() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        let input = "12345678901234567890\nword wrapped at\n  ├─────┼──────────┤";
        let result = t.transform(input).unwrap();
        assert_eq!(
            result,
            "12345678901234567890 word wrapped at\n  ├─────┼──────────┤"
        );
    }

    #[test]
    fn no_near_miss_warning_when_next_line_is_table() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: true,
        };
        // Line width 15 = 75% of 20 (in near-miss range), next line is a table line
        let input = "123456789012345\n  ├─────┼──────┤";
        let (_, diag) = t.transform_with_diagnostics(input).unwrap();
        assert!(
            !diag
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::JoinNearMiss { .. })),
            "no near-miss warning when next line is a table line"
        );
    }

    #[test]
    fn config_skip_table_lines_disabled_joins_table_lines() {
        let t = JoinWrapped {
            wrap_width: 20,
            skip_table_lines: false,
        };
        // With skip_table_lines=false, table lines are treated as normal text
        let input = "┌──────────────────┐\n│ data             │";
        let result = t.transform(input).unwrap();
        // Should join because line width (20) >= threshold (18) and skip is disabled
        assert!(
            !result.contains('\n'),
            "with skip disabled, table lines should be joined"
        );
    }
}
