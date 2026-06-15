use std::path::Path;

use crate::registry;

use super::finding::{AuditKind, ImportConfidence, VulnDepEvidence};

pub fn run_from(root: &Path, cache_dir: &Path, online: bool, max_findings: usize) -> Vec<super::finding::AuditFinding> {
    super::freshness::for_each_deployed_locked(root, max_findings, |manifest, dep, current| {
        let detail = registry::lookup_version(manifest.ecosystem, &dep.name, current, cache_dir, online)?;
        if detail.advisory_keys.is_empty() {
            return None;
        }
        let advisory_ids = detail.advisory_keys.iter().map(|a| a.id.clone()).collect();
        Some(super::deps_reconcile::wrap(
            dep.name.clone(),
            AuditKind::VulnerableDependency(VulnDepEvidence {
                manifest: manifest.path.clone(),
                line: dep.line,
                name: dep.name.clone(),
                version: current.to_string(),
                advisory_ids,
                confidence: ImportConfidence::High,
            }),
        ))
    })
}
