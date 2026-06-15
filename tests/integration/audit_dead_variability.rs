use pulse::audit::finding::{AuditFinding, AuditKind};
use pulse::audit::{self, AuditOpts, PassChoice};
use pulse::config::AuditSuppression;
use std::path::Path;

use crate::audit_common::t;

fn run_dead(root: &Path) -> Vec<AuditFinding> {
    let opts = AuditOpts {
        root: root.to_path_buf(),
        pass: Some(PassChoice::DeadVariability),
        json: false,
        include_tests: false,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    audit::run(&opts, &t().audit)
}

fn dead_macros(findings: &[AuditFinding]) -> Vec<String> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::DeadConditionalBranch(e) => Some(e.macro_name.clone()),
            _ => None,
        })
        .collect()
}

const SRC: &str = "#ifndef FOO\nint dead_one(void) { return 1; }\n#endif\n\n#ifdef FOO\nint live(void) { return 2; }\n#else\nint dead_two(void) { return 3; }\n#endif\n\n#ifdef MISSING\nint maybe(void) { return 4; }\n#endif\n";

fn project_with_compdb(define_flags: &str, src: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("src/feature.c");
    std::fs::create_dir_all(src_path.parent().unwrap()).unwrap();
    std::fs::write(&src_path, src).unwrap();
    let abs = src_path.to_string_lossy();
    let ccj = format!(
        "[{{\"directory\":\"{}\",\"command\":\"cc {} -c {}\",\"file\":\"{}\"}}]",
        dir.path().display(),
        define_flags,
        abs,
        abs
    );
    std::fs::write(dir.path().join("compile_commands.json"), ccj).unwrap();
    dir
}

#[test]
fn ifndef_and_else_of_defined_macro_are_dead() {
    let dir = project_with_compdb("-DFOO", SRC);
    let macros = dead_macros(&run_dead(dir.path()));
    assert_eq!(macros.len(), 2, "the #ifndef body and the #else branch are both unreachable: {macros:?}");
    assert!(macros.iter().all(|m| m == "FOO"), "{macros:?}");
}

#[test]
fn undefined_macro_branch_is_not_flagged() {
    let src = "#ifdef MISSING\nint x(void) { return 1; }\n#endif\n";
    let dir = project_with_compdb("-DFOO", src);
    assert!(
        dead_macros(&run_dead(dir.path())).is_empty(),
        "a macro absent from -D may be #defined in an include — not soundly dead"
    );
}

#[test]
fn undef_in_file_suppresses_the_finding() {
    let src = "#undef FOO\n#ifndef FOO\nint x(void) { return 1; }\n#endif\n";
    let dir = project_with_compdb("-DFOO", src);
    assert!(
        dead_macros(&run_dead(dir.path())).is_empty(),
        "a #undef of the macro makes the -D-based conclusion unsound"
    );
}

#[test]
fn no_compile_db_yields_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("src/feature.c");
    std::fs::create_dir_all(src_path.parent().unwrap()).unwrap();
    std::fs::write(&src_path, SRC).unwrap();
    assert!(dead_macros(&run_dead(dir.path())).is_empty(), "no compile_commands.json → no dead-variability findings");
}
