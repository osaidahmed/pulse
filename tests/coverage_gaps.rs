mod common;

use common::*;
use std::process::Command;

fn pulse_bin() -> String {
    env!("CARGO_BIN_EXE_pulse").to_string()
}

// ===========================================================================
// main.rs coverage: CLI paths
// ===========================================================================

#[test]
fn invalid_args_prints_usage() {
    let out = Command::new(pulse_bin())
        .args(["garbage"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("usage:"), "should print usage on bad args: {}", stderr);
}

#[test]
fn check_unsupported_file_silent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.xyz");
    std::fs::write(&path, "hello world").unwrap();
    let out = Command::new(pulse_bin())
        .args(["check", path.to_str().unwrap()])
        .output()
        .unwrap();
    // Should exit 0 silently (unsupported language)
    assert!(String::from_utf8_lossy(&out.stdout).is_empty());
}

#[test]
fn check_nonexistent_file_silent() {
    let out = Command::new(pulse_bin())
        .args(["check", "/nonexistent/file.py"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).is_empty());
}

#[test]
fn debug_unsupported_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.xyz");
    std::fs::write(&path, "hello world").unwrap();
    let out = Command::new(pulse_bin())
        .args(["debug", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
}
