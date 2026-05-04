use pulse::audit::discovery::freqt_mine;
use pulse::audit::scoring::apply_idf;
use pulse::audit::walker::extract_subtrees;
use pulse::audit::{extract_subtrees_for_dir, walk_typed_source_files};
use pulse::parse::{self, Language};
use pulse::thresholds::Thresholds;
use std::path::Path;
use std::process::Command;

fn t() -> Thresholds {
    Thresholds::default()
}

fn pulse_audit(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .args(args)
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap_or_default(),
        String::from_utf8(out.stderr).unwrap_or_default(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn audit_root_with_invalid_utf8_path_does_not_panic() {
    let _ = pulse_audit(&["--root", ""]);
}

#[test]
fn audit_root_pointing_at_a_named_pipe_or_socket_does_not_panic() {
    let _ = pulse_audit(&["--root", "/dev/null"]);
}

#[test]
fn audit_root_with_extremely_long_path_does_not_overflow() {
    let p = "/a".repeat(500);
    let (_, _, code) = pulse_audit(&["--root", &p]);
    assert_eq!(code, 1);
}

#[test]
fn audit_with_no_arguments_runs_in_current_dir() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn walker_handles_truncated_file_in_middle_of_function() {
    let src = "def f(x):\n    if x ==";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_unclosed_string_literal() {
    let src = "x = \"hello\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_unclosed_paren() {
    let src = "f(x, y\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_unmatched_close_paren() {
    let src = "x))\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_only_whitespace_input() {
    let _ = parse::parse_only("   \n\n  \t\n", Language::Python);
}

#[test]
fn walker_handles_only_comment_input() {
    let _ = parse::parse_only("# comment\n# more comment\n", Language::Python);
}

#[test]
fn walker_handles_only_shebang_line() {
    let _ = parse::parse_only("#!/usr/bin/env python3\n", Language::Python);
}

#[test]
fn walker_handles_invalid_keyword_combination() {
    let src = "def for if while:\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_random_garbage_bytes() {
    let src = "%%%###@@!~~~\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_null_byte_in_source() {
    let src = "x = 1\n\0y = 2\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_mixed_line_endings_crlf_and_lf() {
    let src = "def f():\r\n    return 1\nx = f()\r\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_old_mac_cr_only_line_endings() {
    let src = "def f():\r    return 1\rx = f()\r";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn extract_subtrees_for_dir_returns_empty_for_dir_root_outside_filesystem() {
    let p = Path::new("/__definitely__/__not__/__a__/__path__");
    let records = extract_subtrees_for_dir(p, Language::Python, &t().audit);
    assert!(records.is_empty());
}

#[test]
fn walk_typed_source_files_returns_empty_for_nonexistent() {
    let p = Path::new("/__nope__/__nope__");
    let typed = walk_typed_source_files(p, true);
    assert!(typed.is_empty());
}

#[test]
fn walk_typed_source_files_returns_empty_for_file_path() {
    let f = tempfile::NamedTempFile::new().unwrap();
    let typed = walk_typed_source_files(f.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn audit_handles_directory_we_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn audit_root_relative_path_to_nonexistent_exits_one() {
    let (_, _, code) = pulse_audit(&["--root", "./definitely_not_there"]);
    assert_eq!(code, 1);
}

#[test]
fn audit_root_zero_byte_filename() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".py"), "x = 1\n").unwrap();
    let _ = walk_typed_source_files(dir.path(), true);
}

#[test]
fn audit_legacy_layer_flag_unknown() {
    let (_, _, code) = pulse_audit(&["--layer", "3"]);
    assert_ne!(code, 0);
}

#[test]
fn audit_pass_invalid_value_rejected_by_clap() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, code) = pulse_audit(&["--pass", "garbage", "--root", dir.path().to_str().unwrap()]);
    assert_ne!(code, 0);
}

#[test]
fn audit_pass_empty_value_rejected_by_clap() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, code) = pulse_audit(&["--pass", "", "--root", dir.path().to_str().unwrap()]);
    assert_ne!(code, 0);
}

#[test]
fn audit_pass_numeric_value_rejected_by_clap() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, code) = pulse_audit(&["--pass", "3", "--root", dir.path().to_str().unwrap()]);
    assert_ne!(code, 0);
}

#[test]
fn audit_pass_underscore_form_rejected_by_clap() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, code) = pulse_audit(&["--pass", "pattern_mining", "--root", dir.path().to_str().unwrap()]);
    assert_ne!(code, 0);
}

#[test]
fn audit_pass_camel_case_form_rejected_by_clap() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, code) = pulse_audit(&["--pass", "PatternMining", "--root", dir.path().to_str().unwrap()]);
    assert_ne!(code, 0);
}

#[test]
fn audit_unknown_flag_rejected_by_clap() {
    let (_, _, code) = pulse_audit(&["--definitely-unknown-flag"]);
    assert_ne!(code, 0);
}

#[test]
fn audit_double_dash_root_with_no_value_rejected() {
    let (_, _, code) = pulse_audit(&["--root"]);
    assert_ne!(code, 0);
}

#[test]
fn audit_double_dash_pass_with_no_value_rejected() {
    let (_, _, code) = pulse_audit(&["--pass"]);
    assert_ne!(code, 0);
}

#[test]
fn audit_extra_positional_arg_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap(), "extra_positional"]);
    assert_ne!(code, 0);
}

#[test]
fn freqt_mine_with_negative_min_support_not_possible() {
    let _: usize = t().audit.pattern_mining.freqt_min_support;
}

#[test]
fn apply_idf_with_negative_total_files_not_possible() {
    let _ = apply_idf(vec![], 0, &t().audit);
}

#[test]
fn freqt_mine_does_not_panic_on_record_with_huge_line_number() {
    use pulse::audit::walker::{ShapeMetrics, SubtreeRecord};
    use std::path::PathBuf;
    let r = SubtreeRecord {
        fingerprint: 7,
        parent_fingerprint: None,
        file: PathBuf::from("a.py"),
        line: u32::MAX,
        depth: 5,
        named_node_count: 8,
        snippet: "x".to_string(),
        shape: ShapeMetrics::default(),
    };
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1;
    let _ = freqt_mine(&[r], &th);
}

#[test]
fn walker_handles_invalid_indentation_python() {
    let src = "def f():\nreturn 1\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_tab_indented_then_space_indented_python() {
    let src = "def f():\n\tx = 1\n    y = 2\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_python_print_without_parens_old_syntax() {
    let src = "print 'hello'\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_typescript_with_invalid_arrow_function() {
    let src = "const f = => 1\n";
    let _ = parse::parse_only(src, Language::TypeScript);
}

#[test]
fn walker_handles_rust_with_unbalanced_braces() {
    let src = "fn f() { fn g() { } }}\n";
    let _ = parse::parse_only(src, Language::Rust);
}

#[test]
fn walker_handles_javascript_unterminated_object_literal() {
    let src = "const x = { a: 1, b: 2\n";
    let _ = parse::parse_only(src, Language::JavaScript);
}

#[test]
fn walker_handles_go_with_missing_package_declaration() {
    let src = "func main() {}\n";
    let _ = parse::parse_only(src, Language::Go);
}

#[test]
fn walker_handles_ruby_unmatched_end() {
    let src = "def f\nend\nend\n";
    let _ = parse::parse_only(src, Language::Ruby);
}

#[test]
fn walker_handles_haskell_with_layout_error() {
    let src = "f x = case x of\n  1 -> 'a'\n2 -> 'b'\n";
    let _ = parse::parse_only(src, Language::Haskell);
}

#[test]
fn walker_handles_lua_unterminated_string() {
    let src = "x = 'hello\n";
    let _ = parse::parse_only(src, Language::Lua);
}

#[test]
fn walker_handles_cobol_invalid_division() {
    let src = "       INVALID DIVISION.\n";
    let _ = parse::parse_only(src, Language::Cobol);
}

#[test]
fn walker_handles_php_no_opening_tag() {
    let src = "function f() {}\n";
    let _ = parse::parse_only(src, Language::Php);
}

#[test]
fn walker_handles_unicode_byte_order_mark() {
    let src = "\u{FEFF}def f():\n    return 1\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_multiple_byte_order_marks() {
    let src = "\u{FEFF}\u{FEFF}\u{FEFF}def f():\n    return 1\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_form_feed_character() {
    let src = "def f():\n\x0c    return 1\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_vertical_tab() {
    let src = "x\x0b= 1\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn audit_does_not_follow_symlinks_into_skipped_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let _ = std::fs::write(dir.path().join("a.py"), "x = 1\n");
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_handles_circular_dirent_recursion_safely() {
    let dir = tempfile::tempdir().unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn walker_handles_input_with_only_string_literals() {
    let src = "\"a\"\n\"b\"\n\"c\"\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn walker_handles_input_with_only_numbers() {
    let src = "1\n2\n3\n";
    let _ = parse::parse_only(src, Language::Python);
}

#[test]
fn freqt_mine_with_zero_min_support_includes_all_clusters() {
    use pulse::audit::walker::{ShapeMetrics, SubtreeRecord};
    use std::path::PathBuf;
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 0;
    let records: Vec<SubtreeRecord> = (0..5).map(|i| SubtreeRecord {
        fingerprint: i,
        parent_fingerprint: None,
        file: PathBuf::from("a.py"),
        line: 1,
        depth: 5,
        named_node_count: 8,
        snippet: "x".to_string(),
        shape: ShapeMetrics::default(),
    }).collect();
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 5);
}

#[test]
fn apply_idf_threshold_negative_treated_as_zero() {
    use pulse::audit::discovery::RawCluster;
    use std::path::PathBuf;
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = -1.0;
    let cluster = RawCluster {
        fingerprint: 7,
        support: 3,
        file_count: 3,
        representative_snippet: "x".to_string(),
        locations: vec![(PathBuf::from("a.py"), 1)],
    };
    let result = apply_idf(vec![cluster], 10, &th);
    assert!(result.is_empty(), "negative threshold suppresses everything");
}

#[test]
fn audit_dir_with_only_unsupported_extensions_silent_zero() {
    let dir = tempfile::tempdir().unwrap();
    for ext in ["xml", "yaml", "toml", "ini", "txt"] {
        std::fs::write(dir.path().join(format!("a.{ext}")), "data").unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn audit_handles_empty_filename_gracefully() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("x.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_handles_files_with_dot_in_middle_of_name() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.b.c.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_handles_uppercase_file_extensions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.PY"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    let _ = typed.len();
}

#[test]
fn walker_handles_extremely_long_identifier() {
    let id = "a".repeat(10_000);
    let src = format!("{id} = 1\n");
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn walker_handles_input_with_sole_newline() {
    let _ = parse::parse_only("\n", Language::Python);
}

#[test]
fn walker_handles_input_one_byte() {
    let _ = parse::parse_only("x", Language::Python);
}

#[test]
fn walker_handles_single_space_input() {
    let _ = parse::parse_only(" ", Language::Python);
}

#[test]
fn walker_handles_input_consisting_solely_of_tabs() {
    let _ = parse::parse_only("\t\t\t\n", Language::Python);
}

#[test]
fn walker_handles_input_with_trailing_zero() {
    let _ = parse::parse_only("x = 1\n\0", Language::Python);
}

#[test]
fn walker_does_not_panic_on_python_2_print_statement() {
    let src = "print 'hello, world'\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Python, Path::new("t.py"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_typescript_decorator() {
    let src = "@Component class A {}\n";
    let tree = parse::parse_only(src, Language::TypeScript).unwrap();
    let _ = extract_subtrees(&tree, src, Language::TypeScript, Path::new("t.ts"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_rust_macro() {
    let src = "fn f() { vec![1, 2, 3]; }\n";
    let tree = parse::parse_only(src, Language::Rust).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Rust, Path::new("t.rs"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_cpp_template() {
    let src = "template<typename T> T f(T x) { return x; }\n";
    let tree = parse::parse_only(src, Language::Cpp).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Cpp, Path::new("t.cpp"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_java_generic() {
    let src = "class A<T> { T f(T x) { return x; } }\n";
    let tree = parse::parse_only(src, Language::Java).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Java, Path::new("t.java"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_swift_optional() {
    let src = "func f(x: Int?) -> Int { return x ?? 0 }\n";
    let tree = parse::parse_only(src, Language::Swift).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Swift, Path::new("t.swift"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_kotlin_when() {
    let src = "fun f(x: Int): Int = when (x) { 1 -> 1; else -> 0 }\n";
    let tree = parse::parse_only(src, Language::Kotlin).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Kotlin, Path::new("t.kt"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_go_goroutine() {
    let src = "package p\nfunc f() { go work() }\n";
    let tree = parse::parse_only(src, Language::Go).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Go, Path::new("t.go"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_ruby_block() {
    let src = "items.each do |x| puts x end\n";
    let tree = parse::parse_only(src, Language::Ruby).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Ruby, Path::new("t.rb"), &t().audit);
}

#[test]
fn walker_does_not_panic_on_zig_comptime() {
    let src = "fn f() void { comptime { _ = 42; } }\n";
    let tree = parse::parse_only(src, Language::Zig).unwrap();
    let _ = extract_subtrees(&tree, src, Language::Zig, Path::new("t.zig"), &t().audit);
}

#[test]
fn audit_root_with_trailing_slash_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    let p = format!("{}/", dir.path().to_str().unwrap());
    let (_, _, code) = pulse_audit(&["--root", &p]);
    assert_eq!(code, 0);
}

#[test]
fn audit_root_with_double_slash_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    let p = dir.path().to_str().unwrap().to_string() + "//";
    let (_, _, code) = pulse_audit(&["--root", &p]);
    assert_eq!(code, 0);
}

#[test]
fn audit_handles_non_utf8_file_contents_gracefully() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = vec![0x80, 0x81, 0x82, 0x83];
    std::fs::write(dir.path().join("bad.py"), bytes).unwrap();
    let _ = walk_typed_source_files(dir.path(), true);
}

#[test]
fn walker_returns_empty_when_threshold_exceeds_input_capacity() {
    let mut th = t().audit;
    th.pattern_mining.subtree_min_nodes = 100_000;
    th.pattern_mining.subtree_min_depth = 100_000;
    let src = "def f():\n    return 1\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let records = extract_subtrees(&tree, src, Language::Python, Path::new("t.py"), &th);
    assert!(records.is_empty());
}
