use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pulse::audit::finding::{
    AuditFinding, AuditKind, DivergentChangeEvidence, GodClassEvidence, ImportConfidence, ShotgunSurgeryEvidence,
};
use pulse::audit::hist_crossval::{apply_crossval, crossval_confidence};

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
