use pulse_audit::finding::{AuditFinding, AuditKind};
use pulse_audit::suppression::AuditSuppression;
use pulse_audit::{self as audit, AuditOpts, PassChoice};
use std::path::Path;

use crate::audit_common::t;

fn run_ifdef(root: &Path) -> Vec<AuditFinding> {
    let opts = AuditOpts {
        root: root.to_path_buf(),
        pass: Some(PassChoice::IfdefDensity),
        json: false,
        include_tests: false,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    audit::run(&opts, &t().audit)
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    dir
}

fn densities(findings: &[AuditFinding]) -> Vec<(String, u32)> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::IfdefDensity(e) => Some((e.file.to_string_lossy().into_owned(), e.conditional_count)),
            _ => None,
        })
        .collect()
}

fn conditional_blocks(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&format!("#ifdef FEATURE_{i}\nint v_{i} = {i};\n#endif\n"));
    }
    s
}

#[test]
fn c_file_with_many_conditionals_is_flagged() {
    let min = t().audit.ifdef_min_conditionals as usize;
    let dir = project(&[("src/platform.c", &conditional_blocks(min + 4))]);
    let found = densities(&run_ifdef(dir.path()));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].0.ends_with("platform.c"));
    assert_eq!(found[0].1 as usize, min + 4, "exact conditional-block count is reported");
}

#[test]
fn c_file_below_threshold_is_not_flagged() {
    let dir = project(&[("src/small.c", &conditional_blocks(2))]);
    assert!(densities(&run_ifdef(dir.path())).is_empty(), "a couple of conditionals is below the debt threshold");
}

#[test]
fn cpp_conditionals_are_also_counted() {
    let min = t().audit.ifdef_min_conditionals as usize;
    let dir = project(&[("src/engine.cpp", &conditional_blocks(min + 1))]);
    let found = densities(&run_ifdef(dir.path()));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].0.ends_with("engine.cpp"));
}

#[test]
fn objc_conditionals_are_also_counted() {
    let min = t().audit.ifdef_min_conditionals as usize;
    let dir = project(&[("src/view.m", &conditional_blocks(min + 1))]);
    let found = densities(&run_ifdef(dir.path()));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].0.ends_with("view.m"));
}

#[test]
fn non_c_family_file_is_not_flagged() {
    let min = t().audit.ifdef_min_conditionals as usize;
    let dir = project(&[("src/lib.rs", &conditional_blocks(min + 8))]);
    assert!(densities(&run_ifdef(dir.path())).is_empty(), "preprocessor conditionals only apply to C-family languages");
}

#[test]
fn findings_are_capped_at_max_keeping_the_densest() {
    let min = t().audit.ifdef_min_conditionals as usize;
    let max = t().audit.ifdef_max_findings as usize;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    for i in 0..(max + 5) {
        std::fs::write(dir.path().join(format!("src/f{i}.c")), conditional_blocks(min + 1 + i)).unwrap();
    }
    let found = densities(&run_ifdef(dir.path()));
    assert_eq!(found.len(), max, "capped at the max-findings ceiling, got {}", found.len());
    let densest = (min + 1 + (max + 4)) as u32;
    assert!(found.iter().any(|(_, c)| *c == densest), "the densest file is retained: {found:?}");
}

#[test]
fn ifdef_density_finding_renders_human_and_json() {
    let min = t().audit.ifdef_min_conditionals as usize;
    let dir = project(&[("src/platform.c", &conditional_blocks(min + 4))]);
    let findings = run_ifdef(dir.path());
    let human = pulse_audit::output::format_findings(&findings, Some(dir.path()), &t().audit);
    assert!(human.contains("conditional-compilation density"), "{human}");
    assert!(human.contains("platform.c"), "{human}");
    let json = pulse_audit::output::format_findings_json(&findings, Some(dir.path()));
    assert!(json.contains("\"IfdefDensity\""), "{json}");
}
