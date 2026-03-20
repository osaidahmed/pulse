use std::path::PathBuf;
use std::process::Command;

pub fn fixtures_dir(lang: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(lang)
}

pub fn run_check(lang: &str, fixture: &str) -> String {
    let path = fixtures_dir(lang).join(fixture);
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    String::from_utf8(output.stdout).unwrap()
}

pub fn run_debug(lang: &str, fixture: &str) -> String {
    let path = fixtures_dir(lang).join(fixture);
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    String::from_utf8(output.stderr).unwrap()
}

pub fn run_hook(file_path: &str) -> String {
    let json = format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, file_path);
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(json.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run pulse --hook");
    String::from_utf8(output.stdout).unwrap()
}

pub fn has_smell(output: &str, smell: &str) -> bool {
    output.contains(smell)
}

pub fn has_function(output: &str, func_name: &str) -> bool {
    output.contains(func_name)
}

pub fn function_metric(debug_output: &str, func_name: &str, metric: &str) -> Option<u32> {
    for line in debug_output.lines() {
        if line.contains(func_name) {
            for part in line.split_whitespace() {
                if part.starts_with(&format!("{}=", metric)) {
                    return part.split('=').nth(1)?.parse().ok();
                }
            }
        }
    }
    None
}

pub fn pulse_check_code(code: &str, ext: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{}", ext));
    std::fs::write(&path, code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    String::from_utf8(out.stdout).unwrap()
}

pub fn pulse_debug_code(code: &str, ext: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{}", ext));
    std::fs::write(&path, code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    String::from_utf8(out.stderr).unwrap()
}
