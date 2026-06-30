use pulse_audit::call_walker::calls_and_bindings_from;
use pulse_syntax::parse::Language;

use crate::binding_common::{class_binding, method_env, one_source};

fn env_type(content: &str, ext: &str, lang: Language, method: &str, var: &str) -> Option<String> {
    let (_d, corpus) = one_source(content, ext, lang);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    method_env(&out, method)?.get(var).cloned()
}

#[test]
fn csharp_qualified_and_generic_field_types_use_head() {
    let src = "class Widget {\n  private System.Text.StringBuilder sb;\n  private List<Foo> items;\n  private Bar helper;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("sb").map(String::as_str), Some("StringBuilder"), "qualified_name binds to last segment");
    assert_eq!(w.fields.get("items").map(String::as_str), Some("List"), "generic_name binds to template head");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"));
}

#[test]
fn csharp_property_declaration_is_bound_as_field() {
    let src = "class Widget {\n  public Foo Prop { get; set; }\n  public int Count { get; set; }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("Prop").map(String::as_str), Some("Foo"), "property_declaration binds via bind_field");
    assert!(!w.fields.contains_key("Count"), "int property not bound");
}

#[test]
fn csharp_array_and_tuple_locals_are_not_bound() {
    let src = "class C {\n  void M() {\n    Foo[] arr = Make();\n    (int, int) pair = Make();\n    Baz one = Make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let m = method_env(&out, "M").expect("M env");
    assert!(m.get("arr").is_none(), "array_type local not bound");
    assert!(m.get("pair").is_none(), "tuple_type local not bound");
    assert_eq!(m.get("one").map(String::as_str), Some("Baz"), "a plain typed local still binds");
}

#[test]
fn csharp_nullable_local_is_unwrapped_to_inner_type() {
    assert_eq!(
        env_type("class C { void M() { Foo? x = Make(); } }\n", "Sample.cs", Language::CSharp, "M", "x"),
        Some("Foo".to_string()),
        "nullable_type peels to its inner named type"
    );
}

#[test]
fn csharp_foreach_typed_iterator_binds_implicit_does_not() {
    let src = "class C {\n  void M() {\n    foreach (Item it in xs) { it.Use(); }\n    foreach (var auto in ys) { auto.Use(); }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let m = method_env(&out, "M").expect("M env");
    assert_eq!(m.get("it").map(String::as_str), Some("Item"), "explicitly typed foreach iterator binds");
    assert!(m.get("auto").is_none(), "var foreach iterator (implicit_type) not bound");
}

#[test]
fn csharp_foreach_deconstruction_left_is_not_bound() {
    let src = "class C {\n  void M() {\n    foreach ((var a, var b) in pairs) { a.Use(); }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let m = method_env(&out, "M").expect("M env");
    assert!(m.get("a").is_none() && m.get("b").is_none(), "a non-identifier foreach left is skipped");
}

#[test]
fn csharp_using_statement_declaration_binds_local() {
    let src = "class C {\n  void M() {\n    using (Resource r = Acquire()) {\n      r.Use();\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let m = method_env(&out, "M").expect("M env");
    assert_eq!(m.get("r").map(String::as_str), Some("Resource"), "using-statement declaration binds its local");
}

#[test]
fn csharp_generic_method_type_parameter_local_is_dropped() {
    let src = "class C {\n  void M<T>(T item) {\n    T cached = Make();\n    Concrete real = Make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let m = method_env(&out, "M").expect("M env");
    assert!(m.get("item").is_none(), "method type-parameter param not bound to a class");
    assert!(m.get("cached").is_none(), "method type-parameter local not bound to a class");
    assert_eq!(m.get("real").map(String::as_str), Some("Concrete"), "a concrete local still binds");
}

#[test]
fn csharp_generic_class_type_parameter_field_is_dropped() {
    let src = "class Box<T> {\n  private T item;\n  private Concrete real;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let b = class_binding(&out, "Box").expect("box");
    assert!(!b.fields.contains_key("item"), "class type-parameter field not bound");
    assert_eq!(b.fields.get("real").map(String::as_str), Some("Concrete"));
}

#[test]
fn csharp_qualified_base_class_uses_last_segment() {
    let (_d, corpus) = one_source("class Widget : Pkg.Inner.Base {}\n", "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(w.parents.contains(&"Base".to_string()), "qualified base resolves to its type name");
}

#[test]
fn csharp_interface_declaration_has_no_parents_without_base_list() {
    let (_d, corpus) = one_source("interface IThing { void Do(); }\n", "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let i = class_binding(&out, "IThing").expect("interface as class binding");
    assert!(i.parents.is_empty(), "an interface with no base_list yields no parents");
}

#[test]
fn objc_primitive_local_and_id_param_are_not_bound() {
    let src = "@implementation Repo\n- (void)run:(id)thing {\n    int count;\n    Foo* x;\n}\n@end\n";
    let (_d, corpus) = one_source(src, "sample.m", Language::ObjectiveC);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env = out
        .method_envs
        .iter()
        .map(|(_, e)| e)
        .find(|e| e.get("x").map(String::as_str) == Some("Foo"))
        .expect("an objc method env binds the pointer local x->Foo");
    assert!(env.get("count").is_none(), "primitive_type local hits the type_head fallthrough and is not bound");
    assert!(env.get("thing").is_none(), "an id-typed parameter is not bound to a class");
}

#[test]
fn objc_multiple_typed_params_bind_each() {
    let src = "@implementation Repo\n- (void)connect:(Conn*)c with:(Session*)s {\n}\n@end\n";
    let (_d, corpus) = one_source(src, "sample.m", Language::ObjectiveC);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env =
        out.method_envs.iter().map(|(_, e)| e).find(|e| e.get("c").is_some()).expect("a method env binding params");
    assert_eq!(env.get("c").map(String::as_str), Some("Conn"));
    assert_eq!(env.get("s").map(String::as_str), Some("Session"));
}

#[test]
fn cpp_qualified_and_template_field_types_use_head() {
    let src = "class Widget {\n  ns::Foo helper;\n  Vector<int> items;\n  int count;\n};\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Foo"), "qualified_identifier binds to last segment");
    assert_eq!(w.fields.get("items").map(String::as_str), Some("Vector"), "template_type binds to its name head");
    assert!(!w.fields.contains_key("count"), "primitive field not bound");
}

#[test]
fn cpp_reference_param_declarator_is_walked_plain_param_still_binds() {
    let src = "class Repo {\n public:\n  void run(Bar& b, Baz one) {\n    one.use();\n  }\n};\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env = out
        .method_envs
        .iter()
        .map(|(_, e)| e)
        .find(|e| e.get("one").is_some())
        .expect("a method env binding the plain param");
    assert!(
        env.get("b").is_none(),
        "reference_declarator has no declarator field so the reference param does not bind"
    );
    assert_eq!(env.get("one").map(String::as_str), Some("Baz"), "a plain typed param still binds");
}

#[test]
fn cpp_pointer_returning_method_still_binds_params() {
    let src = "Foo* makeIt(Bar b, Baz c) {\n  b.use();\n  return 0;\n}\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env = out
        .method_envs
        .iter()
        .map(|(_, e)| e)
        .find(|e| e.get("b").is_some())
        .expect("a method env binding params reached through the pointer_declarator wrapper");
    assert_eq!(env.get("b").map(String::as_str), Some("Bar"), "params reachable through pointer_declarator wrapper");
    assert_eq!(env.get("c").map(String::as_str), Some("Baz"));
}

#[test]
fn cpp_qualified_and_template_base_classes_are_parents() {
    let src = "class Widget : public ns::Base, public Mixin<int> {\n};\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(w.parents.contains(&"Base".to_string()), "qualified base resolves to its type name");
    assert!(w.parents.contains(&"Mixin".to_string()), "template base resolves to its template head");
}

#[test]
fn cpp_class_without_base_clause_has_no_parents() {
    let (_d, corpus) = one_source("class Solo {\n  Foo helper;\n};\n", "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let s = class_binding(&out, "Solo").expect("solo");
    assert!(s.parents.is_empty(), "no base_class_clause yields no parents");
    assert_eq!(s.fields.get("helper").map(String::as_str), Some("Foo"));
}

#[test]
fn cpp_qualified_local_in_method_body_binds_to_head() {
    let src = "class Repo {\n public:\n  void run() {\n    ns::Worker w;\n    int n;\n    w.go();\n  }\n};\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env = out
        .method_envs
        .iter()
        .map(|(_, e)| e)
        .find(|e| e.get("w").is_some())
        .expect("a method env binding the qualified local");
    assert_eq!(env.get("w").map(String::as_str), Some("Worker"), "qualified local type binds to last segment");
    assert!(env.get("n").is_none(), "primitive local not bound");
}
