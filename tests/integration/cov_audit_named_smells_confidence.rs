use std::fs;
use std::path::Path;

use pulse::audit::finding::{AuditFinding, AuditKind};
use pulse::audit::suppression::AuditSuppression;
use pulse::audit::{self, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::IgnoreMatcher;
use pulse::thresholds::AuditThresholds;

use crate::audit_common::t;

fn run_named(dir: &Path, audit_t: &AuditThresholds) -> Vec<AuditFinding> {
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: dir.to_path_buf(),
        pass: Some(PassChoice::NamedSmells),
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, dir);
    audit::run_with_filter(&opts, audit_t, &filter)
}

fn write(dir: &Path, name: &str, body: &str) {
    fs::write(dir.path_join(name), body).unwrap();
}

trait PathJoin {
    fn path_join(&self, name: &str) -> std::path::PathBuf;
}
impl PathJoin for Path {
    fn path_join(&self, name: &str) -> std::path::PathBuf {
        self.join(name)
    }
}

#[test]
fn refused_bequest_finding_routes_named_confidence() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "base.py",
        "class Base:\n    def m1(self):\n        return 1\n    def m2(self):\n        return 2\n    def m3(self):\n        return 3\n    def m4(self):\n        return 4\n",
    );
    write(
        dir.path(),
        "child.py",
        "from base import Base\n\n\nclass Child(Base):\n    def m1(self):\n        return 10\n",
    );

    let mut audit_t = t().audit;
    let rb = &mut audit_t.named_smells.refused_bequest;
    rb.enabled = true;
    rb.min_parent_methods = 2;
    rb.max_override_ratio = 0.9;

    let findings = run_named(dir.path(), &audit_t);
    let rb_found = findings.iter().find(|f| matches!(f.kind, AuditKind::RefusedBequest(_)));
    assert!(rb_found.is_some(), "expected a RefusedBequest finding from concrete-parent fixture, got: {findings:#?}");

    let AuditKind::RefusedBequest(e) = &rb_found.unwrap().kind else { unreachable!() };
    assert_eq!(e.subclass_name, "Child");
    assert_eq!(e.parent_name, "Base");

    let default_findings = run_named(dir.path(), &t().audit);
    assert!(
        !default_findings.iter().any(|f| matches!(f.kind, AuditKind::RefusedBequest(_))),
        "refused bequest is opt-in: the default pass must not emit it"
    );
}

#[test]
fn parallel_inheritance_finding_routes_named_confidence() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "readers.py",
        "class Reader:\n    def read(self):\n        return 0\n\n\nclass XmlReader(Reader):\n    def read(self):\n        return 1\n\n\nclass JsonReader(Reader):\n    def read(self):\n        return 2\n\n\nclass YamlReader(Reader):\n    def read(self):\n        return 3\n",
    );
    write(
        dir.path(),
        "writers.py",
        "class Writer:\n    def write(self):\n        return 0\n\n\nclass XmlWriter(Writer):\n    def write(self):\n        return 1\n\n\nclass JsonWriter(Writer):\n    def write(self):\n        return 2\n\n\nclass YamlWriter(Writer):\n    def write(self):\n        return 3\n",
    );

    let findings = run_named(dir.path(), &t().audit);
    let pi_found = findings.iter().any(|f| matches!(f.kind, AuditKind::ParallelInheritance(_)));
    assert!(pi_found, "expected a ParallelInheritance finding from Reader/Writer hierarchies, got: {findings:#?}");
}

#[test]
fn god_class_finding_routes_named_confidence() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "god.py",
        "class God:\n    def a(self, alpha, beta):\n        self.fa = alpha.x + beta.y\n        return self.fa\n    def b(self, gamma, delta):\n        self.fb = gamma.p + delta.q\n        return self.fb\n    def c(self, eps, zeta):\n        self.fc = eps.m + zeta.n\n        return self.fc\n    def d(self, eta, theta):\n        self.fd = eta.r + theta.s\n        return self.fd\n    def e(self, iota, kappa):\n        self.fe = iota.t + kappa.u\n        return self.fe\n",
    );

    let mut audit_t = t().audit;
    let gc = &mut audit_t.named_smells.god_class;
    gc.wmc = 0;
    gc.tcc = 1.0;
    gc.atfd = 0;

    let findings = run_named(dir.path(), &audit_t);
    let gc_found = findings.iter().any(|f| matches!(f.kind, AuditKind::GodClass(_)));
    assert!(
        gc_found,
        "expected a GodClass finding from low-cohesion foreign-access fixture under relaxed thresholds, got: {findings:#?}"
    );
}

#[test]
fn empty_project_produces_no_named_findings_without_panic() {
    let dir = tempfile::tempdir().unwrap();
    let findings = run_named(dir.path(), &t().audit);
    assert!(findings.is_empty());
}
