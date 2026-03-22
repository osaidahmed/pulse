use std::path::PathBuf;
use std::process::Command;

pub use pulse::thresholds::Thresholds;

pub fn t() -> Thresholds {
    Thresholds::default()
}

pub fn fn_padding() -> usize {
    t().fn_loc_warning as usize + 20
}

pub fn file_padding() -> usize {
    t().file_loc_warning as usize + 100
}

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
    // Copy test/fixture files to a tempdir so the hook doesn't skip them
    let (actual_path, _tmpdir) = if file_path.contains("/tests/") || file_path.contains("/test/") {
        let dir = tempfile::tempdir().unwrap();
        let name = std::path::Path::new(file_path).file_name().unwrap();
        let dest = dir.path().join(name);
        std::fs::copy(file_path, &dest).unwrap_or(0);
        (dest.to_str().unwrap().to_string(), Some(dir))
    } else {
        (file_path.to_string(), None)
    };
    let json = format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, actual_path);
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

// ── Threshold-derived generation helpers ──

pub fn cc_branches() -> usize {
    t().cc_warning as usize + 1
}

pub fn declarations_above() -> usize {
    t().max_declarations as usize + 5
}

pub fn functions_above() -> usize {
    t().file_function_count as usize + 1
}

pub fn struct_fields_at() -> usize {
    t().max_struct_fields as usize
}

pub fn struct_fields_above() -> usize {
    t().max_struct_fields as usize + 3
}

pub fn asserts_at() -> usize {
    t().consecutive_asserts_max as usize
}

pub fn asserts_above() -> usize {
    t().consecutive_asserts_max as usize + 5
}

pub fn large_fn_lines() -> usize {
    t().large_fn_loc as usize + 15
}

pub fn embedded_lines_above() -> usize {
    t().embedded_block_loc as usize + 5
}

pub fn args_above() -> usize {
    t().arg_max as usize + 1
}

#[macro_export]
macro_rules! lang_helpers {
    ($ext:expr) => {
        fn check(code: &str) -> String { pulse_check_code(code, $ext) }
        fn debug(code: &str) -> String { pulse_debug_code(code, $ext) }
    };
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
