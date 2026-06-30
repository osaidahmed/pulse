use pulse_audit::call_walker::calls_and_bindings_from;
use pulse_syntax::parse::Language;

use crate::binding_common::{class_binding, method_env, one_source};

fn env_type(content: &str, ext: &str, lang: Language, method: &str, var: &str) -> Option<String> {
    let (_d, corpus) = one_source(content, ext, lang);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    method_env(&out, method)?.get(var).cloned()
}

#[test]
fn java_binds_params_locals_fields_and_parents() {
    let src = "class Widget extends Base implements Drawable {\n  private Foo helper;\n  private int count;\n  public void run(Foo a, int n) {\n    Baz local = make();\n    var inferred = make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("a").map(String::as_str), Some("Foo"), "object param binds");
    assert_eq!(run.get("local").map(String::as_str), Some("Baz"), "typed local binds");
    assert!(run.get("n").is_none(), "int primitive param not bound");
    assert!(run.get("inferred").is_none(), "var local not bound");
    let w = class_binding(&out, "Widget").expect("widget class");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Foo"), "object field binds");
    assert!(!w.fields.contains_key("count"), "int primitive field not bound");
    assert!(
        w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()),
        "superclass and interface are both parents"
    );
}

#[test]
fn java_no_param_method_yields_only_field_scope() {
    let src = "class C {\n  public void run() {\n    Foo x = make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("x").map(String::as_str), Some("Foo"), "a method without formal_parameters still binds locals");
}

#[test]
fn java_enhanced_for_binds_element_type() {
    let src = "class C {\n  public void run(java.util.List<Foo> items) {\n    for (Bar b : items) {\n      b.use();\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("b").map(String::as_str), Some("Bar"), "enhanced-for loop variable binds to its element type");
}

#[test]
fn java_catch_clause_binds_exception_variable() {
    let src = "class C {\n  public void run() {\n    try {\n      go();\n    } catch (MyError ex) {\n      ex.handle();\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("ex").map(String::as_str), Some("MyError"), "catch parameter binds to its declared type");
}

#[test]
fn java_local_inside_lambda_is_scope_bounded() {
    let src = "class C {\n  public void run() {\n    Runnable r = () -> {\n      Foo inside = make();\n      inside.use();\n    };\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert!(run.get("inside").is_none(), "a local declared inside a lambda body is not pulled into the outer method");
}

#[test]
fn java_annotated_type_field_binds_to_underlying_name() {
    let src = "class Widget {\n  @Nullable Foo helper;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget class");
    assert_eq!(
        w.fields.get("helper").map(String::as_str),
        Some("Foo"),
        "an annotated_type field unwraps the annotation and binds the underlying type"
    );
}

#[test]
fn java_constructor_locals_bind_via_constructor_body() {
    let src = "class C {\n  C(Foo a) {\n    Bar local = make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let ctor = method_env(&out, "C").expect("constructor env");
    assert_eq!(ctor.get("a").map(String::as_str), Some("Foo"), "constructor param binds");
    assert_eq!(ctor.get("local").map(String::as_str), Some("Bar"), "constructor body local binds");
}

#[test]
fn java_class_type_parameter_field_is_dropped() {
    let src = "class Box<T> {\n  T item;\n  Concrete real;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.java", Language::Java);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let b = class_binding(&out, "Box").expect("box class");
    assert!(!b.fields.contains_key("item"), "type-parameter-typed field is not bound to a class");
    assert_eq!(b.fields.get("real").map(String::as_str), Some("Concrete"), "concrete field still binds");
}

#[test]
fn swift_binds_params_locals_fields_and_parents() {
    let src = "class Widget: Base, Drawable {\n  let helper: Bar = b()\n  let count: Int = 0\n  func run(env: Bar, count: Int) {\n    let v: Baz = create()\n    var c = compute()\n    let q: Quux? = lookup()\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("env").map(String::as_str), Some("Bar"), "object param binds");
    assert_eq!(run.get("v").map(String::as_str), Some("Baz"), "typed local binds");
    assert_eq!(run.get("q").map(String::as_str), Some("Quux"), "optional local unwraps");
    assert!(run.get("c").is_none(), "inferred local not bound");
    let w = class_binding(&out, "Widget").expect("widget class");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"), "object field binds");
    assert!(!w.fields.contains_key("count"), "Int field not bound");
    assert!(
        w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()),
        "inheritance specifiers become parents"
    );
}

#[test]
fn swift_for_in_loop_item_binds_with_annotation() {
    let src = "class C {\n  func run(xs: Seq) {\n    for item: Bar in xs {\n      item.use()\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("item").map(String::as_str), Some("Bar"), "an annotated for-in item binds to its declared type");
}

#[test]
fn swift_class_type_parameter_field_is_dropped() {
    let src = "class Box<T> {\n  let item: T = m()\n  let real: Concrete = c()\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let b = class_binding(&out, "Box").expect("box class");
    assert!(!b.fields.contains_key("item"), "type-parameter-typed field is not bound");
    assert_eq!(b.fields.get("real").map(String::as_str), Some("Concrete"), "concrete field still binds");
}

#[test]
fn swift_array_type_property_is_not_bound() {
    let src = "class C {\n  let xs: [Bar] = []\n  let one: Baz = make()\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let c = class_binding(&out, "C").expect("class C");
    assert!(!c.fields.contains_key("xs"), "an array_type property is not bound to its element type");
    assert_eq!(c.fields.get("one").map(String::as_str), Some("Baz"), "a plain user_type property still binds");
}

#[test]
fn swift_local_inside_nested_function_is_scope_bounded() {
    let src = "class C {\n  func run() {\n    func inner() {\n      let inside: Foo = make()\n      inside.use()\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert!(
        run.get("inside").is_none(),
        "a local in a nested function declaration does not leak into the outer method"
    );
}

#[test]
fn swift_unannotated_property_yields_no_binding() {
    assert!(
        env_type(
            "class C { func run() { let inferred = make() } }\n",
            "Sample.swift",
            Language::Swift,
            "run",
            "inferred"
        )
        .is_none(),
        "a property with no type annotation is not bound"
    );
}

#[test]
fn csharp_binds_params_locals_fields_and_parents() {
    let src = "class Widget : Base, Drawable {\n  private Foo helper;\n  private int count;\n  public void Run(Foo a, Bar b, int n) {\n    Baz local = Make();\n    var inferred = Make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("run env");
    assert_eq!(run.get("a").map(String::as_str), Some("Foo"), "object param binds");
    assert_eq!(run.get("local").map(String::as_str), Some("Baz"), "typed local binds");
    assert!(run.get("n").is_none(), "int primitive param not bound");
    assert!(run.get("inferred").is_none(), "var local not bound");
    let w = class_binding(&out, "Widget").expect("widget class");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Foo"), "object field binds");
    assert!(!w.fields.contains_key("count"), "int primitive field not bound");
    assert!(
        w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()),
        "base list entries become parents"
    );
}

#[test]
fn csharp_property_member_binds_field() {
    let src = "class Widget {\n  public Foo Helper { get; set; }\n  public int Count { get; set; }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget class");
    assert_eq!(
        w.fields.get("Helper").map(String::as_str),
        Some("Foo"),
        "an object property declaration binds as a field"
    );
    assert!(!w.fields.contains_key("Count"), "an int property is not bound");
}

#[test]
fn csharp_foreach_binds_loop_variable() {
    let src =
        "class C {\n  public void Run(Seq xs) {\n    foreach (Bar item in xs) {\n      item.Use();\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("run env");
    assert_eq!(
        run.get("item").map(String::as_str),
        Some("Bar"),
        "a foreach loop variable binds to its explicitly declared type"
    );
}

#[test]
fn csharp_using_statement_binds_declared_resource() {
    let src =
        "class C {\n  public void Run() {\n    using (Resource res = Open()) {\n      res.Use();\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("run env");
    assert_eq!(
        run.get("res").map(String::as_str),
        Some("Resource"),
        "a using-statement resource declaration binds its variable"
    );
}

#[test]
fn csharp_nullable_field_peels_to_underlying_type() {
    let src = "class Widget {\n  private Foo? helper;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget class");
    assert_eq!(
        w.fields.get("helper").map(String::as_str),
        Some("Foo"),
        "a nullable_type field peels to its underlying named type"
    );
}

#[test]
fn csharp_primary_constructor_base_type_is_a_parent() {
    let src = "class Widget(int x) : Base(x) {\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget class");
    assert!(
        w.parents.contains(&"Base".to_string()),
        "a primary_constructor_base_type in the base list resolves to the base type name"
    );
}

#[test]
fn csharp_array_local_is_not_bound() {
    assert!(
        env_type("class C { void M() { Bar[] xs = G(); } }\n", "Sample.cs", Language::CSharp, "M", "xs").is_none(),
        "an array_type local is not bound to its element type"
    );
}

#[test]
fn csharp_class_type_parameter_field_is_dropped() {
    let src = "class Box<T> {\n  private T item;\n  private Concrete real;\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let b = class_binding(&out, "Box").expect("box class");
    assert!(!b.fields.contains_key("item"), "type-parameter-typed field is not bound");
    assert_eq!(b.fields.get("real").map(String::as_str), Some("Concrete"), "concrete field still binds");
}
