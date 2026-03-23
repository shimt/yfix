pub mod compress_blank;
pub mod dedent;
pub mod join_wrapped;
pub mod strip_ansi;
pub mod strip_line_numbers;
pub mod strip_prompt;
pub mod strip_trailing;

use std::fmt;

use crate::error::TransformerError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Warning {
    LineNumbersBorderline {
        match_rate_pct: u32,
    },
    LineNumbersPartialGutter {
        gutter_width: usize,
        affected_lines: usize,
    },
    JoinNearMiss {
        line_index: usize,
        width: usize,
        wrap_width: usize,
    },
    JoinRelaxedUsed {
        line_index: usize,
        width: usize,
    },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::LineNumbersBorderline { match_rate_pct } => {
                write!(f, "line_numbers_borderline: match_rate={}%", match_rate_pct)
            }
            Warning::LineNumbersPartialGutter {
                gutter_width,
                affected_lines,
            } => {
                write!(
                    f,
                    "line_numbers_partial_gutter: gutter_width={}, {} lines affected",
                    gutter_width, affected_lines
                )
            }
            Warning::JoinNearMiss {
                line_index,
                width,
                wrap_width,
            } => {
                write!(
                    f,
                    "join_near_miss: line {} width={} ({}% of wrap_width={})",
                    line_index,
                    width,
                    width * 100 / wrap_width,
                    wrap_width
                )
            }
            Warning::JoinRelaxedUsed { line_index, width } => {
                write!(f, "join_relaxed_used: line {} width={}", line_index, width)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct TransformDiagnostics {
    pub warnings: Vec<Warning>,
}

pub trait Transformer {
    fn transform(&self, text: &str) -> Result<String, TransformerError>;
    fn name(&self) -> &'static str;

    fn transform_with_diagnostics(
        &self,
        text: &str,
    ) -> Result<(String, TransformDiagnostics), TransformerError> {
        let result = self.transform(text)?;
        Ok((result, TransformDiagnostics::default()))
    }
}
