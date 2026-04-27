use crate::parse::{detect_language, Language};
use std::path::Path;

pub fn is_test_file(path: &str) -> bool {
    let raw = path.replace('\\', "/");
    let p = if raw.starts_with('/') { raw } else { format!("/{raw}") };
    if has_test_dir(&p) {
        return true;
    }
    let name = filename(&p);
    if matches_universal_naming(name) {
        return true;
    }
    let (stem, ext) = split_stem_ext(name);
    if let Some(lang) = detect_language(Path::new(&p)) {
        return matches_language_pattern(stem, ext, lang);
    }
    false
}

fn filename(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

fn split_stem_ext(name: &str) -> (&str, &str) {
    name.rsplit_once('.').map_or((name, ""), |(s, e)| (s, e))
}

const TEST_DIRS: &[&str] = &[
    "/tests/", "/test/", "/__tests__/",
    "/spec/", "/specs/",
    "/Tests/", "/Test/", "/Spec/", "/Specs/",
];

const UNIVERSAL_NAME_PATTERNS: &[&str] = &[
    "_test.", ".test.", "_tests.", ".tests.",
    "_spec.", ".spec.", "_specs.", ".specs.",
    ".e2e.", ".cy.",
];

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn has_test_dir(p: &str) -> bool {
    contains_any(p, TEST_DIRS)
}

fn matches_universal_naming(name: &str) -> bool {
    contains_any(name, UNIVERSAL_NAME_PATTERNS)
}

fn matches_language_pattern(stem: &str, ext: &str, lang: Language) -> bool {
    match lang {
        Language::Python => python_test(stem),
        Language::C | Language::Cpp => c_test(stem),
        Language::R => r_test(stem),
        Language::Lua => lua_test(stem),
        Language::Tcl => ext == "test",
        Language::Zig | Language::D => stem.starts_with("test_"),
        Language::Java | Language::CSharp | Language::Kotlin
        | Language::Swift | Language::Php | Language::ObjectiveC
        | Language::Groovy | Language::Haskell => camel_test_suffix(stem),
        _ => false,
    }
}

fn python_test(stem: &str) -> bool {
    stem.starts_with("test_") || stem == "tests" || stem == "conftest"
}

fn c_test(stem: &str) -> bool {
    stem.starts_with("test_") || stem.starts_with("Test_")
}

fn r_test(stem: &str) -> bool {
    stem.starts_with("test-") || stem.starts_with("test_")
}

fn lua_test(stem: &str) -> bool {
    stem.starts_with("test_") || stem.starts_with("spec_")
}

fn camel_test_suffix(stem: &str) -> bool {
    const SUFFIXES: &[&str] = &["Tests", "Test", "Specs", "Spec"];
    for suffix in SUFFIXES {
        let Some(prefix) = stem.strip_suffix(suffix) else {
            continue;
        };
        if prefix.is_empty() {
            return true;
        }
        let last = prefix.chars().last().unwrap();
        if last.is_alphanumeric() {
            return true;
        }
    }
    false
}
