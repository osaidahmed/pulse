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
fn typescript_dead_store_on_redefinition() {
    let f = smells_of("function f() {\n  let x = 1;\n  x = 2;\n  return x;\n}\n", Language::TypeScript, "ts");
    assert!(has_finding(&f, Smell::DeadStore), "x = 1 is overwritten before any read: {f:?}");
}

#[test]
fn typescript_use_before_def_with_let_is_flagged() {
    let f = smells_of("function f() {\n  log(x);\n  let x = 1;\n  return x;\n}\n", Language::TypeScript, "ts");
    assert!(has_finding(&f, Smell::UseBeforeDef), "reading a let binding before its declaration is a TDZ error: {f:?}");
}

#[test]
fn typescript_declaration_without_initializer_is_not_a_dead_store() {
    let src = "function f(c) {\n  let x;\n  if (c) {\n    x = 1;\n  } else {\n    x = 2;\n  }\n  return x;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "a bare declaration is not a store; the branch assignments are read: {f:?}"
    );
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn typescript_for_counter_is_not_a_dead_store() {
    let src =
        "function f(n) {\n  let sum = 0;\n  for (let i = 0; i < n; i += 1) {\n    sum += i;\n  }\n  return sum;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(!has_finding(&f, Smell::DeadStore), "the loop counter and accumulator are both read: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn typescript_switch_fallthrough_preserves_earlier_store() {
    let src = "function f(k) {\n  let r = 0;\n  switch (k) {\n    case 1:\n      r = 1;\n    case 2:\n      r = r + 1;\n      break;\n  }\n  return r;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(!has_finding(&f, Smell::DeadStore), "case 1 falls through to case 2, which reads r: {f:?}");
}

#[test]
fn typescript_dead_store_inside_switch_case_flagged() {
    let src =
        "function f(k) {\n  switch (k) {\n    case 1:\n      let tmp = compute();\n      break;\n  }\n  return 0;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(has_finding(&f, Smell::DeadStore), "tmp is computed inside the case and never read: {f:?}");
}

#[test]
fn javascript_dead_store_on_redefinition() {
    let f = smells_of("function f() {\n  var x = 1;\n  x = 2;\n  return x;\n}\n", Language::JavaScript, "js");
    assert!(has_finding(&f, Smell::DeadStore), "var x = 1 is overwritten before any read: {f:?}");
}

#[test]
fn typescript_closure_capture_is_not_a_dead_store() {
    let src = "function f() {\n  let x = expensive();\n  const g = () => x;\n  return g();\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(!has_finding(&f, Smell::DeadStore), "x is captured and read by the closure body: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn javascript_closure_capture_is_not_a_dead_store() {
    let src = "function f() {\n  var x = expensive();\n  setTimeout(function () {\n    use(x);\n  });\n}\n";
    let f = smells_of(src, Language::JavaScript, "js");
    assert!(!has_finding(&f, Smell::DeadStore), "x is captured by the nested function: {f:?}");
}

#[test]
fn typescript_partial_array_destructure_is_not_a_dead_store() {
    let src = "function f() {\n  const [a, b] = getArr();\n  return a;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "destructuring is one atomic bind; an unused element is not a dead store: {f:?}"
    );
}

#[test]
fn typescript_partial_object_destructure_is_not_a_dead_store() {
    let src = "function f() {\n  const { a, b } = getObj();\n  return a;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(!has_finding(&f, Smell::DeadStore), "an unused destructured property is not a dead store: {f:?}");
}

#[test]
fn typescript_empty_switch_case_preserves_fallthrough() {
    let src = "function f(k) {\n  switch (k) {\n    case 1:\n      let x = 1;\n      use2(x);\n    case 2:\n    case 3:\n      use(x);\n      break;\n  }\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(
        !has_finding(&f, Smell::UseBeforeDef),
        "x defined in case 1 reaches case 3 through the empty case 2; the chain must hold: {f:?}"
    );
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
fn go_dead_store_on_redefinition() {
    let src = "package main\nfunc f() int {\n\tx := 0\n\tx = 1\n\treturn x\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(has_finding(&f, Smell::DeadStore), "x := 0 is overwritten before any read: {f:?}");
}

#[test]
fn go_multi_name_var_spec_defines_every_name() {
    let src =
        "package main\nfunc f() int {\n\tvar x, y int = 1, 2\n\tprintln(y)\n\ty = 5\n\tprintln(x)\n\treturn y\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(
        !has_finding(&f, Smell::UseBeforeDef),
        "`var x, y` defines both names; the second must not be a use-before-def: {f:?}"
    );
}

#[test]
fn go_use_before_def_flagged() {
    let src = "package main\nfunc f() int {\n\ty := x\n\tx := 1\n\treturn y\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(has_finding(&f, Smell::UseBeforeDef), "x is read before its declaration: {f:?}");
}

#[test]
fn go_clean_function_has_no_cpg_smells() {
    let src = "package main\nfunc f(a int) int {\n\tx := a + 1\n\treturn x\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn go_pre_switch_store_survives_no_match_path() {
    let src = "package main\nfunc f(k int) int {\n\tx := 0\n\tswitch k {\n\tcase 1:\n\t\tx = 1\n\t}\n\treturn x\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "x := 0 is read when no case matches; the switch must not unconditionally kill it: {f:?}"
    );
}

#[test]
fn go_range_loop_collection_and_blank_are_not_dead_stores() {
    let src = "package main\nfunc f() int {\n\txs := getList()\n\tsum := 0\n\tfor _, x := range xs {\n\t\tsum += x\n\t}\n\treturn sum\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(!has_finding(&f, Smell::DeadStore), "xs is read by the range and `_` is the blank identifier: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn go_field_write_is_not_a_dead_store() {
    let src = "package main\nfunc f() {\n\to := newObj()\n\to.field = 1\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "a field write reads the receiver; it is not a local dead store: {f:?}"
    );
}

#[test]
fn go_type_switch_alias_is_not_use_before_def() {
    let src = "package main\nfunc f(x any) {\n\tswitch v := x.(type) {\n\tcase int:\n\t\tuse(v)\n\t}\n\tv := 0\n\tuse2(v)\n}\n";
    let f = smells_of(src, Language::Go, "go");
    assert!(
        !has_finding(&f, Smell::UseBeforeDef),
        "the type-switch binding defines v; it must not collide with a separate local: {f:?}"
    );
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
fn java_dead_store_on_redefinition() {
    let src = "class C {\n  int f() {\n    int x = 1;\n    x = 2;\n    return x;\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(has_finding(&f, Smell::DeadStore), "x = 1 is overwritten before any read: {f:?}");
}

#[test]
fn java_use_before_def_flagged() {
    let src = "class C {\n  int f() {\n    int y = x;\n    int x = 1;\n    return y;\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(has_finding(&f, Smell::UseBeforeDef), "x is read before its declaration: {f:?}");
}

#[test]
fn java_clean_function_has_no_cpg_smells() {
    let src = "class C {\n  int f(int a) {\n    int x = a + 1;\n    return x;\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn java_pre_switch_store_survives_no_match_path() {
    let src = "class C {\n  int f(int k) {\n    int x = 0;\n    switch (k) {\n      case 1:\n        x = 1;\n        break;\n    }\n    return x;\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "x = 0 is read when no case matches; the switch must not unconditionally kill it: {f:?}"
    );
}

#[test]
fn java_augmented_assignment_counter_is_not_a_dead_store() {
    let src = "class C {\n  int f(int[] xs) {\n    int sum = 0;\n    for (int x : xs) {\n      sum += x;\n    }\n    return sum;\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(!has_finding(&f, Smell::DeadStore), "sum is read by the += accumulation: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn java_field_write_is_not_a_dead_store() {
    let src = "class C {\n  int f(Obj o) {\n    o.field = 1;\n    return 0;\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "a field write reads the receiver; it is not a local dead store: {f:?}"
    );
}

#[test]
fn java_lambda_capture_is_not_a_dead_store() {
    let src = "class C {\n  Runnable f() {\n    int x = compute();\n    return () -> use(x);\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(!has_finding(&f, Smell::DeadStore), "x is captured and read by the lambda: {f:?}");
}

#[test]
fn java_switch_pattern_binding_is_a_def_not_use_before_def() {
    let src = "class C {\n  void f(Object o) {\n    switch (o) {\n      case String s:\n        println(s);\n        break;\n      default:\n        break;\n    }\n    int s = compute();\n    use(s);\n  }\n}\n";
    let f = smells_of(src, Language::Java, "java");
    assert!(
        !has_finding(&f, Smell::UseBeforeDef),
        "a pattern binding defines its name; it must not collide with a separate local: {f:?}"
    );
}

#[test]
fn typescript_field_write_is_not_a_dead_store() {
    let src = "function f(obj) {\n  obj.x = 1;\n  return 0;\n}\n";
    let f = smells_of(src, Language::TypeScript, "ts");
    assert!(!has_finding(&f, Smell::DeadStore), "a member write reads the receiver; it is not a dead store: {f:?}");
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
fn csharp_var_initializer_dead_store_is_flagged() {
    let src = "class C {\n  int F() {\n    var x = Compute();\n    x = 5;\n    return x;\n  }\n}\n";
    let f = smells_of(src, Language::CSharp, "cs");
    assert!(has_finding(&f, Smell::DeadStore), "the var initializer is tracked and overwritten before any read: {f:?}");
}

#[test]
fn csharp_use_before_def_flagged() {
    let src = "class C {\n  int F() {\n    int y = x;\n    int x = 1;\n    return y;\n  }\n}\n";
    let f = smells_of(src, Language::CSharp, "cs");
    assert!(has_finding(&f, Smell::UseBeforeDef), "x is read before its declaration: {f:?}");
}

#[test]
fn csharp_clean_function_has_no_cpg_smells() {
    let src = "class C {\n  int F(int a) {\n    var x = a + 1;\n    return x;\n  }\n}\n";
    let f = smells_of(src, Language::CSharp, "cs");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn csharp_pre_switch_store_survives_no_match_path() {
    let src = "class C {\n  int F(int k) {\n    int x = 0;\n    switch (k) {\n      case 1:\n        x = 1;\n        break;\n    }\n    return x;\n  }\n}\n";
    let f = smells_of(src, Language::CSharp, "cs");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "x = 0 is read when no case matches; the switch must not unconditionally kill it: {f:?}"
    );
}

#[test]
fn csharp_switch_pattern_binding_is_a_def_not_use_before_def() {
    let src = "class C {\n  void F(object o) {\n    switch (o) {\n      case string s:\n        Use(s);\n        break;\n      default:\n        break;\n    }\n    int s = Compute();\n    Use2(s);\n  }\n}\n";
    let f = smells_of(src, Language::CSharp, "cs");
    assert!(
        !has_finding(&f, Smell::UseBeforeDef),
        "a pattern binding defines its name; it must not collide with a separate local: {f:?}"
    );
}

#[test]
fn csharp_field_write_is_not_a_dead_store() {
    let src = "class C {\n  int F(Obj o) {\n    o.Field = 1;\n    return 0;\n  }\n}\n";
    let f = smells_of(src, Language::CSharp, "cs");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "a member write reads the receiver; it is not a local dead store: {f:?}"
    );
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
fn c_dead_store_on_redefinition() {
    let f = smells_of("int f() {\n  int x = 0;\n  x = 1;\n  return x;\n}\n", Language::C, "c");
    assert!(has_finding(&f, Smell::DeadStore), "x = 0 is overwritten before any read: {f:?}");
}

#[test]
fn c_use_before_def_flagged() {
    let f = smells_of("int f() {\n  int y = x;\n  int x = 1;\n  return y;\n}\n", Language::C, "c");
    assert!(has_finding(&f, Smell::UseBeforeDef), "x is read before its declaration: {f:?}");
}

#[test]
fn c_clean_function_has_no_cpg_smells() {
    let f = smells_of("int f(int a) {\n  int x = a + 1;\n  return x;\n}\n", Language::C, "c");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "a param read is not use-before-def: {f:?}");
}

#[test]
fn c_uninitialized_declaration_is_not_use_before_def() {
    let f = smells_of("int f() {\n  int y;\n  y = 5;\n  return y;\n}\n", Language::C, "c");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "a bare declaration is not a read of the variable: {f:?}");
}

#[test]
fn c_pre_switch_store_survives_no_match_path() {
    let src =
        "int f(int k) {\n  int x = 0;\n  switch (k) {\n    case 1:\n      x = 1;\n      break;\n  }\n  return x;\n}\n";
    let f = smells_of(src, Language::C, "c");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "x = 0 is read when no case matches; the switch must not unconditionally kill it: {f:?}"
    );
}

#[test]
fn c_field_write_is_not_a_dead_store() {
    let f = smells_of("int f() {\n  struct S *o = get();\n  o->field = 1;\n  return 0;\n}\n", Language::C, "c");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "a field write reads the receiver; it is not a local dead store: {f:?}"
    );
}

#[test]
fn c_pointer_deref_write_is_not_a_dead_store() {
    let f = smells_of("int f() {\n  int x = 0;\n  int *p = &x;\n  *p = 5;\n  return x;\n}\n", Language::C, "c");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "*p = v writes through p, reading it; the pointer is not a dead store: {f:?}"
    );
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
fn cpp_dead_store_on_redefinition() {
    let f = smells_of("int f() {\n  int x = 0;\n  x = 1;\n  return x;\n}\n", Language::Cpp, "cpp");
    assert!(has_finding(&f, Smell::DeadStore), "x = 0 is overwritten before any read: {f:?}");
}

#[test]
fn cpp_range_for_is_clean() {
    let src = "int f() {\n  int sum = 0;\n  for (auto v : getItems()) {\n    sum += v;\n  }\n  return sum;\n}\n";
    let f = smells_of(src, Language::Cpp, "cpp");
    assert!(!has_finding(&f, Smell::DeadStore), "the range var and accumulator are read: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
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

#[test]
fn python_pre_match_store_survives_no_match_path() {
    let src = "def f(k):\n    r = 0\n    match k:\n        case 1:\n            r = 1\n    return r\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "r=0 is read when no case matches; modeling the match must preserve it: {f:?}"
    );
}

#[test]
fn rust_pre_match_store_survives_no_match_path() {
    let src = "fn f(k: i32) -> i32 {\n    let mut r = 0;\n    match k {\n        1 => r = 1,\n        _ => {}\n    }\n    r\n}\n";
    let f = smells_of(src, Language::Rust, "rs");
    assert!(!has_finding(&f, Smell::DeadStore), "r=0 survives the no-match path: {f:?}");
}

#[test]
fn rust_match_guard_read_is_not_a_dead_store() {
    let src =
        "fn f(k: i32) -> i32 {\n    let g = 7;\n    match k {\n        n if n > g => 1,\n        _ => 0,\n    }\n}\n";
    let f = smells_of(src, Language::Rust, "rs");
    assert!(!has_finding(&f, Smell::DeadStore), "g is read in the match-arm guard: {f:?}");
}

#[test]
fn python_match_guard_read_is_not_a_dead_store() {
    let src = "def f(k):\n    g = 7\n    match k:\n        case n if n > g:\n            return 1\n        case _:\n            return 0\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "g is read in the case guard: {f:?}");
}

#[test]
fn python_multi_subject_match_reads_all_subjects() {
    let src = "def f(x):\n    a = x\n    b = x\n    match a, b:\n        case (1, 2):\n            return 1\n        case _:\n            return 0\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "both subjects of `match a, b` are read: {f:?}");
}

#[test]
fn python_if_elif_without_else_falls_through() {
    let src = "def f(x):\n    if x > 0:\n        return 1\n    elif x < 0:\n        return 2\n    return 0\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(
        !has_finding(&f, Smell::UnreachableCode),
        "`return 0` is reachable when x == 0; the elif chain must preserve the fall-through: {f:?}"
    );
}

#[test]
fn python_elif_guard_read_is_not_a_dead_store() {
    let src =
        "def f(x):\n    g = 7\n    if x > 0:\n        return 1\n    elif x > g:\n        return 2\n    return 0\n";
    let f = smells_of(src, Language::Python, "py");
    assert!(!has_finding(&f, Smell::DeadStore), "g is read in the elif condition: {f:?}");
}

#[test]
fn php_unreachable_after_return_flagged() {
    let f = smells_of("<?php\nfunction f() {\n  return 1;\n  $dead = 2;\n}\n", Language::Php, "php");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn php_if_elseif_without_else_falls_through() {
    let src = "<?php\nfunction f($a) {\n  if ($a > 0) {\n    return 1;\n  } elseif ($a < 0) {\n    return 2;\n  }\n  log_it();\n}\n";
    let f = smells_of(src, Language::Php, "php");
    assert!(
        !has_finding(&f, Smell::UnreachableCode),
        "log_it() is reachable when $a == 0; the elseif chain must preserve the fall-through: {f:?}"
    );
}

#[test]
fn php_dead_store_on_redefinition() {
    let f = smells_of("<?php\nfunction f() {\n  $x = 0;\n  $x = 1;\n  return $x;\n}\n", Language::Php, "php");
    assert!(has_finding(&f, Smell::DeadStore), "$x = 0 is overwritten before any read: {f:?}");
}

#[test]
fn php_use_before_def_flagged() {
    let f = smells_of("<?php\nfunction f() {\n  echo $x;\n  $x = 1;\n  return $x;\n}\n", Language::Php, "php");
    assert!(has_finding(&f, Smell::UseBeforeDef), "$x is read before any assignment reaches it: {f:?}");
}

#[test]
fn php_clean_function_has_no_cpg_smells() {
    let f = smells_of("<?php\nfunction f($a) {\n  $x = $a + 1;\n  return $x;\n}\n", Language::Php, "php");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn php_pre_switch_store_survives_no_match_path() {
    let src = "<?php\nfunction f($k) {\n  $x = 0;\n  switch ($k) {\n    case 1:\n      $x = 1;\n      break;\n  }\n  return $x;\n}\n";
    let f = smells_of(src, Language::Php, "php");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "$x = 0 is read when no case matches; the switch must not unconditionally kill it: {f:?}"
    );
}

#[test]
fn php_augmented_assignment_is_not_a_dead_store() {
    let f = smells_of("<?php\nfunction f($a) {\n  $x = 0;\n  $x += $a;\n  return $x;\n}\n", Language::Php, "php");
    assert!(!has_finding(&f, Smell::DeadStore), "$x is read by the += accumulation: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn php_field_write_is_not_a_dead_store() {
    let f = smells_of("<?php\nfunction f() {\n  $o = make();\n  $o->field = 1;\n}\n", Language::Php, "php");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "a property write reads the receiver; it is not a local dead store: {f:?}"
    );
}

#[test]
fn php_global_declaration_is_not_use_before_def() {
    let f = smells_of("<?php\nfunction f() {\n  global $x;\n  $x = 1;\n  return $x;\n}\n", Language::Php, "php");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "a global declaration is not a read of the variable: {f:?}");
}

#[test]
fn php_global_write_only_is_not_a_dead_store() {
    let f = smells_of("<?php\nfunction f() {\n  global $x;\n  $x = compute();\n}\n", Language::Php, "php");
    assert!(!has_finding(&f, Smell::DeadStore), "a write to a global escapes the function; it is not dead: {f:?}");
}

#[test]
fn php_static_var_is_clean() {
    let f = smells_of("<?php\nfunction f() {\n  static $x = 0;\n  $x = 1;\n  return $x;\n}\n", Language::Php, "php");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
}

#[test]
fn php_dynamic_variable_assignment_is_not_a_dead_store() {
    let f = smells_of("<?php\nfunction f() {\n  $x = 'foo';\n  $$x = 1;\n  echo $x;\n}\n", Language::Php, "php");
    assert!(!has_finding(&f, Smell::DeadStore), "$$x = 1 reads $x to compute the target; $x is not dead: {f:?}");
}

#[test]
fn ruby_assignment_records_def_and_use() {
    let du = def_use_of("def f(a)\n  x = a\n  x\nend\n", Language::Ruby, "rb", "f");
    assert!(has(&du, "x", DefUse::Def), "{du:?}");
    assert!(has(&du, "a", DefUse::Use), "{du:?}");
    assert!(has(&du, "x", DefUse::Use), "{du:?}");
}

#[test]
fn ruby_if_condition_use_recorded() {
    let du = def_use_of("def f(flag)\n  if flag\n    return 1\n  end\n  0\nend\n", Language::Ruby, "rb", "f");
    assert!(has(&du, "flag", DefUse::Use), "{du:?}");
}

#[test]
fn ruby_if_else_has_predicate_and_both_branch_labels() {
    let cfg = cfg_of("def f(x)\n  if x\n    y = 1\n  else\n    y = 2\n  end\n  y\nend\n", Language::Ruby, "rb", "f");
    assert!(has_kind(&cfg, NodeKind::Predicate));
    assert!(has_label(&cfg, EdgeLabel::True), "{cfg:?}");
    assert!(has_label(&cfg, EdgeLabel::False), "{cfg:?}");
}

#[test]
fn ruby_loop_has_loop_head_and_back_edge() {
    let cfg = cfg_of(
        "def f(xs)\n  total = 0\n  for x in xs do\n    total = total + x\n  end\n  total\nend\n",
        Language::Ruby,
        "rb",
        "f",
    );
    assert!(has_kind(&cfg, NodeKind::LoopHead));
    assert!(has_label(&cfg, EdgeLabel::Back), "{cfg:?}");
}

#[test]
fn ruby_return_connects_to_exit() {
    let cfg = cfg_of("def f\n  return 1\nend\n", Language::Ruby, "rb", "f");
    assert!(cfg.edges.iter().any(|e| e.to == cfg.exit), "{cfg:?}");
}

#[test]
fn ruby_unreachable_after_return_flagged() {
    let f = smells_of("def f\n  return 1\n  dead = 2\nend\n", Language::Ruby, "rb");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn ruby_redefinition_dead_store_is_suppressed() {
    let f = smells_of("def f\n  x = 0\n  x = 1\n  x\nend\n", Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "ruby dead-store is suppressed: implicit-return tails + linear rescue/ensure make it FP-prone: {f:?}"
    );
}

#[test]
fn ruby_unused_local_before_reassign_is_suppressed() {
    let f = smells_of("def f\n  x = compute\n  x = other\n  puts(x)\nend\n", Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "ruby dead-store is suppressed even for a clear redefinition: {f:?}");
}

#[test]
fn ruby_write_only_local_is_suppressed() {
    let f = smells_of("def f\n  x = compute\n  log(\"done\")\nend\n", Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "ruby dead-store is suppressed even for a write-only local: {f:?}");
}

#[test]
fn ruby_use_before_def_flagged() {
    let f = smells_of("def f\n  puts y\n  y = 1\n  y\nend\n", Language::Ruby, "rb");
    assert!(has_finding(&f, Smell::UseBeforeDef), "y is read before any assignment reaches it: {f:?}");
}

#[test]
fn ruby_clean_method_has_no_cpg_smells() {
    let f = smells_of("def f(a)\n  x = a + 1\n  x\nend\n", Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn ruby_operator_assignment_is_not_a_dead_store() {
    let f = smells_of("def f(a)\n  x = 0\n  x += a\n  x\nend\n", Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "x is read by the += accumulation: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn ruby_unused_param_is_not_a_dead_store() {
    let f = smells_of("def f(a)\n  1\nend\n", Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "an unused parameter is seeded at entry, not a local dead store: {f:?}"
    );
}

#[test]
fn ruby_var_read_only_in_else_branch_is_not_a_dead_store() {
    let src = "def f(a)\n  x = compute\n  if a\n    other\n  else\n    use(x)\n  end\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "x is read inside the else branch; the else body must be threaded: {f:?}"
    );
}

#[test]
fn ruby_pre_case_store_survives_no_match_path() {
    let src = "def f(k)\n  r = 0\n  case k\n  when 1 then r = 1\n  when 2 then r = 2\n  end\n  r\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "r = 0 is read when no `when` matches; the case must preserve the no-match path: {f:?}"
    );
}

#[test]
fn ruby_case_when_arm_store_read_after_is_not_a_dead_store() {
    let src = "def f(k)\n  case k\n  when 1\n    r = 1\n  when 2\n    r = 2\n  end\n  use(r)\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`when` arms do not fall through; r = 1 reaches the read and must not be killed by r = 2: {f:?}"
    );
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn ruby_case_else_branch_read_is_not_a_dead_store() {
    let src = "def f(k)\n  x = compute\n  case k\n  when 1\n    other\n  else\n    use(x)\n  end\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "x is read in the case `else`; that branch must be threaded: {f:?}");
}

#[test]
fn ruby_block_capture_is_not_a_dead_store() {
    let src = "def f(xs)\n  total = 0\n  xs.each { |i| total += i }\n  total\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "total is captured and read inside the block: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn ruby_loop_accumulator_is_not_a_dead_store() {
    let src = "def f(xs)\n  total = 0\n  for x in xs do\n    total = total + x\n  end\n  total\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "the accumulator and loop var are read: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn ruby_while_body_and_condition_are_clean() {
    let src = "def f(n)\n  i = 0\n  while i < n do\n    i = i + 1\n  end\n  i\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "i is read in the condition, body, and after the loop: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn ruby_begin_rescue_is_clean() {
    let src = "def f\n  begin\n    x = risky\n    use(x)\n  rescue => e\n    log(e)\n  end\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "x is read inside the begin body: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "the rescue variable is not read before definition: {f:?}");
}

#[test]
fn ruby_elsif_guard_read_is_not_a_dead_store() {
    let src = "def f(x)\n  g = 7\n  if x > 0\n    return 1\n  elsif x > g\n    return 2\n  end\n  0\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "g is read in the elsif condition: {f:?}");
}

#[test]
fn ruby_attribute_write_is_not_a_dead_store() {
    let f = smells_of("def f(o)\n  o.x = 1\nend\n", Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`o.x = 1` mutates an attribute; `o` is a read, `x` a method name: {f:?}"
    );
}

#[test]
fn ruby_value_of_case_branch_assignments_are_not_dead_stores() {
    let src = "def classify(n)\n  case n\n  when 1\n    label = :one\n  else\n    label = :other\n  end\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "each case-branch's trailing assignment is the implicit return value: {f:?}"
    );
}

#[test]
fn ruby_nested_if_tail_assignments_are_not_dead_stores() {
    let src =
        "def f(a, b)\n  if a\n    if b\n      r = 1\n    else\n      r = 2\n    end\n  else\n    r = 3\n  end\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "tail assignments nested in if-branches are the implicit return: {f:?}"
    );
}

#[test]
fn ruby_or_assignment_tail_is_not_a_dead_store() {
    let src = "def fetch(id)\n  log(\"x\")\n  user = repo.find(id) or raise NotFound\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`user = ... or raise` is a binary tail; user is the implicit return on the success path: {f:?}"
    );
}

#[test]
fn ruby_parenthesized_tail_assignment_is_not_a_dead_store() {
    let f = smells_of("def f\n  (x = compute)\nend\n", Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "a parenthesized trailing assignment is the implicit return: {f:?}");
}

#[test]
fn ruby_ternary_tail_assignment_is_not_a_dead_store() {
    let f = smells_of("def pick(c)\n  c ? (a = 1) : (a = 2)\nend\n", Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "each ternary branch's assignment is the implicit return: {f:?}");
}

#[test]
fn ruby_multi_target_attribute_write_is_not_a_dead_store() {
    let src = "def update(point)\n  point.x, point.y = 1, 2\n  notify\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "parallel attribute assignment reads `point`; x/y are method names, not local stores: {f:?}"
    );
}

#[test]
fn ruby_scope_resolution_assignment_does_not_overtaint_the_namespace() {
    let src = "def install(foo)\n  foo::Bar = compute\n  log_done\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`foo::Bar = x` reads the namespace `foo`; only the constant is written: {f:?}"
    );
}

#[test]
fn ruby_plain_multiple_assignment_records_both_defs() {
    let du = def_use_of("def f\n  a, b = 1, 2\n  use(a)\nend\n", Language::Ruby, "rb", "f");
    assert!(has(&du, "a", DefUse::Def), "left_assignment_list unwrap records each plain target as a def: {du:?}");
    assert!(has(&du, "b", DefUse::Def), "left_assignment_list unwrap records each plain target as a def: {du:?}");
}

#[test]
fn ruby_index_write_is_not_a_dead_store() {
    let f = smells_of("def f(h, k, v)\n  h[k] = v\nend\n", Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "`h[k] = v` reads h and k; no local store occurs: {f:?}");
}

#[test]
fn ruby_setter_chain_builder_is_not_a_dead_store() {
    let src = "def configure(server, port)\n  config = Config.new\n  config.host = server\n  config.port = port\n  config\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(!has_finding(&f, Smell::DeadStore), "the builder receiver is configured via setters and returned: {f:?}");
}

#[test]
fn ruby_implicit_return_assignment_is_not_a_dead_store() {
    let f = smells_of("def fetch_user(id)\n  user = repository.find(id)\nend\n", Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "the trailing assignment's value is the method's implicit return; it is read by the caller: {f:?}"
    );
}

#[test]
fn ruby_value_of_if_branch_assignments_are_not_dead_stores() {
    let src = "def classify(n)\n  if n > 0\n    label = :positive\n  else\n    label = :negative\n  end\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "each branch's trailing assignment is the value of the if-expression and the implicit return: {f:?}"
    );
}

#[test]
fn ruby_modifier_conditional_assignment_is_not_a_dead_store() {
    let src = "def display_name(user)\n  name = user.login\n  name = user.full_name if user.full_name\n  name\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "the line-2 default is read when the postfix-if condition is falsy: {f:?}"
    );
}

#[test]
fn ruby_rescue_modifier_tail_is_not_a_dead_store() {
    let f =
        smells_of("def safe_total(rows)\n  log(:try)\n  parse(rows) rescue result = 0\nend\n", Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`expr rescue x = 0` returns x on the rescue path; suppressed because the tail model cannot seed it: {f:?}"
    );
}

#[test]
fn ruby_method_rescue_clause_is_not_a_dead_store() {
    let src = "def parse(input)\n  value = Integer(input)\nrescue ArgumentError\n  value = 0\nend\n";
    let f = smells_of(src, Language::Ruby, "rb");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "the success-path store is the implicit return; linear rescue threading would falsely kill it: {f:?}"
    );
}

#[test]
fn kotlin_property_records_def_and_use() {
    let du = def_use_of("fun f(a: Int): Int {\n    val x = a\n    return x\n}\n", Language::Kotlin, "kt", "f");
    assert!(has(&du, "x", DefUse::Def), "{du:?}");
    assert!(has(&du, "a", DefUse::Use), "{du:?}");
    assert!(has(&du, "x", DefUse::Use), "{du:?}");
}

#[test]
fn kotlin_if_else_has_predicate_and_both_branch_labels() {
    let src = "fun f(x: Boolean): Int {\n    if (x) {\n        return 1\n    } else {\n        return 2\n    }\n}\n";
    let cfg = cfg_of(src, Language::Kotlin, "kt", "f");
    assert!(has_kind(&cfg, NodeKind::Predicate));
    assert!(has_label(&cfg, EdgeLabel::True), "{cfg:?}");
    assert!(has_label(&cfg, EdgeLabel::False), "{cfg:?}");
}

#[test]
fn kotlin_if_condition_use_recorded() {
    let du = def_use_of(
        "fun f(flag: Boolean): Int {\n    if (flag) {\n        return 1\n    }\n    return 0\n}\n",
        Language::Kotlin,
        "kt",
        "f",
    );
    assert!(has(&du, "flag", DefUse::Use), "{du:?}");
}

#[test]
fn kotlin_loop_has_loop_head_and_back_edge() {
    let src = "fun f(n: Int) {\n    var i = 0\n    while (i < n) {\n        i = i + 1\n    }\n}\n";
    let cfg = cfg_of(src, Language::Kotlin, "kt", "f");
    assert!(has_kind(&cfg, NodeKind::LoopHead));
    assert!(has_label(&cfg, EdgeLabel::Back), "{cfg:?}");
}

#[test]
fn kotlin_return_connects_to_exit() {
    let cfg = cfg_of("fun f(): Int {\n    return 1\n}\n", Language::Kotlin, "kt", "f");
    assert!(cfg.edges.iter().any(|e| e.to == cfg.exit), "{cfg:?}");
}

#[test]
fn kotlin_unreachable_after_return_flagged() {
    let f = smells_of("fun f(): Int {\n    return 1\n    val dead = 2\n}\n", Language::Kotlin, "kt");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn kotlin_use_before_def_flagged() {
    let f = smells_of("fun f(): Int {\n    val a = b\n    val b = 1\n    return a\n}\n", Language::Kotlin, "kt");
    assert!(has_finding(&f, Smell::UseBeforeDef), "b is read before any declaration reaches it: {f:?}");
}

#[test]
fn kotlin_clean_function_has_no_cpg_smells() {
    let f = smells_of("fun f(a: Int): Int {\n    val x = a + 1\n    return x\n}\n", Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn kotlin_expression_body_is_clean() {
    let f = smells_of("fun f(a: Int): Int = a + 1\n", Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "the parameter is read in the expression body: {f:?}");
}

#[test]
fn kotlin_for_loop_accumulator_is_clean() {
    let src = "fun f(xs: List<Int>): Int {\n    var sum = 0\n    for (x in xs) {\n        sum = sum + x\n    }\n    return sum\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "the accumulator is defined before the loop: {f:?}");
}

#[test]
fn kotlin_dead_store_on_redefinition() {
    let f = smells_of("fun f(): Int {\n    var x = 0\n    x = 1\n    return x\n}\n", Language::Kotlin, "kt");
    assert!(has_finding(&f, Smell::DeadStore), "x = 0 is overwritten by x = 1 before any read: {f:?}");
}

#[test]
fn kotlin_unused_initializer_is_a_dead_store() {
    let f = smells_of("fun f(): Int {\n    var x = compute()\n    x = 5\n    return x\n}\n", Language::Kotlin, "kt");
    assert!(has_finding(&f, Smell::DeadStore), "x = compute() is never read before x = 5 overwrites it: {f:?}");
}

#[test]
fn kotlin_both_branch_redefinition_of_initializer_is_a_dead_store() {
    let src = "fun f(c: Boolean): String {\n    var label = \"\"\n    if (c) {\n        label = \"a\"\n    } else {\n        label = \"b\"\n    }\n    return label\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(
        has_finding(&f, Smell::DeadStore),
        "the \"\" initializer is overwritten on every path before any read: {f:?}"
    );
}

#[test]
fn kotlin_when_statement_multi_arm_assignment_is_not_a_dead_store() {
    let src = "fun f(n: Int): String {\n    var label = \"\"\n    when (n) {\n        1 -> label = \"one\"\n        2 -> label = \"two\"\n        else -> label = \"other\"\n    }\n    return label\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "when arms do not fall through; each arm's store reaches the post-when read: {f:?}"
    );
}

#[test]
fn kotlin_when_statement_label_reading_a_local_is_not_a_dead_store() {
    let src = "fun f(n: Int): String {\n    val limit = compute()\n    var r = \"\"\n    when {\n        n > limit -> r = \"hi\"\n        else -> r = \"lo\"\n    }\n    return r\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`limit` is read only in a when-arm label; the condition must be collected as a use: {f:?}"
    );
}

#[test]
fn kotlin_destructuring_partial_use_is_not_a_dead_store() {
    let f = smells_of(
        "fun f(pair: Pair<Int, Int>): Int {\n    val (a, b) = pair\n    return a\n}\n",
        Language::Kotlin,
        "kt",
    );
    assert!(!has_finding(&f, Smell::DeadStore), "an unused destructured component is not a dead store: {f:?}");
}

#[test]
fn kotlin_destructuring_placeholder_is_clean() {
    let f = smells_of(
        "fun f(pair: Pair<Int, Int>): Int {\n    val (a, _) = pair\n    return a\n}\n",
        Language::Kotlin,
        "kt",
    );
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn kotlin_lambda_capture_mutation_is_not_a_dead_store() {
    let f = smells_of(
        "fun f(xs: List<Int>): Int {\n    var acc = 0\n    xs.forEach { acc += it }\n    return acc\n}\n",
        Language::Kotlin,
        "kt",
    );
    assert!(!has_finding(&f, Smell::DeadStore), "acc is read+written inside the captured lambda: {f:?}");
}

#[test]
fn kotlin_write_only_lambda_capture_is_not_a_dead_store() {
    let f =
        smells_of("fun f(xs: List<Int>) {\n    var sum = 0\n    xs.forEach { sum += it }\n}\n", Language::Kotlin, "kt");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "the lambda body is opaque; the capture is recorded as a read, never a killing def: {f:?}"
    );
}

#[test]
fn kotlin_index_write_is_not_a_dead_store() {
    let f = smells_of("fun f(a: IntArray, i: Int) {\n    a[i] = 9\n}\n", Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "`a[i] = 9` reads a and i; no local store occurs: {f:?}");
}

#[test]
fn kotlin_when_subject_local_is_not_a_dead_store() {
    let src = "fun f(n: Int): Int {\n    val key = n * 2 + 1\n    var out = 0\n    when (key) {\n        1 -> out = 10\n        else -> out = 20\n    }\n    return out\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "`key` is read by the when subject; it is not dead: {f:?}");
}

#[test]
fn kotlin_typed_local_does_not_leak_the_type_as_a_dead_store() {
    let f = smells_of(
        "fun f(items: List<Int>): Int {\n    val count: Int = items.size\n    return count\n}\n",
        Language::Kotlin,
        "kt",
    );
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "a type annotation is not a stored value; `Int` must not be flagged: {f:?}"
    );
}

#[test]
fn kotlin_typed_local_redefinition_still_flags_the_variable() {
    let f = smells_of("fun f(): Int {\n    var x: Int = 0\n    x = 1\n    return x\n}\n", Language::Kotlin, "kt");
    assert!(has_finding(&f, Smell::DeadStore), "the typed local `x = 0` is overwritten before any read: {f:?}");
}

#[test]
fn kotlin_for_loop_over_local_collection_is_not_a_dead_store() {
    let src = "fun f(): Int {\n    val items = listOf(1, 2, 3)\n    var sum = 0\n    for (x in items) {\n        sum += x\n    }\n    return sum\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "`items` is read as the for-loop iterable: {f:?}");
}

#[test]
fn kotlin_deferred_val_initialization_is_not_a_dead_store() {
    let src = "fun f(c: Boolean): Int {\n    val x: Int\n    if (c) {\n        x = 1\n    } else {\n        x = 2\n    }\n    return x\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`val x: Int` stores no value; the real stores are the branch assignments: {f:?}"
    );
}

#[test]
fn kotlin_bare_property_write_is_not_a_dead_store() {
    let src = "class C {\n    var count = 0\n    fun reset() {\n        count = 0\n    }\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`count` is a class property, not a local; the write is not dead: {f:?}"
    );
}

#[test]
fn kotlin_short_form_interpolation_read_is_not_a_dead_store() {
    let f = smells_of(
        "fun describe(): String {\n    val name = lookupName()\n    return \"user is $name\"\n}\n",
        Language::Kotlin,
        "kt",
    );
    assert!(!has_finding(&f, Smell::DeadStore), "`name` is read by the `$name` interpolation: {f:?}");
}

#[test]
fn kotlin_unicode_interpolation_read_is_not_a_dead_store() {
    let f = smells_of(
        "fun f(): String {\n    val café = compute()\n    return \"value $café end\"\n}\n",
        Language::Kotlin,
        "kt",
    );
    assert!(!has_finding(&f, Smell::DeadStore), "`café` is read by the `$café` interpolation: {f:?}");
}

#[test]
fn kotlin_function_type_param_label_is_not_a_dead_store() {
    let src = "fun setup() {\n    val onEvent: (event: Event) -> Unit = { e -> log(e) }\n    register(onEvent)\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "`event` is a function-type parameter label, not a value: {f:?}");
}

#[test]
fn unqualified_field_write_is_not_a_dead_store_across_langs() {
    let java = smells_of(
        "class C {\n    int count;\n    void reset() {\n        count = 0;\n    }\n}\n",
        Language::Java,
        "java",
    );
    assert!(!has_finding(&java, Smell::DeadStore), "java bare field write: {java:?}");
    let swift = smells_of(
        "class C {\n    var count = 0\n    func reset() {\n        count = 0\n    }\n}\n",
        Language::Swift,
        "swift",
    );
    assert!(!has_finding(&swift, Smell::DeadStore), "swift bare property write: {swift:?}");
    let cs = smells_of(
        "class C {\n    int count;\n    void Reset() {\n        count = 0;\n    }\n}\n",
        Language::CSharp,
        "cs",
    );
    assert!(!has_finding(&cs, Smell::DeadStore), "c# bare field write: {cs:?}");
    let cpp = smells_of(
        "class C {\n    int count;\n    void reset() {\n        count = 0;\n    }\n};\n",
        Language::Cpp,
        "cpp",
    );
    assert!(!has_finding(&cpp, Smell::DeadStore), "c++ bare field write: {cpp:?}");
}

#[test]
fn declared_local_dead_store_still_flagged_across_langs() {
    let java = smells_of(
        "class C {\n    int f() {\n        int x = 0;\n        x = 1;\n        return x;\n    }\n}\n",
        Language::Java,
        "java",
    );
    assert!(has_finding(&java, Smell::DeadStore), "java declared local dead store: {java:?}");
    let go = smells_of("func f() int {\n    x := 0\n    x = 1\n    return x\n}\n", Language::Go, "go");
    assert!(has_finding(&go, Smell::DeadStore), "go declared local dead store: {go:?}");
}

#[test]
fn kotlin_value_of_when_assignment_is_not_a_dead_store() {
    let src = "fun f(n: Int): String {\n    val label = when (n) {\n        1 -> \"one\"\n        else -> \"other\"\n    }\n    return label\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "label is assigned the when-expression value and returned: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn kotlin_attribute_write_is_not_a_dead_store() {
    let f = smells_of("fun f(o: Obj) {\n    o.x = 1\n}\n", Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::DeadStore), "`o.x = 1` mutates an attribute; it is not a local store: {f:?}");
}

#[test]
fn kotlin_braceless_if_return_keeps_following_code_reachable() {
    let f = smells_of("fun f(c: Boolean) {\n    if (c) return\n    doSomething()\n}\n", Language::Kotlin, "kt");
    assert!(
        !has_finding(&f, Smell::UnreachableCode),
        "`doSomething()` runs when c is false; a dropped braceless return must not make it unreachable: {f:?}"
    );
}

#[test]
fn kotlin_braceless_for_body_is_clean() {
    let f = smells_of("fun f(xs: List<Int>) {\n    for (x in xs) println(x)\n    done()\n}\n", Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::UnreachableCode), "code after a braceless-body loop is reachable: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn kotlin_when_subject_binding_is_clean() {
    let src = "fun g(n: Int): String {\n    val r = when (val k = n % 2) {\n        0 -> \"even\"\n        else -> \"odd\"\n    }\n    return r\n}\n";
    let f = smells_of(src, Language::Kotlin, "kt");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "the when-subject binding `k` must not be flagged: {f:?}");
    assert!(!has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn swift_property_records_def_and_use() {
    let du = def_use_of("func f(a: Int) -> Int {\n    let x = a\n    return x\n}\n", Language::Swift, "swift", "f");
    assert!(has(&du, "x", DefUse::Def), "{du:?}");
    assert!(has(&du, "a", DefUse::Use), "{du:?}");
    assert!(has(&du, "x", DefUse::Use), "{du:?}");
}

#[test]
fn swift_if_else_has_predicate_and_both_branch_labels() {
    let src = "func f(c: Bool) -> Int {\n    if c {\n        return 1\n    } else {\n        return 2\n    }\n}\n";
    let cfg = cfg_of(src, Language::Swift, "swift", "f");
    assert!(has_kind(&cfg, NodeKind::Predicate));
    assert!(has_label(&cfg, EdgeLabel::True), "{cfg:?}");
    assert!(has_label(&cfg, EdgeLabel::False), "{cfg:?}");
}

#[test]
fn swift_if_condition_use_recorded() {
    let du = def_use_of(
        "func f(flag: Bool) -> Int {\n    if flag {\n        return 1\n    }\n    return 0\n}\n",
        Language::Swift,
        "swift",
        "f",
    );
    assert!(has(&du, "flag", DefUse::Use), "{du:?}");
}

#[test]
fn swift_loop_has_loop_head_and_back_edge() {
    let src = "func f(n: Int) {\n    var i = 0\n    while i < n {\n        i = i + 1\n    }\n}\n";
    let cfg = cfg_of(src, Language::Swift, "swift", "f");
    assert!(has_kind(&cfg, NodeKind::LoopHead));
    assert!(has_label(&cfg, EdgeLabel::Back), "{cfg:?}");
}

#[test]
fn swift_return_connects_to_exit() {
    let cfg = cfg_of("func f() -> Int {\n    return 1\n}\n", Language::Swift, "swift", "f");
    assert!(cfg.edges.iter().any(|e| e.to == cfg.exit), "{cfg:?}");
}

#[test]
fn swift_unreachable_after_return_flagged() {
    let f = smells_of("func f() -> Int {\n    return 1\n    let dead = 2\n}\n", Language::Swift, "swift");
    assert!(has_finding(&f, Smell::UnreachableCode), "{f:?}");
}

#[test]
fn swift_dead_store_on_redefinition() {
    let f = smells_of("func f() -> Int {\n    var x = 0\n    x = 1\n    return x\n}\n", Language::Swift, "swift");
    assert!(has_finding(&f, Smell::DeadStore), "x = 0 is overwritten before any read: {f:?}");
}

#[test]
fn swift_use_before_def_flagged() {
    let f = smells_of("func f() -> Int {\n    let a = b\n    let b = 1\n    return a\n}\n", Language::Swift, "swift");
    assert!(has_finding(&f, Smell::UseBeforeDef), "b is read before any declaration reaches it: {f:?}");
}

#[test]
fn swift_clean_function_has_no_cpg_smells() {
    let f = smells_of("func f(a: Int) -> Int {\n    let x = a + 1\n    return x\n}\n", Language::Swift, "swift");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn swift_attribute_write_is_not_a_dead_store() {
    let f = smells_of("func f(o: Obj) {\n    o.x = 1\n}\n", Language::Swift, "swift");
    assert!(!has_finding(&f, Smell::DeadStore), "`o.x = 1` reads the receiver; it is not a local store: {f:?}");
}

#[test]
fn swift_guard_binding_is_clean() {
    let src = "func f(opt: Int?) -> Int {\n    guard let x = opt else {\n        return 0\n    }\n    return x\n}\n";
    let f = smells_of(src, Language::Swift, "swift");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "the guard-bound x must not be flagged: {f:?}");
}

#[test]
fn swift_for_loop_accumulator_is_clean() {
    let src = "func f(xs: [Int]) -> Int {\n    var sum = 0\n    for x in xs {\n        sum = sum + x\n    }\n    return sum\n}\n";
    let f = smells_of(src, Language::Swift, "swift");
    assert!(!has_finding(&f, Smell::DeadStore), "the accumulator and loop var are read: {f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn swift_switch_is_clean() {
    let src = "func f(n: Int) -> String {\n    switch n {\n    case 0:\n        return \"zero\"\n    default:\n        return \"other\"\n    }\n}\n";
    let f = smells_of(src, Language::Swift, "swift");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(!has_finding(&f, Smell::UseBeforeDef), "{f:?}");
}

#[test]
fn swift_subscript_write_is_not_a_dead_store() {
    let f = smells_of("func f(_ a: inout [Int], _ i: Int) {\n    a[i] = 9\n}\n", Language::Swift, "swift");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "`a[i] = 9` writes an element; `a` and the index `i` are reads, not local stores: {f:?}"
    );
}

#[test]
fn swift_nested_subscript_write_is_not_a_dead_store() {
    let f =
        smells_of("func f(_ g: inout [[Int]], _ r: Int, _ c: Int) {\n    g[r][c] = 1\n}\n", Language::Swift, "swift");
    assert!(!has_finding(&f, Smell::DeadStore), "nested subscript indices are reads, not stores: {f:?}");
}

#[test]
fn swift_multi_clause_if_threads_the_body() {
    let src = "func g(a: Int, b: Int) -> Int {\n    let total = a + b\n    if a > 0, b > 0 {\n        return total\n    }\n    return 0\n}\n";
    let f = smells_of(src, Language::Swift, "swift");
    assert!(
        !has_finding(&f, Smell::DeadStore),
        "comma-separated conditions must not be mistaken for the then-body; `total` is read inside it: {f:?}"
    );
}

#[test]
fn swift_multi_clause_guard_is_clean() {
    let src = "func f(a: Int?, b: Int) -> Int {\n    guard let x = a, b > 0 else {\n        return 0\n    }\n    return x + b\n}\n";
    let f = smells_of(src, Language::Swift, "swift");
    assert!(!has_finding(&f, Smell::DeadStore), "{f:?}");
    assert!(
        !has_finding(&f, Smell::UseBeforeDef),
        "the second guard clause `b > 0` reads b; it is not the body: {f:?}"
    );
}
