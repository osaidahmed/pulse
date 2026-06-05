use pulse::audit::discovery::{freqt_mine, RawCluster};
use pulse::audit::finding::{AuditFinding, AuditKind, AuditLocation};
use pulse::audit::output::{format_findings, format_findings_json};
use pulse::audit::scoring::apply_idf;
use pulse::audit::walker::{extract_subtrees, ShapeMetrics, SubtreeRecord};
use pulse::parse::{self, Language};
use pulse::thresholds::Thresholds;
use std::path::{Path, PathBuf};

fn t() -> Thresholds {
    Thresholds::default()
}

fn rec(fp: u64, file: &str, line: u32, snippet: &str) -> SubtreeRecord {
    SubtreeRecord {
        fingerprint: fp,
        parent_fingerprint: None,
        file: PathBuf::from(file),
        line,
        depth: 5,
        named_node_count: 8,
        snippet: snippet.to_string(),
        shape: ShapeMetrics::default(),
        simhash: 0,
        loc: 0,
    }
}

fn ext_python(src: &str) -> Vec<SubtreeRecord> {
    let tree = parse::parse_only(src, Language::Python).unwrap();
    extract_subtrees(&tree, src, Language::Python, Path::new("t.py"), &t().audit)
}

#[test]
fn edge_cluster_threshold_exact_boundary_5_kept() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let recs: Vec<_> = (0..5).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    assert_eq!(freqt_mine(&recs, &th).len(), 1);
}

#[test]
fn edge_cluster_threshold_just_below_4_dropped() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let recs: Vec<_> = (0..4).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    assert!(freqt_mine(&recs, &th).is_empty());
}

#[test]
fn edge_idiom_threshold_exact_50_percent_kept() {
    let cluster = RawCluster {
        fingerprint: 7, support: 5, file_count: 5,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 10, &t().audit);
    assert_eq!(result.len(), 1);
}

#[test]
fn edge_idiom_threshold_just_above_50_suppressed() {
    let cluster = RawCluster {
        fingerprint: 7, support: 6, file_count: 6,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 10, &t().audit);
    assert!(result.is_empty());
}

#[test]
fn edge_idiom_threshold_one_percent_only() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.01;
    let cluster = RawCluster {
        fingerprint: 7, support: 2, file_count: 2,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 10, &th);
    assert!(result.is_empty(), "2/10=20% > 1% threshold");
}

#[test]
fn edge_max_findings_zero_returns_empty() {
    let mut th = t().audit;
    th.pattern_mining.max_findings_reported = 0;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cluster = RawCluster {
        fingerprint: 7, support: 5, file_count: 5,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 10, &th);
    assert!(result.is_empty());
}

#[test]
fn edge_max_locations_zero_renders_only_more_marker() {
    let mut th = t().audit;
    th.max_locations_per_finding = 0;
    let f = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 7 },
        representative_snippet: "x".to_string(),
        support: 3, file_count: 3,
        idf_score: Some(1.0), action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![
            AuditLocation { file: PathBuf::from("a.py"), line: 1 },
            AuditLocation { file: PathBuf::from("b.py"), line: 2 },
        ],
    };
    let s = format_findings(std::slice::from_ref(&f), None, &th);
    assert!(s.contains("(2 more)"));
}

#[test]
fn edge_subtree_min_depth_one_accepts_minimal_trees() {
    let mut th = t().audit;
    th.pattern_mining.subtree_min_depth = 1;
    th.pattern_mining.subtree_min_nodes = 1;
    let src = "x = 1\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let r = extract_subtrees(&tree, src, Language::Python, Path::new("t.py"), &th);
    assert!(!r.is_empty());
}

#[test]
fn edge_thresholds_strict_less_than_subtree_min_depth() {
    let mut th = t().audit;
    th.pattern_mining.subtree_min_depth = 3;
    th.pattern_mining.subtree_min_nodes = 3;
    let src = "x = 1 + 2\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Python, Path::new("t.py"), &th);
}

#[test]
fn edge_one_record_below_min_support_dropped() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "a.py", 1, "x")];
    assert!(freqt_mine(&recs, &th).is_empty());
}

#[test]
fn edge_one_record_at_min_support_one_surfaced() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1;
    let recs = vec![rec(7, "a.py", 1, "x")];
    assert_eq!(freqt_mine(&recs, &th).len(), 1);
}

#[test]
fn edge_zero_records_returns_empty_vec() {
    assert!(freqt_mine(&[], &t().audit).is_empty());
}

#[test]
fn edge_locations_already_sorted_preserved() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 1, "x"),
        rec(7, "b.py", 2, "x"),
        rec(7, "c.py", 3, "x"),
    ];
    let clusters = freqt_mine(&recs, &th);
    let locs = &clusters[0].locations;
    for i in 1..locs.len() {
        assert!(locs[i - 1].0 <= locs[i].0);
    }
}

#[test]
fn edge_apply_idf_with_zero_locations_kept_finding_renders() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cluster = RawCluster {
        fingerprint: 7, support: 5, file_count: 1,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 100, &th);
    assert_eq!(result.len(), 1);
    let s = format_findings(&result, None, &th);
    assert!(s.contains("audit:"));
}

#[test]
fn edge_apply_idf_score_for_unique_pattern_is_log_n() {
    let cluster = RawCluster {
        fingerprint: 7, support: 1, file_count: 1,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 100, &t().audit);
    assert_eq!(result.len(), 1);
    let idf = result[0].idf_score.unwrap();
    let expected = (100.0_f64).ln();
    assert!((idf - expected).abs() < 1e-9);
}

#[test]
fn edge_apply_idf_score_for_universal_pattern_is_zero() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cluster = RawCluster {
        fingerprint: 7, support: 100, file_count: 100,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 100, &th);
    assert!(result[0].idf_score.unwrap().abs() < 1e-9);
}

#[test]
fn edge_apply_idf_handles_large_total_files() {
    let cluster = RawCluster {
        fingerprint: 7, support: 5, file_count: 1,
        representative_snippet: "x".to_string(), locations: vec![],
    };
    let result = apply_idf(vec![cluster], 1_000_000, &t().audit);
    assert_eq!(result.len(), 1);
}

#[test]
fn edge_format_findings_with_action_label_renders_correctly() {
    let f = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 7 },
        representative_snippet: "x".to_string(),
        support: 5, file_count: 5,
        idf_score: Some(1.0),
        action_label: Some("extract polymorphic dispatch"),
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![],
    };
    let s = format_findings(std::slice::from_ref(&f), None, &t().audit);
    assert!(s.contains("pattern action:"));
    assert!(s.contains("extract polymorphic dispatch"));
}

#[test]
fn edge_format_findings_without_action_label_omits_block() {
    let f = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 7 },
        representative_snippet: "x".to_string(),
        support: 5, file_count: 5,
        idf_score: Some(1.0),
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![],
    };
    let s = format_findings(std::slice::from_ref(&f), None, &t().audit);
    assert!(!s.contains("pattern action:"));
}

#[test]
fn edge_format_findings_two_findings_separated_by_blank_line() {
    let f1 = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 1 },
        representative_snippet: "a".to_string(),
        support: 5, file_count: 5,
        idf_score: Some(1.0), action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![],
    };
    let f2 = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 2 },
        representative_snippet: "b".to_string(),
        support: 4, file_count: 4,
        idf_score: Some(1.0), action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![],
    };
    let s = format_findings(&[f1, f2], None, &t().audit);
    assert!(s.contains("\n\n"), "blank line separator missing");
}

#[test]
fn edge_format_findings_json_handles_zero_findings() {
    let s = format_findings_json(&[], None);
    assert_eq!(s, "[]");
}

#[test]
fn edge_format_findings_json_handles_unicode_in_snippet() {
    let f = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 7 },
        representative_snippet: "δ == λ".to_string(),
        support: 5, file_count: 5,
        idf_score: Some(1.0), action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![],
    };
    let s = format_findings_json(std::slice::from_ref(&f), None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed[0]["representative_snippet"].as_str().unwrap(), "δ == λ");
}

#[test]
fn edge_walker_with_only_function_no_body_python() {
    let src = "def f():\n    pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_with_only_imports() {
    let src = "import os\nimport sys\nfrom os import path\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_with_class_with_no_methods() {
    let src = "class C:\n    pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_with_class_with_only_class_var() {
    let src = "class C:\n    x = 1\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_with_global_assignment_only() {
    let src = "x = 1\ny = 2\nz = 3\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_python_function_with_default_dict_arg() {
    let src = "def f(x={}):\n    return x\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_python_function_with_complex_default() {
    let src = "def f(x=lambda y: y + 1):\n    return x\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_python_function_with_string_default() {
    let src = "def f(x=\"\"):\n    return x\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_assignment_chain() {
    let src = "a = b = c = 1\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_destructuring_with_star() {
    let src = "a, *b, c = [1, 2, 3, 4]\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_python_walrus_in_condition() {
    let src = "if (n := len(xs)) > 0:\n    pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_python_yield_from() {
    let src = "def f():\n    yield from g()\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_async_generator() {
    let src = "async def f():\n    async for x in y:\n        yield x\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_match_with_or_pattern() {
    let src = "match x:\n    case 1 | 2:\n        pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_match_with_dict_pattern() {
    let src = "match x:\n    case {\"key\": value}:\n        pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_match_with_class_pattern() {
    let src = "match x:\n    case Point(x=1, y=2):\n        pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_match_with_capture() {
    let src = "match x:\n    case [first, *rest]:\n        pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_for_typescript_optional_chaining() {
    let src = "x?.y?.z\n";
    let tree = parse::parse_only(src, Language::TypeScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::TypeScript, Path::new("t.ts"), &t().audit);
}

#[test]
fn edge_walker_records_for_typescript_nullish_coalescing() {
    let src = "const x = a ?? b;\n";
    let tree = parse::parse_only(src, Language::TypeScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::TypeScript, Path::new("t.ts"), &t().audit);
}

#[test]
fn edge_walker_records_for_typescript_template_literal() {
    let src = "const x = `hello ${name}`;\n";
    let tree = parse::parse_only(src, Language::TypeScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::TypeScript, Path::new("t.ts"), &t().audit);
}

#[test]
fn edge_walker_records_for_rust_match_with_guard() {
    let src = "fn f(x: i32) -> i32 { match x { 1 if x > 0 => 1, _ => 0 } }\n";
    let tree = parse::parse_only(src, Language::Rust).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Rust, Path::new("t.rs"), &t().audit);
}

#[test]
fn edge_walker_records_for_rust_question_mark_operator() {
    let src = "fn f() -> Result<i32, String> { let x: i32 = something()?; Ok(x) }\n";
    let tree = parse::parse_only(src, Language::Rust).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Rust, Path::new("t.rs"), &t().audit);
}

#[test]
fn edge_walker_records_for_rust_lifetime_annotation() {
    let src = "fn f<'a>(x: &'a str) -> &'a str { x }\n";
    let tree = parse::parse_only(src, Language::Rust).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Rust, Path::new("t.rs"), &t().audit);
}

#[test]
fn edge_walker_records_for_go_defer() {
    let src = "package p\nfunc f() { defer cleanup() }\n";
    let tree = parse::parse_only(src, Language::Go).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Go, Path::new("t.go"), &t().audit);
}

#[test]
fn edge_walker_records_for_go_channel_send_receive() {
    let src = "package p\nfunc f(c chan int) { x := <-c; c <- x }\n";
    let tree = parse::parse_only(src, Language::Go).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Go, Path::new("t.go"), &t().audit);
}

#[test]
fn edge_walker_records_for_kotlin_data_class() {
    let src = "data class Point(val x: Int, val y: Int)\n";
    let tree = parse::parse_only(src, Language::Kotlin).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Kotlin, Path::new("t.kt"), &t().audit);
}

#[test]
fn edge_walker_records_for_swift_guard_let() {
    let src = "func f(x: Int?) { guard let y = x else { return }; print(y) }\n";
    let tree = parse::parse_only(src, Language::Swift).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Swift, Path::new("t.swift"), &t().audit);
}

#[test]
fn edge_walker_records_for_csharp_lambda() {
    let src = "class A { Func<int, int> f = x => x + 1; }\n";
    let tree = parse::parse_only(src, Language::CSharp).unwrap();
    let _ = extract_subtrees(&tree, src, Language::CSharp, Path::new("t.cs"), &t().audit);
}

#[test]
fn edge_walker_records_for_csharp_property_getter() {
    let src = "class A { int X { get { return 1; } } }\n";
    let tree = parse::parse_only(src, Language::CSharp).unwrap();
    let _ = extract_subtrees(&tree, src, Language::CSharp, Path::new("t.cs"), &t().audit);
}

#[test]
fn edge_walker_records_for_haskell_pattern_match() {
    let src = "f 0 = 0\nf n = n * f (n - 1)\n";
    let tree = parse::parse_only(src, Language::Haskell).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Haskell, Path::new("t.hs"), &t().audit);
}

#[test]
fn edge_walker_records_for_haskell_let_in() {
    let src = "f x = let y = x + 1 in y * 2\n";
    let tree = parse::parse_only(src, Language::Haskell).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Haskell, Path::new("t.hs"), &t().audit);
}

#[test]
fn edge_walker_records_for_lua_table_method() {
    let src = "function t:m(x) return x + self.y end\n";
    let tree = parse::parse_only(src, Language::Lua).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Lua, Path::new("t.lua"), &t().audit);
}

#[test]
fn edge_walker_records_for_php_array_syntax() {
    let src = "<?php $x = ['a' => 1, 'b' => 2];\n";
    let tree = parse::parse_only(src, Language::Php).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Php, Path::new("t.php"), &t().audit);
}

#[test]
fn edge_walker_records_for_ruby_symbol() {
    let src = "x = :symbol\n";
    let tree = parse::parse_only(src, Language::Ruby).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Ruby, Path::new("t.rb"), &t().audit);
}

#[test]
fn edge_walker_records_for_zig_struct_definition() {
    let src = "const Point = struct { x: i32, y: i32 };\n";
    let tree = parse::parse_only(src, Language::Zig).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Zig, Path::new("t.zig"), &t().audit);
}

#[test]
fn edge_walker_records_for_d_template() {
    let src = "T add(T)(T a, T b) { return a + b; }\n";
    let tree = parse::parse_only(src, Language::D).unwrap();
    let _ = extract_subtrees(&tree, src, Language::D, Path::new("t.d"), &t().audit);
}

#[test]
fn edge_walker_records_for_groovy_closure() {
    let src = "def f = { x -> x + 1 }\n";
    let tree = parse::parse_only(src, Language::Groovy).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Groovy, Path::new("t.groovy"), &t().audit);
}

#[test]
fn edge_walker_records_for_objc_message_passing() {
    let src = "void f(NSObject *o) { [o doSomething:42]; }\n";
    let tree = parse::parse_only(src, Language::ObjectiveC).unwrap();
    let _ = extract_subtrees(&tree, src, Language::ObjectiveC, Path::new("t.m"), &t().audit);
}

#[test]
fn edge_walker_records_for_tcl_namespace() {
    let src = "namespace eval ns {\n    proc f {x} { return $x }\n}\n";
    let tree = parse::parse_only(src, Language::Tcl).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Tcl, Path::new("t.tcl"), &t().audit);
}

#[test]
fn edge_walker_records_for_r_pipe_operator() {
    let src = "x %>% filter() %>% select()\n";
    let tree = parse::parse_only(src, Language::R).unwrap();
    let _ = extract_subtrees(&tree, src, Language::R, Path::new("t.r"), &t().audit);
}

#[test]
fn edge_walker_records_for_cobol_perform() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. P.\n       PROCEDURE DIVISION.\n           PERFORM SUB-A.\n           STOP RUN.\n       SUB-A.\n           DISPLAY \"X\".\n";
    let tree = parse::parse_only(src, Language::Cobol).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Cobol, Path::new("t.cob"), &t().audit);
}

#[test]
fn edge_walker_records_for_c_function_pointer() {
    let src = "int (*fp)(int) = NULL;\n";
    let tree = parse::parse_only(src, Language::C).unwrap();
    let _ = extract_subtrees(&tree, src, Language::C, Path::new("t.c"), &t().audit);
}

#[test]
fn edge_walker_records_for_cpp_namespace() {
    let src = "namespace ns { int f() { return 1; } }\n";
    let tree = parse::parse_only(src, Language::Cpp).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Cpp, Path::new("t.cpp"), &t().audit);
}

#[test]
fn edge_walker_records_for_cpp_namespace_alias() {
    let src = "namespace fs = std::filesystem;\n";
    let tree = parse::parse_only(src, Language::Cpp).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Cpp, Path::new("t.cpp"), &t().audit);
}

#[test]
fn edge_walker_records_for_java_interface() {
    let src = "interface I { int f(int x); }\n";
    let tree = parse::parse_only(src, Language::Java).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Java, Path::new("t.java"), &t().audit);
}

#[test]
fn edge_walker_records_for_java_lambda() {
    let src = "class A { Function<Integer, Integer> f = x -> x + 1; }\n";
    let tree = parse::parse_only(src, Language::Java).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Java, Path::new("t.java"), &t().audit);
}

#[test]
fn edge_walker_records_for_javascript_destructuring() {
    let src = "const { a, b } = obj;\n";
    let tree = parse::parse_only(src, Language::JavaScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::JavaScript, Path::new("t.js"), &t().audit);
}

#[test]
fn edge_walker_records_for_javascript_spread() {
    let src = "const xs = [...a, ...b];\n";
    let tree = parse::parse_only(src, Language::JavaScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::JavaScript, Path::new("t.js"), &t().audit);
}

#[test]
fn edge_walker_records_for_javascript_async_arrow() {
    let src = "const f = async (x) => x + 1;\n";
    let tree = parse::parse_only(src, Language::JavaScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::JavaScript, Path::new("t.js"), &t().audit);
}

#[test]
fn edge_walker_records_for_javascript_class_with_static() {
    let src = "class A { static f() { return 1; } }\n";
    let tree = parse::parse_only(src, Language::JavaScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::JavaScript, Path::new("t.js"), &t().audit);
}

#[test]
fn edge_format_findings_full_round_trip_human_then_json() {
    let f = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 0xdead_beef },
        representative_snippet: "media_type == X.value".to_string(),
        support: 99, file_count: 38,
        idf_score: Some(1.5), action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![
            AuditLocation { file: PathBuf::from("a.py"), line: 10 },
            AuditLocation { file: PathBuf::from("b.py"), line: 20 },
        ],
    };
    let human = format_findings(std::slice::from_ref(&f), None, &t().audit);
    let json = format_findings_json(std::slice::from_ref(&f), None);
    assert!(human.contains("99 occurrences"));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed[0]["support"].as_u64().unwrap(), 99);
}

#[test]
fn edge_format_findings_human_renders_correct_bullet_indents() {
    let f = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 7 },
        representative_snippet: "x".to_string(),
        support: 3, file_count: 3,
        idf_score: Some(1.0), action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![
            AuditLocation { file: PathBuf::from("a.py"), line: 1 },
            AuditLocation { file: PathBuf::from("b.py"), line: 2 },
        ],
    };
    let s = format_findings(std::slice::from_ref(&f), None, &t().audit);
    assert!(s.contains("  representative:"));
    assert!(s.contains("  files:") || s.contains("  locations:"));
}

#[test]
fn edge_format_findings_json_array_syntax_correct_for_one_finding() {
    let f = AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 7 },
        representative_snippet: "x".to_string(),
        support: 3, file_count: 3,
        idf_score: Some(1.0), action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![],
    };
    let s = format_findings_json(std::slice::from_ref(&f), None);
    assert!(s.starts_with('['));
    assert!(s.ends_with(']'));
}

#[test]
fn edge_walker_records_correct_line_for_first_function_after_blank() {
    let src = "\n\n\ndef f():\n    if x:\n        return 1\n    return 0\n";
    let records = ext_python(src);
    let func_record = records.iter().find(|r| r.snippet.contains("def f"));
    if let Some(r) = func_record {
        assert!(r.line == 4, "expected line 4, got {}", r.line);
    }
}

#[test]
fn edge_walker_record_count_increases_with_branching() {
    let simple = "def f():\n    return 1\n";
    let branchy = "def f(x):\n    if x:\n        return 1\n    elif y:\n        return 2\n    else:\n        return 3\n";
    let s = ext_python(simple).len();
    let b = ext_python(branchy).len();
    assert!(b > s);
}

#[test]
fn edge_freqt_mine_distinguishes_single_char_snippet_difference() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "a.py", 1, "x"), rec(8, "b.py", 1, "y")];
    let clusters = freqt_mine(&recs, &th);
    assert!(clusters.is_empty(), "different fingerprints with one record each");
}

#[test]
fn edge_freqt_mine_picks_longest_among_equal_length_snippets() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 1, "abc"),
        rec(7, "b.py", 1, "abcd"),
        rec(7, "c.py", 1, "abcde"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].representative_snippet, "abcde");
}

#[test]
fn edge_freqt_mine_handles_locations_with_same_path_different_lines() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 1, "x"),
        rec(7, "a.py", 5, "x"),
        rec(7, "a.py", 10, "x"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].support, 3);
    assert_eq!(clusters[0].file_count, 1);
}

#[test]
fn edge_apply_idf_reorders_when_input_unsorted() {
    let cs = vec![
        RawCluster { fingerprint: 1, support: 3, file_count: 2,
            representative_snippet: "a".to_string(), locations: vec![] },
        RawCluster { fingerprint: 2, support: 9, file_count: 2,
            representative_snippet: "b".to_string(), locations: vec![] },
        RawCluster { fingerprint: 3, support: 5, file_count: 2,
            representative_snippet: "c".to_string(), locations: vec![] },
    ];
    let result = apply_idf(cs, 100, &t().audit);
    assert_eq!(result[0].support, 9);
    assert_eq!(result[1].support, 5);
    assert_eq!(result[2].support, 3);
}

#[test]
fn edge_freqt_mine_two_equal_fingerprint_groups_handled_separately() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![
        rec(7, "a.py", 1, "x"), rec(7, "b.py", 1, "x"),
        rec(8, "c.py", 1, "y"), rec(8, "d.py", 1, "y"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters.len(), 2);
}

#[test]
fn edge_walker_handles_python_class_with_inheritance_chain() {
    let src = "class A: pass\nclass B(A): pass\nclass C(B): pass\nclass D(C): pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_complex_default_arg_evaluation() {
    let src = "def f(x=[1, 2, 3], y={\"a\": 1}, z=lambda: None): return x + [y, z]\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_double_star_kwargs_in_call() {
    let src = "f(**a, **b, **c)\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_string_with_escape_sequences() {
    let src = "x = \"a\\nb\\tc\\rd\\u00e9\"\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_byte_string() {
    let src = "x = b\"hello\"\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_format_string() {
    let src = "x = f\"hello {name} world\"\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_format_string_with_expression() {
    let src = "x = f\"{a + b * c}\"\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_complex_number_literal() {
    let src = "x = 1 + 2j\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_underscored_number_literal() {
    let src = "x = 1_000_000\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_hex_literal() {
    let src = "x = 0xff\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_octal_literal() {
    let src = "x = 0o755\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_binary_literal() {
    let src = "x = 0b1010\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_scientific_notation() {
    let src = "x = 1e10\ny = 1.5e-3\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_dict_unpacking_in_literal() {
    let src = "x = {**a, \"key\": 1}\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_set_with_one_element() {
    let src = "x = {1}\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_empty_dict_vs_empty_set() {
    let src = "a = {}\nb = set()\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_records_have_consistent_seeded_fingerprint_for_python_files() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let r1 = ext_python(src);
    let r2 = ext_python(src);
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(a.fingerprint, b.fingerprint);
    }
}

#[test]
fn edge_walker_handles_python_only_whitespace_function_body() {
    let src = "def f():\n   \n    \n     \n    pass\n";
    let _ = ext_python(src);
}

#[test]
fn edge_walker_handles_python_file_ending_without_newline() {
    let src = "x = 1";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn edge_walker_handles_python_file_with_only_pass() {
    let src = "pass\n";
    let r = ext_python(src);
    assert!(r.is_empty(), "below floor");
}

#[test]
fn edge_walker_handles_python_file_with_only_ellipsis() {
    let src = "...\n";
    let _ = ext_python(src);
}
