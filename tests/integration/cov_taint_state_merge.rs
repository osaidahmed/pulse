use std::path::Path;

use pulse::audit::finding::{AuditFinding, AuditKind, InjectionEvidence};
use pulse::audit::suppression::AuditSuppression;
use pulse::audit::{self, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::IgnoreMatcher;

use crate::audit_common::*;

fn run_pass(dir: &Path, pass: Option<PassChoice>) -> Vec<AuditFinding> {
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: dir.to_path_buf(),
        pass,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, dir);
    audit::run_with_filter(&opts, &t().audit, &filter)
}

fn taint_findings(name: &str, body: &str) -> Vec<InjectionEvidence> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(name), body).unwrap();
    let findings = run_pass(dir.path(), Some(PassChoice::Taint));
    findings
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::InjectionShape(e) => Some(e),
            _ => None,
        })
        .collect()
}

#[test]
fn merging_two_taints_keeps_the_earlier_source_when_first_is_earlier() {
    let body = "def handler(cursor):\n    a = input()\n    b = combine(a, getenv(\"X\"))\n    cursor.execute(b)\n";
    let found = taint_findings("merge_first.py", body);
    assert_eq!(found.len(), 1, "the merged flow must reach the sink: {found:?}");
    let e = &found[0];
    assert_eq!(e.sink_name, "execute");
    assert_eq!(e.source_name, "input", "earlier() must keep the earlier-line source (input on line 2)");
    assert_eq!(e.source_line, 2, "merge() must select the taint with the smaller source line");
}

#[test]
fn merging_two_taints_keeps_the_earlier_source_when_second_is_earlier() {
    let body = "def handler(cursor):\n    a = input()\n    b = combine(getenv(\"X\"), a)\n    cursor.execute(b)\n";
    let found = taint_findings("merge_second.py", body);
    assert_eq!(found.len(), 1, "the merged flow must reach the sink: {found:?}");
    let e = &found[0];
    assert_eq!(e.sink_name, "execute");
    assert_eq!(e.source_name, "input", "earlier() must keep the earlier-line source regardless of child order");
    assert_eq!(e.source_line, 2, "merge() must select the taint with the smaller source line");
}

#[test]
fn merging_same_line_sources_resolves_deterministically_by_name() {
    let body = "def handler(cursor):\n    b = combine(input(), getenv(\"X\"))\n    cursor.execute(b)\n";
    let found = taint_findings("merge_sameline.py", body);
    assert_eq!(found.len(), 1, "two same-line sources still merge into one surviving taint: {found:?}");
    let e = &found[0];
    assert_eq!(e.sink_name, "execute");
    assert_eq!(e.source_line, 2);
    assert_eq!(e.source_name, "getenv", "with equal source lines earlier() breaks the tie by name (getenv < input)");
}
