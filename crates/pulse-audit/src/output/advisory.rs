use std::path::Path;

use pulse_thresholds::AuditThresholds;

use crate::finding::{AuditFinding, AuditKind};

pub fn dispatch_human(
    out: &mut String,
    f: &AuditFinding,
    root: Option<&Path>,
    t: &AuditThresholds,
    action: &'static str,
) -> bool {
    if let AuditKind::InjectionShape(e) = &f.kind {
        crate::output::taint::write_injection(out, e, root, action);
        return true;
    }
    if let AuditKind::NearDuplicate(e) = &f.kind {
        crate::output::clones::write_clone(out, e, root, t, action);
        return true;
    }
    if let AuditKind::UnnaturalCode(e) = &f.kind {
        crate::output::naturalness::write_unnatural(out, e, root, action);
        return true;
    }
    if let AuditKind::VulnerableCloneSibling(e) = &f.kind {
        crate::output::vuln_clones::write_vuln_clone(out, e, root, action);
        return true;
    }
    false
}

pub fn dispatch_json(f: &AuditFinding, root: Option<&Path>) -> Option<serde_json::Value> {
    if let AuditKind::InjectionShape(e) = &f.kind {
        return Some(crate::output::taint::injection_json(e, root));
    }
    if let AuditKind::NearDuplicate(e) = &f.kind {
        return Some(crate::output::clones::clone_json(e, root));
    }
    if let AuditKind::UnnaturalCode(e) = &f.kind {
        return Some(crate::output::naturalness::unnatural_json(e, root));
    }
    if let AuditKind::VulnerableCloneSibling(e) = &f.kind {
        return Some(crate::output::vuln_clones::vuln_clone_json(e, root));
    }
    None
}
