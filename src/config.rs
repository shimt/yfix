use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformerConfig {
    #[serde(default = "default_true")]
    pub strip_ansi: bool,
    #[serde(default = "default_true")]
    pub strip_line_numbers: bool,
    #[serde(default = "default_true")]
    pub join_wrapped: bool,
    #[serde(default = "default_true")]
    pub dedent: bool,
    #[serde(default = "default_true")]
    pub strip_trailing: bool,
    #[serde(default = "default_true")]
    pub compress_blank: bool,
    #[serde(default = "default_true")]
    pub strip_prompt: bool,
    #[serde(default = "default_true")]
    pub skip_table_lines: bool,
}

fn default_true() -> bool {
    true
}

impl Default for TransformerConfig {
    fn default() -> Self {
        Self {
            strip_ansi: true,
            strip_line_numbers: true,
            join_wrapped: true,
            dedent: true,
            strip_trailing: true,
            compress_blank: true,
            strip_prompt: true,
            skip_table_lines: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_width")]
    pub fallback_width: usize,
    #[serde(default)]
    pub transformers: TransformerConfig,
}

fn default_width() -> usize {
    80
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fallback_width: 80,
            transformers: TransformerConfig::default(),
        }
    }
}

impl Config {
    pub fn load(path: Option<&PathBuf>) -> anyhow::Result<Self> {
        let resolved = path.cloned().or_else(default_config_path);
        match resolved {
            None => Ok(Config::default()),
            Some(p) => {
                if !p.exists() {
                    return Ok(Config::default());
                }
                let content = std::fs::read_to_string(&p)?;
                let config: Config = serde_yaml::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("config parse error in {}: {}", p.display(), e))?;
                Ok(config)
            }
        }
    }
}

pub fn default_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "yfix").map(|dirs| dirs.config_dir().join("config.yaml"))
}

pub fn debug_flag_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "yfix").map(|dirs| dirs.config_dir().join("debug"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let c = Config::default();
        assert_eq!(c.fallback_width, 80);
        assert!(c.transformers.strip_ansi);
        assert!(c.transformers.join_wrapped);
    }

    #[test]
    fn load_nonexistent_path_returns_defaults() {
        let result = Config::load(Some(&PathBuf::from("/nonexistent/path/config.yaml")));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().fallback_width, 80);
    }

    #[test]
    fn load_explicit_nonexistent_path_returns_defaults() {
        let result = Config::load(Some(&PathBuf::from("/tmp/yfix_test_nonexistent.yaml")));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().fallback_width, 80);
    }

    #[test]
    fn load_valid_yaml() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "fallback_width: 120\ntransformers:\n  strip_ansi: false").unwrap();
        let result = Config::load(Some(&f.path().to_path_buf()));
        assert!(result.is_ok());
        let c = result.unwrap();
        assert_eq!(c.fallback_width, 120);
        assert!(!c.transformers.strip_ansi);
        assert!(c.transformers.join_wrapped);
    }

    #[test]
    fn default_config_has_skip_table_lines_enabled() {
        let c = Config::default();
        assert!(c.transformers.skip_table_lines);
    }

    #[test]
    fn load_skip_table_lines_disabled() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "transformers:\n  skip_table_lines: false").unwrap();
        let result = Config::load(Some(&f.path().to_path_buf()));
        assert!(result.is_ok());
        assert!(!result.unwrap().transformers.skip_table_lines);
    }

    #[test]
    fn load_invalid_yaml_returns_err() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "{{{{invalid yaml}}}}").unwrap();
        let result = Config::load(Some(&f.path().to_path_buf()));
        assert!(result.is_err());
    }
}
