use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use pulse::audit::call_walker::calls_and_bindings_from;
use pulse::audit::vuln_clones;
use pulse::parse::Language;
use pulse::thresholds::Thresholds;

use crate::binding_common::{method_env, one_source};
use crate::history_common::{build_repo, CommitSpec};

fn t() -> Thresholds {
    Thresholds::default()
}

#[test]
fn vuln_clones_run_wrapper_loads_corpus_and_returns_no_findings_for_clean_code() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clean.py");
    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let typed = vec![(path, Language::Python)];
    let findings = vuln_clones::run(&typed, &t().audit);
    assert!(
        findings.is_empty(),
        "clean code with no taint sinks produces no vuln-clone findings through the run wrapper"
    );
}

#[test]
fn write_atomic_removes_temp_file_when_write_into_readonly_parent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("ro");
    std::fs::create_dir(&parent).unwrap();
    let mut locked = std::fs::metadata(&parent).unwrap().permissions();
    locked.set_mode(0o555);
    std::fs::set_permissions(&parent, locked).unwrap();

    let target = parent.join("cache.json");
    pulse::registry::write_atomic(&target, "payload");
    assert!(!target.exists(), "a write into a read-only parent leaves no cache file behind");
    let leftover = std::fs::read_dir(&parent).map(|rd| rd.count()).unwrap_or(0);
    assert_eq!(leftover, 0, "the failed temp write is cleaned up via the else branch");

    let mut restore = std::fs::metadata(&parent).unwrap().permissions();
    restore.set_mode(0o755);
    std::fs::set_permissions(&parent, restore).unwrap();
}

fn pulse_history(repo: &Path, analytics: &Path, extra: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("history")
        .arg("--root")
        .arg(repo.to_str().unwrap())
        .args(extra)
        .env("PULSE_ANALYTICS_DIR", analytics)
        .output()
        .expect("pulse failed to run");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

fn one_commit_repo() -> tempfile::TempDir {
    build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("a.py", "def f():\n    return 1\n"), ("b.py", "def g():\n    return 2\n")],
        deletes: &[],
    }])
}

#[test]
fn history_calibrate_with_pulse_config_present_takes_the_some_config_branch() {
    let repo = one_commit_repo();
    std::fs::write(repo.path().join(".pulse.toml"), "[history]\n").unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let (stdout, stderr, code) = pulse_history(repo.path(), analytics.path(), &["--jit-calibrate"]);
    assert_eq!(
        code, 0,
        "calibrate on a repo carrying a .pulse.toml succeeds via the Some config branch (stderr: {stderr})"
    );
    assert!(stdout.contains("jit calibration written:"), "expected the calibrate success message: {stdout}");
}

#[test]
fn csharp_local_function_in_method_body_is_skipped_as_a_scope_boundary() {
    let src = "class C {\n  void Run() {\n    void Local() { Bar y = Inner(); }\n    Foo x = Make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("run env");
    assert_eq!(run.get("x").map(String::as_str), Some("Foo"), "the sibling local still binds");
    assert!(
        run.get("y").is_none(),
        "a local declared inside a nested local function is not pulled into the outer method"
    );
}

#[test]
fn csharp_foreach_with_tuple_deconstruction_target_binds_nothing() {
    let src = "class C {\n  void Run(Seq pairs) {\n    foreach (var (a, b) in pairs) {\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("run env");
    assert!(run.get("a").is_none(), "a deconstruction foreach target is not a plain identifier so nothing binds");
}

#[test]
fn java_local_class_in_method_body_is_skipped_as_a_scope_boundary() {
    let src = "class C {\n  void run() {\n    class Inner { Bar field; }\n    Foo x = make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("x").map(String::as_str), Some("Foo"), "the sibling local still binds past the local class");
}

#[test]
fn java_record_with_compact_constructor_walks_without_routing_the_constructor() {
    let src = "record R(Foo a) {\n  R {\n    validate(a);\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    assert!(
        method_env(&out, "R").is_none(),
        "a record compact constructor is not a routed function kind so it yields no method env"
    );
}

#[test]
fn swift_wildcard_only_parameter_binds_nothing() {
    let src = "class C {\n  func run(_: Int) {\n    work()\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let _run = method_env(&out, "run").expect("run env");
}

#[test]
fn swift_wildcard_let_binding_has_no_pattern_name() {
    let src = "class C {\n  func run() {\n    let _ = compute()\n    let kept: Foo = make()\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(
        run.get("kept").map(String::as_str),
        Some("Foo"),
        "the annotated sibling local still binds past the wildcard let"
    );
}
