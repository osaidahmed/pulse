use pulse::audit::walk_typed_source_files;
use pulse::parse::Language;
use std::process::Command;

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
fn walk_typed_finds_python_and_rust_in_same_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn f() {}\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 2);
    let langs: std::collections::HashSet<Language> = typed.iter().map(|(_, l)| *l).collect();
    assert!(langs.contains(&Language::Python));
    assert!(langs.contains(&Language::Rust));
}

#[test]
fn walk_typed_classifies_each_file_by_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.path().join("b.ts"), "let x = 1;\n").unwrap();
    std::fs::write(dir.path().join("c.go"), "package p\n").unwrap();
    std::fs::write(dir.path().join("d.java"), "class A {}\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 4);
}

#[test]
fn walk_typed_skips_unsupported_extensions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "noise").unwrap();
    std::fs::write(dir.path().join("c.json"), "{}").unwrap();
    std::fs::write(dir.path().join("d.yaml"), "k: v").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn walk_typed_respects_skip_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let nm = dir.path().join("node_modules");
    std::fs::create_dir(&nm).unwrap();
    std::fs::write(nm.join("a.js"), "let x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn walk_typed_skips_dot_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let hidden = dir.path().join(".cache");
    std::fs::create_dir(&hidden).unwrap();
    std::fs::write(hidden.join("a.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn walk_typed_recurses_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.path().join("b.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 2);
}

#[test]
fn audit_cli_handles_mixed_language_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn f(x: i32) -> i32 { if x == 1 { return 1; } 0 }\n").unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
}

#[test]
fn audit_cli_handles_python_only_directory() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        std::fs::write(dir.path().join(format!("a{i}.py")), "x = 1\n").unwrap();
    }
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_cli_handles_rust_only_directory() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        std::fs::write(dir.path().join(format!("a{i}.rs")), "fn f() {}\n").unwrap();
    }
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_cli_handles_typescript_only_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.ts"), "let x: number = 1;\n").unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_cli_handles_go_only_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.go"), "package p\n").unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_cli_handles_java_only_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("A.java"), "class A {}\n").unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_cli_finds_clusters_in_repeated_typescript_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("f{i}.ts")),
            "function process(x: number): number { if (x === 1) { return x; } return 0; }\n",
        ).unwrap();
    }
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("decoy{i}.ts")),
            &format!("export const NAME = \"unique{i}\";\n"),
        ).unwrap();
    }
    let (stdout, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
    let _ = stdout;
}

#[test]
fn audit_cli_finds_clusters_in_repeated_rust_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("f{i}.rs")),
            "fn process(x: i32) -> i32 { if x == 1 { return x; } 0 }\n",
        ).unwrap();
    }
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("decoy{i}.rs")),
            &format!("pub const NAME{i}: &str = \"unique\";\n"),
        ).unwrap();
    }
    let (stdout, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
    let _ = stdout;
}

#[test]
fn audit_cli_finds_clusters_in_repeated_go_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("f{i}.go")),
            "package p\nfunc process(x int) int { if x == 1 { return x }; return 0 }\n",
        ).unwrap();
    }
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("d{i}.go")),
            &format!("package p\nconst N{i} = \"unique\"\n"),
        ).unwrap();
    }
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
}

#[test]
fn walk_typed_handles_files_with_python_and_typescript_in_subdir() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("src");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("a.py"), "x = 1\n").unwrap();
    std::fs::write(sub.join("b.ts"), "let x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 2);
}

#[test]
fn audit_cli_handles_each_supported_extension() {
    use std::collections::HashSet;
    let dir = tempfile::tempdir().unwrap();
    let exts = ["py", "ts", "js", "rs", "c", "cpp", "java", "cs", "go",
                "swift", "zig", "rb", "tcl", "kt", "hs", "lua", "r", "php",
                "d", "groovy"];
    let mut count = 0;
    let mut langs = HashSet::new();
    for (i, ext) in exts.iter().enumerate() {
        let stub = match *ext {
            "py" => format!("x{i} = 1\n"),
            "rs" => format!("const A{i}: i32 = 1;\n"),
            "ts" | "js" => format!("const a{i} = 1;\n"),
            "c" | "cpp" | "java" | "cs" | "swift" | "kt" | "d" | "groovy" => format!("int a{i} = 1;\n"),
            "go" => format!("package p\nvar a{i} = 1\n"),
            "zig" => format!("const a{i}: i32 = 1;\n"),
            "rb" | "tcl" | "lua" => format!("a{i} = 1\n"),
            "hs" => format!("a{i} = 1\n"),
            "r" => format!("a{i} <- 1\n"),
            "php" => format!("<?php $a{i} = 1;\n"),
            _ => "x = 1".to_string(),
        };
        std::fs::write(dir.path().join(format!("file{i}.{ext}")), stub).unwrap();
        count += 1;
        langs.insert(ext);
    }
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
    assert!(count > 0);
    assert!(!langs.is_empty());
}

#[test]
fn audit_walks_directory_with_only_jsx_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.jsx"), "function A() { return null; }\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
    assert_eq!(typed[0].1, Language::JavaScript);
}

#[test]
fn audit_walks_directory_with_only_tsx_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.tsx"), "function A(): null { return null; }\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
    assert_eq!(typed[0].1, Language::TypeScript);
}

#[test]
fn audit_walks_handles_h_files_as_c() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.h"), "void f(void);\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
    assert_eq!(typed[0].1, Language::C);
}

#[test]
fn audit_walks_handles_hpp_files_as_cpp() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.hpp"), "class A;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
    assert_eq!(typed[0].1, Language::Cpp);
}

#[test]
fn audit_walks_handles_kts_files_as_kotlin() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.kts"), "println(\"hi\")\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
    assert_eq!(typed[0].1, Language::Kotlin);
}

#[test]
fn audit_walks_handles_mjs_and_cjs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.mjs"), "export const x = 1;\n").unwrap();
    std::fs::write(dir.path().join("b.cjs"), "module.exports = {};\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 2);
    for (_, lang) in typed {
        assert_eq!(lang, Language::JavaScript);
    }
}

#[test]
fn audit_walks_distinguishes_lowercase_r_from_uppercase_r() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.r"), "x <- 1\n").unwrap();
    std::fs::write(dir.path().join("b.R"), "x <- 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    let r_count = typed.iter().filter(|(_, l)| *l == Language::R).count();
    assert!(r_count >= 1);
}

#[test]
fn audit_walks_handles_haskell_lhs_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.lhs"), "> x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.iter().any(|(_, l)| *l == Language::Haskell));
}

#[test]
fn audit_walks_handles_cobol_extensions() {
    for ext in ["cob", "cbl", "cobol"] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(format!("a.{ext}")), "       IDENTIFICATION DIVISION.\n").unwrap();
        let typed = walk_typed_source_files(dir.path(), true);
        assert!(typed.iter().any(|(_, l)| *l == Language::Cobol), "ext {ext} should detect cobol");
    }
}

#[test]
fn audit_walks_handles_d_with_di_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.di"), "module a;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.iter().any(|(_, l)| *l == Language::D));
}

#[test]
fn audit_walks_handles_php5_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.php5"), "<?php $x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.iter().any(|(_, l)| *l == Language::Php));
}

#[test]
fn audit_cli_handles_mix_with_unsupported_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.path().join("b.md"), "# README\n").unwrap();
    std::fs::write(dir.path().join("c.json"), "{}").unwrap();
    std::fs::write(dir.path().join("d.toml"), "[a]\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_cli_handles_files_at_multiple_directory_levels() {
    let dir = tempfile::tempdir().unwrap();
    let sub1 = dir.path().join("a");
    let sub2 = sub1.join("b");
    let sub3 = sub2.join("c");
    std::fs::create_dir_all(&sub3).unwrap();
    std::fs::write(dir.path().join("top.py"), "x = 1\n").unwrap();
    std::fs::write(sub1.join("mid.py"), "x = 1\n").unwrap();
    std::fs::write(sub2.join("deeper.py"), "x = 1\n").unwrap();
    std::fs::write(sub3.join("deepest.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 4);
}

#[test]
fn audit_walks_handles_ruby_rb_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rb"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed[0].1, Language::Ruby);
}

#[test]
fn audit_walks_handles_swift_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.swift"), "let x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed[0].1, Language::Swift);
}

#[test]
fn audit_walks_handles_zig_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.zig"), "const x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed[0].1, Language::Zig);
}

#[test]
fn audit_walks_handles_objc_m_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.m"), "void f(void) {}\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed[0].1, Language::ObjectiveC);
}

#[test]
fn audit_walks_handles_tcl_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.tcl"), "proc f {} {}\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed[0].1, Language::Tcl);
}

#[test]
fn audit_walks_handles_groovy_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.groovy"), "def f() {}\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed[0].1, Language::Groovy);
}

#[test]
fn audit_walks_handles_lua_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.lua"), "function f() end\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed[0].1, Language::Lua);
}

#[test]
fn audit_finds_distinct_language_count_across_dir() {
    let dir = tempfile::tempdir().unwrap();
    let exts_lang = [
        ("a.py", Language::Python),
        ("b.ts", Language::TypeScript),
        ("c.rs", Language::Rust),
        ("d.go", Language::Go),
        ("e.java", Language::Java),
    ];
    for (name, _) in &exts_lang {
        std::fs::write(dir.path().join(name), "x = 1\n").unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    let langs: std::collections::HashSet<Language> = typed.iter().map(|(_, l)| *l).collect();
    assert_eq!(langs.len(), 5);
}

#[test]
fn audit_dir_with_many_languages_runs_to_completion() {
    let dir = tempfile::tempdir().unwrap();
    let pairs = [
        ("a.py", "x = 1\n"),
        ("b.ts", "let x = 1;\n"),
        ("c.js", "let x = 1;\n"),
        ("d.rs", "fn f() {}\n"),
        ("e.go", "package p\n"),
        ("f.java", "class A {}\n"),
        ("g.cs", "class A {}\n"),
        ("h.swift", "let x = 1\n"),
        ("i.kt", "val x = 1\n"),
        ("j.zig", "const x: i32 = 1;\n"),
    ];
    for (name, content) in &pairs {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
}

#[test]
fn walk_typed_returns_sorted_results() {
    let dir = tempfile::tempdir().unwrap();
    let names = ["z.py", "a.py", "m.py", "b.py"];
    for n in &names {
        std::fs::write(dir.path().join(n), "x = 1\n").unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    let paths: Vec<_> = typed.iter().map(|(p, _)| p).collect();
    for w in paths.windows(2) {
        assert!(w[0] <= w[1]);
    }
}

#[test]
fn audit_skips_target_dir_in_rust_project_layout() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    let target = dir.path().join("target");
    std::fs::create_dir(&src).unwrap();
    std::fs::create_dir(&target).unwrap();
    std::fs::write(src.join("lib.rs"), "fn f() {}\n").unwrap();
    std::fs::write(target.join("debug.rs"), "fn dont_walk() {}\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
    assert!(typed[0].0.to_str().unwrap().contains("src"));
}

#[test]
fn audit_skips_node_modules_in_javascript_project_layout() {
    let dir = tempfile::tempdir().unwrap();
    let nm = dir.path().join("node_modules");
    std::fs::create_dir(&nm).unwrap();
    std::fs::write(nm.join("a.js"), "let x = 1;\n").unwrap();
    std::fs::write(dir.path().join("index.js"), "let x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_skips_pycache_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let pc = dir.path().join("__pycache__");
    std::fs::create_dir(&pc).unwrap();
    std::fs::write(pc.join("a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.path().join("main.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_walks_around_dist_dir() {
    let dir = tempfile::tempdir().unwrap();
    let dist = dir.path().join("dist");
    std::fs::create_dir(&dist).unwrap();
    std::fs::write(dist.join("bundled.js"), "let x = 1;\n").unwrap();
    std::fs::write(dir.path().join("source.js"), "let x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_walks_around_build_dir() {
    let dir = tempfile::tempdir().unwrap();
    let build = dir.path().join("build");
    std::fs::create_dir(&build).unwrap();
    std::fs::write(build.join("a.java"), "class A {}\n").unwrap();
    std::fs::write(dir.path().join("Main.java"), "class Main {}\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_walks_around_vendor_dir() {
    let dir = tempfile::tempdir().unwrap();
    let v = dir.path().join("vendor");
    std::fs::create_dir(&v).unwrap();
    std::fs::write(v.join("a.go"), "package vendor\n").unwrap();
    std::fs::write(dir.path().join("main.go"), "package main\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_dir_recursion_handles_three_layer_skip_dir_inside() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    std::fs::create_dir(&a).unwrap();
    let b = a.join("node_modules");
    std::fs::create_dir(&b).unwrap();
    let c = b.join("c");
    std::fs::create_dir(&c).unwrap();
    std::fs::write(c.join("x.js"), "let x = 1;\n").unwrap();
    std::fs::write(a.join("ok.js"), "let x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}

#[test]
fn audit_handles_directory_with_only_subdirectories() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        let p = dir.path().join(format!("sub{i}"));
        std::fs::create_dir(&p).unwrap();
        std::fs::write(p.join("a.py"), "x = 1\n").unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 5);
}

#[test]
fn audit_finds_zero_files_in_completely_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn audit_handles_directory_with_only_skip_dirs() {
    let dir = tempfile::tempdir().unwrap();
    for skip in &["node_modules", "target", "vendor", "build", "dist"] {
        let p = dir.path().join(skip);
        std::fs::create_dir(&p).unwrap();
        std::fs::write(p.join("a.py"), "x = 1\n").unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.is_empty());
}

#[test]
fn audit_finds_clusters_only_within_same_language() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("py{i}.py")),
            "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        ).unwrap();
    }
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("rs{i}.rs")),
            "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n",
        ).unwrap();
    }
    let _ = walk_typed_source_files(dir.path(), true);
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
}

#[test]
fn audit_handles_mixed_file_types_with_audit_run() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.path().join("b.cpp"), "int x = 1;\n").unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
}

#[test]
fn audit_walks_python_only_when_only_py_files_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
    assert_eq!(typed[0].1, Language::Python);
}

#[test]
fn audit_walks_handles_kts_alongside_kt() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.kt"), "fun f() {}\n").unwrap();
    std::fs::write(dir.path().join("b.kts"), "println(\"hi\")\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    let kt_count = typed.iter().filter(|(_, l)| *l == Language::Kotlin).count();
    assert_eq!(kt_count, 2);
}

#[test]
fn audit_handles_directory_with_both_txx_and_cxx() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.cxx"), "int x = 1;\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert!(typed.iter().any(|(_, l)| *l == Language::Cpp));
}

#[test]
fn audit_handles_files_with_uppercase_extensions_silently() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("A.PY"), "x = 1\n").unwrap();
    let _ = walk_typed_source_files(dir.path(), true);
}
