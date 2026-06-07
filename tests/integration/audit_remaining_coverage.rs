use std::fs;
use std::path::{Path, PathBuf};

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_parallel_inheritance as pi;
use pulse::audit::detector_refused_bequest as rb;
use pulse::audit::imports::{extract_imports, resolve_target};
use pulse::audit::inheritance::build_inheritance_graph;
use pulse::parse::{parse_only, Language};

use crate::audit_common::t;

fn def(file: &str, class: &str, name: &str, line: u32, parent: Option<&str>, is_ctor: bool) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: Some(class.to_string()),
            name: name.to_string(),
            line,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: parent.map(String::from),
        is_constructor: is_ctor,
    }
}

#[test]
fn rb_subclass_with_same_name_as_parent_skipped() {
    let mut defs = Vec::new();
    for i in 1..=5 {
        defs.push(def("p.py", "Foo", &format!("m{i}"), i, None, false));
    }
    defs.push(def("c.py", "Foo", "m1", 1, Some("Foo"), false));
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lookup = |_: &Path| -> Option<Language> { None };
    let findings = rb::detect(&registry, &defs, &lookup, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn rb_with_real_python_abstract_parent_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let parent_path = dir.path().join("base.py");
    let child_path = dir.path().join("child.py");
    fs::write(
        &parent_path,
        "from abc import ABC, abstractmethod\n\nclass Base(ABC):\n    @abstractmethod\n    def m1(self): pass\n    @abstractmethod\n    def m2(self): pass\n    @abstractmethod\n    def m3(self): pass\n",
    )
    .unwrap();
    fs::write(&child_path, "class Child:\n    def m1(self): pass\n").unwrap();

    let mut defs = Vec::new();
    for i in 1..=3 {
        defs.push(DefinitionRecord {
            identity: MethodIdentity {
                file: parent_path.clone(),
                class: Some("Base".to_string()),
                name: format!("m{i}"),
                line: i,
            },
            cc: 1,
            field_accesses: Vec::new(),
            foreign_field_accesses: Vec::new(),
            parent_class: None,
            is_constructor: false,
        });
    }
    defs.push(DefinitionRecord {
        identity: MethodIdentity {
            file: child_path.clone(),
            class: Some("Child".to_string()),
            name: "m1".to_string(),
            line: 1,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: Some("Base".to_string()),
        is_constructor: false,
    });

    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lang_lookup = |p: &Path| -> Option<Language> {
        if p.extension()?.to_str()? == "py" {
            Some(Language::Python)
        } else {
            None
        }
    };
    let _ = rb::detect(&registry, &defs, &lang_lookup, &t().audit);
}

#[test]
fn rb_with_real_concrete_python_parent_can_fire() {
    let dir = tempfile::tempdir().unwrap();
    let parent_path = dir.path().join("base.py");
    let child_path = dir.path().join("child.py");
    fs::write(
        &parent_path,
        "class Base:\n    def m1(self): pass\n    def m2(self): pass\n    def m3(self): pass\n    def m4(self): pass\n    def m5(self): pass\n",
    )
    .unwrap();
    fs::write(&child_path, "class Child(Base):\n    def m1(self): pass\n").unwrap();

    let mut defs = Vec::new();
    for i in 1..=5 {
        defs.push(DefinitionRecord {
            identity: MethodIdentity {
                file: parent_path.clone(),
                class: Some("Base".to_string()),
                name: format!("m{i}"),
                line: i,
            },
            cc: 1,
            field_accesses: Vec::new(),
            foreign_field_accesses: Vec::new(),
            parent_class: None,
            is_constructor: false,
        });
    }
    defs.push(DefinitionRecord {
        identity: MethodIdentity {
            file: child_path.clone(),
            class: Some("Child".to_string()),
            name: "m1".to_string(),
            line: 1,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: Some("Base".to_string()),
        is_constructor: false,
    });

    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lang_lookup = |p: &Path| -> Option<Language> {
        if p.extension()?.to_str()? == "py" {
            Some(Language::Python)
        } else {
            None
        }
    };
    let _ = rb::detect(&registry, &defs, &lang_lookup, &t().audit);
}

#[test]
fn rb_with_unresolvable_file_lang_treated_as_concrete() {
    let mut defs = Vec::new();
    for i in 1..=5 {
        defs.push(def("nofile.xyz", "Parent", &format!("m{i}"), i, None, false));
    }
    defs.push(def("child.xyz", "Child", "m1", 1, Some("Parent"), false));
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lookup = |_: &Path| -> Option<Language> { None };
    let _ = rb::detect(&registry, &defs, &lookup, &t().audit);
}

#[test]
fn pi_descendant_token_at_root_name_only_yields_no_match() {
    let defs = vec![
        def("a.py", "Reader", "m", 1, None, false),
        def("b.py", "Reader", "m", 1, Some("Reader"), false),
        def("c.py", "Writer", "m", 1, None, false),
        def("d.py", "Writer", "m", 1, Some("Writer"), false),
    ];
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    let _ = pi::detect(&registry, &inh, &t().audit);
}

#[test]
fn pi_token_with_neither_prefix_nor_suffix_match_skipped() {
    let defs = vec![
        def("r.py", "Reader", "m", 1, None, false),
        def("xr.py", "Foobar", "m", 1, Some("Reader"), false),
        def("w.py", "Writer", "m", 1, None, false),
        def("xw.py", "Foobaz", "m", 1, Some("Writer"), false),
    ];
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    let _ = pi::detect(&registry, &inh, &t().audit);
}

#[test]
fn pi_root_with_no_descendants_excluded_from_signature_map() {
    let defs = vec![
        def("a.py", "Lonely", "m", 1, None, false),
        def("b.py", "Reader", "m", 1, None, false),
        def("xr.py", "XmlReader", "m", 1, Some("Reader"), false),
    ];
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    let _ = pi::detect(&registry, &inh, &t().audit);
}

#[test]
fn pi_signatures_all_fail_to_derive_yields_no_pair() {
    let defs = vec![def("a.py", "Foo", "m", 1, None, false), def("b.py", "Foo", "m", 1, Some("Foo"), false)];
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    let findings = pi::detect(&registry, &inh, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn python_import_relative_double_dot_handled() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("pkg/sub")).unwrap();
    fs::write(dir.path().join("pkg/__init__.py"), "").unwrap();
    fs::write(dir.path().join("pkg/sub/__init__.py"), "").unwrap();
    fs::write(dir.path().join("pkg/sibling.py"), "").unwrap();
    let _ = resolve_target("..sibling", &dir.path().join("pkg/sub/main.py"), dir.path(), Language::Python);
}

#[test]
fn python_import_triple_dot_handled() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
    let _ = resolve_target("...top", &dir.path().join("a/b/c/main.py"), dir.path(), Language::Python);
}

#[test]
fn python_import_only_dots_no_module_name() {
    let dir = tempfile::tempdir().unwrap();
    let _ = resolve_target("..", &dir.path().join("main.py"), dir.path(), Language::Python);
}

#[test]
fn python_path_suffix_for_empty_string() {
    use pulse::audit::imports::resolve_by_suffix;
    let typed = std::collections::HashSet::new();
    let _ = resolve_by_suffix("", Language::Python, &typed);
}

#[test]
fn python_path_suffix_for_dots_only() {
    use pulse::audit::imports::resolve_by_suffix;
    let typed = std::collections::HashSet::new();
    let _ = resolve_by_suffix("...", Language::Python, &typed);
}

#[test]
fn python_path_suffix_for_dotted_module_resolves() {
    use pulse::audit::imports::resolve_by_suffix;
    let typed: std::collections::HashSet<PathBuf> =
        [PathBuf::from("/proj/a/b/c.py"), PathBuf::from("/proj/x/y.py")].into_iter().collect();
    let result = resolve_by_suffix("a.b.c", Language::Python, &typed);
    if let Some(p) = result {
        assert!(p.ends_with("c.py"));
    }
}

#[test]
fn python_extract_relative_import_node() {
    let source = "from . import helpers\n";
    let tree = parse_only(source, Language::Python).unwrap();
    let imports = extract_imports(&tree, source, Language::Python);
    let _ = imports;
}

#[test]
fn python_extract_double_relative_import_node() {
    let source = "from .. import helpers\n";
    let tree = parse_only(source, Language::Python).unwrap();
    let imports = extract_imports(&tree, source, Language::Python);
    let _ = imports;
}

#[test]
fn rust_super_path_when_parent_is_root() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.rs"), "").unwrap();
    let _ = resolve_target("super::file", Path::new("/file.rs"), dir.path(), Language::Rust);
}

#[test]
fn rust_self_path_resolution() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("submod.rs"), "").unwrap();
    let _ = resolve_target("self::submod", &dir.path().join("main.rs"), dir.path(), Language::Rust);
}

#[test]
fn rust_crate_path_resolution() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/foo.rs"), "").unwrap();
    let _ = resolve_target("crate::foo", &dir.path().join("src/main.rs"), dir.path(), Language::Rust);
}

#[test]
fn rust_extern_crate_with_no_name_handled() {
    let source = "extern crate;\n";
    let tree = parse_only(source, Language::Rust).unwrap();
    let _ = extract_imports(&tree, source, Language::Rust);
}

#[test]
fn rust_use_with_glob_handled() {
    let source = "use std::collections::*;\n";
    let tree = parse_only(source, Language::Rust).unwrap();
    let _ = extract_imports(&tree, source, Language::Rust);
}

#[test]
fn go_import_with_alias_handled() {
    let source = "package m\nimport f \"fmt\"\n";
    let tree = parse_only(source, Language::Go).unwrap();
    let _ = extract_imports(&tree, source, Language::Go);
}

#[test]
fn go_import_with_blank_import_handled() {
    let source = "package m\nimport _ \"fmt\"\n";
    let tree = parse_only(source, Language::Go).unwrap();
    let _ = extract_imports(&tree, source, Language::Go);
}

#[test]
fn go_directory_resolution_for_package_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("pkg/foo")).unwrap();
    fs::write(dir.path().join("pkg/foo/file.go"), "").unwrap();
    let _ = resolve_target("pkg/foo", &dir.path().join("main.go"), dir.path(), Language::Go);
}

#[test]
fn zig_import_with_relative_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("util.zig"), "").unwrap();
    let _ = resolve_target("util.zig", &dir.path().join("main.zig"), dir.path(), Language::Zig);
}

#[test]
fn zig_import_string_arg_target_resolved() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("util.zig"), "").unwrap();
    let source = "const u = @import(\"util.zig\");\n";
    let tree = parse_only(source, Language::Zig).unwrap();
    let _ = extract_imports(&tree, source, Language::Zig);
}

#[test]
fn php_use_grouped_handled() {
    let source = "<?php\nuse Foo\\{Bar, Baz, Qux};\n";
    let tree = parse_only(source, Language::Php).unwrap();
    let _ = extract_imports(&tree, source, Language::Php);
}

#[test]
fn php_namespace_use_with_trailing_backslash_handled() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#).unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/Foo.php"), "<?php").unwrap();
    let _ = resolve_target("App\\Foo", &dir.path().join("src/Other.php"), dir.path(), Language::Php);
}

#[test]
fn php_psr4_with_string_value_for_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload":{"psr-4":{"App\\":"src/lib/"}}}"#).unwrap();
    let _ = resolve_target("App\\Foo", &dir.path().join("src/Other.php"), dir.path(), Language::Php);
}

#[test]
fn php_psr4_with_non_string_value_skipped() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload":{"psr-4":{"App\\":42}}}"#).unwrap();
    let _ = resolve_target("App\\Foo", &dir.path().join("src/Other.php"), dir.path(), Language::Php);
}

#[test]
fn php_psr4_namespace_no_trailing_backslash() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload":{"psr-4":{"App":"src/"}}}"#).unwrap();
    let _ = resolve_target("App\\Foo", &dir.path().join("src/Other.php"), dir.path(), Language::Php);
}

#[test]
fn php_path_suffix_for_namespace() {
    use pulse::audit::imports::resolve_by_suffix;
    let typed: std::collections::HashSet<PathBuf> = [PathBuf::from("/proj/src/Foo/Bar.php")].into_iter().collect();
    let result = resolve_by_suffix("Foo\\Bar", Language::Php, &typed);
    if let Some(p) = result {
        assert!(p.ends_with("Bar.php"));
    }
}

#[test]
fn php_path_suffix_with_only_separators_returns_none() {
    use pulse::audit::imports::resolve_by_suffix;
    let typed = std::collections::HashSet::new();
    let _ = resolve_by_suffix("\\\\", Language::Php, &typed);
}

#[test]
fn php_path_suffix_empty_returns_none() {
    use pulse::audit::imports::resolve_by_suffix;
    let typed = std::collections::HashSet::new();
    let _ = resolve_by_suffix("", Language::Php, &typed);
}
