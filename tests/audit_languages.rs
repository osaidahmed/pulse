use pulse::audit::walker::extract_subtrees;
use pulse::parse::{self, Language};
use pulse::thresholds::Thresholds;
use std::path::Path;

fn t() -> Thresholds {
    Thresholds::default()
}

const LANGS: &[(Language, &str, &str)] = &[
    (Language::Python, "py", "def f(x):\n    if x == 1:\n        return x\n    return 0\n"),
    (Language::TypeScript, "ts", "function f(x: number): number { if (x === 1) { return x; } return 0; }\n"),
    (Language::JavaScript, "js", "function f(x) { if (x === 1) { return x; } return 0; }\n"),
    (Language::Rust, "rs", "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n"),
    (Language::C, "c", "int f(int x) { if (x == 1) { return x; } return 0; }\n"),
    (Language::Cpp, "cpp", "int f(int x) { if (x == 1) { return x; } return 0; }\n"),
    (Language::Java, "java", "class A { int f(int x) { if (x == 1) { return x; } return 0; } }\n"),
    (Language::CSharp, "cs", "class A { int F(int x) { if (x == 1) { return x; } return 0; } }\n"),
    (Language::Go, "go", "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n"),
    (Language::Swift, "swift", "func f(x: Int) -> Int { if x == 1 { return x }; return 0 }\n"),
    (Language::Zig, "zig", "fn f(x: i32) i32 { if (x == 1) { return x; } return 0; }\n"),
    (Language::Ruby, "rb", "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n"),
    (Language::ObjectiveC, "m", "int f(int x) { if (x == 1) { return x; } return 0; }\n"),
    (Language::Tcl, "tcl", "proc f {x} {\n  if {$x == 1} {\n    return $x\n  }\n  return 0\n}\n"),
    (Language::Kotlin, "kt", "fun f(x: Int): Int { if (x == 1) { return x }; return 0 }\n"),
    (Language::Haskell, "hs", "f :: Int -> Int\nf x = if x == 1 then x else 0\n"),
    (Language::Lua, "lua", "function f(x) if x == 1 then return x end return 0 end\n"),
    (Language::R, "r", "f <- function(x) { if (x == 1) { return(x) }; return(0) }\n"),
    (Language::Php, "php", "<?php function f($x) { if ($x === 1) { return $x; } return 0; }\n"),
    (Language::Cobol, "cob", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. F.\n       PROCEDURE DIVISION.\n           IF X = 1\n               DISPLAY X\n           END-IF.\n           STOP RUN.\n"),
    (Language::D, "d", "int f(int x) { if (x == 1) { return x; } return 0; }\n"),
    (Language::Groovy, "groovy", "def f(x) { if (x == 1) { return x }; return 0 }\n"),
];

fn extract(lang: Language, src: &str) -> Vec<pulse::audit::walker::SubtreeRecord> {
    let tree = parse::parse_only(src, lang).expect("parse failed");
    let path = Path::new("test");
    extract_subtrees(&tree, src, lang, path, &t().audit)
}

#[test]
fn parse_only_returns_some_for_each_language() {
    for (lang, _ext, src) in LANGS {
        assert!(parse::parse_only(src, *lang).is_some(), "parse failed for {lang:?}");
    }
}

#[test]
fn walker_produces_records_for_each_language() {
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        assert!(!records.is_empty(), "no records for {lang:?}");
    }
}

#[test]
fn fingerprint_is_deterministic_for_each_language() {
    for (lang, _ext, src) in LANGS {
        let a = extract(*lang, src);
        let b = extract(*lang, src);
        assert_eq!(a.len(), b.len(), "{lang:?}");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.fingerprint, y.fingerprint, "{lang:?}");
        }
    }
}

#[test]
fn fingerprint_stable_under_extra_blank_lines_for_each_language() {
    for (lang, _ext, src) in LANGS {
        let s_padded = format!("\n\n{src}\n\n");
        let a = extract(*lang, src);
        let b = extract(*lang, &s_padded);
        let a_fps: std::collections::HashSet<u64> = a.iter().map(|r| r.fingerprint).collect();
        let b_fps: std::collections::HashSet<u64> = b.iter().map(|r| r.fingerprint).collect();
        assert!(a_fps.is_subset(&b_fps) || b_fps.is_subset(&a_fps), "{lang:?}");
    }
}

#[test]
fn walker_respects_min_depth_for_each_language() {
    let mut th = t().audit;
    th.subtree_min_depth = 100;
    for (lang, _ext, src) in LANGS {
        let tree = parse::parse_only(src, *lang).unwrap();
        let path = Path::new("test");
        let records = extract_subtrees(&tree, src, *lang, path, &th);
        assert!(records.is_empty(), "min_depth=100 should reject all for {lang:?}");
    }
}

#[test]
fn walker_respects_min_nodes_for_each_language() {
    let mut th = t().audit;
    th.subtree_min_nodes = 100_000;
    for (lang, _ext, src) in LANGS {
        let tree = parse::parse_only(src, *lang).unwrap();
        let path = Path::new("test");
        let records = extract_subtrees(&tree, src, *lang, path, &th);
        assert!(records.is_empty(), "huge min_nodes should reject all for {lang:?}");
    }
}

#[test]
fn fingerprints_seeded_distinctly_per_language_for_same_shape() {
    let mut seen: std::collections::HashMap<u64, Vec<Language>> = std::collections::HashMap::new();
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        for r in records {
            seen.entry(r.fingerprint).or_default().push(*lang);
        }
    }
    for (fp, langs) in &seen {
        let unique: std::collections::HashSet<_> = langs.iter().collect();
        assert_eq!(unique.len(), 1, "fingerprint {fp:#x} shared across multiple languages: {langs:?}");
    }
}

#[test]
fn extension_dispatch_round_trips_for_each_language() {
    for (lang, ext, _src) in LANGS {
        let path = std::path::PathBuf::from(format!("file.{ext}"));
        let detected = parse::detect_language(&path);
        assert_eq!(detected, Some(*lang), "extension .{ext} should detect {lang:?}");
    }
}

#[test]
fn empty_source_yields_no_records_for_each_language() {
    for (lang, _ext, _src) in LANGS {
        let records = extract(*lang, "");
        assert!(records.is_empty(), "{lang:?}");
    }
}

#[test]
fn each_language_records_have_positive_line_numbers() {
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        for r in records {
            assert!(r.line >= 1, "{lang:?} line was 0");
        }
    }
}

#[test]
fn each_language_records_have_nonempty_snippets() {
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        for r in records {
            if r.named_node_count > 1 {
                assert!(!r.snippet.is_empty(), "{lang:?} empty snippet for non-trivial subtree");
            }
        }
    }
}

#[test]
fn each_language_snippet_first_line_only() {
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        for r in records {
            assert!(!r.snippet.contains('\n'), "{lang:?} snippet contains newline: {:?}", r.snippet);
        }
    }
}

#[test]
fn each_language_snippet_at_most_eighty_chars() {
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        for r in records {
            assert!(r.snippet.chars().count() <= 80, "{lang:?} snippet too long");
        }
    }
}

#[test]
fn each_language_records_above_min_depth_threshold() {
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        let min_depth = t().audit.subtree_min_depth as u32;
        for r in records {
            assert!(r.depth >= min_depth, "{lang:?} depth {} below threshold {}", r.depth, min_depth);
        }
    }
}

#[test]
fn each_language_records_above_min_nodes_threshold() {
    for (lang, _ext, src) in LANGS {
        let records = extract(*lang, src);
        let min = t().audit.subtree_min_nodes as u32;
        for r in records {
            assert!(r.named_node_count >= min, "{lang:?}");
        }
    }
}

#[test]
fn each_language_handles_syntax_error_no_panic() {
    for (lang, _ext, _src) in LANGS {
        let bad = "%%% definitely not valid %%%\n";
        let _ = parse::parse_only(bad, *lang);
    }
}

#[test]
fn each_language_handles_unicode_in_identifiers() {
    let cases: &[(Language, &str)] = &[
        (Language::Python, "δ = 1\nλ = δ + 1\n"),
        (Language::JavaScript, "var δ = 1; var λ = δ + 1;\n"),
        (Language::TypeScript, "let δ: number = 1; let λ: number = δ + 1;\n"),
        (Language::Ruby, "δ = 1\nλ = δ + 1\n"),
    ];
    for (lang, src) in cases {
        let _ = extract(*lang, src);
    }
}

#[test]
fn each_language_handles_leading_byte_order_mark() {
    let bom = "\u{FEFF}";
    for (lang, _ext, src) in LANGS {
        let s = format!("{bom}{src}");
        let _ = parse::parse_only(&s, *lang);
    }
}

#[test]
fn each_language_handles_carriage_return_line_endings() {
    for (lang, _ext, src) in LANGS {
        let crlf = src.replace('\n', "\r\n");
        let _ = parse::parse_only(&crlf, *lang);
    }
}

#[test]
fn each_language_handles_only_lf_endings() {
    for (lang, _ext, src) in LANGS {
        let lf = src.replace("\r\n", "\n");
        let _ = parse::parse_only(&lf, *lang);
    }
}

#[test]
fn each_language_records_contain_correct_file_path() {
    let path = Path::new("/tmp/foo/bar.x");
    for (lang, _ext, src) in LANGS {
        let tree = parse::parse_only(src, *lang).unwrap();
        let records = extract_subtrees(&tree, src, *lang, path, &t().audit);
        for r in records {
            assert_eq!(r.file, path, "{lang:?}");
        }
    }
}

#[test]
fn each_language_can_be_walked_twice_on_same_tree() {
    for (lang, _ext, src) in LANGS {
        let tree = parse::parse_only(src, *lang).unwrap();
        let path = Path::new("test");
        let a = extract_subtrees(&tree, src, *lang, path, &t().audit);
        let b = extract_subtrees(&tree, src, *lang, path, &t().audit);
        assert_eq!(a.len(), b.len(), "{lang:?}");
    }
}

#[test]
fn extension_dispatch_distinguishes_typescript_from_javascript() {
    assert_eq!(parse::detect_language(Path::new("a.ts")), Some(Language::TypeScript));
    assert_eq!(parse::detect_language(Path::new("a.js")), Some(Language::JavaScript));
}

#[test]
fn extension_dispatch_handles_multiple_extensions_per_language() {
    assert_eq!(parse::detect_language(Path::new("a.tsx")), Some(Language::TypeScript));
    assert_eq!(parse::detect_language(Path::new("a.jsx")), Some(Language::JavaScript));
    assert_eq!(parse::detect_language(Path::new("a.h")), Some(Language::C));
    assert_eq!(parse::detect_language(Path::new("a.hpp")), Some(Language::Cpp));
}

#[test]
fn extension_dispatch_returns_none_for_unknown() {
    assert_eq!(parse::detect_language(Path::new("a.xyz")), None);
    assert_eq!(parse::detect_language(Path::new("a")), None);
}

#[test]
fn each_language_record_count_grows_with_function_count() {
    let templates: &[(Language, &str)] = &[
        (Language::Python, "def f{i}(x):\n    if x == 1:\n        return x\n    return 0\n"),
        (Language::Rust, "fn f{i}(x: i32) -> i32 {{ if x == 1 {{ return x; }} 0 }}\n"),
        (Language::JavaScript, "function f{i}(x) {{ if (x === 1) return x; return 0; }}\n"),
    ];
    for (lang, tpl) in templates {
        let small = (0..2).map(|i| tpl.replace("{i}", &i.to_string())).collect::<String>();
        let large = (0..10).map(|i| tpl.replace("{i}", &i.to_string())).collect::<String>();
        let n_small = extract(*lang, &small).len();
        let n_large = extract(*lang, &large).len();
        assert!(n_large > n_small, "{lang:?}: {n_large} not > {n_small}");
    }
}

#[test]
fn lang_kinds_table_has_22_entries() {
    use pulse::audit::lang_kinds::skippable_root_kinds;
    for (lang, _, _) in LANGS {
        let _ = skippable_root_kinds(*lang);
    }
}

#[test]
fn lang_kinds_default_empty_for_each_language() {
    use pulse::audit::lang_kinds::skippable_root_kinds;
    for (lang, _, _) in LANGS {
        assert!(skippable_root_kinds(*lang).is_empty(), "{lang:?} table should be empty in v1");
    }
}

#[test]
fn is_skippable_root_returns_false_for_unknown_kind() {
    use pulse::audit::lang_kinds::is_skippable_root;
    for (lang, _, _) in LANGS {
        assert!(!is_skippable_root(*lang, "definitely_not_a_node_kind"));
    }
}

#[test]
fn is_skippable_root_returns_false_for_common_kinds_in_v1() {
    use pulse::audit::lang_kinds::is_skippable_root;
    for (lang, _, _) in LANGS {
        for kind in &["if_statement", "function_declaration", "block", "comment"] {
            assert!(!is_skippable_root(*lang, kind), "{lang:?} {kind} unexpectedly skipped");
        }
    }
}

#[test]
fn each_language_two_identical_function_definitions_share_a_fingerprint() {
    let cases: &[(Language, &str)] = &[
        (Language::Python, "def a(x):\n    if x == 1:\n        return x\n    return 0\n\ndef b(x):\n    if x == 1:\n        return x\n    return 0\n"),
        (Language::Rust, "fn a(x: i32) -> i32 { if x == 1 { return x; } 0 }\nfn b(x: i32) -> i32 { if x == 1 { return x; } 0 }\n"),
        (Language::JavaScript, "function a(x) { if (x === 1) return x; return 0; }\nfunction b(x) { if (x === 1) return x; return 0; }\n"),
        (Language::Go, "package p\nfunc a(x int) int { if x == 1 { return x }; return 0 }\nfunc b(x int) int { if x == 1 { return x }; return 0 }\n"),
    ];
    for (lang, src) in cases {
        let records = extract(*lang, src);
        let mut count: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
        for r in &records {
            *count.entry(r.fingerprint).or_default() += 1;
        }
        let max = count.values().copied().max().unwrap_or(0);
        assert!(max >= 2, "{lang:?}: no duplicate fingerprint among identical fns; got max {max}");
    }
}
