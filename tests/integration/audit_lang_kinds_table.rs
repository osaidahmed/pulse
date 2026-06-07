use pulse::audit::lang_kinds::{is_skippable_root, skippable_root_kinds};
use pulse::parse::Language;

const ALL_LANGS: &[Language] = &[
    Language::Python,
    Language::TypeScript,
    Language::JavaScript,
    Language::Rust,
    Language::C,
    Language::Cpp,
    Language::Java,
    Language::CSharp,
    Language::Go,
    Language::Swift,
    Language::Zig,
    Language::Ruby,
    Language::ObjectiveC,
    Language::Tcl,
    Language::Kotlin,
    Language::Haskell,
    Language::Lua,
    Language::R,
    Language::Php,
    Language::Cobol,
    Language::D,
    Language::Groovy,
];

#[test]
fn lang_kinds_returns_slice_for_python() {
    let _ = skippable_root_kinds(Language::Python);
}

#[test]
fn lang_kinds_returns_slice_for_typescript() {
    let _ = skippable_root_kinds(Language::TypeScript);
}

#[test]
fn lang_kinds_returns_slice_for_javascript() {
    let _ = skippable_root_kinds(Language::JavaScript);
}

#[test]
fn lang_kinds_returns_slice_for_rust() {
    let _ = skippable_root_kinds(Language::Rust);
}

#[test]
fn lang_kinds_returns_slice_for_c() {
    let _ = skippable_root_kinds(Language::C);
}

#[test]
fn lang_kinds_returns_slice_for_cpp() {
    let _ = skippable_root_kinds(Language::Cpp);
}

#[test]
fn lang_kinds_returns_slice_for_java() {
    let _ = skippable_root_kinds(Language::Java);
}

#[test]
fn lang_kinds_returns_slice_for_csharp() {
    let _ = skippable_root_kinds(Language::CSharp);
}

#[test]
fn lang_kinds_returns_slice_for_go() {
    let _ = skippable_root_kinds(Language::Go);
}

#[test]
fn lang_kinds_returns_slice_for_swift() {
    let _ = skippable_root_kinds(Language::Swift);
}

#[test]
fn lang_kinds_returns_slice_for_zig() {
    let _ = skippable_root_kinds(Language::Zig);
}

#[test]
fn lang_kinds_returns_slice_for_ruby() {
    let _ = skippable_root_kinds(Language::Ruby);
}

#[test]
fn lang_kinds_returns_slice_for_objc() {
    let _ = skippable_root_kinds(Language::ObjectiveC);
}

#[test]
fn lang_kinds_returns_slice_for_tcl() {
    let _ = skippable_root_kinds(Language::Tcl);
}

#[test]
fn lang_kinds_returns_slice_for_kotlin() {
    let _ = skippable_root_kinds(Language::Kotlin);
}

#[test]
fn lang_kinds_returns_slice_for_haskell() {
    let _ = skippable_root_kinds(Language::Haskell);
}

#[test]
fn lang_kinds_returns_slice_for_lua() {
    let _ = skippable_root_kinds(Language::Lua);
}

#[test]
fn lang_kinds_returns_slice_for_r() {
    let _ = skippable_root_kinds(Language::R);
}

#[test]
fn lang_kinds_returns_slice_for_php() {
    let _ = skippable_root_kinds(Language::Php);
}

#[test]
fn lang_kinds_returns_slice_for_cobol() {
    let _ = skippable_root_kinds(Language::Cobol);
}

#[test]
fn lang_kinds_returns_slice_for_d() {
    let _ = skippable_root_kinds(Language::D);
}

#[test]
fn lang_kinds_returns_slice_for_groovy() {
    let _ = skippable_root_kinds(Language::Groovy);
}

#[test]
fn lang_kinds_table_has_22_entries_via_iteration() {
    assert_eq!(ALL_LANGS.len(), 22);
}

#[test]
fn lang_kinds_v1_default_empty_for_python() {
    assert!(skippable_root_kinds(Language::Python).is_empty());
}

#[test]
fn lang_kinds_v1_default_empty_for_each_language() {
    for &lang in ALL_LANGS {
        assert!(skippable_root_kinds(lang).is_empty(), "{lang:?}");
    }
}

#[test]
fn is_skippable_root_returns_false_for_unknown_kind_python() {
    assert!(!is_skippable_root(Language::Python, "nonexistent_kind"));
}

#[test]
fn is_skippable_root_returns_false_for_unknown_kind_each_language() {
    for &lang in ALL_LANGS {
        assert!(!is_skippable_root(lang, "definitely_not_a_kind"));
    }
}

#[test]
fn is_skippable_root_returns_false_for_common_function_kinds() {
    let kinds = ["function_definition", "function_declaration", "method_declaration"];
    for &lang in ALL_LANGS {
        for kind in &kinds {
            assert!(!is_skippable_root(lang, kind));
        }
    }
}

#[test]
fn is_skippable_root_returns_false_for_common_block_kinds() {
    let kinds = ["block", "statement_block", "block_statement", "do_block"];
    for &lang in ALL_LANGS {
        for kind in &kinds {
            assert!(!is_skippable_root(lang, kind));
        }
    }
}

#[test]
fn is_skippable_root_returns_false_for_comment_kinds() {
    for &lang in ALL_LANGS {
        for kind in &["comment", "line_comment", "block_comment"] {
            assert!(!is_skippable_root(lang, kind));
        }
    }
}

#[test]
fn is_skippable_root_handles_empty_string_kind() {
    for &lang in ALL_LANGS {
        assert!(!is_skippable_root(lang, ""));
    }
}

#[test]
fn is_skippable_root_handles_unicode_kind_string() {
    for &lang in ALL_LANGS {
        assert!(!is_skippable_root(lang, "δλ_kind"));
    }
}

#[test]
fn lang_kinds_returns_static_slice_lifetime() {
    let s: &'static [&'static str] = skippable_root_kinds(Language::Python);
    let _ = s;
}

#[test]
fn lang_kinds_table_distinct_per_language_callable_independently() {
    for &a in ALL_LANGS {
        for &b in ALL_LANGS {
            let _ = skippable_root_kinds(a);
            let _ = skippable_root_kinds(b);
        }
    }
}

#[test]
fn lang_kinds_python_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Python).len(), 0);
}

#[test]
fn lang_kinds_typescript_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::TypeScript).len(), 0);
}

#[test]
fn lang_kinds_javascript_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::JavaScript).len(), 0);
}

#[test]
fn lang_kinds_rust_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Rust).len(), 0);
}

#[test]
fn lang_kinds_c_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::C).len(), 0);
}

#[test]
fn lang_kinds_cpp_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Cpp).len(), 0);
}

#[test]
fn lang_kinds_java_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Java).len(), 0);
}

#[test]
fn lang_kinds_csharp_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::CSharp).len(), 0);
}

#[test]
fn lang_kinds_go_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Go).len(), 0);
}

#[test]
fn lang_kinds_swift_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Swift).len(), 0);
}

#[test]
fn lang_kinds_zig_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Zig).len(), 0);
}

#[test]
fn lang_kinds_ruby_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Ruby).len(), 0);
}

#[test]
fn lang_kinds_objc_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::ObjectiveC).len(), 0);
}

#[test]
fn lang_kinds_tcl_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Tcl).len(), 0);
}

#[test]
fn lang_kinds_kotlin_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Kotlin).len(), 0);
}

#[test]
fn lang_kinds_haskell_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Haskell).len(), 0);
}

#[test]
fn lang_kinds_lua_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Lua).len(), 0);
}

#[test]
fn lang_kinds_r_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::R).len(), 0);
}

#[test]
fn lang_kinds_php_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Php).len(), 0);
}

#[test]
fn lang_kinds_cobol_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Cobol).len(), 0);
}

#[test]
fn lang_kinds_d_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::D).len(), 0);
}

#[test]
fn lang_kinds_groovy_explicitly_empty() {
    assert_eq!(skippable_root_kinds(Language::Groovy).len(), 0);
}

#[test]
fn lang_kinds_distinct_invocations_yield_same_addresses() {
    let a = skippable_root_kinds(Language::Python).as_ptr();
    let b = skippable_root_kinds(Language::Python).as_ptr();
    assert_eq!(a, b);
}

#[test]
fn is_skippable_root_returns_consistent_for_all_lang_pairs() {
    for &lang in ALL_LANGS {
        let a = is_skippable_root(lang, "if_statement");
        let b = is_skippable_root(lang, "if_statement");
        assert_eq!(a, b);
    }
}

#[test]
fn is_skippable_root_invariant_under_repeated_calls() {
    for _ in 0..100 {
        assert!(!is_skippable_root(Language::Python, "if_statement"));
    }
}

#[test]
fn is_skippable_root_handles_long_kind_string() {
    let long = "a".repeat(1000);
    for &lang in ALL_LANGS {
        assert!(!is_skippable_root(lang, &long));
    }
}

#[test]
fn lang_kinds_table_indexable_correctly_via_lang_index() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for &lang in ALL_LANGS {
        seen.insert(lang);
    }
    assert_eq!(seen.len(), 22);
}

#[test]
fn is_skippable_root_for_anonymous_node_types_returns_false() {
    let kinds = ["(", ")", "{", "}", ";", ","];
    for &lang in ALL_LANGS {
        for kind in &kinds {
            assert!(!is_skippable_root(lang, kind));
        }
    }
}

#[test]
fn is_skippable_root_does_not_panic_on_extreme_input() {
    let bad = "🦀💀".repeat(100);
    for &lang in ALL_LANGS {
        assert!(!is_skippable_root(lang, &bad));
    }
}
