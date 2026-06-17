use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::config;
use crate::thresholds::AuditThresholds;

use super::corpus::Corpus;
use super::finding::{AuditFinding, AuditKind};

pub struct Stratum {
    pub dir: PathBuf,
    pub thresholds: AuditThresholds,
}

pub fn nested_strata(corpus: &Corpus, root: &Path) -> Vec<Stratum> {
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for file in &corpus.files {
        let mut cur = file.path.parent();
        while let Some(dir) = cur {
            if dir == root || !dir.starts_with(root) {
                break;
            }
            dirs.insert(dir.to_path_buf());
            cur = dir.parent();
        }
    }
    dirs.into_iter()
        .filter_map(|dir| {
            let cfg = config::local_config(&dir)?;
            let thresholds = config::resolve_base_thresholds(config::load_config(&cfg).as_ref()).audit;
            Some(Stratum { dir, thresholds })
        })
        .collect()
}

fn other_primary(kind: &AuditKind) -> Option<&Path> {
    match kind {
        AuditKind::ShotgunSurgery(e) => Some(&e.method_file),
        AuditKind::FeatureEnvy(e) => Some(&e.method_file),
        AuditKind::ParallelInheritance(e) => Some(&e.root_a.file),
        AuditKind::RefusedBequest(e) => Some(&e.subclass_file),
        _ => None,
    }
}

pub fn primary_file(finding: &AuditFinding) -> Option<&Path> {
    match &finding.kind {
        AuditKind::DivergentChange(e) => Some(&e.class_file),
        AuditKind::GodClass(e) => Some(&e.class_file),
        AuditKind::LowConceptualCohesion(e) => Some(&e.class_file),
        AuditKind::MultivariateAnomaly(e) => Some(&e.class_file),
        _ => other_primary(&finding.kind),
    }
}

pub fn owning_dir<'a>(file: &Path, strata: &'a [Stratum]) -> Option<&'a Path> {
    strata
        .iter()
        .filter(|s| file.starts_with(&s.dir))
        .max_by_key(|s| s.dir.components().count())
        .map(|s| s.dir.as_path())
}

pub fn is_root_owned(finding: &AuditFinding, strata: &[Stratum]) -> bool {
    primary_file(finding).is_none_or(|p| owning_dir(p, strata).is_none())
}

pub fn is_owned_by(finding: &AuditFinding, dir: &Path, strata: &[Stratum]) -> bool {
    primary_file(finding).and_then(|p| owning_dir(p, strata)) == Some(dir)
}
