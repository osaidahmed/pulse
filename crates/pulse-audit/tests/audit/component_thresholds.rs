use std::fs;
use std::path::{Path, PathBuf};

use pulse_audit::component_thresholds::{nested_strata, owning_dir, Stratum};
use pulse_audit::corpus::Corpus;
use pulse_audit::finding::{AuditFinding, AuditKind};
use pulse_audit::suppression::AuditSuppression;
use pulse_audit::{self as audit, AuditOpts, IgnoreFilter, PassChoice};
use pulse_config::IgnoreMatcher;
use pulse_syntax::parse::Language;
use pulse_thresholds::AuditThresholds;

use crate::audit_common::t;

const GOD_SRC: &str = "class God:\n    def a(self, alpha, beta):\n        self.fa = alpha.x + beta.y\n        return self.fa\n    def b(self, gamma, delta):\n        self.fb = gamma.p + delta.q\n        return self.fb\n    def c(self, eps, zeta):\n        self.fc = eps.m + zeta.n\n        return self.fc\n    def d(self, eta, theta):\n        self.fd = eta.r + theta.s\n        return self.fd\n    def e(self, iota, kappa):\n        self.fe = iota.t + kappa.u\n        return self.fe\n";

const RELAXED_GOD_CLASS: &str = "[thresholds.named_smells.god_class]\nwmc = 0\ntcc = 1.0\natfd = 0\n";
const SUPPRESS_GOD_CLASS: &str = "[thresholds.named_smells.god_class]\nwmc = 999999\n";

fn relaxed_root() -> AuditThresholds {
    let mut audit_t = t().audit;
    let gc = &mut audit_t.named_smells.god_class;
    gc.wmc = 0;
    gc.tcc = 1.0;
    gc.atfd = 0;
    audit_t
}

fn run_named(root: &Path, audit_t: &AuditThresholds) -> Vec<AuditFinding> {
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: root.to_path_buf(),
        pass: Some(PassChoice::NamedSmells),
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, root);
    audit::run_with_filter(&opts, audit_t, &filter)
}

fn god_class_files(findings: &[AuditFinding]) -> Vec<PathBuf> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::GodClass(e) => Some(e.class_file.clone()),
            _ => None,
        })
        .collect()
}

fn write_member(root: &Path, member: &str, config: Option<&str>) {
    let dir = root.join(member);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("god.py"), GOD_SRC).unwrap();
    if let Some(cfg) = config {
        fs::write(dir.join(".pulse.toml"), cfg).unwrap();
    }
}

#[test]
fn owning_dir_picks_deepest_prefix_and_none_outside() {
    let strata = vec![
        Stratum { dir: PathBuf::from("/repo/a"), thresholds: t().audit },
        Stratum { dir: PathBuf::from("/repo/a/b"), thresholds: t().audit },
    ];
    assert_eq!(owning_dir(Path::new("/repo/a/b/f.py"), &strata), Some(Path::new("/repo/a/b")));
    assert_eq!(owning_dir(Path::new("/repo/a/f.py"), &strata), Some(Path::new("/repo/a")));
    assert_eq!(owning_dir(Path::new("/repo/c/f.py"), &strata), None);
}

#[test]
fn nested_strata_discovers_member_config_with_resolved_thresholds() {
    let dir = tempfile::tempdir().unwrap();
    write_member(dir.path(), "member", Some(SUPPRESS_GOD_CLASS));
    fs::write(dir.path().join("top.py"), GOD_SRC).unwrap();
    let typed = vec![
        (dir.path().join("member").join("god.py"), Language::Python),
        (dir.path().join("top.py"), Language::Python),
    ];
    let corpus = Corpus::load(&typed);
    let strata = nested_strata(&corpus, dir.path());
    assert_eq!(strata.len(), 1, "only the member dir carries a config");
    assert_eq!(strata[0].dir, dir.path().join("member"));
    assert_eq!(strata[0].thresholds.named_smells.god_class.wmc, 999_999, "member config overrides god_class.wmc");
}

#[test]
fn nested_strata_empty_without_any_nested_config() {
    let dir = tempfile::tempdir().unwrap();
    write_member(dir.path(), "member", None);
    let typed = vec![(dir.path().join("member").join("god.py"), Language::Python)];
    let corpus = Corpus::load(&typed);
    assert!(nested_strata(&corpus, dir.path()).is_empty());
}

#[test]
fn member_config_tightens_so_only_its_god_class_fires() {
    let dir = tempfile::tempdir().unwrap();
    write_member(dir.path(), "strict", Some(RELAXED_GOD_CLASS));
    write_member(dir.path(), "loose", None);

    let files = god_class_files(&run_named(dir.path(), &t().audit));
    assert!(
        files.iter().any(|p| p.starts_with(dir.path().join("strict"))),
        "strict member's relaxed config fires: {files:?}"
    );
    assert!(
        !files.iter().any(|p| p.starts_with(dir.path().join("loose"))),
        "loose member uses default thresholds: {files:?}"
    );
}

#[test]
fn member_config_loosens_so_its_god_class_is_suppressed() {
    let dir = tempfile::tempdir().unwrap();
    write_member(dir.path(), "strict", None);
    write_member(dir.path(), "loose", Some(SUPPRESS_GOD_CLASS));

    let files = god_class_files(&run_named(dir.path(), &relaxed_root()));
    assert!(
        files.iter().any(|p| p.starts_with(dir.path().join("strict"))),
        "strict member inherits the relaxed root: {files:?}"
    );
    assert!(
        !files.iter().any(|p| p.starts_with(dir.path().join("loose"))),
        "loose member's config suppresses it: {files:?}"
    );
}

#[test]
fn no_nested_config_treats_every_member_uniformly() {
    let dir = tempfile::tempdir().unwrap();
    write_member(dir.path(), "alpha", None);
    write_member(dir.path(), "beta", None);

    let files = god_class_files(&run_named(dir.path(), &relaxed_root()));
    assert!(files.iter().any(|p| p.starts_with(dir.path().join("alpha"))), "{files:?}");
    assert!(files.iter().any(|p| p.starts_with(dir.path().join("beta"))), "{files:?}");
}
