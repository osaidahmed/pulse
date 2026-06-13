use std::io::Write;
use std::path::Path;
use std::process::Command;

fn hook(file: &Path, baseline: &Path) -> String {
    let json = format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, file.display());
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", baseline)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
            c.wait_with_output()
        })
        .expect("hook");
    String::from_utf8(output.stdout).unwrap()
}

const HELPER: &str = "def helper(items):\n    total = 0\n    count = 0\n    for item in items:\n        if item > 0:\n            total += item\n            count += item\n    return total + count\n";

fn write(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn duplicating_a_function_into_a_second_file_is_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let bl = tempfile::tempdir().unwrap();
    let alpha = write(dir.path(), "alpha.py", HELPER);
    let beta = write(dir.path(), "beta.py", HELPER);
    assert!(!hook(&alpha, bl.path()).contains("cross-file duplication"), "first file has nothing to clone yet");
    let out = hook(&beta, bl.path());
    assert!(out.contains("cross-file duplication"), "second identical file is flagged: {out}");
    assert!(out.contains("alpha.py"), "names the file it duplicates: {out}");
}

#[test]
fn a_structurally_different_function_is_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let bl = tempfile::tempdir().unwrap();
    let alpha = write(dir.path(), "alpha.py", HELPER);
    let other =
        "def other(data):\n    result = []\n    while data:\n        result.append(data.pop())\n    return result\n";
    let beta = write(dir.path(), "beta.py", other);
    hook(&alpha, bl.path());
    assert!(!hook(&beta, bl.path()).contains("cross-file duplication"));
}

#[test]
fn re_editing_the_same_file_is_not_a_cross_file_clone() {
    let dir = tempfile::tempdir().unwrap();
    let bl = tempfile::tempdir().unwrap();
    let alpha = write(dir.path(), "alpha.py", HELPER);
    hook(&alpha, bl.path());
    assert!(!hook(&alpha, bl.path()).contains("cross-file duplication"), "a file never clones itself");
}

#[test]
fn a_function_below_the_size_floor_is_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let bl = tempfile::tempdir().unwrap();
    let tiny = "def tiny(x):\n    return x + 1\n";
    let alpha = write(dir.path(), "alpha.py", tiny);
    let beta = write(dir.path(), "beta.py", tiny);
    hook(&alpha, bl.path());
    assert!(!hook(&beta, bl.path()).contains("cross-file duplication"), "trivial functions stay below the floor");
}
