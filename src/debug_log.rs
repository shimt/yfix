use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::transformer::Warning;

pub fn debug_log_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "yfix").map(|dirs| dirs.config_dir().join("debug.log"))
}

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub id: u32,
    pub timestamp: String,
    pub version: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub width: usize,
    pub width_source: String,
    pub is_ssh: bool,
    pub output_targets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<Warning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub flagged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flagged_comment: Option<String>,
}

pub fn next_sequence_id(log_path: &Path) -> u32 {
    let file = match fs::File::open(log_path) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let reader = std::io::BufReader::new(file);
    let mut last_id = 0u32;
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
            if entry.id > last_id {
                last_id = entry.id;
            }
        }
    }
    last_id + 1
}

pub fn write_entry(log_path: &Path, entry: &LogEntry) -> anyhow::Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string(entry)?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{}", json)?;

    Ok(())
}

fn base_entry(version: &str, log_path: &Path) -> LogEntry {
    let id = next_sequence_id(log_path);
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%S%:z")
        .to_string();
    LogEntry {
        id,
        timestamp,
        version: version.to_string(),
        entry_type: "trace".to_string(),
        width: 0,
        width_source: String::new(),
        is_ssh: false,
        output_targets: vec![],
        input: None,
        trace: vec![],
        warnings: vec![],
        error: None,
        flagged: false,
        flagged_comment: None,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_trace_entry(
    version: &str,
    width: usize,
    width_source: String,
    is_ssh: bool,
    output_targets: Vec<String>,
    input: &str,
    trace: Vec<String>,
    warnings: Vec<Warning>,
    log_path: &Path,
) -> LogEntry {
    let mut entry = base_entry(version, log_path);
    entry.entry_type = "trace".to_string();
    entry.width = width;
    entry.width_source = width_source;
    entry.is_ssh = is_ssh;
    entry.output_targets = output_targets;
    entry.input = Some(input.to_string());
    entry.trace = trace;
    entry.warnings = warnings;
    entry
}

pub fn build_error_entry(
    version: &str,
    width: usize,
    width_source: String,
    is_ssh: bool,
    error_msg: &str,
    log_path: &Path,
) -> LogEntry {
    let mut entry = base_entry(version, log_path);
    entry.entry_type = "error".to_string();
    entry.width = width;
    entry.width_source = width_source;
    entry.is_ssh = is_ssh;
    entry.error = Some(error_msg.to_string());
    entry
}

pub fn flag_last_entry(log_path: &Path, comment: Option<&str>) -> anyhow::Result<()> {
    let content = fs::read_to_string(log_path)
        .map_err(|_| anyhow::anyhow!("debug log not found: {}", log_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        anyhow::bail!("no log entries found in {}", log_path.display());
    }

    // Find the last valid JSON line
    let mut last_idx = None;
    for (i, line) in lines.iter().enumerate().rev() {
        if serde_json::from_str::<LogEntry>(line).is_ok() {
            last_idx = Some(i);
            break;
        }
    }

    let idx = last_idx
        .ok_or_else(|| anyhow::anyhow!("no valid log entries found in {}", log_path.display()))?;

    let mut entry: LogEntry = serde_json::from_str(lines[idx])?;
    entry.flagged = true;
    entry.flagged_comment = comment.map(|c| c.to_string()).filter(|c| !c.is_empty());

    let updated_line = serde_json::to_string(&entry)?;

    let mut output = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i == idx {
            output.push_str(&updated_line);
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }

    fs::write(log_path, output)?;

    eprintln!("yfix: flagged entry #{:03}", entry.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id_from_empty() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp.path(), "").unwrap();
        assert_eq!(next_sequence_id(tmp.path()), 1);
    }

    #[test]
    fn next_id_from_nonexistent() {
        assert_eq!(next_sequence_id(Path::new("/nonexistent/file")), 1);
    }

    #[test]
    fn write_and_read_entry() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let entry = build_trace_entry(
            "test",
            80,
            "TmuxPane".to_string(),
            false,
            vec!["tmux-buffer".to_string()],
            "hello",
            vec!["[input] 1 lines".to_string()],
            vec![],
            tmp.path(),
        );
        write_entry(tmp.path(), &entry).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        let parsed: LogEntry = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.width, 80);
        assert!(!parsed.flagged);
    }

    #[test]
    fn sequential_ids() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        for _ in 0..3 {
            let entry = build_trace_entry(
                "test",
                80,
                "TmuxPane".to_string(),
                false,
                vec![],
                "hello",
                vec![],
                vec![],
                tmp.path(),
            );
            write_entry(tmp.path(), &entry).unwrap();
        }
        let content = fs::read_to_string(tmp.path()).unwrap();
        let entries: Vec<LogEntry> = content
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, 1);
        assert_eq!(entries[1].id, 2);
        assert_eq!(entries[2].id, 3);
    }

    #[test]
    fn write_entry_with_warnings() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let entry = build_trace_entry(
            "test",
            80,
            "TmuxPane".to_string(),
            false,
            vec![],
            "hello",
            vec![],
            vec![Warning::LineNumbersBorderline { match_rate_pct: 55 }],
            tmp.path(),
        );
        write_entry(tmp.path(), &entry).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("line_numbers_borderline"));
        assert!(content.contains("55"));
    }

    #[test]
    fn flag_last_entry_with_comment() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let entry = build_trace_entry(
            "test",
            80,
            "T".into(),
            false,
            vec![],
            "hello",
            vec![],
            vec![],
            tmp.path(),
        );
        write_entry(tmp.path(), &entry).unwrap();

        flag_last_entry(tmp.path(), Some("bad result")).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        let parsed: LogEntry = serde_json::from_str(content.trim()).unwrap();
        assert!(parsed.flagged);
        assert_eq!(parsed.flagged_comment.as_deref(), Some("bad result"));
    }

    #[test]
    fn flag_last_entry_without_comment() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let entry = build_trace_entry(
            "test",
            80,
            "T".into(),
            false,
            vec![],
            "hello",
            vec![],
            vec![],
            tmp.path(),
        );
        write_entry(tmp.path(), &entry).unwrap();

        flag_last_entry(tmp.path(), None).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        let parsed: LogEntry = serde_json::from_str(content.trim()).unwrap();
        assert!(parsed.flagged);
        assert!(parsed.flagged_comment.is_none());
    }

    #[test]
    fn flag_only_last_of_multiple() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        for _ in 0..3 {
            let entry = build_trace_entry(
                "test",
                80,
                "T".into(),
                false,
                vec![],
                "hello",
                vec![],
                vec![],
                tmp.path(),
            );
            write_entry(tmp.path(), &entry).unwrap();
        }

        flag_last_entry(tmp.path(), Some("oops")).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();
        let entries: Vec<LogEntry> = content
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        assert!(!entries[0].flagged);
        assert!(!entries[1].flagged);
        assert!(entries[2].flagged);
    }
}
