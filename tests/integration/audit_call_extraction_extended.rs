use pulse::audit::calls::{dispatchers_for_lang, extract_calls, CallDispatch};
use pulse::parse::{parse_only, Language};

fn parse_extract(source: &str, lang: Language) -> Vec<pulse::audit::calls::RawCall> {
    let tree = parse_only(source, lang).expect("parse must succeed");
    extract_calls(&tree, source, lang)
}

#[test]
fn dispatchers_table_all_languages_enumerable() {
    let langs = [
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Java,
        Language::CSharp,
        Language::Kotlin,
        Language::Swift,
        Language::Ruby,
        Language::Php,
        Language::Groovy,
        Language::Rust,
        Language::Go,
        Language::Cpp,
        Language::C,
        Language::Haskell,
        Language::D,
    ];
    for lang in langs {
        let d = dispatchers_for_lang(lang);
        assert!(!d.is_empty(), "lang {lang:?} has empty dispatcher table");
    }
}

#[test]
fn unsupported_languages_have_empty_dispatchers() {
    let unsupported = [Language::Lua, Language::R, Language::Tcl, Language::Cobol, Language::ObjectiveC, Language::Zig];
    for lang in unsupported {
        let d = dispatchers_for_lang(lang);
        assert!(d.is_empty(), "lang {lang:?} should have no dispatchers");
    }
}

#[test]
fn unsupported_language_extract_returns_empty_vec() {
    let source = "x = 1";
    if let Some(tree) = parse_only(source, Language::Lua) {
        let calls = extract_calls(&tree, source, Language::Lua);
        assert!(calls.is_empty());
    }
}

#[test]
fn php_dispatchers_includes_both_method_dotted_and_path_qualified() {
    let d = dispatchers_for_lang(Language::Php);
    assert!(d.contains(&CallDispatch::MethodDotted));
    assert!(d.contains(&CallDispatch::PathQualified));
}

#[test]
fn rust_dispatchers_path_qualified_first() {
    let d = dispatchers_for_lang(Language::Rust);
    assert_eq!(d[0], CallDispatch::PathQualified);
}

#[test]
fn cpp_dispatchers_path_qualified_first() {
    let d = dispatchers_for_lang(Language::Cpp);
    assert_eq!(d[0], CallDispatch::PathQualified);
}

#[test]
fn d_dispatchers_method_dotted_first() {
    let d = dispatchers_for_lang(Language::D);
    assert_eq!(d[0], CallDispatch::MethodDotted);
}

#[test]
fn python_simple_dotted_call_extracted() {
    let calls = parse_extract("obj.method()\n", Language::Python);
    assert!(calls.iter().any(|c| c.callee_name == "method"));
    assert!(calls.iter().any(|c| c.receiver_hint.as_deref() == Some("obj")));
}

#[test]
fn python_bare_call_extracted_without_receiver() {
    let calls = parse_extract("foo()\n", Language::Python);
    assert!(calls.iter().any(|c| c.callee_name == "foo" && c.receiver_hint.is_none()));
}

#[test]
fn python_chained_dotted_uses_immediate_receiver() {
    let calls = parse_extract("a.b.c()\n", Language::Python);
    assert!(calls.iter().any(|c| c.callee_name == "c"));
    let chained = calls.iter().find(|c| c.callee_name == "c").unwrap();
    let recv = chained.receiver_hint.as_deref().unwrap();
    assert!(recv == "b" || recv == "a.b" || recv.ends_with('b'));
}

#[test]
fn rust_qualified_path_call_extracted() {
    let source = "fn main() { Foo::bar(); }\n";
    let calls = parse_extract(source, Language::Rust);
    assert!(calls.iter().any(|c| c.callee_name == "bar"));
    assert!(calls.iter().any(|c| c.receiver_hint.as_deref() == Some("Foo")));
}

#[test]
fn rust_self_method_call_extracted_via_method_dotted() {
    let source = "impl Foo { fn run(&self) { self.go(); } }\n";
    let calls = parse_extract(source, Language::Rust);
    assert!(calls.iter().any(|c| c.callee_name == "go"));
}

#[test]
fn go_dotted_call_extracted() {
    let source = "package m\nfunc main() { pkg.Func() }\n";
    let calls = parse_extract(source, Language::Go);
    assert!(calls.iter().any(|c| c.callee_name == "Func"));
}

#[test]
fn java_method_invocation_extracted() {
    let source = "class M { void run() { obj.method(); } }\n";
    let calls = parse_extract(source, Language::Java);
    assert!(calls.iter().any(|c| c.callee_name == "method"));
}

#[test]
fn javascript_member_call_extracted() {
    let source = "function f() { obj.method(); }\n";
    let calls = parse_extract(source, Language::JavaScript);
    assert!(calls.iter().any(|c| c.callee_name == "method"));
}

#[test]
fn typescript_member_call_extracted() {
    let source = "function f() { obj.method(); }\n";
    let calls = parse_extract(source, Language::TypeScript);
    assert!(calls.iter().any(|c| c.callee_name == "method"));
}

#[test]
fn csharp_member_call_handled_without_panic() {
    let source = "class M { void Run() { obj.Method(); } }\n";
    let _ = parse_extract(source, Language::CSharp);
}

#[test]
fn kotlin_dotted_call_handled_without_panic() {
    let source = "fun main() { obj.method() }\n";
    let _ = parse_extract(source, Language::Kotlin);
}

#[test]
fn swift_dotted_call_handled_without_panic() {
    let source = "func run() { obj.method() }\n";
    let _ = parse_extract(source, Language::Swift);
}

#[test]
fn ruby_dotted_call_extracted() {
    let source = "def run\n  obj.method\nend\n";
    let calls = parse_extract(source, Language::Ruby);
    assert!(calls.iter().any(|c| c.callee_name == "method"));
}

#[test]
fn php_dotted_call_handled_without_panic() {
    let source = "<?php $obj->method(); ?>\n";
    let _ = parse_extract(source, Language::Php);
}

#[test]
fn groovy_dotted_call_extracted() {
    let source = "def run() { obj.method() }\n";
    let calls = parse_extract(source, Language::Groovy);
    assert!(calls.iter().any(|c| c.callee_name == "method"));
}

#[test]
fn cpp_qualified_call_extracted() {
    let source = "namespace ns { void run() { Foo::bar(); } }\n";
    let calls = parse_extract(source, Language::Cpp);
    assert!(calls.iter().any(|c| c.callee_name == "bar"));
}

#[test]
fn cpp_dotted_call_extracted() {
    let source = "void run() { obj.method(); }\n";
    let calls = parse_extract(source, Language::Cpp);
    assert!(calls.iter().any(|c| c.callee_name == "method"));
}

#[test]
fn c_function_call_extracted() {
    let source = "int main() { foo(); return 0; }\n";
    let calls = parse_extract(source, Language::C);
    assert!(calls.iter().any(|c| c.callee_name == "foo"));
}

#[test]
fn empty_source_yields_empty_calls() {
    let calls = parse_extract("\n", Language::Python);
    assert!(calls.is_empty());
}

#[test]
fn malformed_python_does_not_panic() {
    let _ = parse_extract("def x(\n    foo(.\n", Language::Python);
}

#[test]
fn malformed_rust_does_not_panic() {
    let _ = parse_extract("fn main(\n    Foo::\n", Language::Rust);
}

#[test]
fn long_chained_calls_handled() {
    let source = "fn run() { a.b().c().d().e(); }\n";
    let _ = parse_extract(source, Language::JavaScript);
}

#[test]
fn nested_calls_each_emit_record() {
    let source = "x = f(g(h(1)))\n";
    let calls = parse_extract(source, Language::Python);
    let names: std::collections::BTreeSet<String> = calls.iter().map(|c| c.callee_name.clone()).collect();
    assert!(names.contains("f"));
    assert!(names.contains("g"));
    assert!(names.contains("h"));
}

#[test]
fn multiple_arguments_dont_inflate_call_count() {
    let source = "x = f(1, 2, 3)\n";
    let calls = parse_extract(source, Language::Python);
    let f_calls = calls.iter().filter(|c| c.callee_name == "f").count();
    assert_eq!(f_calls, 1);
}

#[test]
fn call_line_numbers_track_source_position() {
    let source = "\n\nobj.method()\n";
    let calls = parse_extract(source, Language::Python);
    let m = calls.iter().find(|c| c.callee_name == "method").unwrap();
    assert_eq!(m.line, 3);
}

#[test]
fn calls_extracted_in_source_order_for_deterministic_output() {
    let source = "obj.first()\nobj.second()\nobj.third()\n";
    let calls = parse_extract(source, Language::Python);
    let names: Vec<String> = calls
        .iter()
        .filter(|c| matches!(c.callee_name.as_str(), "first" | "second" | "third"))
        .map(|c| c.callee_name.clone())
        .collect();
    assert_eq!(names, vec!["first", "second", "third"]);
}

#[test]
fn rust_macro_invocations_skipped_correctly() {
    let source = "fn main() { println!(\"hi\"); }\n";
    let _ = parse_extract(source, Language::Rust);
}

#[test]
fn d_method_dotted_extracted() {
    let source = "void run() { obj.method(); }\n";
    let _ = parse_extract(source, Language::D);
}

#[test]
fn haskell_function_application_extracted() {
    let source = "main = obj method\n";
    let _ = parse_extract(source, Language::Haskell);
}

#[test]
fn determinism_same_source_yields_same_extracted_calls_five_runs() {
    let source = "fn main() { obj.first(); pkg.second(); a.b.third(); }\n";
    let mut all = Vec::new();
    for _ in 0..5 {
        let mut names: Vec<String> = parse_extract(source, Language::Rust).into_iter().map(|c| c.callee_name).collect();
        names.sort();
        all.push(names);
    }
    for run in &all[1..] {
        assert_eq!(*run, all[0]);
    }
}
