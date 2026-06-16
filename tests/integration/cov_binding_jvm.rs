use pulse::audit::call_walker::calls_and_bindings_from;
use pulse::parse::Language;

use crate::binding_common::{class_binding, method_env, one_source};

fn java_type(content: &str, method: &str, var: &str) -> Option<String> {
    let (_d, corpus) = one_source(content, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    method_env(&out, method)?.get(var).cloned()
}

fn kotlin_type(content: &str, method: &str, var: &str) -> Option<String> {
    let (_d, corpus) = one_source(content, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    method_env(&out, method)?.get(var).cloned()
}

#[test]
fn java_generic_type_field_binds_to_template_head() {
    let src = "class C {\n  private List<Bar> items;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let c = class_binding(&out, "C").expect("class C");
    assert_eq!(
        c.fields.get("items").map(String::as_str),
        Some("List"),
        "generic_type field binds to the erased head, not the type argument"
    );
}

#[test]
fn java_nested_scoped_type_field_uses_last_segment() {
    let src = "class C {\n  private Outer.Inner widget;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let c = class_binding(&out, "C").expect("class C");
    assert_eq!(
        c.fields.get("widget").map(String::as_str),
        Some("Inner"),
        "scoped_type_identifier resolves to the trailing nested type name"
    );
}

#[test]
fn java_annotated_type_field_binds_to_underlying_type() {
    let src = "class C {\n  @NonNull Foo helper;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let c = class_binding(&out, "C").expect("class C");
    assert_eq!(
        c.fields.get("helper").map(String::as_str),
        Some("Foo"),
        "an annotated type field resolves through the annotation to its underlying type"
    );
}

#[test]
fn java_annotated_generic_argument_field_binds_to_head() {
    let src = "class C {\n  List<@NonNull Foo> items;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let c = class_binding(&out, "C").expect("class C");
    assert_eq!(
        c.fields.get("items").map(String::as_str),
        Some("List"),
        "an annotation inside a type argument does not change the field head binding"
    );
}

#[test]
fn java_var_local_is_not_bound() {
    let src = "class C {\n  void run() {\n    var x = make();\n  }\n}\n";
    assert!(java_type(src, "run", "x").is_none(), "var local has no nominal type to bind");
}

#[test]
fn java_primitive_locals_and_params_are_not_bound() {
    let src =
        "class C {\n  void run(int n, boolean flag) {\n    double d = compute();\n    long[] xs = collect();\n  }\n}\n";
    assert!(java_type(src, "run", "n").is_none(), "integral param not bound");
    assert!(java_type(src, "run", "flag").is_none(), "boolean param not bound");
    assert!(java_type(src, "run", "d").is_none(), "floating_point local not bound");
    assert!(java_type(src, "run", "xs").is_none(), "array_type local not bound");
}

#[test]
fn java_varargs_parameter_binds_to_element_type() {
    let src = "class C {\n  void run(Foo first, Bar... rest) {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("first").map(String::as_str), Some("Foo"));
    assert_eq!(run.get("rest").map(String::as_str), Some("Bar"), "spread_parameter binds the declared element type");
}

#[test]
fn java_enhanced_for_binds_loop_variable() {
    let src = "class C {\n  void run(java.util.List xs) {\n    for (Item it : xs) {\n      it.use();\n    }\n  }\n}\n";
    assert_eq!(java_type(src, "run", "it").as_deref(), Some("Item"), "enhanced_for_statement binds its loop variable");
}

#[test]
fn java_catch_parameter_binds_to_exception_type() {
    let src = "class C {\n  void run() {\n    try {\n      go();\n    } catch (MyException e) {\n      e.report();\n    }\n  }\n}\n";
    assert_eq!(
        java_type(src, "run", "e").as_deref(),
        Some("MyException"),
        "a single-type catch parameter binds the caught type"
    );
}

#[test]
fn java_multi_catch_binds_caught_variable() {
    let src = "class C {\n  void run() {\n    try {\n      go();\n    } catch (FooException | BarException e) {\n      handle();\n    }\n  }\n}\n";
    let bound = java_type(src, "run", "e");
    assert!(bound.is_some(), "a multi-catch variable still produces a binding: {bound:?}");
}

#[test]
fn java_method_type_parameter_is_dropped() {
    let src = "class C {\n  <T> void run(T value) {\n    value.use();\n  }\n}\n";
    assert!(java_type(src, "run", "value").is_none(), "a method-level generic type parameter must not bind to a class");
}

#[test]
fn java_superclass_only_is_extracted_as_parent() {
    let src = "class Widget extends Base {\n  void run() {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(w.parents.contains(&"Base".to_string()));
    assert_eq!(w.parents.len(), 1, "no super_interfaces present");
}

#[test]
fn java_interface_list_without_superclass_is_extracted() {
    let src = "class Widget implements Drawable, Serializable {\n  void run() {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(w.parents.contains(&"Drawable".to_string()));
    assert!(w.parents.contains(&"Serializable".to_string()));
    assert!(!w.parents.iter().any(|p| p == "Base"), "no superclass present");
}

#[test]
fn java_interface_with_no_implemented_types_has_no_parents() {
    let src = "interface Marker {\n  void run();\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    if let Some(marker) = class_binding(&out, "Marker") {
        assert!(marker.parents.is_empty(), "a bodyless-parent interface yields no parents");
    }
}

#[test]
fn java_field_with_qualified_type_uses_last_segment() {
    let src = "class C {\n  private pkg.sub.Service svc;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let c = class_binding(&out, "C").expect("class C");
    assert_eq!(
        c.fields.get("svc").map(String::as_str),
        Some("Service"),
        "a fully qualified field type resolves to the unqualified head"
    );
}

#[test]
fn java_lambda_body_does_not_leak_local_into_method_env() {
    let src = "class C {\n  void run() {\n    Runnable r = () -> {\n      Foo inner = make();\n      inner.use();\n    };\n  }\n}\n";
    assert!(
        java_type(src, "run", "inner").is_none(),
        "a lambda body is a scope boundary; its locals do not enter the enclosing method env"
    );
}

#[test]
fn kotlin_constructor_property_binds_non_primitive_only() {
    let src = "class Widget(val helper: Bar, val size: Int, name: Baz) {\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"), "val ctor property binds");
    assert!(w.fields.get("size").is_none(), "Int ctor property not bound");
    assert!(w.fields.get("name").is_none(), "a plain (non-val/var) ctor param is not a property field");
}

#[test]
fn kotlin_var_constructor_property_binds() {
    let src = "class Widget(var helper: Bar) {\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"), "var ctor property binds");
}

#[test]
fn kotlin_vararg_constructor_property_is_not_bound() {
    let src = "class Widget(vararg val items: Item, val single: Item) {\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(w.fields.get("items").is_none(), "a vararg ctor property is not bound");
    assert_eq!(w.fields.get("single").map(String::as_str), Some("Item"), "a following non-vararg ctor property binds");
}

#[test]
fn kotlin_property_var_field_binds() {
    let src = "class Widget {\n  var helper: Bar = make()\n  val size: Int = 0\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"));
    assert!(w.fields.get("size").is_none(), "Int property field not bound");
}

#[test]
fn kotlin_nullable_local_binds_to_inner_type() {
    let src = "class C {\n  fun run() {\n    val q: Baz? = lookup()\n  }\n}\n";
    assert_eq!(kotlin_type(src, "run", "q").as_deref(), Some("Baz"), "nullable_type unwraps to its inner user type");
}

#[test]
fn kotlin_function_type_local_is_not_bound() {
    let src = "class C {\n  fun run() {\n    val cb: (Int) -> Unit = handler()\n  }\n}\n";
    assert!(kotlin_type(src, "run", "cb").is_none(), "a function_type local is not a class receiver");
}

#[test]
fn kotlin_primitive_local_is_not_bound() {
    let src = "class C {\n  fun run() {\n    val s: String = name()\n  }\n}\n";
    assert!(kotlin_type(src, "run", "s").is_none(), "a built-in primitive type local is not bound");
}

#[test]
fn kotlin_for_loop_variable_binds() {
    let src = "class C {\n  fun run(xs: Items) {\n    for (it: Item in xs) {\n      it.use()\n    }\n  }\n}\n";
    assert_eq!(kotlin_type(src, "run", "it").as_deref(), Some("Item"), "a typed for-loop variable_declaration binds");
}

#[test]
fn kotlin_companion_object_is_a_scope_boundary() {
    let src = "class C {\n  fun run() {\n    val outer: Foo = make()\n    companion object {\n      val inner: Bar = build()\n    }\n  }\n}\n";
    assert_eq!(kotlin_type(src, "run", "outer").as_deref(), Some("Foo"));
    assert!(kotlin_type(src, "run", "inner").is_none(), "a companion_object body is a scope boundary");
}

#[test]
fn kotlin_nested_object_declaration_is_a_scope_boundary() {
    let src = "class C {\n  fun run() {\n    val outer: Foo = make()\n    object Helper {\n      val inner: Bar = build()\n    }\n  }\n}\n";
    assert_eq!(kotlin_type(src, "run", "outer").as_deref(), Some("Foo"));
    assert!(kotlin_type(src, "run", "inner").is_none(), "an object_declaration body is a scope boundary");
}

#[test]
fn kotlin_class_with_no_delegation_specifiers_has_no_parents() {
    let src = "class Solo {\n  fun run() {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let solo = class_binding(&out, "Solo").expect("solo");
    assert!(solo.parents.is_empty(), "no delegation_specifiers yields no parents");
}

#[test]
fn kotlin_constructor_invocation_parent_is_extracted() {
    let src = "class Widget : Base(), Drawable {\n  fun run() {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(w.parents.contains(&"Base".to_string()), "a constructor_invocation delegation resolves to its type head");
    assert!(w.parents.contains(&"Drawable".to_string()), "a bare user_type delegation is a parent");
}

#[test]
fn kotlin_explicit_delegation_parent_is_extracted() {
    let src = "class Widget(d: Drawable) : Drawable by d {\n  fun run() {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(
        w.parents.contains(&"Drawable".to_string()),
        "an explicit_delegation (by) specifier resolves to the delegated interface"
    );
}

#[test]
fn kotlin_method_type_parameter_is_dropped() {
    let src = "class C {\n  fun <T> run(value: T) {\n    value.use()\n  }\n}\n";
    assert!(
        kotlin_type(src, "run", "value").is_none(),
        "a method-level generic type parameter must not bind to a class"
    );
}

#[test]
fn kotlin_vararg_function_parameter_followed_by_normal_binds_normal() {
    let src = "class C {\n  fun run(vararg items: Item, dep: Service) {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert!(run.get("items").is_none(), "vararg parameter element type not bound");
    assert_eq!(run.get("dep").map(String::as_str), Some("Service"), "a trailing non-vararg parameter binds");
}

#[test]
fn kotlin_qualified_local_uses_last_segment() {
    let src = "class C {\n  fun run() {\n    val svc: pkg.sub.Service = locate()\n  }\n}\n";
    assert_eq!(
        kotlin_type(src, "run", "svc").as_deref(),
        Some("Service"),
        "a fully qualified type resolves to the unqualified head"
    );
}

#[test]
fn kotlin_generic_local_binds_to_head() {
    let src = "class C {\n  fun run() {\n    val items: Holder<Bar> = build()\n  }\n}\n";
    assert_eq!(
        kotlin_type(src, "run", "items").as_deref(),
        Some("Holder"),
        "a generic user_type binds to its erased head"
    );
}
