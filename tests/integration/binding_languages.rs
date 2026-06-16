use pulse::audit::call_graph::{CallGraph, MethodIndex};
use pulse::audit::call_walker::calls_and_bindings_from;
use pulse::parse::Language;

use crate::binding_common::{analyze, class_binding, method_env, one_source, targets_named};

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
fn typescript_nested_function_drops_class_type_parameter() {
    let src = "class Box<Item> {\n  m(): void {\n    function inner(x: Item) { x.run(); }\n  }\n}\n";
    assert!(
        env_type(src, "Sample.ts", Language::TypeScript, "inner", "x").is_none(),
        "a class type variable is dropped even in a nested function"
    );
}

#[test]
fn swift_multi_declarator_property_is_not_mis_bound() {
    let src = "class C {\n  func f() {\n    let a = makeFoo(), b: Bar\n  }\n}\n";
    assert!(
        env_type(src, "Sample.swift", Language::Swift, "f", "a").is_none(),
        "an inferred first declarator is not bound to a later declarator's annotation"
    );
}

#[test]
fn d_array_and_pointer_locals_are_not_bound() {
    let src = "class C { void f() { Bar[] xs; Foo* p; Baz one; } }\n";
    let (_d, corpus) = one_source(src, "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = method_env(&out, "f").expect("f env");
    assert!(f.get("xs").is_none(), "array local not bound to element type");
    assert!(f.get("p").is_none(), "pointer local not bound to pointee type");
    assert_eq!(f.get("one").map(String::as_str), Some("Baz"), "a plain typed local still binds");
}

#[test]
fn d_qualified_base_class_uses_last_segment() {
    let (_d, corpus) = one_source("class Widget : pkg.Base {}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert!(w.parents.contains(&"Base".to_string()), "qualified base resolves to the type name, not the package");
}

#[test]
fn go_binds_receiver_params_and_locals() {
    let src = "package p\ntype Repo struct {}\nfunc (r *Repo) Run(dep Service) {\n\tvar b Worker\n\tx := make()\n\t_ = x\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("Run env");
    assert_eq!(run.get("r").map(String::as_str), Some("Repo"), "receiver binds to its type");
    assert_eq!(run.get("dep").map(String::as_str), Some("Service"));
    assert_eq!(run.get("b").map(String::as_str), Some("Worker"));
    assert!(run.get("x").is_none(), "short-var-declaration (inferred) local is not bound");
}

#[test]
fn go_receiver_method_call_resolves_via_enrichment() {
    let src = "package p\ntype Repo struct {}\nfunc (r *Repo) helper() {}\nfunc (r *Repo) Run() {\n\tr.helper()\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let a = analyze(corpus.files.first().unwrap());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    let idx = bound
        .registry
        .methods
        .iter()
        .position(|m| m.class.as_deref() == Some("Repo") && m.name == "Run")
        .map(|i| MethodIndex(i as u32))
        .expect("Repo.Run definition");
    let targets: Vec<_> = bound
        .adjacency
        .outgoing(idx)
        .iter()
        .filter_map(|e| bound.registry.get(e.target))
        .filter(|m| m.name == "helper")
        .collect();
    assert_eq!(targets.len(), 1, "the (file,line) enrichment + receiver binding resolve r.helper() to Repo.helper");
    assert_eq!(targets[0].class.as_deref(), Some("Repo"));
}

#[test]
fn go_struct_named_fields_are_bound() {
    let src = "package p\ntype Server struct {\n\tconn Conn\n\tname string\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let server = class_binding(&out, "Server").expect("Server class");
    assert_eq!(server.fields.get("conn").map(String::as_str), Some("Conn"));
    assert!(server.fields.get("name").is_none(), "string primitive field not bound");
}

#[test]
fn go_struct_field_typed_call_resolves() {
    let src = "package p\ntype Conn struct {}\nfunc (c *Conn) Send() {}\ntype Server struct {\n\tconn Conn\n}\nfunc (s *Server) Run() {\n\ts.conn.Send()\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let a = analyze(corpus.files.first().unwrap());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    let idx = bound
        .registry
        .methods
        .iter()
        .position(|m| m.class.as_deref() == Some("Server") && m.name == "Run")
        .map(|i| MethodIndex(i as u32))
        .expect("Server.Run definition");
    let targets: Vec<_> = bound
        .adjacency
        .outgoing(idx)
        .iter()
        .filter_map(|e| bound.registry.get(e.target))
        .filter(|m| m.name == "Send")
        .collect();
    assert_eq!(targets.len(), 1, "s.conn.Send() resolves to Conn.Send via struct-field binding");
    assert_eq!(targets[0].class.as_deref(), Some("Conn"));
}

#[test]
fn go_struct_embedding_is_extracted_as_parents() {
    let src = "package p\ntype Base struct {}\ntype Server struct {\n\t*Base\n\tname string\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let server = class_binding(&out, "Server").expect("Server class");
    assert!(server.parents.contains(&"Base".to_string()), "embedded type is a parent");
    assert!(!server.parents.contains(&"name".to_string()), "named field is not an embedded parent");
}

#[test]
fn go_promoted_method_resolves_via_embedded_type() {
    let src = "package p\ntype Base struct {}\nfunc (b *Base) Helper() {}\ntype Server struct {\n\t*Base\n}\nfunc (s *Server) Run() {\n\ts.Helper()\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let a = analyze(corpus.files.first().unwrap());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    let idx = bound
        .registry
        .methods
        .iter()
        .position(|m| m.class.as_deref() == Some("Server") && m.name == "Run")
        .map(|i| MethodIndex(i as u32))
        .expect("Server.Run definition");
    let targets: Vec<_> = bound
        .adjacency
        .outgoing(idx)
        .iter()
        .filter_map(|e| bound.registry.get(e.target))
        .filter(|m| m.name == "Helper")
        .collect();
    assert_eq!(targets.len(), 1, "s.Helper() resolves to the embedded Base.Helper via the ancestor walk");
    assert_eq!(targets[0].class.as_deref(), Some("Base"));
}

#[test]
fn cpp_binds_class_members_and_parents() {
    let src = "class Widget : public Base {\n  Foo helper;\n  Bar* ptr;\n  int count;\n};\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Foo"));
    assert_eq!(w.fields.get("ptr").map(String::as_str), Some("Bar"), "pointer field binds to its pointee");
    assert!(w.fields.get("count").is_none(), "primitive field not bound");
    assert!(w.parents.contains(&"Base".to_string()));
}

#[test]
fn cpp_out_of_line_method_is_associated_with_its_class() {
    let src = "class Repo { public: void run(); };\nvoid Repo::run() {\n  Foo x;\n}\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let a = analyze(corpus.files.first().unwrap());
    let run = a.defs.iter().find(|d| d.identity.name == "run" && d.identity.class.as_deref() == Some("Repo"));
    assert!(run.is_some(), "out-of-line Repo::run is associated with class Repo");
}

#[test]
fn cpp_typed_receiver_resolves_via_binding_and_enrichment() {
    let src = "class Other { public: void foo() {} };\nclass Repo {\n public:\n  void run() {\n    Other o;\n    o.foo();\n  }\n};\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let a = analyze(corpus.files.first().unwrap());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    let idx = bound
        .registry
        .methods
        .iter()
        .position(|m| m.class.as_deref() == Some("Repo") && m.name == "run")
        .map(|i| MethodIndex(i as u32))
        .expect("Repo.run definition");
    let targets: Vec<_> = bound
        .adjacency
        .outgoing(idx)
        .iter()
        .filter_map(|e| bound.registry.get(e.target))
        .filter(|m| m.name == "foo")
        .collect();
    assert_eq!(targets.len(), 1, "o.foo() resolves to Other.foo via local binding + caller enrichment");
    assert_eq!(targets[0].class.as_deref(), Some("Other"));
}

#[test]
fn objc_message_send_resolves_via_binding_and_enrichment() {
    let src = "@implementation Bar\n- (void)doThing {}\n@end\n@implementation Repo\n- (void)run:(Bar*)b {\n    [b doThing];\n}\n@end\n";
    let (_d, corpus) = one_source(src, "sample.m", Language::ObjectiveC);
    let a = analyze(corpus.files.first().unwrap());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    let idx = bound
        .registry
        .methods
        .iter()
        .position(|m| m.class.as_deref() == Some("Repo") && m.name == "run")
        .map(|i| MethodIndex(i as u32))
        .expect("Repo.run definition");
    let targets: Vec<_> = bound
        .adjacency
        .outgoing(idx)
        .iter()
        .filter_map(|e| bound.registry.get(e.target))
        .filter(|m| m.name == "doThing")
        .collect();
    assert_eq!(targets.len(), 1, "[b doThing] resolves to Bar.doThing via param binding + caller enrichment");
    assert_eq!(targets[0].class.as_deref(), Some("Bar"));
}

#[test]
fn objc_binds_method_param_and_local() {
    let src = "@implementation Repo\n- (void)run:(Bar*)b {\n    Foo* x;\n}\n@end\n";
    let (_d, corpus) = one_source(src, "sample.m", Language::ObjectiveC);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let found = out.method_envs.iter().any(|(_, env)| {
        env.get("b").map(String::as_str) == Some("Bar") && env.get("x").map(String::as_str) == Some("Foo")
    });
    assert!(found, "objc method binds param b->Bar (pointer) and local x->Foo; envs: {:?}", out.method_envs);
}

#[test]
fn go_closure_param_does_not_leak_to_enclosing_method_binding() {
    let src = "package p\ntype RealConn struct {}\nfunc (c *RealConn) Send() {}\ntype FakeConn struct {}\nfunc (c *FakeConn) Send() {}\ntype Server struct {}\nfunc (s *Server) Run() {\n\tvar conn RealConn\n\tprocess(func(conn FakeConn) {\n\t\tconn.Send()\n\t})\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let a = analyze(corpus.files.first().unwrap());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    let run = bound
        .registry
        .methods
        .iter()
        .find(|m| m.class.as_deref() == Some("Server") && m.name == "Run")
        .cloned()
        .expect("Server.Run definition");
    assert!(
        targets_named(&bound, &run, "Send").is_empty(),
        "a closure param shadowing a method var must not fabricate a receiver edge from the enclosing method"
    );
}

#[test]
fn go_receiver_type_param_is_not_bound_as_a_type() {
    let src = "package p\nfunc (s *Box[T]) Set(v T) {\n\tv.Apply()\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let set = method_env(&out, "Set").expect("Set env");
    assert!(
        set.get("v").is_none(),
        "a parameter typed by a receiver-level generic type parameter must not bind to a class"
    );
    assert_eq!(set.get("s").map(String::as_str), Some("Box"), "the receiver itself still binds to its base type");
}

#[test]
fn cpp_nested_template_field_binds_to_inner_type_not_template() {
    let src = "class C {\n  Cache<int>::Handle handle;\n};\n";
    let (_d, corpus) = one_source(src, "sample.cpp", Language::Cpp);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let c = class_binding(&out, "C").expect("class C");
    assert_eq!(
        c.fields.get("handle").map(String::as_str),
        Some("Handle"),
        "Cache<int>::Handle binds to the nested type, not the template head"
    );
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
