use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;

use pulse::analyze::{self, AnalysisResultFull};
use pulse::interaction::{tier_for, FindingTier};
use pulse::smells::{self, Finding, Location};
use pulse::{analytics, baselines, config, hook, output, parse, test_detection, thresholds};

const CHECKPOINT_INTERVAL: u32 = 5;
const CHECKPOINT_INTERVAL_NEW: u32 = 2;

fn is_checkpoint(edit_count: u32) -> bool {
    let interval = if edit_count <= CHECKPOINT_INTERVAL_NEW { CHECKPOINT_INTERVAL_NEW } else { CHECKPOINT_INTERVAL };
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
    let Some(analysis) = analyze::analyze_source(&h.file_path, &source, lang, cfg, analyze::ScanOptions::hook(scope))
    else {
        process::exit(0);
    };

    if scope.is_none() {
        pulse::analysis_cache::store(&h.file_path, &source, &analysis);
    }
    let mut findings = pulse::interaction::suppress_subsumed(collect_hook_findings(&h, &analysis, edit_count));
    findings.extend(pulse::import_check::run(&h.file_path, &source, lang, h.original_file.as_deref()));
    let clones = pulse::session_clones::check(&h.file_path, &analysis.metrics.functions, &analysis.thresholds);
    findings.extend(hook::filter_by_edit_range(clones, h.edit_range));
    if findings.is_empty() {
        process::exit(0);
    }
    analytics::log_findings(&h, &findings, &analysis.filename, &analysis.metrics.functions, &source);
    emit_findings(&findings, &analysis, lang, &source, path);
}

fn is_ignored_by(cfg_root: Option<&(config::PulseConfig, PathBuf)>, path: &Path) -> bool {
    cfg_root.is_some_and(|(c, root)| config::is_ignored_with_root(c, root, path))
}

fn collect_hook_findings(h: &hook::HookInput, analysis: &AnalysisResultFull, edit_count: u32) -> Vec<Finding> {
    let func_baseline = baselines::load_function_baseline(&h.file_path);

    let func_findings: Vec<Finding> =
        analysis.findings.iter().filter(|f| !matches!(f.location, Location::Module)).cloned().collect();

    let in_range = hook::filter_by_edit_range(func_findings, h.edit_range);
    let mut findings = pulse::baseline_ratchet::filter_worsened(in_range, &func_baseline, &analysis.metrics);

    collect_module_findings(&h.file_path, edit_count, &mut findings, analysis);
    findings
}

fn emit_findings(
    findings: &[Finding],
    analysis: &AnalysisResultFull,
    lang: parse::Language,
    source: &str,
    path: &Path,
) {
    let t = &analysis.thresholds;
    let ranked = pulse::intensity::rank_findings(findings, &analysis.metrics, lang, t);
    let (blocking, advisory): (Vec<Finding>, Vec<Finding>) =
        ranked.into_iter().partition(|f| tier_for(f.smell) == FindingTier::Blocking);
    let advisory_ctx = (!advisory.is_empty()).then(|| output::format_advisory(&advisory, &analysis.filename));
    if blocking.is_empty() {
        emit_advisory_only(advisory_ctx);
        return;
    }
    let reason = build_block_reason(&blocking, analysis, t, source, path);
    emit_decision(&reason, advisory_ctx);
}

fn build_block_reason(
    blocking: &[Finding],
    analysis: &AnalysisResultFull,
    t: &thresholds::Thresholds,
    source: &str,
    path: &Path,
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
    let mut reason = match detect_constraint_conflict(blocking, analysis.fn_count(), t) {
        Some(note) => format!("{}\n{}\n{}", compact.trim(), note, budget),
        None => format!("{}\n{}", compact.trim(), budget),
    };
    push_line(&mut reason, extract_hint_for(blocking, source, path));
    push_line(&mut reason, pulse::history::jit_risk::hook_advisory(source, path));
    reason
}

fn push_line(reason: &mut String, line: Option<String>) {
    if let Some(line) = line {
        reason.push('\n');
        reason.push_str(&line);
    }
}

fn extract_hint_for(blocking: &[Finding], source: &str, path: &Path) -> Option<String> {
    let lang = parse::detect_language(path)?;
    let finding = blocking.iter().find(|f| is_extractable(f.smell))?;
    let (start, end) = match &finding.location {
        Location::Function { start_line, end_line, .. } => (*start_line, *end_line),
        Location::Module => return None,
    };
    let region = pulse::extract::suggest_extract(source, lang, pulse::extract::LineSpan { start, end })?;
    Some(format!(
        "[extract] lines {}-{} are nested {} levels deep — extracting that block into a helper would flatten the function",
        region.start_line, region.end_line, region.nesting
    ))
}

fn is_extractable(smell: smells::Smell) -> bool {
    matches!(smell, smells::Smell::ComplexMethod | smells::Smell::GodMethod | smells::Smell::DeepNestedComplexity)
}

fn emit_decision(reason: &str, advisory_ctx: Option<String>) {
    println!("{}", hook::render_block(hook::active_protocol(), reason, advisory_ctx.as_deref()));
}

fn emit_advisory_only(advisory_ctx: Option<String>) {
    let Some(ctx) = advisory_ctx else { return };
    println!("{}", hook::render_advisory(hook::active_protocol(), &ctx));
}

fn detect_constraint_conflict(findings: &[Finding], fn_count: u32, t: &thresholds::Thresholds) -> Option<&'static str> {
    let fn_tight = fn_count + 2 >= t.module.file_function_count;
    let has_cc_finding =
        findings.iter().any(|f| matches!(f.smell, smells::Smell::ComplexMethod | smells::Smell::GodMethod));
    if fn_tight && has_cc_finding {
        return Some(
            "[conflict] fn count and per-function complexity are both constrained — merge only low-cc functions",
        );
    }
    None
}

fn collect_module_findings(
    file_path: &str,
    edit_count: u32,
    findings: &mut Vec<Finding>,
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
        if let Some((_, regressions)) = regressions_for(file_path, &analysis.filename, &analysis.findings) {
            findings.extend(regressions);
        }
    }
}

pub fn run_stop() {
    let manifest = std::fs::read_to_string(baselines::baseline_dir().join("manifest.txt")).unwrap_or_default();

    let memo: RefCell<HashMap<String, Option<Rc<StopAnalysis>>>> = RefCell::new(HashMap::new());
    let analyze = |p: &str| -> Option<Rc<StopAnalysis>> {
        memo.borrow_mut().entry(p.to_string()).or_insert_with(|| stop_analysis(p).map(Rc::new)).clone()
    };

    let mut all_regressions: Vec<(String, Vec<Finding>)> = Vec::new();
    let mut checked: HashSet<String> = HashSet::new();
    for file_path in manifest.lines().filter(|l| !l.trim().is_empty()) {
        checked.insert(canonical(file_path));
        if test_detection::is_test_file(file_path) || baselines::is_fixture_file(file_path) {
            continue;
        }
        let Some(analysis) = analyze(file_path) else { continue };
        if let Some((filename, regressions)) = regressions_for(file_path, &analysis.filename, &analysis.findings) {
            all_regressions.push((filename, regressions));
        }
    }
    all_regressions.extend(pulse::turn_scan::turn_regressions(&checked));

    if !all_regressions.is_empty() {
        let reason = output::format_stop(&all_regressions);
        println!("{}", hook::render_block(hook::active_protocol(), &reason, None));
    }

    let move_pool = build_move_pool(&manifest, &analyze);
    analytics::resolve(&move_pool, |p| {
        analyze(p).map(|r| {
            let functions = r.functions.iter().map(|(name, _)| name.clone()).collect();
            (r.findings.clone(), functions)
        })
    });
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
    pulse::turn_scan::stamp_turn_head();
}

struct StopAnalysis {
    filename: String,
    findings: Vec<Finding>,
    functions: Vec<(String, u64)>,
}

fn stop_analysis(file_path: &str) -> Option<StopAnalysis> {
    let path = Path::new(file_path);
    let filename = path.file_name()?.to_string_lossy().into_owned();
    let cfg = config::load_config(path);
    let lang = parse::detect_language(path)?;
    let expected = pulse::analysis_cache::thresholds_fingerprint(&config::resolve_thresholds(cfg.as_ref(), lang));
    if let Some(cached) = pulse::analysis_cache::load(file_path, expected) {
        return Some(StopAnalysis { filename, findings: cached.findings(), functions: cached.functions });
    }
    let r = analyze::analyze_file(file_path, cfg.as_ref(), analyze::ScanOptions::hook(None))?;
    let functions = r.metrics.functions.iter().map(|f| (f.name.clone(), f.structural_hash)).collect();
    Some(StopAnalysis { filename: r.filename, findings: r.findings, functions })
}

fn regressions_for(file_path: &str, filename: &str, findings: &[Finding]) -> Option<(String, Vec<Finding>)> {
    let baseline = baselines::load_baseline(file_path);
    let regressions = analyze::module_regressions(findings, &baseline);
    if regressions.is_empty() {
        return None;
    }
    Some((filename.to_string(), regressions))
}

fn build_move_pool<F: Fn(&str) -> Option<Rc<StopAnalysis>>>(
    manifest: &str,
    analyze: &F,
) -> HashMap<u64, HashSet<String>> {
    let mut pool: HashMap<u64, HashSet<String>> = HashMap::new();
    for file_path in manifest.lines().filter(|l| !l.trim().is_empty()) {
        if let Some(r) = analyze(file_path) {
            let key = canonical(file_path);
            for (_, hash) in r.functions.iter().filter(|(_, h)| *h != 0) {
                pool.entry(*hash).or_default().insert(key.clone());
            }
        }
    }
    pool
}

fn canonical(path: &str) -> String {
    std::fs::canonicalize(path).ok().and_then(|p| p.to_str().map(String::from)).unwrap_or_else(|| path.to_string())
}

pub fn run_cleanup() {
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
    pulse::turn_scan::stamp_turn_head();
}
