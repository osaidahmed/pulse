use pulse::analyze::{analyze_source, ScanOptions};
use pulse::config::PulseConfig;
use pulse::cpg::cfg::{Cfg, EdgeLabel, NodeKind};
use pulse::cpg::defuse::{DefUse, DefUseRecord};
use pulse::parse::Language;
use pulse::smells::{Finding, Smell};

fn cpg_config() -> PulseConfig {
    toml::from_str("[thresholds.cpg]\nenabled = true\n").unwrap()
}

fn cfg_of(src: &str, lang: Language, ext: &str, fname: &str) -> Cfg {
    let cfg = cpg_config();
    let path = format!("t.{ext}");
    let result = analyze_source(&path, src, lang, Some(&cfg), ScanOptions::check()).expect("analyze");
    let func = result
        .metrics
        .functions
        .iter()
        .find(|f| f.name == fname)
        .expect("function present");
    func.cpg.as_ref().expect("cpg populated when enabled").cfg.clone()
}

fn has_label(cfg: &Cfg, label: EdgeLabel) -> bool {
    cfg.edges.iter().any(|e| e.label == label)
}

fn has_kind(cfg: &Cfg, kind: NodeKind) -> bool {
    cfg.nodes.iter().any(|n| n.kind == kind)
}

#[test]
fn python_if_else_has_predicate_and_both_branch_labels() {
    let src = "def f(x):\n    if x:\n        y = 1\n    else:\n        y = 2\n    return y\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    assert!(has_kind(&cfg, NodeKind::Predicate));
    assert!(has_label(&cfg, EdgeLabel::True), "{cfg:?}");
    assert!(has_label(&cfg, EdgeLabel::False), "{cfg:?}");
}

#[test]
fn python_if_without_else_has_false_fallthrough() {
    let src = "def f(x):\n    if x:\n        y = 1\n    return 0\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    assert!(has_label(&cfg, EdgeLabel::False), "{cfg:?}");
}

#[test]
fn python_loop_has_loop_head_and_back_edge() {
    let src = "def f(xs):\n    total = 0\n    for x in xs:\n        total = total + x\n    return total\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    assert!(has_kind(&cfg, NodeKind::LoopHead));
    assert!(has_label(&cfg, EdgeLabel::Back), "{cfg:?}");
}

#[test]
fn python_return_connects_to_exit() {
    let src = "def f():\n    return 1\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    assert!(cfg.edges.iter().any(|e| e.to == cfg.exit), "{cfg:?}");
}

#[test]
fn python_statement_after_return_is_unreachable() {
    let src = "def f():\n    return 1\n    dead = 2\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    let orphan = cfg.nodes.iter().any(|n| {
        n.kind == NodeKind::Stmt
            && n.id != cfg.entry
            && n.id != cfg.exit
            && !cfg.edges.iter().any(|e| e.to == n.id)
    });
    assert!(orphan, "code after return should have no incoming edge: {cfg:?}");
}

#[test]
fn python_try_except_has_handler_and_to_handler_edge() {
    let src = "def f():\n    try:\n        risky()\n    except ValueError:\n        recover()\n    return 0\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    assert!(has_kind(&cfg, NodeKind::Handler));
    assert!(has_label(&cfg, EdgeLabel::ToHandler), "{cfg:?}");
}

#[test]
fn rust_if_else_has_predicate_and_branches() {
    let src = "fn f(x: bool) -> i32 {\n    if x {\n        let a = 1;\n        a\n    } else {\n        let b = 2;\n        b\n    }\n}\n";
    let cfg = cfg_of(src, Language::Rust, "rs", "f");
    assert!(has_kind(&cfg, NodeKind::Predicate));
    assert!(has_label(&cfg, EdgeLabel::True), "{cfg:?}");
    assert!(has_label(&cfg, EdgeLabel::False), "{cfg:?}");
}

#[test]
fn rust_while_loop_has_back_edge() {
    let src = "fn f(n: i32) {\n    let mut i = 0;\n    while i < n {\n        i += 1;\n    }\n}\n";
    let cfg = cfg_of(src, Language::Rust, "rs", "f");
    assert!(has_kind(&cfg, NodeKind::LoopHead));
    assert!(has_label(&cfg, EdgeLabel::Back), "{cfg:?}");
}

#[test]
fn cpg_disabled_by_default_yields_no_cfg() {
    let src = "def f(x):\n    if x:\n        return 1\n    return 0\n";
    let result = analyze_source("t.py", src, Language::Python, None, ScanOptions::check()).unwrap();
    let func = result.metrics.functions.iter().find(|f| f.name == "f").unwrap();
    assert!(func.cpg.is_none(), "cpg must be None when the feature is disabled");
}

fn def_use_of(src: &str, lang: Language, ext: &str, fname: &str) -> Vec<DefUseRecord> {
    let cfg = cpg_config();
    let path = format!("t.{ext}");
    let result = analyze_source(&path, src, lang, Some(&cfg), ScanOptions::check()).expect("analyze");
    let func = result.metrics.functions.iter().find(|f| f.name == fname).expect("function");
    func.cpg.as_ref().expect("cpg populated").def_use.clone()
}

fn has(records: &[DefUseRecord], name: &str, kind: DefUse) -> bool {
    records.iter().any(|r| r.name == name && r.kind == kind)
}

#[test]
fn python_assignment_records_def_and_use() {
    let du = def_use_of("def f(a):\n    x = a\n    return x\n", Language::Python, "py", "f");
    assert!(has(&du, "x", DefUse::Def), "{du:?}");
    assert!(has(&du, "a", DefUse::Use), "{du:?}");
    assert!(has(&du, "x", DefUse::Use), "{du:?}");
}

#[test]
fn python_augmented_assignment_is_def_and_use() {
    let du = def_use_of("def f():\n    x = 0\n    x += 1\n    return x\n", Language::Python, "py", "f");
    assert!(has(&du, "x", DefUse::Def), "{du:?}");
    assert!(has(&du, "x", DefUse::Use), "{du:?}");
}

#[test]
fn python_if_condition_use_recorded() {
    let du = def_use_of("def f(flag):\n    if flag:\n        return 1\n    return 0\n", Language::Python, "py", "f");
    assert!(has(&du, "flag", DefUse::Use), "{du:?}");
}

#[test]
fn rust_let_records_def_and_use() {
    let du = def_use_of("fn f(a: i32) -> i32 {\n    let x = a;\n    x\n}\n", Language::Rust, "rs", "f");
    assert!(has(&du, "x", DefUse::Def), "{du:?}");
    assert!(has(&du, "a", DefUse::Use), "{du:?}");
}

#[test]
fn def_use_records_carry_block_ids_in_range() {
    let du = def_use_of("def f(a):\n    x = a\n    return x\n", Language::Python, "py", "f");
    let cfg = cfg_of("def f(a):\n    x = a\n    return x\n", Language::Python, "py", "f");
    let n = cfg.nodes.len() as u32;
    assert!(!du.is_empty());
    assert!(du.iter().all(|r| r.block < n), "every record references a real cfg node");
}

#[test]
fn def_use_empty_when_cpg_disabled() {
    let src = "def f(a):\n    x = a\n    return x\n";
    let result = analyze_source("t.py", src, Language::Python, None, ScanOptions::check()).unwrap();
    let func = result.metrics.functions.iter().find(|f| f.name == "f").unwrap();
    assert!(func.cpg.is_none());
}

fn smells_of(src: &str, lang: Language, ext: &str) -> Vec<Finding> {
    let cfg = cpg_config();
    analyze_source(&format!("t.{ext}"), src, lang, Some(&cfg), ScanOptions::check())
        .expect("analyze")
        .findings
}

fn has_finding(findings: &[Finding], smell: Smell) -> bool {
    findings.iter().any(|f| f.smell == smell)
}

#[test]
fn unreachable_code_after_return_flagged() {
    let f = smells_of("def f():\n    return 1\n    dead = 2\n", Language::Python, "py");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn dead_store_flagged_on_redefinition_without_use() {
    let f = smells_of("def f():\n    x = 1\n    x = 2\n    return x\n", Language::Python, "py");
    assert!(has_finding(&f, Smell::DeadStore), "{f:?}");
}

#[test]
fn use_before_def_flagged() {
    let f = smells_of("def f():\n    print(x)\n    x = 1\n", Language::Python, "py");
    assert!(has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn clean_function_has_no_cpg_smells() {
    let f = smells_of("def f(a):\n    x = a + 1\n    return x\n", Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn param_use_is_not_use_before_def() {
    let f = smells_of("def f(a, b):\n    return a + b\n", Language::Python, "py");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "params are defined: {f:?}");
}

#[test]
fn cpg_smells_off_when_disabled() {
    let findings =
        analyze_source("t.py", "def f():\n    return 1\n    dead = 2\n", Language::Python, None, ScanOptions::check())
            .unwrap()
            .findings;
    assert!(!has_finding(&findings, Smell::UnreachableCode));
}

#[test]
fn smell_arrays_are_index_aligned() {
    for (i, &s) in pulse::smells::ALL_SMELLS.iter().enumerate() {
        assert_eq!(s as usize, i, "ALL_SMELLS discriminant order broken at {i}");
        assert!(!s.as_str().is_empty(), "{s:?} display name");
        assert!(!s.snake_name().is_empty(), "{s:?} snake name");
        assert!(!pulse::output::action_for(s, "").is_empty(), "{s:?} action");
        assert_eq!(
            pulse::smells::smell_from_snake_case(s.snake_name()),
            Some(s),
            "{s:?} snake round-trip"
        );
    }
}

#[test]
fn cfg_entry_and_exit_distinct() {
    let src = "def f():\n    x = 1\n    return x\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    assert_ne!(cfg.entry, cfg.exit);
    assert_eq!(cfg.nodes[cfg.entry as usize].kind, NodeKind::Entry);
    assert_eq!(cfg.nodes[cfg.exit as usize].kind, NodeKind::Exit);
}
