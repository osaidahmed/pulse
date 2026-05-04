use std::path::PathBuf;

use pulse::audit::call_walker::{calls_for_file, LocatedCall};
use pulse::parse::Language;

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

fn callee_names(calls: &[LocatedCall]) -> Vec<String> {
    let mut out: Vec<String> = calls.iter().map(|c| c.call.callee_name.clone()).collect();
    out.sort();
    out
}

#[test]
fn python_bare_function_call_extracted() {
    let src = "def caller():\n    foo(1)\n    bar(2)\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let names = callee_names(&calls);
    assert!(names.contains(&"foo".to_string()));
    assert!(names.contains(&"bar".to_string()));
}

#[test]
fn python_method_call_with_self_receiver() {
    let src = "class Foo:\n    def caller(self):\n        self.helper()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let helper = calls.iter().find(|c| c.call.callee_name == "helper");
    assert!(helper.is_some());
    assert_eq!(helper.unwrap().call.receiver_hint.as_deref(), Some("self"));
}

#[test]
fn python_method_call_with_object_receiver() {
    let src = "def caller():\n    obj.method(1)\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let m = calls.iter().find(|c| c.call.callee_name == "method");
    assert!(m.is_some());
    assert_eq!(m.unwrap().call.receiver_hint.as_deref(), Some("obj"));
}

#[test]
fn python_caller_method_populated_for_call_in_method() {
    let src = "class Foo:\n    def my_method(self):\n        self.helper()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let helper = calls.iter().find(|c| c.call.callee_name == "helper").unwrap();
    let caller = helper.caller.as_ref().unwrap();
    assert_eq!(caller.name, "my_method");
    assert_eq!(caller.class.as_deref(), Some("Foo"));
}

#[test]
fn python_caller_at_module_level_has_no_class() {
    let src = "def caller():\n    target()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let t = calls.iter().find(|c| c.call.callee_name == "target").unwrap();
    let caller = t.caller.as_ref().unwrap();
    assert_eq!(caller.name, "caller");
    assert_eq!(caller.class, None);
}

#[test]
fn python_method_chain_uses_rightmost_segment_as_receiver() {
    let src = "def f():\n    a.b.c.method()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let m = calls.iter().find(|c| c.call.callee_name == "method").unwrap();
    assert_eq!(m.call.receiver_hint.as_deref(), Some("c"));
}

#[test]
fn python_no_calls_in_pure_data_module() {
    let src = "x = 1\ny = 2\nz = x + y\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    assert_eq!(calls.len(), 0);
}

#[test]
fn python_call_with_keyword_args_still_extracted() {
    let src = "def f():\n    func(x=1, y=2)\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    assert!(calls.iter().any(|c| c.call.callee_name == "func"));
}

#[test]
fn python_call_inside_branch_extracted() {
    let src = "def f(x):\n    if x:\n        target()\n    else:\n        other()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let names = callee_names(&calls);
    assert!(names.contains(&"target".to_string()));
    assert!(names.contains(&"other".to_string()));
}

#[test]
fn python_line_number_preserved() {
    let src = "def f():\n    a()\n    b()\n    c()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let a = calls.iter().find(|c| c.call.callee_name == "a").unwrap();
    assert_eq!(a.call.line, 2);
    let c = calls.iter().find(|c| c.call.callee_name == "c").unwrap();
    assert_eq!(c.call.line, 4);
}

#[test]
fn rust_qualified_path_call_extracted() {
    let src = "fn main() { Foo::bar(); }\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let calls = calls_for_file(&path, Language::Rust);
    let bar = calls.iter().find(|c| c.call.callee_name == "bar");
    assert!(bar.is_some());
    assert_eq!(bar.unwrap().call.receiver_hint.as_deref(), Some("Foo"));
}

#[test]
fn rust_method_call_with_self() {
    let src = "struct Foo;\nimpl Foo { fn caller(&self) { self.helper(); } fn helper(&self) {} }\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let calls = calls_for_file(&path, Language::Rust);
    let helper = calls.iter().find(|c| c.call.callee_name == "helper");
    assert!(helper.is_some());
}

#[test]
fn rust_caller_method_within_impl_carries_struct_name() {
    let src = "struct Foo;\nimpl Foo {\n    fn method(&self) {\n        target();\n    }\n}\nfn target() {}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let calls = calls_for_file(&path, Language::Rust);
    let t = calls.iter().find(|c| c.call.callee_name == "target");
    assert!(t.is_some());
    let caller = t.unwrap().caller.as_ref().unwrap();
    assert_eq!(caller.name, "method");
    assert_eq!(caller.class.as_deref(), Some("Foo"));
}

#[test]
fn java_method_invocation_extracted() {
    let src = "class Foo { void caller() { obj.method(); } }\n";
    let (_dir, path) = write_tempfile(src, "java");
    let calls = calls_for_file(&path, Language::Java);
    let m = calls.iter().find(|c| c.call.callee_name == "method");
    assert!(m.is_some());
}

#[test]
fn javascript_function_call_extracted() {
    let src = "function caller() { foo(); bar(); }\n";
    let (_dir, path) = write_tempfile(src, "js");
    let calls = calls_for_file(&path, Language::JavaScript);
    let names = callee_names(&calls);
    assert!(names.contains(&"foo".to_string()));
}

#[test]
fn javascript_method_call_extracted() {
    let src = "function f() { obj.method(1); }\n";
    let (_dir, path) = write_tempfile(src, "js");
    let calls = calls_for_file(&path, Language::JavaScript);
    let m = calls.iter().find(|c| c.call.callee_name == "method");
    assert!(m.is_some());
}

#[test]
fn typescript_method_call_extracted() {
    let src = "function f() { obj.method(1); }\n";
    let (_dir, path) = write_tempfile(src, "ts");
    let calls = calls_for_file(&path, Language::TypeScript);
    let m = calls.iter().find(|c| c.call.callee_name == "method");
    assert!(m.is_some());
}

#[test]
fn go_function_call_extracted() {
    let src = "package main\nfunc main() { Foo() }\nfunc Foo() {}\n";
    let (_dir, path) = write_tempfile(src, "go");
    let calls = calls_for_file(&path, Language::Go);
    let f = calls.iter().find(|c| c.call.callee_name == "Foo");
    assert!(f.is_some());
}

#[test]
fn go_dotted_call_extracted() {
    let src = "package main\nfunc main() { fmt.Println(\"x\") }\n";
    let (_dir, path) = write_tempfile(src, "go");
    let calls = calls_for_file(&path, Language::Go);
    let p = calls.iter().find(|c| c.call.callee_name == "Println");
    assert!(p.is_some());
}

#[test]
fn missing_file_returns_empty_no_panic() {
    let p = PathBuf::from("/nonexistent/path.py");
    let calls = calls_for_file(&p, Language::Python);
    assert!(calls.is_empty());
}

#[test]
fn empty_file_returns_empty() {
    let (_dir, path) = write_tempfile("", "py");
    let calls = calls_for_file(&path, Language::Python);
    assert!(calls.is_empty());
}

#[test]
fn malformed_source_does_not_panic() {
    let src = "def f(:\n    foo(\n";
    let (_dir, path) = write_tempfile(src, "py");
    let _calls = calls_for_file(&path, Language::Python);
}

#[test]
fn determinism_two_runs_yield_same_results() {
    let src = "def f(): a(); b(); c()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let r1 = calls_for_file(&path, Language::Python);
    let r2 = calls_for_file(&path, Language::Python);
    let n1: Vec<_> = r1.iter().map(|c| &c.call.callee_name).collect();
    let n2: Vec<_> = r2.iter().map(|c| &c.call.callee_name).collect();
    assert_eq!(n1, n2);
}

#[test]
fn whitespace_changes_dont_change_callees() {
    let s1 = "def f():\n    foo()\n    bar()\n";
    let s2 = "def f():\n\n    foo()\n\n    bar()\n";
    let (_d1, p1) = write_tempfile(s1, "py");
    let (_d2, p2) = write_tempfile(s2, "py");
    let mut n1 = callee_names(&calls_for_file(&p1, Language::Python));
    let mut n2 = callee_names(&calls_for_file(&p2, Language::Python));
    n1.sort();
    n2.sort();
    assert_eq!(n1, n2);
}

#[test]
fn comment_changes_dont_change_callees() {
    let s1 = "def f():\n    foo()\n";
    let s2 = "def f():  # comment\n    foo()  # another\n";
    let (_d1, p1) = write_tempfile(s1, "py");
    let (_d2, p2) = write_tempfile(s2, "py");
    let n1 = callee_names(&calls_for_file(&p1, Language::Python));
    let n2 = callee_names(&calls_for_file(&p2, Language::Python));
    assert_eq!(n1, n2);
}

#[test]
fn nested_function_calls_each_extracted() {
    let src = "def f():\n    a(b(c()))\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let names = callee_names(&calls);
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"b".to_string()));
    assert!(names.contains(&"c".to_string()));
}

#[test]
fn module_level_call_has_no_caller_method() {
    let src = "def f(): pass\nf()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let module_call = calls.iter().find(|c| c.call.line == 2);
    assert!(module_call.is_some());
    assert!(module_call.unwrap().caller.is_none());
}

#[test]
fn multiple_callers_resolve_to_distinct_methods() {
    let src = "class A:\n    def m1(self):\n        target()\n    def m2(self):\n        target()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let calls = calls_for_file(&path, Language::Python);
    let target_calls: Vec<_> = calls.iter().filter(|c| c.call.callee_name == "target").collect();
    assert_eq!(target_calls.len(), 2);
    let names: Vec<_> = target_calls
        .iter()
        .filter_map(|c| c.caller.as_ref().map(|m| &m.name))
        .collect();
    assert!(names.contains(&&"m1".to_string()));
    assert!(names.contains(&&"m2".to_string()));
}
