use std::path::PathBuf;

use pulse_audit::finding::{
    AuditFinding, AuditKind, AuditLocation, CloneClusterEvidence, ImportConfidence, InjectionEvidence,
    NaturalnessEvidence, VulnCloneEvidence,
};
use pulse_audit::output::advisory::{dispatch_human, dispatch_json};
use pulse_thresholds::AuditThresholds;

fn wrap(kind: AuditKind) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: String::new(),
        support: 1,
        file_count: 1,
        idf_score: None,
        action_label: None,
        locations: Vec::new(),
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
    }
}

fn injection(crossed_opacity: bool) -> AuditKind {
    AuditKind::InjectionShape(InjectionEvidence {
        file: PathBuf::from("src/db.php"),
        function: "run_query".into(),
        source_name: "getParameter".into(),
        source_line: 3,
        sink_name: "mysqli_query".into(),
        sink_line: 7,
        tainted_var: "id".into(),
        crossed_opacity,
        confidence: ImportConfidence::High,
    })
}

#[test]
fn injection_shape_human_and_json() {
    let f = wrap(injection(true));
    let t = AuditThresholds::default();
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, Some(&PathBuf::from("src")), &t, "use a prepared statement"));
    assert!(out.contains("injection shape — run_query"), "{out}");
    assert!(out.contains("getParameter() at line 3"), "{out}");
    assert!(out.contains("mysqli_query() at line 7"), "{out}");
    assert!(out.contains("dynamic dispatch"), "crossed_opacity note expected: {out}");
    assert!(out.contains("action:"), "{out}");
    let j = dispatch_json(&f, None).expect("json");
    assert_eq!(j["kind"], "InjectionShape");
    assert_eq!(j["function"], "run_query");
}

#[test]
fn injection_without_opacity_or_action() {
    let f = wrap(injection(false));
    let t = AuditThresholds::default();
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, None, &t, ""));
    assert!(!out.contains("dynamic dispatch"), "no opacity note when not crossed: {out}");
    assert!(!out.contains("action:"), "no action line when action is empty: {out}");
}

#[test]
fn near_duplicate_human_and_json() {
    let e = CloneClusterEvidence {
        members: vec![
            AuditLocation { file: PathBuf::from("src/a.rs"), line: 10 },
            AuditLocation { file: PathBuf::from("src/b.rs"), line: 20 },
        ],
        member_count: 2,
        max_loc: 40,
        representative: "fn dup() {}".into(),
        confidence: ImportConfidence::Medium,
    };
    let f = wrap(AuditKind::NearDuplicate(e));
    let t = AuditThresholds::default();
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, None, &t, "extract a shared helper"));
    assert!(out.contains("near-duplicate clones"), "{out}");
    assert!(out.contains("representative:"), "{out}");
    assert!(out.contains("src/a.rs:10"), "{out}");
    let j = dispatch_json(&f, None).expect("json");
    assert_eq!(j["kind"], "NearDuplicate");
    assert_eq!(j["member_count"], 2);
}

#[test]
fn near_duplicate_without_representative() {
    let e = CloneClusterEvidence {
        members: vec![AuditLocation { file: PathBuf::from("src/a.rs"), line: 1 }],
        member_count: 1,
        max_loc: 5,
        representative: String::new(),
        confidence: ImportConfidence::Low,
    };
    let f = wrap(AuditKind::NearDuplicate(e));
    let t = AuditThresholds::default();
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, None, &t, ""));
    assert!(!out.contains("representative:"), "no representative line when empty: {out}");
}

#[test]
fn unnatural_code_human_and_json() {
    let e = NaturalnessEvidence {
        file: PathBuf::from("src/odd.py"),
        function: "weird".into(),
        line: 12,
        surprisal: 9.5,
        zscore: 3.2,
        confidence: ImportConfidence::High,
    };
    let f = wrap(AuditKind::UnnaturalCode(e));
    let t = AuditThresholds::default();
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, None, &t, "simplify the control flow"));
    assert!(out.contains("unnatural code — weird"), "{out}");
    assert!(out.contains("bits/token"), "{out}");
    let j = dispatch_json(&f, None).expect("json");
    assert_eq!(j["kind"], "UnnaturalCode");
}

#[test]
fn vulnerable_clone_human_and_json() {
    let e = VulnCloneEvidence {
        file: PathBuf::from("src/copy.php"),
        line: 4,
        vuln_file: PathBuf::from("src/orig.php"),
        vuln_line: 8,
        sink_name: "system".into(),
        confidence: ImportConfidence::Medium,
    };
    let f = wrap(AuditKind::VulnerableCloneSibling(e));
    let t = AuditThresholds::default();
    let mut out = String::new();
    assert!(dispatch_human(&mut out, &f, None, &t, "review for the same sink"));
    assert!(out.contains("vulnerable clone sibling"), "{out}");
    assert!(out.contains("system() sink"), "{out}");
    let j = dispatch_json(&f, None).expect("json");
    assert_eq!(j["kind"], "VulnerableCloneSibling");
}

#[test]
fn non_advisory_kind_is_not_dispatched() {
    let f = wrap(AuditKind::ZeroEdgeProject { module_count: 3 });
    let t = AuditThresholds::default();
    let mut out = String::new();
    assert!(!dispatch_human(&mut out, &f, None, &t, "x"), "non-advisory kind must return false");
    assert!(out.is_empty(), "{out}");
    assert!(dispatch_json(&f, None).is_none(), "non-advisory kind must return None");
}
