use crate::config::Config;
use crate::error::TransformerError;
use crate::transformer::{
    compress_blank::CompressBlank, dedent::Dedent, join_wrapped::JoinWrapped,
    strip_ansi::StripAnsi, strip_line_numbers::StripLineNumbers, strip_prompt::StripPrompt,
    strip_trailing::StripTrailing, Transformer, Warning,
};

pub struct ProcessResult {
    pub text: String,
    pub trace: Vec<String>,
    pub warnings: Vec<Warning>,
}

pub struct Processor {
    transformers: Vec<Box<dyn Transformer>>,
}

impl Processor {
    pub fn from_config(config: &Config, wrap_width: usize) -> Self {
        let tc = &config.transformers;
        let mut transformers: Vec<Box<dyn Transformer>> = Vec::new();

        if tc.strip_ansi {
            transformers.push(Box::new(StripAnsi));
        }
        if tc.strip_line_numbers {
            transformers.push(Box::new(StripLineNumbers));
        }
        if tc.join_wrapped {
            transformers.push(Box::new(JoinWrapped {
                wrap_width,
                skip_table_lines: tc.skip_table_lines,
            }));
        }
        if tc.dedent {
            transformers.push(Box::new(Dedent));
        }
        if tc.strip_trailing {
            transformers.push(Box::new(StripTrailing));
        }
        if tc.compress_blank {
            transformers.push(Box::new(CompressBlank));
        }
        if tc.strip_prompt {
            transformers.push(Box::new(StripPrompt));
        }

        Self { transformers }
    }

    pub fn process(&self, text: &str) -> Result<String, TransformerError> {
        let mut current = text.to_string();
        for t in &self.transformers {
            current = t.transform(&current)?;
        }
        Ok(current)
    }

    /// Process with diagnostic trace and warnings.
    pub fn process_with_trace(&self, text: &str) -> Result<ProcessResult, TransformerError> {
        use unicode_width::UnicodeWidthStr;
        let mut trace = Vec::new();
        let mut all_warnings = Vec::new();
        let mut current = text.to_string();

        trace.push(format!("[input] {} lines", current.lines().count()));
        for line in current.lines() {
            trace.push(format!("  w={:3} | {}", UnicodeWidthStr::width(line), line));
        }

        for t in &self.transformers {
            let prev = current.clone();
            let (result, diag) = t.transform_with_diagnostics(&current)?;
            current = result;
            all_warnings.extend(diag.warnings);

            if current != prev {
                trace.push(format!("[{}] changed", t.name()));
                for line in current.lines() {
                    trace.push(format!("  w={:3} | {}", UnicodeWidthStr::width(line), line));
                }
            } else {
                trace.push(format!("[{}] no change", t.name()));
            }
        }

        Ok(ProcessResult {
            text: current,
            trace,
            warnings: all_warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, TransformerConfig};

    #[test]
    fn process_with_all_disabled_returns_input() {
        let config = Config {
            fallback_width: 80,
            transformers: TransformerConfig {
                strip_ansi: false,
                strip_line_numbers: false,
                join_wrapped: false,
                dedent: false,
                strip_trailing: false,
                compress_blank: false,
                strip_prompt: false,
                skip_table_lines: true,
            },
        };
        let p = Processor::from_config(&config, 80);
        assert_eq!(p.process("hello\nworld").unwrap(), "hello\nworld");
    }

    #[test]
    fn process_strips_ansi_and_trailing() {
        let config = Config::default();
        let p = Processor::from_config(&config, 80);
        let input = "\x1b[31mhello\x1b[0m   ";
        let result = p.process(input).unwrap();
        assert_eq!(result, "hello");
    }
}
