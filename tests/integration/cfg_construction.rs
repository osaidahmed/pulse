use pulse::analyze::{analyze_source, ScanOptions};
use pulse::config::PulseConfig;
use pulse::cpg::cfg::{Cfg, EdgeLabel, NodeKind};
use pulse::parse::Language;

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

#[test]
fn cfg_entry_and_exit_distinct() {
    let src = "def f():\n    x = 1\n    return x\n";
    let cfg = cfg_of(src, Language::Python, "py", "f");
    assert_ne!(cfg.entry, cfg.exit);
    assert_eq!(cfg.nodes[cfg.entry as usize].kind, NodeKind::Entry);
    assert_eq!(cfg.nodes[cfg.exit as usize].kind, NodeKind::Exit);
}
