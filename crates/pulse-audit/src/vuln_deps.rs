use std::path::Path;

use pulse_buildmeta::registry;
use pulse_imports::normalize;

use super::corpus::Corpus;
use super::deps_reconcile::{collect_imports, is_imported};
use super::finding::{AuditFinding, AuditKind, ImportConfidence, VulnDepEvidence};

pub fn run_from(
    root: &Path,
    corpus: &Corpus,
    cache_dir: &Path,
    online: bool,
    max_findings: usize,
) -> Vec<AuditFinding> {
    let imported = collect_imports(corpus);
    super::freshness::for_each_deployed_locked(root, max_findings, |manifest, dep, current| {
        let detail = registry::lookup_version(manifest.ecosystem, &dep.name, current, cache_dir, online)?;
        if detail.advisory_keys.is_empty() {
            return None;
        }
        let advisory_ids = detail.advisory_keys.iter().map(|a| a.id.clone()).collect();
        let reachable = is_imported(manifest.ecosystem, &normalize(&dep.name), &imported);
        let confidence = if reachable { ImportConfidence::High } else { ImportConfidence::Low };
        Some(super::deps_reconcile::wrap(
            dep.name.clone(),
            AuditKind::VulnerableDependency(VulnDepEvidence {
                manifest: manifest.path.clone(),
                line: dep.line,
                name: dep.name.clone(),
                version: current.to_string(),
                advisory_ids,
                reachable,
                confidence,
            }),
        ))
    })
}
