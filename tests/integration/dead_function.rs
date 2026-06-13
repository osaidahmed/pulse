use std::process::Command;

struct GitProject {
    dir: tempfile::TempDir,
    baseline_dir: tempfile::TempDir,
}

impl GitProject {
    fn new() -> Self {
        let p = Self { dir: tempfile::tempdir().unwrap(), baseline_dir: tempfile::tempdir().unwrap() };
        p.git(&["init", "-q"]);
        p
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["-c", "user.email=t@example.com", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git available");
        assert!(output.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    fn write(&self, name: &str, body: &str) {
        std::fs::write(self.dir.path().join(name), body).unwrap();
    }

    fn commit_all(&self) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", "x", "--allow-empty"]);
    }

    fn pulse(&self, flag: &str) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
            .arg(flag)
            .current_dir(self.dir.path())
            .env("PULSE_BASELINE_DIR", self.baseline_dir.path())
            .stdin(std::process::Stdio::null())
            .output()
            .expect("failed to run pulse");
        String::from_utf8(output.stdout).unwrap()
    }
}

fn flags(out: &str) -> bool {
    out.to_lowercase().contains("unused function")
}

#[test]
fn an_orphaned_new_function_is_flagged() {
    let p = GitProject::new();
    p.write("app.py", "def entrypoint():\n    return 0\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("app.py", "def entrypoint():\n    return 0\n\ndef compute_zztop_metric():\n    return 42\n");
    let out = p.pulse("--stop");
    assert!(flags(&out), "an added function referenced nowhere is flagged: {out}");
    assert!(out.contains("compute_zztop_metric"), "names the orphan: {out}");
}

#[test]
fn a_new_function_called_by_existing_code_is_not_flagged() {
    let p = GitProject::new();
    p.write("app.py", "def entrypoint():\n    return 0\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("app.py", "def entrypoint():\n    return helper_zztop()\n\ndef helper_zztop():\n    return 1\n");
    let out = p.pulse("--stop");
    assert!(!flags(&out), "a called function is not dead: {out}");
}

#[test]
fn a_function_referenced_as_a_callback_is_not_flagged() {
    let p = GitProject::new();
    p.write("app.py", "def setup():\n    return 0\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("app.py", "def setup():\n    return [on_zztop]\n\ndef on_zztop():\n    return 1\n");
    let out = p.pulse("--stop");
    assert!(!flags(&out), "a name passed as a reference counts as used: {out}");
}

#[test]
fn a_decorated_new_function_is_not_flagged() {
    let p = GitProject::new();
    p.write("base.py", "def entrypoint():\n    return 0\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("routes.py", "@register\ndef zztop_route():\n    return 1\n");
    let out = p.pulse("--stop");
    assert!(!flags(&out), "a decorated function is framework-managed: {out}");
}

#[test]
fn a_dunder_method_is_not_flagged() {
    let p = GitProject::new();
    p.write("widget.py", "class Widget:\n    pass\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("widget.py", "class Widget:\n    def __init__(self):\n        self.x = 1\n");
    let out = p.pulse("--stop");
    assert!(!flags(&out), "dunder methods are invoked by the language: {out}");
}

#[test]
fn a_preexisting_uncalled_function_is_not_flagged() {
    let p = GitProject::new();
    p.write("app.py", "def lonely_zztop():\n    return 1\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("app.py", "# a trailing comment\ndef lonely_zztop():\n    return 1\n");
    let out = p.pulse("--stop");
    assert!(!out.contains("lonely_zztop"), "only functions added this session are considered: {out}");
}
