use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::parse::{self, Language};

use super::finding::{DefectProneEvidence, HistoryFinding, HistoryKind};
use super::thresholds::SzzThresholds;

const FIX_TOKENS: &[&str] = &["fix", "fixes", "fixed", "bug", "bugfix", "hotfix", "defect", "regression"];
const MAX_FILES_PER_FIX: usize = 25;
const MAX_RANGES_PER_FILE: usize = 40;

#[derive(Default)]
struct Tally {
    fixes: HashSet<String>,
    introducers: HashSet<String>,
}

pub fn rank(root: &Path, typed: &[(PathBuf, Language)], t: &SzzThresholds) -> Vec<HistoryFinding> {
    if !t.enabled {
        return Vec::new();
    }
    let Some(top) = super::git::repo_toplevel(root) else {
        return Vec::new();
    };
    let lang_by_rel = rel_lang_map(typed, &top);
    let mut tallies: HashMap<(String, String), Tally> = HashMap::new();
    let mut cosmetic: HashMap<(String, String), bool> = HashMap::new();
    let mut attr = Attr { top: &top, lang_by_rel: &lang_by_rel, tallies: &mut tallies, cosmetic: &mut cosmetic };
    for fix in fix_commits(&top, t.max_fix_commits) {
        attr.fix(&fix);
    }
    build_findings(&top, &tallies, t)
}

fn rel_lang_map(typed: &[(PathBuf, Language)], top: &Path) -> HashMap<String, Language> {
    let canon_top = std::fs::canonicalize(top).unwrap_or_else(|_| top.to_path_buf());
    typed
        .iter()
        .filter_map(|(p, l)| {
            let canon = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
            Some((canon.strip_prefix(&canon_top).ok()?.to_string_lossy().into_owned(), *l))
        })
        .collect()
}

fn git_out(top: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(top).args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn fix_commits(top: &Path, max: u32) -> Vec<String> {
    let window = max.saturating_mul(8).max(max);
    let log = git_out(top, &["log", "--no-merges", &format!("-n{window}"), "--pretty=format:%H\x1f%s"]);
    log.unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\x1f'))
        .filter(|(_, subject)| is_fix(subject))
        .map(|(hash, _)| hash.to_string())
        .take(max as usize)
        .collect()
}

fn is_fix(subject: &str) -> bool {
    subject.to_ascii_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).any(|w| FIX_TOKENS.contains(&w))
}

struct Attr<'a> {
    top: &'a Path,
    lang_by_rel: &'a HashMap<String, Language>,
    tallies: &'a mut HashMap<(String, String), Tally>,
    cosmetic: &'a mut HashMap<(String, String), bool>,
}

impl Attr<'_> {
    fn fix(&mut self, fix: &str) {
        for rel in changed_files(self.top, fix, self.lang_by_rel) {
            let ranges = deleted_ranges(self.top, fix, &rel);
            if ranges.is_empty() {
                continue;
            }
            let Some(&lang) = self.lang_by_rel.get(&rel) else {
                continue;
            };
            let spans = function_spans(self.top, &format!("{fix}^"), &rel, lang);
            for range in ranges {
                self.range(fix, &rel, range, &spans);
            }
        }
    }

    fn range(&mut self, fix: &str, rel: &str, range: (u32, u32), spans: &[(String, u32, u32)]) {
        let Some(function) = function_at_line(spans, range.0) else {
            return;
        };
        for introducer in blame_range(self.top, fix, rel, range) {
            if is_cosmetic(self.top, &introducer, rel, self.lang_by_rel, self.cosmetic) {
                continue;
            }
            let tally = self.tallies.entry((rel.to_string(), function.clone())).or_default();
            tally.fixes.insert(fix.to_string());
            tally.introducers.insert(introducer);
        }
    }
}

fn function_spans(top: &Path, rev: &str, rel: &str, lang: Language) -> Vec<(String, u32, u32)> {
    let Some(src) = git_out(top, &["show", &format!("{rev}:{rel}")]) else {
        return Vec::new();
    };
    parse::parse_and_walk_guarded(&src, lang)
        .map(|m| m.functions.iter().map(|f| (f.name.clone(), f.start_line, f.end_line)).collect())
        .unwrap_or_default()
}

fn function_at_line(spans: &[(String, u32, u32)], line: u32) -> Option<String> {
    spans
        .iter()
        .filter(|(name, start, end)| !name.is_empty() && *start <= line && line <= *end)
        .max_by_key(|(_, start, _)| *start)
        .map(|(name, _, _)| name.clone())
}

fn changed_files(top: &Path, fix: &str, lang_by_rel: &HashMap<String, Language>) -> Vec<String> {
    git_out(top, &["diff-tree", "--no-commit-id", "--name-only", "-r", fix])
        .unwrap_or_default()
        .lines()
        .filter(|l| lang_by_rel.contains_key(*l))
        .take(MAX_FILES_PER_FIX)
        .map(String::from)
        .collect()
}

fn deleted_ranges(top: &Path, fix: &str, rel: &str) -> Vec<(u32, u32)> {
    let diff = git_out(top, &["diff", &format!("{fix}^"), fix, "--", rel]).unwrap_or_default();
    coalesce(deleted_lines(&diff))
}

fn deleted_lines(diff: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut cur = 0u32;
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("@@") {
            cur = hunk_old_start(rest);
        } else if line.starts_with("---") || line.starts_with("+++") {
        } else if line.starts_with('-') {
            out.push(cur);
            cur += 1;
        } else if !line.starts_with('+') {
            cur += 1;
        }
    }
    out
}

fn hunk_old_start(rest: &str) -> u32 {
    let after_minus = rest.split('-').nth(1).unwrap_or_default();
    let token = after_minus.split([',', ' ']).find(|s| !s.trim().is_empty()).unwrap_or_default();
    token.trim().parse().unwrap_or(1)
}

fn coalesce(mut lines: Vec<u32>) -> Vec<(u32, u32)> {
    lines.sort_unstable();
    lines.dedup();
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for n in lines {
        match ranges.last_mut() {
            Some(last) if n == last.1 + 1 => last.1 = n,
            _ => ranges.push((n, n)),
        }
    }
    ranges.truncate(MAX_RANGES_PER_FILE);
    ranges
}

fn blame_range(top: &Path, fix: &str, rel: &str, range: (u32, u32)) -> HashSet<String> {
    let loc = format!("{},{}", range.0, range.1);
    let blame = git_out(top, &["blame", "-w", "-M", "--porcelain", "-L", &loc, &format!("{fix}^"), "--", rel]);
    porcelain_shas(&blame.unwrap_or_default()).into_iter().collect()
}

fn porcelain_shas(blame: &str) -> Vec<String> {
    blame
        .lines()
        .filter_map(|l| l.split(' ').next())
        .filter(|w| w.len() == 40 && w.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(String::from)
        .collect()
}

fn is_cosmetic(
    top: &Path,
    introducer: &str,
    rel: &str,
    lang_by_rel: &HashMap<String, Language>,
    cache: &mut HashMap<(String, String), bool>,
) -> bool {
    let key = (introducer.to_string(), rel.to_string());
    if let Some(&cached) = cache.get(&key) {
        return cached;
    }
    let result = compute_cosmetic(top, introducer, rel, lang_by_rel);
    cache.insert(key, result);
    result
}

fn compute_cosmetic(top: &Path, introducer: &str, rel: &str, lang_by_rel: &HashMap<String, Language>) -> bool {
    let Some(&lang) = lang_by_rel.get(rel) else {
        return false;
    };
    let Some(after) = git_out(top, &["show", &format!("{introducer}:{rel}")]) else {
        return false;
    };
    match git_out(top, &["show", &format!("{introducer}^:{rel}")]) {
        Some(before) => fingerprints(&before, lang) == fingerprints(&after, lang),
        None => false,
    }
}

fn fingerprints(source: &str, lang: Language) -> Vec<u64> {
    let mut hashes: Vec<u64> = parse::parse_and_walk_guarded(source, lang)
        .map(|m| m.functions.iter().map(|f| f.structural_hash).collect())
        .unwrap_or_default();
    hashes.sort_unstable();
    hashes
}

fn build_findings(top: &Path, tallies: &HashMap<(String, String), Tally>, t: &SzzThresholds) -> Vec<HistoryFinding> {
    let mut rows: Vec<DefectProneEvidence> = tallies
        .iter()
        .filter(|(_, tally)| tally.fixes.len() as u32 >= t.min_inducing)
        .map(|((rel, function), tally)| DefectProneEvidence {
            file: top.join(rel),
            function: function.clone(),
            fix_count: tally.fixes.len() as u32,
            introducer_count: tally.introducers.len() as u32,
        })
        .collect();
    rows.sort_by(|a, b| {
        b.fix_count.cmp(&a.fix_count).then_with(|| a.file.cmp(&b.file)).then_with(|| a.function.cmp(&b.function))
    });
    rows.truncate(t.max_findings_reported as usize);
    rows.into_iter().map(|e| HistoryFinding { kind: HistoryKind::DefectProneFile(e), action_label: None }).collect()
}
