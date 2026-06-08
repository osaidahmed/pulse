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
    let func = result.metrics.functions.iter().find(|f| f.name == fname).expect("function present");
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
        n.kind == NodeKind::Stmt && n.id != cfg.entry && n.id != cfg.exit && !cfg.edges.iter().any(|e| e.to == n.id)
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
    analyze_source(&format!("t.{ext}"), src, lang, Some(&cfg), ScanOptions::check()).expect("analyze").findings
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
fn field_write_is_not_a_dead_store() {
    let f = smells_of("def f(self):\n    self.x = 1\n    return self\n", Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "field write is not a local dead store: {f:?}");
}

#[test]
fn subscript_write_is_not_a_dead_store() {
    let f = smells_of("def f(arr, i):\n    arr[i] = 0\n    return arr\n", Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "element write is not a local dead store: {f:?}");
}

#[test]
fn try_body_def_used_in_handler_no_false_positive() {
    let src = "def f():\n    try:\n        x = safe()\n        risky(x)\n    except Exception:\n        return x\n    return 0\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "try-body def reaches the handler: {f:?}");
}

#[test]
fn try_else_clause_use_is_not_dead_store() {
    let src = "def f(n):\n    try:\n        y = risky(n)\n    except Exception:\n        return 0\n    else:\n        return y\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "else-clause use keeps the try def live: {f:?}");
}

#[test]
fn reassigned_param_is_not_use_before_def() {
    let f = smells_of("def f(x):\n    x += 1\n    return x\n", Language::Python, "py");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "params are defined at entry: {f:?}");
}

#[test]
fn unused_param_is_not_a_dead_store() {
    let f = smells_of("def f(unused):\n    return 1\n", Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "an unused param is not a dead store: {f:?}");
}

#[test]
fn smell_arrays_are_index_aligned() {
    for (i, &s) in pulse::smells::ALL_SMELLS.iter().enumerate() {
        assert_eq!(s as usize, i, "ALL_SMELLS discriminant order broken at {i}");
        assert!(!s.as_str().is_empty(), "{s:?} display name");
        assert!(!s.snake_name().is_empty(), "{s:?} snake name");
        assert!(!pulse::output::action_for(s, "").is_empty(), "{s:?} action");
        assert_eq!(pulse::smells::smell_from_snake_case(s.snake_name()), Some(s), "{s:?} snake round-trip");
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

#[test]
fn typescript_if_else_has_predicate_and_branches() {
    let src = "function f(x) {\n  if (x) {\n    let y = 1;\n  } else {\n    let y = 2;\n  }\n  return 0;\n}\n";
    let cfg = cfg_of(src, Language::TypeScript, "ts", "f");
    assert!(has_kind(&cfg, NodeKind::Predicate));
    assert!(has_label(&cfg, EdgeLabel::True), "{cfg:?}");
    assert!(has_label(&cfg, EdgeLabel::False), "{cfg:?}");
}

#[test]
fn typescript_unreachable_after_return_flagged() {
    let f = smells_of("function f() {\n  return 1;\n  let dead = 2;\n}\n", Language::TypeScript, "ts");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn typescript_clean_function_has_no_cpg_smells() {
    let src = "function f(a, b) {\n  let s = a + b;\n  if (a) {\n    s = s + 1;\n  }\n  return s;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn typescript_arrow_switch_for_have_no_cpg_false_positive() {
    let src = "function f(items, k) {\n  items.forEach((item) => {\n    const label = item.name;\n    render(label);\n  });\n  let r = 0;\n  switch (k) {\n    case 1:\n      r = 1;\n      break;\n  }\n  for (let i = 0; i < 10; i += 1) {\n    use(i);\n  }\n  return r;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(!has_finding(&f, Smell::DeadStore), "arrow/switch/for must not produce a spurious dead store: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn javascript_unreachable_after_return_flagged() {
    let f = smells_of("function f() {\n  return 1;\n  var dead = 2;\n}\n", Language::JavaScript, "js");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn javascript_var_hoisting_is_not_use_before_def() {
    let f = smells_of("function f() {\n  log(x);\n  var x = 1;\n  return x;\n}\n", Language::JavaScript, "js");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "var is hoisted; reading it before the declaration is legal: {f:?}");
}

#[test]
fn javascript_clean_function_has_no_cpg_smells() {
    let f = smells_of("function f(a) {\n  var x = a + 1;\n  return x;\n}\n", Language::JavaScript, "js");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn go_unreachable_after_return_flagged() {
    let f = smells_of("package main\nfunc f() {\n\treturn\n\tg()\n}\n", Language::Go, "go");
    assert!(
        has_finding(&f, Smell::UnreachableCode),
        "go wraps statements in statement_list; the builder must descend: {f:?}"
    );
}

#[test]
fn go_if_else_clean_has_no_cpg_smells() {
    let src = "package main\nfunc f(a int) int {\n\tif a > 0 {\n\t\treturn 1\n\t} else {\n\t\treturn 2\n\t}\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(!has_finding(&f, Smell::UnreachableCode), "both branches return; nothing follows: {f:?}");
}

#[test]
fn java_unreachable_after_return_flagged() {
    let f = smells_of("class C {\n  void f() {\n    return;\n    int dead = 2;\n  }\n}\n", Language::Java, "java");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn java_if_else_clean_has_no_cpg_smells() {
    let src = "class C {\n  int f(int a) {\n    if (a > 0) {\n      return 1;\n    } else {\n      return 2;\n    }\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(!has_finding(&f, Smell::UnreachableCode), "an unwrapped else block must be walked, not orphaned: {f:?}");
}

#[test]
fn csharp_unreachable_after_return_flagged() {
    let f = smells_of("class C {\n  void F() {\n    return;\n    int dead = 2;\n  }\n}\n", Language::CSharp, "cs");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn csharp_if_else_clean_has_no_cpg_smells() {
    let src = "class C {\n  int F(int a) {\n    if (a > 0) {\n      return 1;\n    } else {\n      return 2;\n    }\n  }\n}\n";
    let f = smells_of(src, Language::CSharp, "cs");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn c_unreachable_after_return_flagged() {
    let f = smells_of("int f() {\n  return 1;\n  int dead = 2;\n}\n", Language::C, "c");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn c_if_else_clean_has_no_cpg_smells() {
    let src = "int f(int a) {\n  if (a > 0) {\n    return 1;\n  } else {\n    return 2;\n  }\n}\n";
    let f = smells_of(src, Language::C, "c");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn cpp_unreachable_after_return_flagged() {
    let f = smells_of("int f() {\n  return 1;\n  int dead = 2;\n}\n", Language::Cpp, "cpp");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn cpp_if_else_clean_has_no_cpg_smells() {
    let src = "int f(int a) {\n  if (a > 0) {\n    return 1;\n  } else {\n    return 2;\n  }\n}\n";
    let f = smells_of(src, Language::Cpp, "cpp");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn python_nested_function_local_is_not_a_false_positive() {
    let src = "def outer():\n    def inner():\n        tmp = make()\n        return tmp\n    return inner()\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "a nested fn local must not leak into the outer block: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn rust_closure_local_is_not_a_false_positive() {
    let src = "fn outer() -> i32 {\n    let g = || {\n        let r = make();\n        r\n    };\n    g()\n}\n";
    let f = smells_of(src, Language::Rust, "rs");
    assert!(!has_finding(&f, Smell::DeadStore), "a closure local must not leak into the outer block: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn rust_match_arm_local_is_not_a_false_positive() {
    let src = "fn f(k: i32) -> i32 {\n    let y = match k {\n        1 => {\n            let r = compute();\n            r\n        }\n        _ => 0,\n    };\n    y\n}\n";
    let f = smells_of(src, Language::Rust, "rs");
    assert!(!has_finding(&f, Smell::DeadStore), "a match-arm local defined-then-used must not be a dead store: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn python_match_case_local_is_not_a_false_positive() {
    let src = "def f(k):\n    match k:\n        case 1:\n            r = compute()\n            return r\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "a match-case local defined-then-used must not be a dead store: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}
