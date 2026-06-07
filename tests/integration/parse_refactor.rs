use pulse::parse::{self, Language};

const ALL_LANGS: &[(Language, &str, &str)] = &[
    (Language::Python, "py", "def f():\n    return 1\n"),
    (Language::TypeScript, "ts", "function f(): number { return 1; }\n"),
    (Language::JavaScript, "js", "function f() { return 1; }\n"),
    (Language::Rust, "rs", "fn f() -> i32 { 1 }\n"),
    (Language::C, "c", "int f(void) { return 1; }\n"),
    (Language::Cpp, "cpp", "int f() { return 1; }\n"),
    (Language::Java, "java", "class A { int f() { return 1; } }\n"),
    (Language::CSharp, "cs", "class A { int F() { return 1; } }\n"),
    (Language::Go, "go", "package p\nfunc f() int { return 1 }\n"),
    (Language::Swift, "swift", "func f() -> Int { return 1 }\n"),
    (Language::Zig, "zig", "fn f() i32 { return 1; }\n"),
    (Language::Ruby, "rb", "def f\n  1\nend\n"),
    (Language::ObjectiveC, "m", "int f(void) { return 1; }\n"),
    (Language::Tcl, "tcl", "proc f {} { return 1 }\n"),
    (Language::Kotlin, "kt", "fun f(): Int { return 1 }\n"),
    (Language::Haskell, "hs", "f :: Int\nf = 1\n"),
    (Language::Lua, "lua", "function f() return 1 end\n"),
    (Language::R, "r", "f <- function() 1\n"),
    (Language::Php, "php", "<?php function f() { return 1; }\n"),
    (Language::Cobol, "cob", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. F.\n"),
    (Language::D, "d", "int f() { return 1; }\n"),
    (Language::Groovy, "groovy", "def f() { return 1 }\n"),
];

#[test]
fn parse_only_returns_some_for_valid_python() {
    let tree = parse::parse_only("def f():\n    return 1\n", Language::Python);
    assert!(tree.is_some());
}

#[test]
fn parse_only_returns_some_for_syntactically_invalid_python() {
    let tree = parse::parse_only("def f(:\n    1\n", Language::Python);
    assert!(tree.is_some(), "tree-sitter recovers from syntax errors");
}

#[test]
fn parse_only_root_node_kind_is_module_for_python() {
    let tree = parse::parse_only("x = 1\n", Language::Python).unwrap();
    assert_eq!(tree.root_node().kind(), "module");
}

#[test]
fn parse_only_root_node_walkable() {
    let tree = parse::parse_only("def f():\n    pass\n", Language::Python).unwrap();
    let mut count = 0;
    let mut cursor = tree.root_node().walk();
    for _ in tree.root_node().children(&mut cursor) {
        count += 1;
    }
    assert!(count > 0, "root must have at least one named child");
}

#[test]
fn parse_only_returns_some_for_each_language() {
    for (lang, _ext, src) in ALL_LANGS {
        let tree = parse::parse_only(src, *lang);
        assert!(tree.is_some(), "parse_only returned None for {lang:?}");
    }
}

#[test]
fn walk_only_produces_same_metrics_as_parse_and_walk_python() {
    let src = "def f(x):\n    if x > 0:\n        return 1\n    return 0\n";
    let combined = parse::parse_and_walk(src, Language::Python).unwrap();
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let split = parse::walk_only(&tree, src, Language::Python);
    assert_eq!(combined.functions.len(), split.functions.len());
    assert_eq!(combined.module.total_loc, split.module.total_loc);
    assert_eq!(combined.module.sum_cc, split.module.sum_cc);
    assert_eq!(combined.module.total_functions, split.module.total_functions);
}

#[test]
fn walk_only_matches_parse_and_walk_for_each_language() {
    for (lang, _ext, src) in ALL_LANGS {
        let combined = parse::parse_and_walk(src, *lang);
        let tree = parse::parse_only(src, *lang);
        assert!(combined.is_some() && tree.is_some(), "parse failed for {lang:?}");
        let split = parse::walk_only(tree.as_ref().unwrap(), src, *lang);
        let combined = combined.unwrap();
        assert_eq!(combined.functions.len(), split.functions.len(), "function count mismatch for {lang:?}");
        assert_eq!(combined.module.total_loc, split.module.total_loc, "loc mismatch for {lang:?}");
        assert_eq!(combined.module.sum_cc, split.module.sum_cc, "cc mismatch for {lang:?}");
    }
}

#[test]
fn walk_only_can_be_called_twice_on_same_tree() {
    let src = "def f():\n    return 1\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let a = parse::walk_only(&tree, src, Language::Python);
    let b = parse::walk_only(&tree, src, Language::Python);
    assert_eq!(a.functions.len(), b.functions.len());
    assert_eq!(a.module.total_loc, b.module.total_loc);
    assert_eq!(a.module.sum_cc, b.module.sum_cc);
}

#[test]
fn parse_and_walk_shim_returns_same_result_post_refactor() {
    let src = "def f(x):\n    return x + 1\n";
    let direct = parse::parse_and_walk(src, Language::Python).unwrap();
    let manual = {
        let tree = parse::parse_only(src, Language::Python).unwrap();
        parse::walk_only(&tree, src, Language::Python)
    };
    assert_eq!(direct.functions.len(), manual.functions.len());
    assert_eq!(direct.module.total_loc, manual.module.total_loc);
    assert_eq!(direct.module.sum_cc, manual.module.sum_cc);
}

#[test]
fn parse_and_walk_returns_some_for_valid_python() {
    let result = parse::parse_and_walk("def f():\n    return 1\n", Language::Python);
    assert!(result.is_some());
    let m = result.unwrap();
    assert_eq!(m.functions.len(), 1);
}

#[test]
fn parse_and_walk_handles_empty_source() {
    let result = parse::parse_and_walk("", Language::Python);
    assert!(result.is_some(), "empty source still parses to a (probably empty) tree");
}

#[test]
fn parse_only_handles_empty_source() {
    let tree = parse::parse_only("", Language::Python);
    assert!(tree.is_some());
}

#[test]
fn walk_only_handles_empty_source() {
    let tree = parse::parse_only("", Language::Python).unwrap();
    let m = parse::walk_only(&tree, "", Language::Python);
    assert_eq!(m.functions.len(), 0);
    assert_eq!(m.module.total_loc, 0);
}

#[test]
fn parse_only_independent_of_walk_only() {
    let src = "def a():\n    pass\ndef b():\n    pass\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    drop(parse::walk_only(&tree, src, Language::Python));
    let again = parse::walk_only(&tree, src, Language::Python);
    assert_eq!(again.functions.len(), 2);
}

#[test]
fn analyze_via_split_matches_combined_for_realistic_python() {
    let src = include_str!("../fixtures/python/production_service.py");
    let combined = parse::parse_and_walk(src, Language::Python).unwrap();
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let split = parse::walk_only(&tree, src, Language::Python);
    assert_eq!(combined.functions.len(), split.functions.len());
    for (a, b) in combined.functions.iter().zip(split.functions.iter()) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.cc, b.cc);
        assert_eq!(a.cognitive_complexity, b.cognitive_complexity);
        assert_eq!(a.start_line, b.start_line);
        assert_eq!(a.end_line, b.end_line);
        assert_eq!(a.structural_hash, b.structural_hash);
        assert_eq!(a.skeleton_hash, b.skeleton_hash);
    }
}

#[test]
fn parse_only_tree_outlives_borrow() {
    let src = "def f():\n    return 1\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let kind = tree.root_node().kind().to_string();
    assert_eq!(kind, "module");
}
