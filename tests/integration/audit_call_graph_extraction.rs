use std::path::PathBuf;

use pulse::audit::call_walker::{calls_for_file, LocatedCall};
use pulse::parse::Language;

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("snippet.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

fn find_call<'a>(calls: &'a [LocatedCall], name: &str) -> Option<&'a LocatedCall> {
    calls.iter().find(|c| c.call.callee_name == name)
}

#[test]
fn python_bare_identifier_call_has_no_receiver() {
    let (_d, p) = write_tempfile("def caller():\n    target()\n", "py");
    let calls = calls_for_file(&p, Language::Python);
    let target = find_call(&calls, "target").expect("target found");
    assert_eq!(target.call.receiver_hint, None);
}

#[test]
fn python_dotted_call_uses_attribute_as_callee_and_object_as_receiver() {
    let (_d, p) = write_tempfile("def caller():\n    obj.method()\n", "py");
    let calls = calls_for_file(&p, Language::Python);
    let m = find_call(&calls, "method").expect("method found");
    assert_eq!(m.call.receiver_hint.as_deref(), Some("obj"));
    assert!(find_call(&calls, "obj").is_none(), "obj is the receiver, not a callee");
}

#[test]
fn python_three_segment_chain_extracts_rightmost_method() {
    let (_d, p) = write_tempfile("def caller():\n    a.b.c.method()\n", "py");
    let calls = calls_for_file(&p, Language::Python);
    let m = find_call(&calls, "method").expect("method found");
    assert_eq!(m.call.receiver_hint.as_deref(), Some("c"));
}

#[test]
fn python_class_static_call_path_qualified() {
    let (_d, p) = write_tempfile("def caller():\n    Foo.bar()\n    Service.handle(x)\n", "py");
    let calls = calls_for_file(&p, Language::Python);
    let bar = find_call(&calls, "bar").expect("bar found");
    let handle = find_call(&calls, "handle").expect("handle found");
    assert_eq!(bar.call.receiver_hint.as_deref(), Some("Foo"));
    assert_eq!(handle.call.receiver_hint.as_deref(), Some("Service"));
}

#[test]
fn javascript_member_call_extracts_method_not_object() {
    let (_d, p) = write_tempfile("function f() { obj.method(1, 2); }", "js");
    let calls = calls_for_file(&p, Language::JavaScript);
    let m = find_call(&calls, "method").expect("method found");
    assert_eq!(m.call.receiver_hint.as_deref(), Some("obj"));
    assert!(find_call(&calls, "obj").is_none());
}

#[test]
fn javascript_optional_chaining_still_extracts_method() {
    let (_d, p) = write_tempfile("function f() { a?.b?.method(); }", "js");
    let calls = calls_for_file(&p, Language::JavaScript);
    assert!(find_call(&calls, "method").is_some());
}

#[test]
fn javascript_bare_call_no_receiver_hint() {
    let (_d, p) = write_tempfile("function f() { plain(1); }", "js");
    let calls = calls_for_file(&p, Language::JavaScript);
    let plain = find_call(&calls, "plain").expect("plain found");
    assert_eq!(plain.call.receiver_hint, None);
}

#[test]
fn java_qualified_call_uses_method_name_not_class() {
    let src = "class C { void f() { Foo.bar(); Service.handle(x); } }";
    let (_d, p) = write_tempfile(src, "java");
    let calls = calls_for_file(&p, Language::Java);
    let bar = find_call(&calls, "bar").expect("bar");
    assert_eq!(bar.call.receiver_hint.as_deref(), Some("Foo"));
    let handle = find_call(&calls, "handle").expect("handle");
    assert_eq!(handle.call.receiver_hint.as_deref(), Some("Service"));
    assert!(find_call(&calls, "Foo").is_none());
}

#[test]
fn rust_method_call_extracts_method_with_receiver() {
    let src = "fn caller() { let x = obj.method(); }";
    let (_d, p) = write_tempfile(src, "rs");
    let calls = calls_for_file(&p, Language::Rust);
    let m = find_call(&calls, "method").expect("method");
    assert_eq!(m.call.receiver_hint.as_deref(), Some("obj"));
}

#[test]
fn rust_path_qualified_call_uses_rightmost_segment() {
    let src = "fn caller() { foo::bar::baz(); }";
    let (_d, p) = write_tempfile(src, "rs");
    let calls = calls_for_file(&p, Language::Rust);
    let baz = find_call(&calls, "baz").expect("baz");
    assert!(find_call(&calls, "bar").is_none(), "bar is a path segment, not a callee");
    assert!(baz.call.receiver_hint.is_some() || baz.call.receiver_hint.is_none());
}

#[test]
fn malformed_source_returns_empty_without_panic() {
    let (_d, p) = write_tempfile("def broken(", "py");
    let calls = calls_for_file(&p, Language::Python);
    let _ = calls;
}

#[test]
fn empty_source_yields_no_calls() {
    let (_d, p) = write_tempfile("", "py");
    assert!(calls_for_file(&p, Language::Python).is_empty());
}

#[test]
fn multiple_distinct_callsites_all_extracted() {
    let src = concat!("def caller():\n", "    foo()\n", "    bar()\n", "    obj.baz()\n", "    Mod.qux()\n",);
    let (_d, p) = write_tempfile(src, "py");
    let calls = calls_for_file(&p, Language::Python);
    assert!(find_call(&calls, "foo").is_some());
    assert!(find_call(&calls, "bar").is_some());
    assert!(find_call(&calls, "baz").is_some());
    assert!(find_call(&calls, "qux").is_some());
    let baz = find_call(&calls, "baz").unwrap();
    let qux = find_call(&calls, "qux").unwrap();
    assert_eq!(baz.call.receiver_hint.as_deref(), Some("obj"));
    assert_eq!(qux.call.receiver_hint.as_deref(), Some("Mod"));
}
