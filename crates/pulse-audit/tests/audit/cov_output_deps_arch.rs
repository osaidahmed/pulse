use std::path::{Path, PathBuf};

use pulse_audit::finding::{
    AuditFinding, AuditKind, BloatedDepEvidence, ConstraintEvidence, IfdefDensityEvidence, ImportConfidence,
    OutdatedDepEvidence, OverFragmentationEvidence, PhantomDepEvidence, StrictnessEvidence,
    UndeclaredModuleDepEvidence, UnusedDeclaredDepEvidence, VulnDepEvidence,
};
use pulse_audit::output::arch::{arch_json, write_arch};
use pulse_audit::output::deps::{dispatch_human, dispatch_json};

fn finding_for(kind: AuditKind) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}

fn human(kind: AuditKind, action: &'static str) -> String {
    let f = finding_for(kind);
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, None, action));
    out
}

fn human_rooted(kind: AuditKind, root: &Path, action: &'static str) -> String {
    let f = finding_for(kind);
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, Some(root), action));
    out
}

fn json(kind: AuditKind) -> serde_json::Value {
    let f = finding_for(kind);
    dispatch_json(&f, None).expect("dispatch_json yields a value")
}

#[test]
fn bloated_with_constraint_and_action_renders_all_lines() {
    let e = BloatedDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 14,
        name: "leftpad".to_string(),
        constraint: "^1.2".to_string(),
        confidence: ImportConfidence::High,
    };
    let out = human(AuditKind::BloatedDependency(e), "remove it");
    assert!(out.contains("audit: bloated dependency — leftpad"), "{out}");
    assert!(out.contains("declared at:   Cargo.toml:14"), "{out}");
    assert!(out.contains("constraint:    ^1.2"), "{out}");
    assert!(out.contains("no analyzed source file imports"), "{out}");
    assert!(out.contains("confidence:    high"), "{out}");
    assert!(out.contains("action:        remove it"), "{out}");
}

#[test]
fn bloated_empty_constraint_omits_constraint_line() {
    let e = BloatedDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 9,
        name: "noop".to_string(),
        constraint: String::new(),
        confidence: ImportConfidence::Low,
    };
    let out = human(AuditKind::BloatedDependency(e), "");
    assert!(!out.contains("constraint:"), "empty constraint suppresses the line: {out}");
    assert!(!out.contains("action:"), "empty action suppresses the action line: {out}");
    assert!(out.contains("confidence:    low"), "{out}");
}

#[test]
fn bloated_json_carries_constraint_and_confidence() {
    let e = BloatedDepEvidence {
        manifest: PathBuf::from("pkg/Cargo.toml"),
        line: 3,
        name: "huge".to_string(),
        constraint: "=2.0".to_string(),
        confidence: ImportConfidence::Medium,
    };
    let j = json(AuditKind::BloatedDependency(e));
    assert_eq!(j["kind"], "BloatedDependency");
    assert_eq!(j["manifest"], "pkg/Cargo.toml");
    assert_eq!(j["line"], 3);
    assert_eq!(j["name"], "huge");
    assert_eq!(j["constraint"], "=2.0");
    assert_eq!(j["confidence"], "medium");
}

#[test]
fn outdated_abandoned_branch_shows_status_line() {
    let e = OutdatedDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 20,
        name: "stale".to_string(),
        current_version: "0.1.0".to_string(),
        latest_version: "0.1.0".to_string(),
        missed_releases: 0,
        abandoned: true,
        confidence: ImportConfidence::High,
    };
    let out = human(AuditKind::OutdatedDependency(e), "replace it");
    assert!(out.contains("audit: outdated dependency — stale"), "{out}");
    assert!(out.contains("version:       0.1.0 (latest 0.1.0)"), "{out}");
    assert!(out.contains("status:        abandoned"), "{out}");
    assert!(!out.contains("behind:"), "abandoned branch omits behind line: {out}");
    assert!(out.contains("action:        replace it"), "{out}");
}

#[test]
fn outdated_behind_branch_shows_missed_releases() {
    let e = OutdatedDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 21,
        name: "lagging".to_string(),
        current_version: "1.0.0".to_string(),
        latest_version: "1.5.0".to_string(),
        missed_releases: 7,
        abandoned: false,
        confidence: ImportConfidence::Medium,
    };
    let out = human(AuditKind::OutdatedDependency(e), "");
    assert!(out.contains("version:       1.0.0 (latest 1.5.0)"), "{out}");
    assert!(out.contains("behind:        7 newer releases published"), "{out}");
    assert!(!out.contains("status:"), "behind branch omits status line: {out}");
    assert!(!out.contains("action:"), "{out}");
}

#[test]
fn outdated_json_carries_abandoned_and_releases() {
    let e = OutdatedDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 22,
        name: "pkg".to_string(),
        current_version: "2.0".to_string(),
        latest_version: "3.0".to_string(),
        missed_releases: 4,
        abandoned: true,
        confidence: ImportConfidence::BestEffort,
    };
    let j = json(AuditKind::OutdatedDependency(e));
    assert_eq!(j["kind"], "OutdatedDependency");
    assert_eq!(j["current_version"], "2.0");
    assert_eq!(j["latest_version"], "3.0");
    assert_eq!(j["missed_releases"], 4);
    assert_eq!(j["abandoned"], true);
    assert_eq!(j["confidence"], "best-effort");
}

#[test]
fn vuln_reachable_branch_marks_imported() {
    let e = VulnDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 30,
        name: "openssl".to_string(),
        version: "0.9.0".to_string(),
        advisory_ids: vec!["RUSTSEC-0001".to_string(), "RUSTSEC-0002".to_string()],
        reachable: true,
        confidence: ImportConfidence::High,
    };
    let out = human(AuditKind::VulnerableDependency(e), "upgrade");
    assert!(out.contains("audit: vulnerable dependency — openssl 0.9.0"), "{out}");
    assert!(out.contains("advisories:    RUSTSEC-0001, RUSTSEC-0002"), "{out}");
    assert!(out.contains("reachability:  imported in analyzed source"), "{out}");
    assert!(out.contains("action:        upgrade"), "{out}");
}

#[test]
fn vuln_not_reachable_branch_marks_likely_unreachable() {
    let e = VulnDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 31,
        name: "dormant".to_string(),
        version: "1.0".to_string(),
        advisory_ids: vec!["CVE-9999".to_string()],
        reachable: false,
        confidence: ImportConfidence::Low,
    };
    let out = human(AuditKind::VulnerableDependency(e), "");
    assert!(out.contains("reachability:  not imported — likely unreachable"), "{out}");
    assert!(!out.contains("imported in analyzed source"), "{out}");
    assert!(!out.contains("action:"), "{out}");
}

#[test]
fn vuln_json_carries_advisories_and_reachable() {
    let e = VulnDepEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 32,
        name: "lib".to_string(),
        version: "4.2".to_string(),
        advisory_ids: vec!["A-1".to_string()],
        reachable: false,
        confidence: ImportConfidence::Medium,
    };
    let j = json(AuditKind::VulnerableDependency(e));
    assert_eq!(j["kind"], "VulnerableDependency");
    assert_eq!(j["version"], "4.2");
    assert_eq!(j["advisories"][0], "A-1");
    assert_eq!(j["reachable"], false);
    assert_eq!(j["confidence"], "medium");
}

#[test]
fn phantom_human_and_json() {
    let e = PhantomDepEvidence {
        file: PathBuf::from("src/lib.rs"),
        line: 5,
        name: "ghost".to_string(),
        confidence: ImportConfidence::Low,
    };
    let out = human(AuditKind::PhantomDependency(e.clone()), "declare it");
    assert!(out.contains("audit: phantom dependency — ghost"), "{out}");
    assert!(out.contains("imported at:   src/lib.rs:5"), "{out}");
    assert!(out.contains("present only in the lockfile"), "{out}");
    assert!(out.contains("action:        declare it"), "{out}");

    let j = json(AuditKind::PhantomDependency(e));
    assert_eq!(j["kind"], "PhantomDependency");
    assert_eq!(j["file"], "src/lib.rs");
    assert_eq!(j["name"], "ghost");
    assert_eq!(j["confidence"], "low");
}

#[test]
fn constraint_human_and_json() {
    let e = ConstraintEvidence {
        manifest: PathBuf::from("Cargo.toml"),
        line: 40,
        name: "wildcard".to_string(),
        constraint: "*".to_string(),
        problem: "unbounded version range".to_string(),
        confidence: ImportConfidence::Medium,
    };
    let out = human(AuditKind::ConstraintSmell(e.clone()), "");
    assert!(out.contains("audit: constraint smell — wildcard"), "{out}");
    assert!(out.contains("declared at:   Cargo.toml:40"), "{out}");
    assert!(out.contains("problem:       unbounded version range"), "{out}");
    assert!(!out.contains("action:"), "{out}");

    let j = json(AuditKind::ConstraintSmell(e));
    assert_eq!(j["kind"], "ConstraintSmell");
    assert_eq!(j["constraint"], "*");
    assert_eq!(j["problem"], "unbounded version range");
    assert_eq!(j["confidence"], "medium");
}

#[test]
fn undeclared_module_human_and_json() {
    let e = UndeclaredModuleDepEvidence {
        from_component: "billing".to_string(),
        to_component: "auth".to_string(),
        file: PathBuf::from("billing/charge.rs"),
        line: 12,
        import_target: "auth::token".to_string(),
        confidence: ImportConfidence::High,
    };
    let out = human(AuditKind::UndeclaredModuleDependency(e.clone()), "declare the dep");
    assert!(out.contains("audit: undeclared module dependency — billing → auth"), "{out}");
    assert!(out.contains("imported at:   billing/charge.rs:12"), "{out}");
    assert!(out.contains("import:        auth::token"), "{out}");
    assert!(out.contains("`billing` declares no dependency on `auth`"), "{out}");
    assert!(out.contains("action:        declare the dep"), "{out}");

    let j = json(AuditKind::UndeclaredModuleDependency(e));
    assert_eq!(j["kind"], "UndeclaredModuleDependency");
    assert_eq!(j["from_component"], "billing");
    assert_eq!(j["to_component"], "auth");
    assert_eq!(j["import"], "auth::token");
    assert_eq!(j["confidence"], "high");
}

#[test]
fn unused_declared_human_and_json() {
    let e = UnusedDeclaredDepEvidence {
        manifest: PathBuf::from("billing/Cargo.toml"),
        line: 7,
        from_component: "billing".to_string(),
        to_component: "reporting".to_string(),
        confidence: ImportConfidence::Low,
    };
    let out = human(AuditKind::UnusedDeclaredDependency(e.clone()), "remove it");
    assert!(out.contains("audit: unused declared dependency — billing → reporting"), "{out}");
    assert!(out.contains("declared at:   billing/Cargo.toml:7"), "{out}");
    assert!(out.contains("no source file in `billing` imports or references `reporting`"), "{out}");
    assert!(out.contains("action:        remove it"), "{out}");

    let j = json(AuditKind::UnusedDeclaredDependency(e));
    assert_eq!(j["kind"], "UnusedDeclaredDependency");
    assert_eq!(j["from_component"], "billing");
    assert_eq!(j["to_component"], "reporting");
    assert_eq!(j["confidence"], "low");
}

#[test]
fn strictness_debt_metric_block_human_and_json() {
    let e = StrictnessEvidence {
        file: PathBuf::from("src/app.ts"),
        line: 100,
        any_count: 42,
        confidence: ImportConfidence::Medium,
    };
    let out = human(AuditKind::StrictnessDebt(e.clone()), "enable strict mode");
    assert!(out.contains("audit: type strictness debt — src/app.ts"), "{out}");
    assert!(out.contains("first `any`:   src/app.ts:100"), "{out}");
    assert!(out.contains("any count:     42 explicit `any` annotations"), "{out}");
    assert!(out.contains("confidence:    medium"), "{out}");
    assert!(out.contains("action:        enable strict mode"), "{out}");

    let j = json(AuditKind::StrictnessDebt(e));
    assert_eq!(j["kind"], "StrictnessDebt");
    assert_eq!(j["file"], "src/app.ts");
    assert_eq!(j["any_count"], 42);
    assert_eq!(j["confidence"], "medium");
}

#[test]
fn ifdef_density_metric_block_human_and_json() {
    let e = IfdefDensityEvidence {
        file: PathBuf::from("src/port.c"),
        line: 3,
        conditional_count: 18,
        confidence: ImportConfidence::High,
    };
    let out = human(AuditKind::IfdefDensity(e.clone()), "");
    assert!(out.contains("audit: conditional-compilation density — src/port.c"), "{out}");
    assert!(out.contains("first `#if`:    src/port.c:3"), "{out}");
    assert!(out.contains("conditionals:  18 preprocessor conditional blocks"), "{out}");
    assert!(out.contains("confidence:    high"), "{out}");
    assert!(!out.contains("action:"), "{out}");

    let j = json(AuditKind::IfdefDensity(e));
    assert_eq!(j["kind"], "IfdefDensity");
    assert_eq!(j["file"], "src/port.c");
    assert_eq!(j["conditional_count"], 18);
    assert_eq!(j["confidence"], "high");
}

#[test]
fn deps_display_path_strips_root_prefix() {
    let e = BloatedDepEvidence {
        manifest: PathBuf::from("repo/crate/Cargo.toml"),
        line: 1,
        name: "x".to_string(),
        constraint: String::new(),
        confidence: ImportConfidence::Low,
    };
    let root = PathBuf::from("repo");
    let out = human_rooted(AuditKind::BloatedDependency(e), &root, "");
    assert!(out.contains("declared at:   crate/Cargo.toml:1"), "{out}");
}

#[test]
fn non_deps_kind_returns_false_for_human() {
    let f = finding_for(AuditKind::ZeroEdgeProject { module_count: 4 });
    let mut out = String::new();
    assert!(!dispatch_human(&mut out, &f, None, "x"), "non-deps kind must return false");
    assert!(out.is_empty(), "{out}");
}

#[test]
fn non_deps_kind_yields_no_json() {
    let f = finding_for(AuditKind::ZeroEdgeProject { module_count: 4 });
    assert!(dispatch_json(&f, None).is_none(), "non-deps kind must yield no json");
}

#[test]
fn arch_kind_is_not_a_deps_finding() {
    let frag = OverFragmentationEvidence {
        component: PathBuf::from("src/widgets"),
        coupled_files: 6,
        internal_edges: 9,
        avg_loc: 30,
        density: 1.5,
        confidence: ImportConfidence::Low,
    };
    let f = finding_for(AuditKind::OverFragmentation(frag));
    let mut out = String::new();
    assert!(!dispatch_human(&mut out, &f, None, "x"), "arch kind is not handled by deps human dispatch");
    assert!(dispatch_json(&f, None).is_none(), "arch kind yields no deps json");
}

#[test]
fn over_fragmentation_human_renders_all_lines() {
    let e = OverFragmentationEvidence {
        component: PathBuf::from("src/widgets"),
        coupled_files: 8,
        internal_edges: 20,
        avg_loc: 25,
        density: 2.5,
        confidence: ImportConfidence::Low,
    };
    let kind = AuditKind::OverFragmentation(e);
    let mut out = String::new();
    assert!(write_arch(&mut out, &kind, Some(&PathBuf::from("src")), "consolidate them"));
    assert!(out.contains("audit: over-fragmented directory — widgets"), "{out}");
    assert!(out.contains("coupled files: 8"), "{out}");
    assert!(out.contains("avg size:      25 LOC/file"), "{out}");
    assert!(out.contains("intra-imports: 20"), "{out}");
    assert!(out.contains("density:       2.50 imports/file"), "{out}");
    assert!(out.contains("confidence:    low"), "{out}");
    assert!(out.contains("action:        consolidate them"), "{out}");
}

#[test]
fn over_fragmentation_json_carries_all_fields() {
    let e = OverFragmentationEvidence {
        component: PathBuf::from("src/widgets"),
        coupled_files: 8,
        internal_edges: 20,
        avg_loc: 25,
        density: 2.5,
        confidence: ImportConfidence::BestEffort,
    };
    let kind = AuditKind::OverFragmentation(e);
    let j = arch_json(&kind, None).expect("json");
    assert_eq!(j["kind"], "OverFragmentation");
    assert_eq!(j["component"], "src/widgets");
    assert_eq!(j["coupled_files"], 8);
    assert_eq!(j["avg_loc"], 25);
    assert_eq!(j["internal_edges"], 20);
    assert_eq!(j["confidence"], "best-effort");
}
