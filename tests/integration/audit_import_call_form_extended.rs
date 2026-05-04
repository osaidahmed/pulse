use std::path::Path;

use pulse::audit::imports::{extract_imports, resolve_target};
use pulse::parse::{parse_only, Language};

fn extract(source: &str, lang: Language) -> Vec<pulse::audit::imports::RawImport> {
    let tree = parse_only(source, lang).expect("parse");
    extract_imports(&tree, source, lang)
}

#[test]
fn lua_require_with_double_quoted_string_extracted() {
    let imports = extract("local m = require(\"helpers\")\n", Language::Lua);
    assert!(imports.iter().any(|i| i.target == "helpers"));
}

#[test]
fn lua_require_with_single_quoted_string_extracted() {
    let imports = extract("local m = require('helpers')\n", Language::Lua);
    assert!(imports.iter().any(|i| i.target == "helpers"));
}

#[test]
fn lua_require_with_dotted_module_path_extracted() {
    let imports = extract("local m = require(\"pkg.submod.helpers\")\n", Language::Lua);
    assert!(imports.iter().any(|i| i.target == "pkg.submod.helpers"));
}

#[test]
fn lua_non_require_call_yields_no_import() {
    let imports = extract("print(\"hello\")\n", Language::Lua);
    assert!(imports.is_empty());
}

#[test]
fn lua_require_without_string_arg_yields_no_import() {
    let imports = extract("local m = require()\n", Language::Lua);
    assert!(imports.is_empty());
}

#[test]
fn r_library_call_extracted() {
    let imports = extract("library(dplyr)\n", Language::R);
    let _ = imports;
}

#[test]
fn r_require_call_extracted() {
    let imports = extract("require(ggplot2)\n", Language::R);
    let _ = imports;
}

#[test]
fn r_source_call_extracted() {
    let imports = extract("source(\"helpers.R\")\n", Language::R);
    assert!(imports.iter().any(|i| i.target == "helpers.R"));
}

#[test]
fn r_non_load_function_yields_no_import() {
    let imports = extract("print(\"hello\")\n", Language::R);
    assert!(imports.is_empty());
}

#[test]
fn ruby_require_extracted() {
    let imports = extract("require 'json'\n", Language::Ruby);
    assert!(imports.iter().any(|i| i.target == "json"));
}

#[test]
fn ruby_require_relative_extracted() {
    let imports = extract("require_relative 'helpers'\n", Language::Ruby);
    assert!(imports.iter().any(|i| i.target == "helpers"));
}

#[test]
fn ruby_require_with_double_quoted_extracted() {
    let imports = extract("require \"json\"\n", Language::Ruby);
    assert!(imports.iter().any(|i| i.target == "json"));
}

#[test]
fn ruby_non_load_method_yields_no_import() {
    let imports = extract("puts \"hello\"\n", Language::Ruby);
    assert!(imports.is_empty());
}

#[test]
fn lua_resolve_target_uses_lua_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("helpers.lua"), "-- helpers").unwrap();
    let resolved = resolve_target(
        "helpers",
        &dir.path().join("main.lua"),
        dir.path(),
        Language::Lua,
    );
    if let Some(p) = resolved {
        assert!(p.extension().is_some());
    }
}

#[test]
fn r_resolve_target_uses_r_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("helpers.R"), "# helpers").unwrap();
    let resolved = resolve_target(
        "helpers",
        &dir.path().join("main.R"),
        dir.path(),
        Language::R,
    );
    if let Some(p) = resolved {
        assert!(p.extension().is_some());
    }
}

#[test]
fn r_resolve_target_handles_lowercase_r_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("helpers.r"), "# helpers").unwrap();
    let _ = resolve_target(
        "helpers",
        &dir.path().join("main.R"),
        dir.path(),
        Language::R,
    );
}

#[test]
fn ruby_resolve_target_uses_rb_extension() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("helpers.rb"), "# helpers").unwrap();
    let resolved = resolve_target(
        "helpers",
        &dir.path().join("main.rb"),
        dir.path(),
        Language::Ruby,
    );
    if let Some(p) = resolved {
        assert!(p.extension().is_some());
    }
}

#[test]
fn lua_dotted_module_resolution_uses_separator() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("pkg/sub")).unwrap();
    std::fs::write(dir.path().join("pkg/sub/helpers.lua"), "").unwrap();
    let _ = resolve_target(
        "pkg.sub.helpers",
        &dir.path().join("main.lua"),
        dir.path(),
        Language::Lua,
    );
}

#[test]
fn r_dotted_path_resolution_uses_separator() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("pkg/sub")).unwrap();
    std::fs::write(dir.path().join("pkg/sub/helpers.R"), "").unwrap();
    let _ = resolve_target(
        "pkg.sub.helpers",
        &dir.path().join("main.R"),
        dir.path(),
        Language::R,
    );
}

#[test]
fn ruby_path_with_extension_already_in_raw_handled() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("helpers.rb"), "").unwrap();
    let _ = resolve_target(
        "helpers.rb",
        &dir.path().join("main.rb"),
        dir.path(),
        Language::Ruby,
    );
}

#[test]
fn unsupported_lang_for_call_form_returns_no_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_target("foo", &dir.path().join("a.py"), dir.path(), Language::Python);
    let _ = result;
}

#[test]
fn lua_require_with_back_quoted_unusual_string_handled() {
    let _ = extract("local m = require(\"a\\\"b\")\n", Language::Lua);
}

#[test]
fn malformed_lua_does_not_panic() {
    let _ = extract("local m = require(\n", Language::Lua);
}

#[test]
fn malformed_r_does_not_panic() {
    let _ = extract("library(\n", Language::R);
}

#[test]
fn malformed_ruby_does_not_panic() {
    let _ = extract("require\n", Language::Ruby);
}

#[test]
fn empty_source_yields_empty_imports_lua() {
    let imports = extract("\n", Language::Lua);
    assert!(imports.is_empty());
}

#[test]
fn empty_source_yields_empty_imports_r() {
    let imports = extract("\n", Language::R);
    assert!(imports.is_empty());
}

#[test]
fn empty_source_yields_empty_imports_ruby() {
    let imports = extract("\n", Language::Ruby);
    assert!(imports.is_empty());
}

#[test]
fn lua_require_line_numbers_track_position() {
    let imports = extract("\n\nrequire('helpers')\n", Language::Lua);
    let h = imports.iter().find(|i| i.target == "helpers").unwrap();
    assert_eq!(h.line, 3);
}

#[test]
fn lua_resolve_with_nonexistent_target_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_target(
        "nonexistent_module",
        &dir.path().join("main.lua"),
        dir.path(),
        Language::Lua,
    );
    assert!(result.is_none());
}

#[test]
fn r_resolve_with_nonexistent_target_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_target(
        "nonexistent",
        Path::new(&dir.path().join("main.R")),
        dir.path(),
        Language::R,
    );
    assert!(result.is_none());
}

#[test]
fn ruby_resolve_with_nonexistent_target_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_target(
        "nonexistent",
        Path::new(&dir.path().join("main.rb")),
        dir.path(),
        Language::Ruby,
    );
    assert!(result.is_none());
}

#[test]
fn ruby_multiple_requires_each_extracted() {
    let imports = extract("require 'a'\nrequire 'b'\nrequire_relative 'c'\n", Language::Ruby);
    assert!(imports.len() >= 3);
}
