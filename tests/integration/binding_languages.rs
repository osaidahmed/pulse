use pulse::audit::call_walker::calls_and_bindings_from;
use pulse::parse::Language;

use crate::binding_common::{class_binding, method_env, one_source};

fn env_type(content: &str, ext: &str, lang: Language, method: &str, var: &str) -> Option<String> {
    let (_d, corpus) = one_source(content, ext, lang);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    method_env(&out, method)?.get(var).cloned()
}

#[test]
fn kotlin_binds_params_locals_fields_and_parents() {
    let src = "class Widget : Base(), Drawable {\n  val helper: Bar = makeBar()\n  var count: Int = 0\n  fun run(env: Env, n: Int) {\n    val b: Bar = env.bar()\n    val inf = env.derive()\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("env").map(String::as_str), Some("Env"));
    assert_eq!(run.get("b").map(String::as_str), Some("Bar"));
    assert!(run.get("n").is_none(), "Int primitive not bound");
    assert!(run.get("inf").is_none(), "inferred local not bound");
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"));
    assert!(w.fields.get("count").is_none(), "Int field not bound");
    assert!(w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()));
}

#[test]
fn kotlin_drops_class_type_parameter() {
    let (_d, corpus) =
        one_source("class Box<T>(val item: T) { val real: Concrete = c() }\n", "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let b = class_binding(&out, "Box").expect("box");
    assert!(b.fields.get("item").is_none(), "T type-parameter field not bound");
    assert_eq!(b.fields.get("real").map(String::as_str), Some("Concrete"));
}

#[test]
fn typescript_binds_params_locals_fields_and_parents() {
    let src = "class Widget extends Base implements Drawable {\n  helper: Helper;\n  count: number;\n  run(b: Bar, c: Baz): void {\n    const f: Frobber = make();\n    let g = make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.ts", Language::TypeScript);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("b").map(String::as_str), Some("Bar"));
    assert_eq!(run.get("f").map(String::as_str), Some("Frobber"));
    assert!(run.get("g").is_none(), "inferred const not bound");
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Helper"));
    assert!(w.fields.get("count").is_none(), "number primitive field not bound");
    assert!(w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()));
}

#[test]
fn typescript_conflicting_local_is_poisoned() {
    let src = "class A {\n  m(): void {\n    if (cond) { const x: Foo = a(); } else { const x: Bar = b(); }\n  }\n}\n";
    assert!(env_type(src, "Sample.ts", Language::TypeScript, "m", "x").is_none());
}

#[test]
fn swift_binds_params_locals_fields_and_parents() {
    let src = "class Widget: Base, Drawable {\n  let helper: Bar = b()\n  func run(env: Bar, count: Int) {\n    let b: Bar = create()\n    var c = compute()\n    let q: Baz? = lookup()\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("env").map(String::as_str), Some("Bar"));
    assert_eq!(run.get("b").map(String::as_str), Some("Bar"));
    assert_eq!(run.get("q").map(String::as_str), Some("Baz"), "optional unwrapped");
    assert!(run.get("c").is_none(), "inferred local not bound");
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"));
    assert!(w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()));
}

#[test]
fn csharp_binds_params_locals_fields_and_parents() {
    let src = "class Widget : Base, Drawable {\n  private Foo helper;\n  private int count;\n  public void Run(Foo a, Bar b, int n) {\n    Baz local = Make();\n    var inferred = Make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.cs", Language::CSharp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("run env");
    assert_eq!(run.get("a").map(String::as_str), Some("Foo"));
    assert_eq!(run.get("local").map(String::as_str), Some("Baz"));
    assert!(run.get("n").is_none(), "int primitive not bound");
    assert!(run.get("inferred").is_none(), "var not bound");
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Foo"));
    assert!(w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()));
}

#[test]
fn rust_binds_typed_params_and_explicit_locals() {
    let src = "struct Repo { helper: Bar }\nimpl Repo {\n  fn run(&self, dep: Service) {\n    let b: Worker = make();\n    let inferred = make();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "sample.rs", Language::Rust);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("dep").map(String::as_str), Some("Service"));
    assert_eq!(run.get("b").map(String::as_str), Some("Worker"));
    assert!(run.get("inferred").is_none(), "inferred let not bound");
}

#[test]
fn swift_binds_parameter_name_not_argument_label() {
    let src = "class C {\n  func f(_ dest: Bar, to other: Baz) {\n    dest.use()\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = method_env(&out, "f").expect("f env");
    assert_eq!(f.get("dest").map(String::as_str), Some("Bar"), "binds the local name, not the label");
    assert_eq!(f.get("other").map(String::as_str), Some("Baz"));
    assert!(f.get("_").is_none() && f.get("to").is_none(), "argument labels are not bound");
}

#[test]
fn swift_variadic_parameter_is_not_bound() {
    assert!(
        env_type("class C { func f(values: Bar...) {} }\n", "Sample.swift", Language::Swift, "f", "values").is_none()
    );
}

#[test]
fn rust_impl_type_parameter_is_dropped() {
    let src = "struct Wrapper;\nimpl<Item> Wrapper {\n  fn get(&self) {\n    let x: Item = make();\n  }\n}\n";
    assert!(
        env_type(src, "sample.rs", Language::Rust, "get", "x").is_none(),
        "impl-level generic not bound to a class"
    );
}

#[test]
fn kotlin_vararg_parameter_is_not_bound() {
    let src = "class C { fun m(vararg items: Item, single: Item) {} }\n";
    let (_d, corpus) = one_source(src, "Sample.kt", Language::Kotlin);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let m = method_env(&out, "m").expect("m env");
    assert!(m.get("items").is_none(), "vararg element type not bound");
    assert_eq!(m.get("single").map(String::as_str), Some("Item"), "the following non-vararg param still binds");
}

#[test]
fn csharp_dynamic_local_is_not_bound() {
    assert!(env_type("class C { void M() { dynamic x = G(); } }\n", "Sample.cs", Language::CSharp, "M", "x").is_none());
}

#[test]
fn d_binds_params_locals_fields_and_parents() {
    let src = "class Widget : Base, Drawable {\n  Foo helper;\n  void run(Bar b) {\n    Baz local;\n  }\n}\n";
    let (_d, corpus) = one_source(src, "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert_eq!(run.get("b").map(String::as_str), Some("Bar"));
    assert_eq!(run.get("local").map(String::as_str), Some("Baz"));
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Foo"));
    assert!(w.parents.contains(&"Base".to_string()) && w.parents.contains(&"Drawable".to_string()));
}
