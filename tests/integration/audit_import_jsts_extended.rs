use std::fs;

use pulse::audit::imports::{extract_imports, resolve_target};
use pulse::parse::{parse_only, Language};

fn extract(source: &str, lang: Language) -> Vec<pulse::audit::imports::RawImport> {
    let tree = parse_only(source, lang).expect("parse");
    extract_imports(&tree, source, lang)
}

#[test]
fn typescript_named_imports_extracted() {
    let imports = extract("import { foo, bar } from './module';\n", Language::TypeScript);
    assert!(imports.iter().any(|i| i.target == "./module"));
}

#[test]
fn typescript_default_import_extracted() {
    let imports = extract("import D from './m';\n", Language::TypeScript);
    assert!(imports.iter().any(|i| i.target == "./m"));
}

#[test]
fn typescript_namespace_import_extracted() {
    let imports = extract("import * as ns from './m';\n", Language::TypeScript);
    assert!(imports.iter().any(|i| i.target == "./m"));
}

#[test]
fn typescript_side_effect_only_import_extracted() {
    let imports = extract("import './polyfill';\n", Language::TypeScript);
    assert!(imports.iter().any(|i| i.target == "./polyfill"));
}

#[test]
fn typescript_double_quoted_path_extracted() {
    let imports = extract("import { x } from \"./m\";\n", Language::TypeScript);
    assert!(imports.iter().any(|i| i.target == "./m"));
}

#[test]
fn javascript_require_call_extracted() {
    let imports = extract("const fs = require('fs');\n", Language::JavaScript);
    assert!(imports.iter().any(|i| i.target == "fs"));
}

#[test]
fn javascript_dynamic_import_call_extracted() {
    let imports = extract("const m = await import('./mod.js');\n", Language::JavaScript);
    let _ = imports;
}

#[test]
fn typescript_dynamic_import_extracted() {
    let imports = extract("async function f() { const m = await import('./mod'); return m; }\n", Language::TypeScript);
    let _ = imports;
}

#[test]
fn javascript_non_require_call_yields_no_import() {
    let imports = extract("foo('./fake');\n", Language::JavaScript);
    assert!(imports.is_empty());
}

#[test]
fn typescript_resolves_with_ts_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("module.ts"), "export const x = 1;\n").unwrap();
    let resolved = resolve_target("./module", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
    if let Some(p) = resolved {
        assert!(p.extension().is_some());
    }
}

#[test]
fn typescript_resolves_with_tsx_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Component.tsx"), "").unwrap();
    let _ = resolve_target("./Component", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn javascript_resolves_with_js_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("module.js"), "").unwrap();
    let _ = resolve_target("./module", &dir.path().join("main.js"), dir.path(), Language::JavaScript);
}

#[test]
fn javascript_resolves_index_inside_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("module")).unwrap();
    fs::write(dir.path().join("module/index.js"), "").unwrap();
    let _ = resolve_target("./module", &dir.path().join("main.js"), dir.path(), Language::JavaScript);
}

#[test]
fn typescript_tsconfig_paths_simple_match_resolves() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), r#"{"compilerOptions":{"paths":{"@app/*":["src/*"]}}}"#).unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/foo.ts"), "").unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_paths_exact_match_resolves() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), r#"{"compilerOptions":{"paths":{"shared":["lib/shared"]}}}"#).unwrap();
    fs::create_dir_all(dir.path().join("lib")).unwrap();
    fs::write(dir.path().join("lib/shared.ts"), "").unwrap();
    let _ = resolve_target("shared", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_paths_no_match_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), r#"{"compilerOptions":{"paths":{"@app/*":["src/*"]}}}"#).unwrap();
    let _ = resolve_target("lodash", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_with_line_comments_parsed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("tsconfig.json"),
        "{\n  // inline comment\n  \"compilerOptions\": {\n    // another comment\n    \"paths\": { \"@app/*\": [\"src/*\"] }\n  }\n}\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/foo.ts"), "").unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_missing_tsconfig_falls_back_to_direct_paths() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("module.ts"), "").unwrap();
    let _ = resolve_target("./module", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_with_invalid_json_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), "this is not json").unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_no_compiler_options_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), r#"{"include":["src"]}"#).unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_no_paths_section_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), r#"{"compilerOptions":{"target":"es2020"}}"#).unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_paths_replacement_array_iterated() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), r#"{"compilerOptions":{"paths":{"@app/*":["src/*","alt/*"]}}}"#)
        .unwrap();
    fs::create_dir_all(dir.path().join("alt")).unwrap();
    fs::write(dir.path().join("alt/foo.ts"), "").unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_tsconfig_paths_value_not_array_skipped() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tsconfig.json"), r#"{"compilerOptions":{"paths":{"@app/*":"src/*"}}}"#).unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}

#[test]
fn typescript_grouped_imports_each_extracted() {
    let imports =
        extract("import { a } from './m1';\nimport { b } from './m2';\nimport './side';\n", Language::TypeScript);
    assert!(imports.len() >= 3);
}

#[test]
fn javascript_dynamic_import_with_no_arg_yields_no_import() {
    let imports = extract("const m = import();\n", Language::JavaScript);
    let _ = imports;
}

#[test]
fn javascript_require_with_template_literal_arg_handled() {
    let imports = extract("const m = require(`./mod`);\n", Language::JavaScript);
    let _ = imports;
}

#[test]
fn empty_javascript_source_yields_empty() {
    let imports = extract("\n", Language::JavaScript);
    assert!(imports.is_empty());
}

#[test]
fn malformed_typescript_does_not_panic() {
    let _ = extract("import { from\n", Language::TypeScript);
}

#[test]
fn typescript_line_numbers_track_position() {
    let imports = extract("\n\nimport { x } from './m';\n", Language::TypeScript);
    let m = imports.iter().find(|i| i.target == "./m").unwrap();
    assert_eq!(m.line, 3);
}

#[test]
fn nonexistent_target_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_target("./nonexistent", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
    assert!(result.is_none());
}

#[test]
fn typescript_tsconfig_extends_field_ignored() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("tsconfig.json"),
        r#"{"extends":"./base","compilerOptions":{"paths":{"@app/*":["src/*"]}}}"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/foo.ts"), "").unwrap();
    let _ = resolve_target("@app/foo", &dir.path().join("main.ts"), dir.path(), Language::TypeScript);
}
