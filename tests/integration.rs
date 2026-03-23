use std::process::Command;

fn yfix(args: &[&str], stdin: &str) -> (String, String, i32) {
    use std::io::Write;
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_yfix"))
        .args(args)
        .env("YFIX_DEBUG_OVERRIDE", "off")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn yfix");

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = cmd.wait_with_output().unwrap();

    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn stdout_output_with_ansi_stripped() {
    let (out, _err, code) = yfix(&["--output", "stdout"], "\x1b[31mhello\x1b[0m world   ");
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "hello world");
}

#[test]
fn stdout_output_with_line_numbers() {
    let input = "  1 hello\n  2 world\n  3 foo";
    let (out, _err, code) = yfix(&["--output", "stdout"], input);
    assert_eq!(code, 0);
    assert!(out.contains("hello"));
    assert!(!out.contains("  1 "));
}

#[test]
fn empty_input_exits_zero() {
    let (out, _err, code) = yfix(&["--output", "stdout"], "");
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "");
}

#[test]
fn show_terminal_outputs_to_stderr() {
    let (_out, err, code) = yfix(&["--output", "stdout", "--show-terminal"], "hello");
    assert_eq!(code, 0);
    assert!(err.contains("[yfix] env:"));
    assert!(err.contains("[yfix] width:"));
    assert!(err.contains("[yfix] output:"));
}

#[test]
fn help_ai_outputs_markdown() {
    let (out, _err, code) = yfix(&["--help-ai"], "");
    assert_eq!(code, 0);
    assert!(out.contains("# yfix"));
    assert!(out.contains("tmux copy-mode"));
}
