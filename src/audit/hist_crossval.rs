use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::finding::{AuditFinding, AuditKind, ImportConfidence};

pub fn apply_crossval(findings: &mut [AuditFinding], history_flagged: Option<&HashSet<PathBuf>>) {
    for finding in findings {
        if let AuditKind::ShotgunSurgery(e) = &mut finding.kind {
            e.confidence = crossval_confidence(e.confidence, history_flagged, &e.method_file);
        }
    }
}

pub fn crossval_confidence(
    current: ImportConfidence,
    history_flagged: Option<&HashSet<PathBuf>>,
    file: &Path,
) -> ImportConfidence {
    match history_flagged {
        None => current,
        Some(flagged) if flagged.contains(file) => {
            if current >= ImportConfidence::Medium {
                ImportConfidence::High
            } else {
                current
            }
        }
        Some(_) => current.min(ImportConfidence::Low),
    }
}
