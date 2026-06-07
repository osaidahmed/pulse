use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pulse::audit::finding::{
    AuditFinding, AuditKind, DivergentChangeEvidence, GodClassEvidence, ImportConfidence, ShotgunSurgeryEvidence,
};
use pulse::audit::hist_crossval::{apply_crossval, crossval_confidence};
use pulse::audit::{self, walk_typed_source_files_filtered, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::{resolve_base_thresholds, AuditSuppression, IgnoreMatcher, PulseConfig};
use pulse::history::thresholds::HistoryThresholds;
use pulse::history::{changeshotgun_files, HistoryOpts};
use pulse::thresholds::AuditThresholds;

use crate::audit_common::t;
use crate::history_common::{build_repo, CommitSpec};

fn wrap(kind: AuditKind) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        locations: Vec::new(),
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
    }
}

fn shotgun(file: &str, confidence: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::ShotgunSurgery(ShotgunSurgeryEvidence {
        method_file: PathBuf::from(file),
        method_class: None,
        method_name: "m".into(),
        method_line: 1,
        changing_classes: 5,
        changing_methods: 5,
        fanout: 5,
        confidence,
        caller_samples: Vec::new(),
        name_collision_count: 0,
        additional_definitions: Vec::new(),
    }))
}

fn divergent(file: &str, confidence: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::DivergentChange(DivergentChangeEvidence {
        class_file: PathBuf::from(file),
        class_name: "C".into(),
        changing_classes: 5,
        fanout: 5,
        method_count: 5,
        confidence,
    }))
}

fn god_class(file: &str, confidence: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from(file),
        class_name: "C".into(),
        wmc: 50,
        tcc: 0.1,
        atfd: 10,
        method_count: 30,
        confidence,
    }))
}

fn conf_of(f: &AuditFinding) -> ImportConfidence {
    match &f.kind {
        AuditKind::ShotgunSurgery(e) => e.confidence,
        AuditKind::DivergentChange(e) => e.confidence,
        AuditKind::FeatureEnvy(e) => e.confidence,
        AuditKind::GodClass(e) => e.confidence,
        _ => panic!("unexpected kind"),
    }
}

fn flagged(files: &[&str]) -> HashSet<PathBuf> {
    files.iter().map(PathBuf::from).collect()
}

#[test]
fn no_history_leaves_confidence_unchanged() {
    assert_eq!(crossval_confidence(ImportConfidence::Medium, None, Path::new("a.rs")), ImportConfidence::Medium);
}

#[test]
fn corroborated_medium_upgrades_to_high() {
    let set = flagged(&["a.rs"]);
    assert_eq!(crossval_confidence(ImportConfidence::Medium, Some(&set), Path::new("a.rs")), ImportConfidence::High);
}

#[test]
fn corroboration_does_not_amplify_a_weak_structural_signal() {
    let set = flagged(&["a.rs"]);
    assert_eq!(
        crossval_confidence(ImportConfidence::BestEffort, Some(&set), Path::new("a.rs")),
        ImportConfidence::BestEffort
    );
    assert_eq!(crossval_confidence(ImportConfidence::Low, Some(&set), Path::new("a.rs")), ImportConfidence::Low);
}

#[test]
fn uncorroborated_caps_a_strong_signal_at_low() {
    let set = flagged(&["other.rs"]);
    assert_eq!(crossval_confidence(ImportConfidence::High, Some(&set), Path::new("a.rs")), ImportConfidence::Low);
}

#[test]
fn uncorroborated_does_not_upgrade_a_weaker_signal() {
    let set = flagged(&["other.rs"]);
    assert_eq!(
        crossval_confidence(ImportConfidence::BestEffort, Some(&set), Path::new("a.rs")),
        ImportConfidence::BestEffort
    );
}

#[test]
fn corroboration_requires_an_exact_path_form() {
    let set = flagged(&["./a.rs"]);
    assert_eq!(
        crossval_confidence(ImportConfidence::Medium, Some(&set), Path::new("a.rs")),
        ImportConfidence::Low,
        "a path-form mismatch reads as uncorroborated — slice b must canonicalize both sides"
    );
}

#[test]
fn apply_crossval_upgrades_a_corroborated_shotgun() {
    let mut findings = vec![shotgun("a.rs", ImportConfidence::Medium)];
    apply_crossval(&mut findings, Some(&flagged(&["a.rs"])));
    assert_eq!(conf_of(&findings[0]), ImportConfidence::High);
}

#[test]
fn apply_crossval_downgrades_an_uncorroborated_shotgun() {
    let mut findings = vec![shotgun("a.rs", ImportConfidence::High)];
    apply_crossval(&mut findings, Some(&flagged(&["other.rs"])));
    assert_eq!(conf_of(&findings[0]), ImportConfidence::Low);
}

#[test]
fn apply_crossval_without_history_is_a_noop() {
    let mut findings = vec![shotgun("a.rs", ImportConfidence::Medium)];
    apply_crossval(&mut findings, None);
    assert_eq!(conf_of(&findings[0]), ImportConfidence::Medium);
}

#[test]
fn apply_crossval_leaves_divergent_change_untouched() {
    let mut findings = vec![divergent("a.rs", ImportConfidence::Medium)];
    apply_crossval(&mut findings, Some(&flagged(&["a.rs"])));
    assert_eq!(
        conf_of(&findings[0]),
        ImportConfidence::Medium,
        "divergent change has no file-level HIST analog; cross-val is deferred"
    );
}

#[test]
fn apply_crossval_leaves_god_class_untouched() {
    let mut findings = vec![god_class("a.rs", ImportConfidence::Medium)];
    apply_crossval(&mut findings, Some(&flagged(&["other.rs"])));
    assert_eq!(conf_of(&findings[0]), ImportConfidence::Medium, "GodClass is not a HIST cross-val target");
}

fn co_change_repo() -> tempfile::TempDir {
    let w1 = [("core/hub.py", "1\n"), ("p1/a.py", "1\n"), ("p2/b.py", "1\n"), ("p3/c.py", "1\n"), ("p4/d.py", "1\n")];
    let w2 = [("core/hub.py", "2\n"), ("p1/a.py", "2\n"), ("p2/b.py", "2\n"), ("p3/c.py", "2\n"), ("p4/d.py", "2\n")];
    let w3 = [("core/hub.py", "3\n"), ("p1/a.py", "3\n"), ("p2/b.py", "3\n"), ("p3/c.py", "3\n"), ("p4/d.py", "3\n")];
    build_repo(&[
        CommitSpec { author: "a <a@x>", message: "c1", writes: &w1, deletes: &[] },
        CommitSpec { author: "a <a@x>", message: "c2", writes: &w2, deletes: &[] },
        CommitSpec { author: "a <a@x>", message: "c3", writes: &w3, deletes: &[] },
    ])
}

fn opts_for(root: &std::path::Path) -> HistoryOpts {
    HistoryOpts { root: root.to_path_buf(), include_tests: true, since: None, max_commits: None }
}

#[test]
fn changeshotgun_files_returns_none_outside_a_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, dir.path());
    assert!(changeshotgun_files(&opts_for(dir.path()), &HistoryThresholds::default(), &filter).is_none());
}

#[test]
fn changeshotgun_files_flags_a_cross_package_hub() {
    let repo = co_change_repo();
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, repo.path());
    let flagged = changeshotgun_files(&opts_for(repo.path()), &HistoryThresholds::default(), &filter)
        .expect("a git repo with co-change history yields a flagged set");
    assert!(
        flagged.contains(&repo.path().join("core/hub.py")),
        "hub co-changes across four packages; got: {flagged:?}"
    );
}

#[test]
fn changeshotgun_files_paths_match_the_audit_walk_form() {
    let repo = co_change_repo();
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, repo.path());
    let flagged = changeshotgun_files(&opts_for(repo.path()), &HistoryThresholds::default(), &filter).expect("flagged");

    let walk: HashSet<PathBuf> =
        walk_typed_source_files_filtered(repo.path(), true, &filter).into_iter().map(|(p, _)| p).collect();
    assert!(!flagged.is_empty(), "fixture must produce at least one change-shotgun file");
    for f in &flagged {
        assert!(
            walk.contains(f),
            "flagged path {f:?} must match the audit walk form — the cross-val matching contract"
        );
    }
}

#[test]
fn audit_cross_validate_history_config_is_reachable() {
    let cfg: PulseConfig = toml::from_str("[audit]\ncross_validate_history = true\n").expect("parse config");
    let thr = resolve_base_thresholds(Some(&cfg));
    assert!(thr.audit.cross_validate_history, "[audit] cross_validate_history must reach the resolved thresholds");
}

#[test]
fn audit_cross_validate_history_defaults_off() {
    assert!(!resolve_base_thresholds(None).audit.cross_validate_history);
    let cfg: PulseConfig = toml::from_str("").expect("parse empty config");
    assert!(!resolve_base_thresholds(Some(&cfg)).audit.cross_validate_history, "default is opt-in (off)");
}

#[test]
fn audit_with_cross_validation_runs_deterministically_on_a_git_repo() {
    let repo = co_change_repo();
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, repo.path());
    let opts = AuditOpts {
        root: repo.path().to_path_buf(),
        pass: Some(PassChoice::NamedSmells),
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let thresholds = AuditThresholds { cross_validate_history: true, ..t().audit };

    let r1 = audit::run_with_filter(&opts, &thresholds, &filter);
    let r2 = audit::run_with_filter(&opts, &thresholds, &filter);
    assert_eq!(r1.len(), r2.len(), "cross-validation over a git repo must be deterministic");
}
