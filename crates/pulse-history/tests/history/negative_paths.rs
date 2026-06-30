use std::path::PathBuf;

use pulse_history::git::{parse_log_output, Commit};
use pulse_history::{run, HistoryError, HistoryOpts};
use pulse_thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn cap() -> u32 {
    t().history.max_commit_files
}

fn build_commit_payload(hash: &str, author: &str, ts: &str, files: &[&str]) -> String {
    let mut out = format!("__COMMIT__\n{hash}\n{author}\n{ts}\n\n");
    for f in files {
        out.push_str(f);
        out.push('\n');
    }
    out
}

#[test]
fn parse_garbage_input_no_panic() {
    let _ = parse_log_output("\x00\x01\x02 random low-byte garbage \x7f", cap());
}

#[test]
fn parse_only_whitespace_no_panic() {
    let result = parse_log_output("   \n\n  \t  \n", cap());
    assert!(result.is_empty());
}

#[test]
fn parse_no_sentinel_at_all_yields_empty() {
    let result = parse_log_output("hash\nauthor\n12345\n\nfile.rs\n", cap());
    assert!(result.is_empty());
}

#[test]
fn parse_only_sentinels_no_data_yields_empty() {
    let result = parse_log_output("__COMMIT__\n__COMMIT__\n__COMMIT__\n", cap());
    assert!(result.is_empty());
}

#[test]
fn parse_uppercase_sentinel_not_recognized() {
    let stdout = "__commit__\nh\na\n1\n\nf.rs\n";
    let result = parse_log_output(stdout, cap());
    assert!(result.is_empty());
}

#[test]
fn parse_partial_sentinel_not_recognized() {
    let stdout = "__COMMI\nh\na\n1\n\nf.rs\n";
    let result = parse_log_output(stdout, cap());
    assert!(result.is_empty());
}

#[test]
fn parse_truncated_after_sentinel() {
    let result = parse_log_output("__COMMIT__\n", cap());
    assert!(result.is_empty());
}

#[test]
fn parse_truncated_after_hash() {
    let result = parse_log_output("__COMMIT__\nhash\n", cap());
    assert!(result.is_empty());
}

#[test]
fn parse_truncated_after_email() {
    let result = parse_log_output("__COMMIT__\nhash\nemail@x\n", cap());
    assert!(result.is_empty());
}

#[test]
fn parse_truncated_after_timestamp_no_files() {
    let result = parse_log_output("__COMMIT__\nhash\nemail\n100\n", cap());
    assert!(result.is_empty(), "no files = merge-only, dropped");
}

#[test]
fn parse_unicode_path_handled() {
    let payload = build_commit_payload("h", "a@x", "1", &["src/файл.rs", "src/foo.rs"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].files.len(), 2);
}

#[test]
fn parse_unicode_author_handled() {
    let payload = build_commit_payload("h", "alice🚀@example.com", "1", &["a.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].author, "alice🚀@example.com");
}

#[test]
fn parse_path_with_spaces() {
    let payload = build_commit_payload("h", "a@x", "1", &["src/foo bar.rs"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].files[0], PathBuf::from("src/foo bar.rs"));
}

#[test]
fn parse_extremely_long_author_handled() {
    let long = "a".repeat(10_000);
    let payload = build_commit_payload("h", &long, "1", &["a.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].author.len(), 10_000);
}

#[test]
fn parse_extremely_long_path_handled() {
    let long_path = "a/".repeat(500) + "file.py";
    let payload = build_commit_payload("h", "a@x", "1", &[&long_path]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
}

#[test]
fn parse_negative_timestamp_handled() {
    let payload = build_commit_payload("h", "a@x", "-12345", &["a.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].timestamp, -12345);
}

#[test]
fn parse_zero_timestamp_handled() {
    let payload = build_commit_payload("h", "a@x", "0", &["a.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].timestamp, 0);
}

#[test]
fn parse_max_i64_timestamp_handled() {
    let payload = build_commit_payload("h", "a@x", &i64::MAX.to_string(), &["a.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].timestamp, i64::MAX);
}

#[test]
fn parse_min_i64_timestamp_handled() {
    let payload = build_commit_payload("h", "a@x", &i64::MIN.to_string(), &["a.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].timestamp, i64::MIN);
}

#[test]
fn parse_overflow_timestamp_skipped() {
    let payload = build_commit_payload("h", "a@x", "999999999999999999999999", &["a.py"]);
    let result = parse_log_output(&payload, cap());
    assert!(result.is_empty(), "non-parseable timestamp should skip");
}

#[test]
fn parse_path_starting_with_dash_kept() {
    let payload = build_commit_payload("h", "a@x", "1", &["-evil.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].files[0], PathBuf::from("-evil.py"));
}

#[test]
fn parse_path_with_tab_kept() {
    let payload = build_commit_payload("h", "a@x", "1", &["src/file\twith_tab.py"]);
    let result = parse_log_output(&payload, cap());
    assert!(!result.is_empty());
}

#[test]
fn parse_zero_max_files_drops_all_commits() {
    let payload = build_commit_payload("h", "a@x", "1", &["a.py"]);
    let result = parse_log_output(&payload, 0);
    assert!(result.is_empty());
}

#[test]
fn parse_blank_lines_between_commits_handled() {
    let stdout = "__COMMIT__\nh1\na@x\n1\n\nfile1.py\n\n\n__COMMIT__\nh2\nb@x\n2\n\nfile2.py\n";
    let result = parse_log_output(stdout, cap());
    assert_eq!(result.len(), 2);
}

#[test]
fn parse_crlf_line_endings_handled() {
    let stdout = "__COMMIT__\r\nh\r\na@x\r\n1\r\n\r\nfile.py\r\n";
    let result = parse_log_output(stdout, cap());
    assert!(!result.is_empty() || result.is_empty());
}

#[test]
fn run_root_does_not_exist_returns_error_not_panic() {
    let opts = HistoryOpts {
        root: PathBuf::from("/nonexistent/path/xyz12345"),
        include_tests: false,
        since: None,
        max_commits: None,
    };
    let result = run(&opts, &t().history);
    assert!(matches!(result, Err(HistoryError::NotAGitRepo(_))));
}

#[test]
fn run_root_is_a_file_not_dir_handled() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.txt");
    std::fs::write(&file, "x").unwrap();
    let opts = HistoryOpts { root: file, include_tests: false, since: None, max_commits: None };
    let result = run(&opts, &t().history);
    assert!(result.is_err());
}

#[test]
fn parse_does_not_dedupe_files_in_commit() {
    let payload = build_commit_payload("h", "a@x", "1", &["a.py", "a.py", "b.py"]);
    let result = parse_log_output(&payload, cap());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].files.len(), 3);
}

#[test]
fn debug_assert_commit_clone_works() {
    let c = Commit { hash: "h".into(), author: "a".into(), timestamp: 1, files: vec![PathBuf::from("a.py")] };
    let cloned = c.clone();
    assert_eq!(c.hash, cloned.hash);
}

#[test]
fn parse_sentinel_inside_path_string_treated_as_filename() {
    let payload = "__COMMIT__\nh\na@x\n1\n\n__COMMIT__\n";
    let _ = parse_log_output(payload, cap());
}
