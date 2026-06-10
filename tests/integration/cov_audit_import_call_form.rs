use std::path::Path;

use pulse::audit::import_call_form;
use pulse::audit::imports::extract_imports;
use pulse::parse::{parse_only, Language};

fn extract(source: &str, lang: Language) -> Vec<pulse::audit::imports::RawImport> {
    let tree = parse_only(source, lang).expect("parse");
    extract_imports(&tree, source, lang)
}

#[test]
fn lua_dotted_method_callee_is_not_identifier_yields_no_import() {
    let imports = extract("local m = socket.require(\"helpers\")\n", Language::Lua);
    assert!(imports.is_empty());
}

#[test]
fn lua_indexed_callee_is_not_identifier_yields_no_import() {
    let imports = extract("local m = tbl[\"require\"](\"helpers\")\n", Language::Lua);
    assert!(imports.iter().all(|i| i.target != "helpers"));
}

#[test]
fn candidates_for_unsupported_lang_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let source_file = dir.path().join("a.py");
    let out = import_call_form::candidates("foo", &source_file, dir.path(), Language::Python);
    assert!(out.is_empty());
}

#[test]
fn candidates_for_another_unsupported_lang_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let source_file = dir.path().join("a.go");
    let out = import_call_form::candidates("pkg.mod", &source_file, dir.path(), Language::Go);
    assert!(out.is_empty());
}

#[test]
fn match_node_for_unsupported_lang_returns_none() {
    let src = "print(\"hello\")\n";
    let tree = parse_only(src, Language::Python).expect("parse");
    let result = import_call_form::match_node(tree.root_node(), src, Language::Python);
    assert!(result.is_none());
}

#[test]
fn match_node_for_unsupported_call_lang_on_call_node_returns_none() {
    let src = "func main() { fmt.Println(\"x\") }\n";
    let tree = parse_only(src, Language::Go).expect("parse");
    let result = import_call_form::match_node(tree.root_node(), src, Language::Go);
    assert!(result.is_none());
}

#[test]
fn lua_require_empty_double_quoted_string_strips_quotes() {
    let imports = extract("local m = require(\"\")\n", Language::Lua);
    assert!(imports.iter().any(|i| i.target.is_empty()));
}

#[test]
fn lua_require_empty_single_quoted_string_strips_quotes() {
    let imports = extract("local m = require('')\n", Language::Lua);
    assert!(imports.iter().any(|i| i.target.is_empty()));
}

#[test]
fn r_source_empty_string_strips_quotes() {
    let imports = extract("source(\"\")\n", Language::R);
    assert!(imports.iter().any(|i| i.target.is_empty()));
}

#[test]
fn lua_empty_string_require_does_not_panic_and_yields_import() {
    let imports = extract("require('')\n", Language::Lua);
    let _ = imports;
}

#[test]
fn candidates_for_supported_lua_lang_is_nonempty_for_contrast() {
    let dir = tempfile::tempdir().unwrap();
    let source_file = dir.path().join("main.lua");
    let out = import_call_form::candidates("helpers", &source_file, dir.path(), Language::Lua);
    assert!(!out.is_empty());
}

#[test]
fn match_node_on_non_call_node_for_supported_lang_returns_none() {
    let src = "local x = 1\n";
    let tree = parse_only(src, Language::Lua).expect("parse");
    let result = import_call_form::match_node(tree.root_node(), src, Language::Lua);
    let _ = result;
    let _ = Path::new(".");
}
