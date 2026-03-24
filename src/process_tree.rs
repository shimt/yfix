/// Returns true if any ancestor process of `start_pid` is an SSH or mosh daemon.
pub fn has_remote_ancestor(start_pid: u32) -> bool {
    walk_ancestors(start_pid, is_remote_process)
}

/// Returns true if `name` matches a known remote-access daemon.
fn is_remote_process(name: &str) -> bool {
    name == "sshd" || name.starts_with("sshd-") || name.contains("mosh-server")
}

/// Walk the process tree from `start_pid` toward PID 1.
/// Returns true if `predicate` matches any ancestor's process name.
fn walk_ancestors(start_pid: u32, predicate: fn(&str) -> bool) -> bool {
    let mut pid = start_pid;
    while pid > 1 {
        let (name, ppid) = match proc_info_impl(pid) {
            Some(info) => info,
            None => break,
        };
        if predicate(&name) {
            return true;
        }
        if ppid == 0 || ppid == pid {
            break;
        }
        pid = ppid;
    }
    false
}

#[cfg(target_os = "linux")]
fn proc_info_impl(pid: u32) -> Option<(String, u32)> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let mut name = None;
    let mut ppid = None;
    for line in status.lines() {
        if let Some(val) = line.strip_prefix("Name:\t") {
            name = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("PPid:\t") {
            ppid = val.trim().parse::<u32>().ok();
        }
        if name.is_some() && ppid.is_some() {
            break;
        }
    }
    Some((name?, ppid?))
}

#[cfg(target_os = "macos")]
fn proc_info_impl(pid: u32) -> Option<(String, u32)> {
    use std::mem;
    // Safety:
    // 1. `mem::zeroed()` is valid for `proc_bsdinfo`: it is a POD struct with no
    //    reference or non-null-pointer fields, so an all-zero bit pattern is valid.
    // 2. `proc_pidinfo` with `PROC_PIDTBSDINFO` writes exactly one `proc_bsdinfo`
    //    struct into `info` on success. The return value equals the struct size on
    //    success and is negative on failure; we check `ret < size` before reading.
    // 3. `CStr::from_ptr(pbi_name/pbi_comm)`: both arrays are zero-initialized and
    //    the kernel fills them via `strlcpy`, guaranteeing a null byte within bounds.
    unsafe {
        let mut info: libc::proc_bsdinfo = mem::zeroed();
        let size = mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
        let ret = libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        );
        if ret < size {
            return None;
        }
        let name = std::ffi::CStr::from_ptr(info.pbi_name.as_ptr())
            .to_string_lossy()
            .into_owned();
        // Fall back to pbi_comm if pbi_name is empty (short name, MAXCOMLEN chars).
        let name = if name.is_empty() {
            std::ffi::CStr::from_ptr(info.pbi_comm.as_ptr())
                .to_string_lossy()
                .into_owned()
        } else {
            name
        };
        let ppid = info.pbi_ppid;
        Some((name, ppid))
    }
}

#[cfg(all(unix, not(target_os = "linux"), not(target_os = "macos")))]
fn proc_info_impl(pid: u32) -> Option<(String, u32)> {
    proc_info_ps_fallback(pid)
}

#[cfg(not(unix))]
fn proc_info_impl(_pid: u32) -> Option<(String, u32)> {
    None
}

#[cfg(all(unix, not(target_os = "linux"), not(target_os = "macos")))]
fn proc_info_ps_fallback(pid: u32) -> Option<(String, u32)> {
    use std::process::Command;
    let out = Command::new("ps")
        .args(["-o", "comm=,ppid=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    let mut parts = s.split_whitespace();
    let name = parts.next()?.to_string();
    let ppid = parts.next()?.parse::<u32>().ok()?;
    Some((name, ppid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_is_not_remote() {
        // The test runner itself should not have sshd/mosh-server as ancestor.
        assert!(!has_remote_ancestor(std::process::id()));
    }

    #[test]
    fn pid_1_terminates_walk() {
        // Walking from PID 1 should return false without infinite loop.
        assert!(!has_remote_ancestor(1));
    }

    #[test]
    fn nonexistent_pid_returns_false() {
        // PID 9999999 almost certainly doesn't exist.
        assert!(!has_remote_ancestor(9_999_999));
    }

    #[test]
    fn is_remote_process_matches_sshd() {
        assert!(is_remote_process("sshd"));
        assert!(is_remote_process("sshd-session"));
        assert!(is_remote_process("mosh-server"));
        assert!(!is_remote_process("bash"));
        assert!(!is_remote_process("tmux"));
        assert!(!is_remote_process("cargo-test"));
    }
}
