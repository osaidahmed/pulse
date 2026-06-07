use std::collections::HashSet;
use std::path::PathBuf;

use crate::parse::Language;
use crate::thresholds::AuditThresholds;

use super::duplication_clusters::{self, CloneMember};
use super::finding::{AuditFinding, AuditKind, ImportConfidence, VulnCloneEvidence};
use super::taint;

struct Sink {
    file: PathBuf,
    line: u32,
    name: String,
}

pub fn run(typed_files: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let sinks = taint_sinks(&taint::run(typed_files, thresholds));
    if sinks.is_empty() {
        return Vec::new();
    }
    let clusters = duplication_clusters::cluster_members(typed_files, thresholds);
    let mut seen: HashSet<(PathBuf, u32)> = HashSet::new();
    let mut out = Vec::new();
    for cluster in &clusters {
        propagate(cluster, &sinks, &mut seen, &mut out);
    }
    out
}

fn taint_sinks(findings: &[AuditFinding]) -> Vec<Sink> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::InjectionShape(e) => {
                Some(Sink { file: e.file.clone(), line: e.sink_line, name: e.sink_name.clone() })
            }
            _ => None,
        })
        .collect()
}

fn propagate(cluster: &[CloneMember], sinks: &[Sink], seen: &mut HashSet<(PathBuf, u32)>, out: &mut Vec<AuditFinding>) {
    let Some(vuln) = cluster.iter().find_map(|m| sink_in_member(m, sinks)) else {
        return;
    };
    for member in cluster {
        if sink_in_member(member, sinks).is_none() && seen.insert((member.file.clone(), member.line)) {
            out.push(build_finding(member, vuln));
        }
    }
}

fn sink_in_member<'a>(member: &CloneMember, sinks: &'a [Sink]) -> Option<&'a Sink> {
    let end = member.line.saturating_add(member.loc);
    sinks.iter().find(|s| s.file == member.file && member.line <= s.line && s.line < end)
}

fn build_finding(member: &CloneMember, vuln: &Sink) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::VulnerableCloneSibling(VulnCloneEvidence {
            file: member.file.clone(),
            line: member.line,
            vuln_file: vuln.file.clone(),
            vuln_line: vuln.line,
            sink_name: vuln.name.clone(),
            confidence: ImportConfidence::BestEffort,
        }),
        representative_snippet: String::new(),
        support: 1,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}
