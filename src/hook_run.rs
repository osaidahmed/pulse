use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;

use crate::analyze::{self, AnalysisResultFull};
use crate::interaction::{tier_for, FindingTier};
use crate::smells::{self, Finding, Location};
use crate::{analytics, baselines, config, hook, output, parse, test_detection, thresholds};

const CHECKPOINT_INTERVAL: u32 = 5;
const CHECKPOINT_INTERVAL_NEW: u32 = 2;

fn is_checkpoint(edit_count: u32) -> bool {
    let interval = if edit_count <= CHECKPOINT_INTERVAL_NEW {
        CHECKPOINT_INTERVAL_NEW
    } else {
        CHECKPOINT_INTERVAL
    };
    edit_count.is_multiple_of(interval)
}

fn edit_scope_for(h: &hook::HookInput, edit_count: u32) -> Option<(usize, usize)> {
    if edit_count <= 1 || is_checkpoint(edit_count) {
        return None;
    }
    h.edit_byte_range
}

pub fn run_hook(h: hook::HookInput) {
    if std::env::var("PULSE_DISABLE").is_ok() || test_detection::is_test_file(&h.file_path) {
        return;
    }
    let path = Path::new(&h.file_path);
    let cfg_root = config::load_config_with_root(path);
    let cfg = cfg_root.as_ref().map(|(c, _)| c);
    if is_ignored_by(cfg_root.as_ref(), path) {
        return;
    }
    analytics::save_session_id(&h);

    let Ok(source) = std::fs::read_to_string(path) else {
        process::exit(0);
    };
    baselines::cache_baseline(&h, cfg, &source);
    let Some(lang) = parse::detect_language(path) else {
        process::exit(0);
    };
    let edit_count = baselines::increment_edit_count(&h.file_path);
    let scope = edit_scope_for(&h, edit_count);
    let Some(analysis) =
        analyze::analyze_source(&h.file_path, &source, lang, cfg, analyze::ScanOptions::hook(scope))
    else {
        process::exit(0);
    };

    let findings = collect_hook_findings(&h, &analysis, cfg, edit_count);
    if findings.is_empty() {
        process::exit(0);
    }
    analytics::log_findings(&h, &findings, &analysis.filename, &analysis.metrics.functions);
    emit_findings(&findings, &analysis);
}

fn is_ignored_by(cfg_root: Option<&(config::PulseConfig, PathBuf)>, path: &Path) -> bool {
    cfg_root.is_some_and(|(c, root)| config::is_ignored_with_root(c, root, path))
}

fn collect_hook_findings(
    h: &hook::HookInput,
    analysis: &AnalysisResultFull,
    cfg: Option<&config::PulseConfig>,
    edit_count: u32,
) -> Vec<Finding> {
    let func_baseline = baselines::load_function_baseline(&h.file_path);

    let func_findings: Vec<Finding> = analysis
        .findings
        .iter()
        .filter(|f| !matches!(f.location, Location::Module))
        .cloned()
        .collect();

    let mut findings: Vec<Finding> = hook::filter_by_edit_range(func_findings, h.edit_range)
        .into_iter()
        .filter(|f| !baselines::is_preexisting_finding(f, &func_baseline))
        .collect();

    collect_module_findings(&h.file_path, edit_count, &mut findings, cfg, analysis);
    findings
}

fn emit_findings(findings: &[Finding], analysis: &AnalysisResultFull) {
    let t = &analysis.thresholds;
    let ranked = crate::intensity::rank_findings(findings, &analysis.metrics, t);
    let (blocking, advisory): (Vec<Finding>, Vec<Finding>) =
        ranked.into_iter().partition(|f| tier_for(f.smell) == FindingTier::Blocking);
    let advisory_ctx = (!advisory.is_empty())
        .then(|| output::format_advisory(&advisory, &analysis.filename));
    if blocking.is_empty() {
        emit_advisory_only(advisory_ctx);
        return;
    }
    let reason = build_block_reason(&blocking, analysis, t);
    emit_decision(&reason, advisory_ctx);
}

fn build_block_reason(
    blocking: &[Finding],
    analysis: &AnalysisResultFull,
    t: &thresholds::Thresholds,
) -> String {
    let budget = format!(
        "[budget] fn={}/{} loc={}/{} cc={}/{}",
        analysis.fn_count(),
        t.module.file_function_count,
        analysis.total_loc(),
        t.module.file_loc_warning,
        analysis.sum_cc(),
        t.module.file_total_cc,
    );
    let compact = output::format_compact(blocking, &analysis.filename);
    match detect_constraint_conflict(blocking, analysis.fn_count(), t) {
        Some(note) => format!("{}\n{}\n{}", compact.trim(), note, budget),
        None => format!("{}\n{}", compact.trim(), budget),
    }
}

fn emit_decision(reason: &str, advisory_ctx: Option<String>) {
    let mut decision = serde_json::json!({ "decision": "block", "reason": reason.trim() });
    if let Some(ctx) = advisory_ctx {
        decision["hookSpecificOutput"] = advisory_payload(&ctx);
    }
    println!("{decision}");
}

fn emit_advisory_only(advisory_ctx: Option<String>) {
    let Some(ctx) = advisory_ctx else { return };
    let out = serde_json::json!({ "hookSpecificOutput": advisory_payload(&ctx) });
    println!("{out}");
}

fn advisory_payload(ctx: &str) -> serde_json::Value {
    serde_json::json!({
        "hookEventName": "PostToolUse",
        "additionalContext": ctx.trim(),
    })
}

fn detect_constraint_conflict(
    findings: &[Finding],
    fn_count: u32,
    t: &thresholds::Thresholds,
) -> Option<&'static str> {
    let fn_tight = fn_count + 2 >= t.module.file_function_count;
    let has_cc_finding = findings.iter().any(|f| {
        matches!(
            f.smell,
            smells::Smell::ComplexMethod | smells::Smell::GodMethod
        )
    });
    if fn_tight && has_cc_finding {
        return Some("[conflict] fn count and per-function complexity are both constrained — merge only low-cc functions");
    }
    None
}

fn collect_module_findings(
    file_path: &str,
    edit_count: u32,
    findings: &mut Vec<Finding>,
    cfg: Option<&config::PulseConfig>,
    analysis: &AnalysisResultFull,
) {
    if edit_count == 1 {
        let baseline = baselines::load_baseline(file_path);
        findings.extend(
            analysis
                .findings
                .iter()
                .filter(|f| matches!(f.location, Location::Module))
                .filter(|f| baseline.get(f.smell.as_str()).copied().unwrap_or(0) == 0)
                .cloned(),
        );
    }

    if is_checkpoint(edit_count) && !test_detection::is_test_file(file_path) {
        if let Some((_, regressions)) = detect_regressions(file_path, cfg, Some(analysis)) {
            findings.extend(regressions);
        }
    }
}

pub fn run_stop() {
    let Ok(manifest) = std::fs::read_to_string(baselines::baseline_dir().join("manifest.txt"))
    else {
        return;
    };

    let cfg = config::load_config(Path::new("."));
    let memo: RefCell<HashMap<String, Option<Rc<AnalysisResultFull>>>> =
        RefCell::new(HashMap::new());
    let analyze = |p: &str| -> Option<Rc<AnalysisResultFull>> {
        memo.borrow_mut()
            .entry(p.to_string())
            .or_insert_with(|| {
                analyze::analyze_file(p, cfg.as_ref(), analyze::ScanOptions::hook(None)).map(Rc::new)
            })
            .clone()
    };

    let mut all_regressions: Vec<(String, Vec<Finding>)> = Vec::new();
    for file_path in manifest.lines().filter(|l| !l.trim().is_empty()) {
        if test_detection::is_test_file(file_path) || baselines::is_fixture_file(file_path) {
            continue;
        }
        let analysis = analyze(file_path);
        if let Some((filename, regressions)) =
            detect_regressions(file_path, cfg.as_ref(), analysis.as_deref())
        {
            all_regressions.push((filename, regressions));
        }
    }

    if !all_regressions.is_empty() {
        let reason = output::format_stop(&all_regressions);
        let decision = serde_json::json!({
            "decision": "block",
            "reason": reason.trim()
        });
        println!("{decision}");
    }

    let move_pool = build_move_pool(&manifest, &analyze);
    analytics::resolve(&move_pool, |p| {
        analyze(p).map(|r| {
            let functions = r.metrics.functions.iter().map(|f| f.name.clone()).collect();
            (r.findings.clone(), functions)
        })
    });
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
}

fn build_move_pool<F: Fn(&str) -> Option<Rc<AnalysisResultFull>>>(
    manifest: &str,
    analyze: &F,
) -> HashMap<u64, HashSet<String>> {
    let mut pool: HashMap<u64, HashSet<String>> = HashMap::new();
    for file_path in manifest.lines().filter(|l| !l.trim().is_empty()) {
        if let Some(r) = analyze(file_path) {
            let key = canonical(file_path);
            for fm in &r.metrics.functions {
                pool.entry(fm.structural_hash).or_default().insert(key.clone());
            }
        }
    }
    pool
}

fn canonical(path: &str) -> String {
    std::fs::canonicalize(path)
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| path.to_string())
}

fn detect_regressions(
    file_path: &str,
    cfg: Option<&config::PulseConfig>,
    precomputed: Option<&AnalysisResultFull>,
) -> Option<(String, Vec<Finding>)> {
    let baseline = baselines::load_baseline(file_path);
    let owned = match precomputed {
        Some(_) => None,
        None => Some(analyze::analyze_file(file_path, cfg, analyze::ScanOptions::hook(None))?),
    };
    let result = precomputed.or(owned.as_ref())?;
    let regressions = analyze::module_regressions(result, &baseline);
    if regressions.is_empty() {
        return None;
    }
    Some((result.filename.clone(), regressions))
}

pub fn run_cleanup() {
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
}
