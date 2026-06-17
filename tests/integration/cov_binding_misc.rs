use std::path::PathBuf;

use tree_sitter::Node;

use pulse::audit::binding::{self, BindingTable, ClassBinding, TypeEnv};
use pulse::audit::binding_extract::{bind_c_decl, c_declarator_name, collect_c_locals, head_of, EnvBuilder};
use pulse::audit::call_graph::MethodIdentity;
use pulse::audit::call_walker::calls_and_bindings_from;
use pulse::parse::{parse_only, Language};
use pulse::walk::node_text;

use crate::binding_common::one_source;

fn ident(file: &str, class: Option<&str>, name: &str, line: u32) -> MethodIdentity {
    MethodIdentity { file: PathBuf::from(file), class: class.map(str::to_string), name: name.to_string(), line }
}

fn env_of(pairs: &[(&str, &str)]) -> TypeEnv {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn class_with(name: &str, file: &str, parents: &[&str], fields: &[(&str, &str)]) -> ClassBinding {
    ClassBinding {
        file: PathBuf::from(file),
        name: name.to_string(),
        parents: parents.iter().map(std::string::ToString::to_string).collect(),
        fields: env_of(fields),
    }
}

#[test]
fn var_type_returns_binding_and_none_for_missing_method_or_name() {
    let mut table = BindingTable::default();
    let caller = ident("a.java", Some("C"), "run", 3);
    table.insert_method(caller.clone(), env_of(&[("b", "Bar")]));
    assert_eq!(table.var_type(&caller, "b"), Some("Bar"));
    assert!(table.var_type(&caller, "absent").is_none(), "an unknown variable yields no type");
    let stranger = ident("a.java", Some("C"), "other", 9);
    assert!(table.var_type(&stranger, "b").is_none(), "a method with no recorded env yields no type");
}

#[test]
fn insert_method_skips_empty_environments() {
    let mut table = BindingTable::default();
    let caller = ident("a.java", Some("C"), "run", 3);
    table.insert_method(caller.clone(), TypeEnv::new());
    assert!(table.var_type(&caller, "anything").is_none(), "an empty env is never recorded");
}

#[test]
fn field_type_requires_a_class_and_matching_file() {
    let mut table = BindingTable::default();
    table.insert_class(class_with("C", "a.java", &[], &[("helper", "Foo")]));
    let with_class = ident("a.java", Some("C"), "run", 3);
    assert_eq!(table.field_type(&with_class, "helper"), Some("Foo"));
    assert!(table.field_type(&with_class, "missing").is_none(), "an absent field name yields no type");
    let no_class = ident("a.java", None, "run", 3);
    assert!(table.field_type(&no_class, "helper").is_none(), "a caller with no class has no field types");
    let wrong_file = ident("b.java", Some("C"), "run", 3);
    assert!(table.field_type(&wrong_file, "helper").is_none(), "fields are keyed by (file, class)");
}

#[test]
fn class_field_types_at_returns_env_or_none() {
    let mut table = BindingTable::default();
    table.insert_class(class_with("C", "a.java", &[], &[("helper", "Foo")]));
    let env = table.class_field_types_at(&PathBuf::from("a.java"), "C").expect("env present");
    assert_eq!(env.get("helper").map(String::as_str), Some("Foo"));
    assert!(table.class_field_types_at(&PathBuf::from("a.java"), "Other").is_none(), "unknown class has no env");
}

#[test]
fn field_types_for_name_collects_across_files_and_skips_other_classes() {
    let mut table = BindingTable::default();
    table.insert_class(class_with("Widget", "a.java", &[], &[("helper", "Foo")]));
    table.insert_class(class_with("Widget", "b.java", &[], &[("other", "Bar")]));
    table.insert_class(class_with("Different", "c.java", &[], &[("x", "Ignored")]));
    let mut types = table.field_types_for_name("Widget");
    types.sort_unstable();
    assert_eq!(types, vec!["Bar", "Foo"], "field types for a class name span all files declaring it");
    assert!(table.field_types_for_name("Nonexistent").is_empty(), "an unknown class name yields no field types");
}

#[test]
fn class_with_no_fields_is_known_but_has_no_field_env() {
    let mut table = BindingTable::default();
    table.insert_class(class_with("Empty", "a.java", &["Base"], &[]));
    assert!(table.is_known_class("Empty"), "a class with no fields is still registered as known");
    assert!(!table.is_known_class("Ghost"), "a class never inserted is not known");
    assert!(table.class_field_types_at(&PathBuf::from("a.java"), "Empty").is_none(), "no fields means no field env");
    assert_eq!(table.ancestors("Empty"), vec!["Base".to_string()], "a fieldless class still records its parents");
}

#[test]
fn record_parents_ignores_empty_and_keeps_first_consistent_parents() {
    let mut table = BindingTable::default();
    table.insert_class(class_with("Leaf", "a.java", &[], &[("f", "Foo")]));
    assert!(table.ancestors("Leaf").is_empty(), "a class declared with no parents has no ancestors");
    table.insert_class(class_with("Node", "a.java", &["Base"], &[]));
    table.insert_class(class_with("Node", "b.java", &["Base"], &[]));
    assert_eq!(table.ancestors("Node"), vec!["Base".to_string()], "re-declaring with identical parents is a no-op");
}

#[test]
fn ancestors_skips_a_poisoned_root_and_a_poisoned_intermediate() {
    let mut table = BindingTable::default();
    table.insert_class(class_with("Top", "a.java", &["X"], &[]));
    table.insert_class(class_with("Top", "b.java", &["Y"], &[]));
    assert!(table.ancestors("Top").is_empty(), "a divergently-parented root returns no ancestors");

    let mut chain = BindingTable::default();
    chain.insert_class(class_with("A", "a.java", &["Mid"], &[]));
    chain.insert_class(class_with("Mid", "a.java", &["Up"], &[]));
    chain.insert_class(class_with("Mid", "b.java", &["Down"], &[]));
    assert_eq!(
        chain.ancestors("A"),
        vec!["Mid".to_string()],
        "a poisoned intermediate is reported once but its further ancestors are not walked"
    );
}

#[test]
fn ancestors_dedup_diamond_inheritance() {
    let mut table = BindingTable::default();
    table.insert_class(class_with("D", "a.java", &["B", "C"], &[]));
    table.insert_class(class_with("B", "a.java", &["A"], &[]));
    table.insert_class(class_with("C", "a.java", &["A"], &[]));
    table.insert_class(class_with("A", "a.java", &[], &[]));
    let anc = table.ancestors("D");
    let a_count = anc.iter().filter(|p| p.as_str() == "A").count();
    assert_eq!(a_count, 1, "a shared grandparent is visited only once in a diamond");
    assert!(anc.contains(&"B".to_string()) && anc.contains(&"C".to_string()));
}

#[test]
fn ancestors_walk_is_bounded_for_long_linear_chains() {
    let mut table = BindingTable::default();
    let total = 200usize;
    for i in 0..total {
        let name = format!("C{i}");
        let parent = format!("C{}", i + 1);
        table.insert_class(class_with(&name, "a.java", &[parent.as_str()], &[]));
    }
    let anc = table.ancestors("C0");
    assert!(anc.len() < total, "a very long linear ancestry is capped by the walk limit, got {}", anc.len());
    assert_eq!(anc.first().map(String::as_str), Some("C1"), "the nearest ancestor is still the first reported");
}

fn caller_class_via(src: &str, name: &str, lang: Language) -> Option<String> {
    let tree = parse_only(src, lang).expect("parse");
    let target = find_kind(tree.root_node(), name)?;
    binding::caller_class(target, src, lang)
}

fn find_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_kind(child, kind) {
            return Some(found);
        }
    }
    None
}

#[test]
fn caller_class_resolves_go_pointer_and_value_receivers() {
    let ptr = "package p\ntype Repo struct {}\nfunc (r *Repo) Run() {}\n";
    assert_eq!(caller_class_via(ptr, "method_declaration", Language::Go).as_deref(), Some("Repo"));
    let value = "package p\ntype Repo struct {}\nfunc (r Repo) Run() {}\n";
    assert_eq!(
        caller_class_via(value, "method_declaration", Language::Go).as_deref(),
        Some("Repo"),
        "a value receiver resolves to its type identifier directly"
    );
}

#[test]
fn caller_class_is_none_for_non_go_languages() {
    let src = "class C { void run() {} }\n";
    assert!(
        caller_class_via(src, "method_declaration", Language::Java).is_none(),
        "only Go dispatches a caller class; other languages return none"
    );
}

#[test]
fn supports_reflects_the_registered_binder_set() {
    assert!(binding::supports(Language::D));
    assert!(binding::supports(Language::Go));
    assert!(binding::supports(Language::Rust));
    assert!(binding::supports(Language::Swift));
    assert!(!binding::supports(Language::Python), "an unsupported language has no binder");
}

#[test]
fn class_field_types_dispatch_yields_empty_for_unsupported_language() {
    let tree = parse_only("class C:\n    pass\n", Language::Python).expect("parse");
    let root = tree.root_node();
    assert!(binding::class_field_types(root, "class C:\n    pass\n", Language::Python).is_empty());
    assert!(binding::class_parents(root, "class C:\n    pass\n", Language::Python).is_empty());
    assert!(binding::method_var_types(root, "class C:\n    pass\n", Language::Python).is_empty());
}

#[test]
fn head_of_strips_generics_qualifiers_and_optionals() {
    assert_eq!(head_of("Map<String, Bar>").as_deref(), Some("Map"), "generic arguments are stripped");
    assert_eq!(head_of("a::b::Foo").as_deref(), Some("Foo"), "the last path segment after :: wins");
    assert_eq!(head_of("pkg.sub.Bar").as_deref(), Some("Bar"), "the last dotted segment wins");
    assert_eq!(head_of("Optional?").as_deref(), Some("Optional"), "a trailing optional marker is dropped");
    assert_eq!(head_of("List<Box<Inner>>::Item").as_deref(), Some("Item"), "nested generics and a qualifier combine");
    assert_eq!(head_of("Outer<Mid<Deep>>").as_deref(), Some("Outer"), "only the head survives stripping");
}

#[test]
fn head_of_is_none_for_empty_and_pure_generic_text() {
    assert!(head_of("").is_none(), "empty text has no head");
    assert!(head_of("   ").is_none(), "whitespace-only text has no head");
    assert!(head_of("<T>").is_none(), "a bare generic with no head is dropped");
    assert!(head_of("Foo?").is_some(), "an optional with a real head is kept");
}

#[test]
fn env_builder_inserts_keeps_consistent_and_poisons_conflicts() {
    let mut builder = EnvBuilder::default();
    builder.bind("a".into(), "Foo".into());
    builder.bind("a".into(), "Foo".into());
    builder.bind("b".into(), "Bar".into());
    builder.bind("c".into(), "First".into());
    builder.bind("c".into(), "Second".into());
    builder.bind("c".into(), "Third".into());
    let env = builder.into_env(&std::collections::BTreeSet::new());
    assert_eq!(env.get("a").map(String::as_str), Some("Foo"), "an identical re-bind is a no-op");
    assert_eq!(env.get("b").map(String::as_str), Some("Bar"));
    assert!(!env.contains_key("c"), "a conflicting bind poisons the name, and later binds to it are ignored");
}

#[test]
fn env_builder_into_env_drops_type_variables() {
    let mut builder = EnvBuilder::default();
    builder.bind("item".into(), "T".into());
    builder.bind("real".into(), "Concrete".into());
    let mut tvars = std::collections::BTreeSet::new();
    tvars.insert("T".to_string());
    let env = builder.into_env(&tvars);
    assert!(!env.contains_key("item"), "a binding to a type variable is removed");
    assert_eq!(env.get("real").map(String::as_str), Some("Concrete"), "a concrete binding survives");
}

fn type_head_for_test(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "type_identifier" => head_of(node_text(node, source)),
        _ => None,
    }
}

fn first_declaration(node: Node<'_>) -> Option<Node<'_>> {
    find_kind(node, "declaration")
}

#[test]
fn c_declarator_name_unwraps_pointer_init_and_reference_declarators() {
    let src = "void f() {\n  Foo plain;\n  Bar* ptr;\n  Baz init = make();\n}\n";
    let tree = parse_only(src, Language::Cpp).expect("parse");
    let body = find_kind(tree.root_node(), "compound_statement").expect("body");
    let mut names = Vec::new();
    let mut cursor = body.walk();
    for stmt in body.children(&mut cursor) {
        if stmt.kind() != "declaration" {
            continue;
        }
        let mut inner = stmt.walk();
        for child in stmt.children(&mut inner) {
            if let Some(name) = c_declarator_name(child, src) {
                names.push(name);
            }
        }
    }
    assert!(names.contains(&"plain".to_string()), "a bare identifier declarator yields its name");
    assert!(names.contains(&"ptr".to_string()), "a pointer declarator recurses to the inner name");
    assert!(names.contains(&"init".to_string()), "an init declarator recurses to the declared name");
}

#[test]
fn c_declarator_name_is_none_for_non_declarator_nodes() {
    let src = "int x;\n";
    let tree = parse_only(src, Language::Cpp).expect("parse");
    let decl = first_declaration(tree.root_node()).expect("declaration");
    let ty = decl.child_by_field_name("type").expect("type child");
    assert!(c_declarator_name(ty, src).is_none(), "a primitive_type node is not a declarator");
}

#[test]
fn bind_c_decl_binds_named_decl_and_skips_typeless() {
    let src = "void f() {\n  Foo a;\n}\n";
    let tree = parse_only(src, Language::Cpp).expect("parse");
    let decl = first_declaration(tree.root_node()).expect("declaration");
    let mut builder = EnvBuilder::default();
    bind_c_decl(decl, src, &mut builder, type_head_for_test);
    let env = builder.into_env(&std::collections::BTreeSet::new());
    assert_eq!(env.get("a").map(String::as_str), Some("Foo"), "a typed declaration binds its declarator to the type");
}

#[test]
fn bind_c_decl_returns_early_when_type_head_yields_nothing() {
    let src = "void f() {\n  int n;\n}\n";
    let tree = parse_only(src, Language::Cpp).expect("parse");
    let decl = first_declaration(tree.root_node()).expect("declaration");
    let mut builder = EnvBuilder::default();
    bind_c_decl(decl, src, &mut builder, type_head_for_test);
    let env = builder.into_env(&std::collections::BTreeSet::new());
    assert!(!env.contains_key("n"), "when the type head resolves to none, no binding is recorded");
}

#[test]
fn collect_c_locals_recurses_and_honors_scope_boundaries() {
    let src = "void f() {\n  Foo a;\n  if (cond) {\n    Bar b;\n  }\n  [](){ Baz inner; }();\n  int n;\n}\n";
    let tree = parse_only(src, Language::Cpp).expect("parse");
    let body = find_kind(tree.root_node(), "compound_statement").expect("body");
    let mut builder = EnvBuilder::default();
    collect_c_locals(body, src, &mut builder, type_head_for_test, &["lambda_expression"]);
    let env = builder.into_env(&std::collections::BTreeSet::new());
    assert_eq!(env.get("a").map(String::as_str), Some("Foo"), "a top-level local is collected");
    assert_eq!(env.get("b").map(String::as_str), Some("Bar"), "a nested-block local is collected via recursion");
    assert!(!env.contains_key("inner"), "a local inside a boundary node is not collected");
    assert!(!env.contains_key("n"), "a primitive local is not bound");
}

#[test]
fn d_templated_base_class_never_leaks_the_template_argument() {
    let src = "class Widget : Container!(Item) {}\n";
    let (_d, corpus) = one_source(src, "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = out.classes.iter().find(|c| c.name == "Widget").expect("widget");
    assert!(
        !w.parents.contains(&"Item".to_string()),
        "the template argument is never recorded as a base class; parents: {:?}",
        w.parents
    );
}

#[test]
fn d_concrete_field_binds_alongside_a_template_instance_field() {
    let src = "class Box {\n  Holder!(int) wrapped;\n  Concrete real;\n}\n";
    let (_d, corpus) = one_source(src, "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let b = out.classes.iter().find(|c| c.name == "Box").expect("box");
    assert_eq!(b.fields.get("real").map(String::as_str), Some("Concrete"), "a plain typed field binds");
    assert!(b.fields.get("wrapped") != Some(&"int".to_string()), "a template-instance field never binds the argument");
}

#[test]
fn d_method_with_foreach_collects_a_plain_local_through_recursion() {
    let src = "class C {\n  void f() {\n    Outer head;\n    foreach (Element e; items) {\n      Inner deep;\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = out.method_envs.iter().find(|(id, _)| id.name == "f").map(|(_, e)| e).expect("f env");
    assert_eq!(f.get("head").map(String::as_str), Some("Outer"), "a top-level local binds");
    assert_eq!(f.get("deep").map(String::as_str), Some("Inner"), "a local nested inside a foreach binds via recursion");
}

#[test]
fn swift_optional_array_and_dictionary_fields() {
    let src = concat!(
        "class Widget {\n",
        "  let helper: Bar = make()\n",
        "  let maybe: Baz? = lookup()\n",
        "  let many: [Item] = []\n",
        "  let map: [String: Item] = [:]\n",
        "}\n"
    );
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = out.classes.iter().find(|c| c.name == "Widget").expect("widget");
    assert_eq!(w.fields.get("helper").map(String::as_str), Some("Bar"));
    assert_eq!(w.fields.get("maybe").map(String::as_str), Some("Baz"), "an optional field unwraps to its base type");
    assert!(!w.fields.contains_key("many"), "an array field is not bound to its element type");
    assert!(!w.fields.contains_key("map"), "a dictionary field is not bound to its value type");
}

#[test]
fn swift_generic_method_drops_its_own_type_parameter() {
    let src = "class C {\n  func make<Element>(seed: Element, real: Concrete) {\n    seed.use()\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let m = out.method_envs.iter().find(|(id, _)| id.name == "make").map(|(_, e)| e).expect("make env");
    assert!(m.get("seed").is_none(), "a parameter typed by a method type parameter is dropped");
    assert_eq!(m.get("real").map(String::as_str), Some("Concrete"), "a concrete parameter still binds");
}

#[test]
fn swift_for_loop_body_local_binds_through_recursion() {
    let src = "class C {\n  func f() {\n    for item: Element in collection {\n      let deep: Inner = make()\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = out.method_envs.iter().find(|(id, _)| id.name == "f").map(|(_, e)| e).expect("f env");
    assert_eq!(f.get("deep").map(String::as_str), Some("Inner"), "a local inside a for body binds via recursion");
    assert!(f.get("item").map(String::as_str) != Some("Int"), "the typed item never binds to an unrelated type");
}

#[test]
fn swift_inheritance_specifier_field_form_extracts_parent() {
    let src = "class Derived: Base {\n  func run() {}\n}\n";
    let (_d, corpus) = one_source(src, "Sample.swift", Language::Swift);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let d = out.classes.iter().find(|c| c.name == "Derived").expect("derived");
    assert!(
        d.parents.contains(&"Base".to_string()),
        "an inheritance specifier yields the base type; got {:?}",
        d.parents
    );
}

#[test]
fn rust_reference_and_generic_param_types_resolve_to_head() {
    let src = concat!(
        "struct Repo;\n",
        "impl Repo {\n",
        "  fn run(&self, dep: &Service, list: Vec<Worker>, prim: &i32) {\n",
        "    let owned: Engine = make();\n",
        "  }\n",
        "}\n"
    );
    let (_d, corpus) = one_source(src, "sample.rs", Language::Rust);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = out.method_envs.iter().find(|(id, _)| id.name == "run").map(|(_, e)| e).expect("run env");
    assert_eq!(run.get("dep").map(String::as_str), Some("Service"), "a reference parameter peels to its inner type");
    assert_eq!(run.get("list").map(String::as_str), Some("Vec"), "a generic parameter binds to the generic head");
    assert_eq!(run.get("owned").map(String::as_str), Some("Engine"));
    assert!(run.get("prim").is_none(), "a reference to a primitive is not bound");
}

#[test]
fn rust_scoped_field_type_and_tuple_field_dropped() {
    let src = "struct Repo {\n  helper: outer::Service,\n  pair: (i32, i32),\n}\n";
    let (_d, corpus) = one_source(src, "sample.rs", Language::Rust);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let repo = out.classes.iter().find(|c| c.name == "Repo").expect("repo");
    assert_eq!(
        repo.fields.get("helper").map(String::as_str),
        Some("Service"),
        "a scoped type binds to its last segment"
    );
    assert!(!repo.fields.contains_key("pair"), "a tuple-typed field is not bound");
}

#[test]
fn rust_supertrait_bounds_become_parents() {
    let src = "trait Base {}\ntrait Marker {}\ntrait Derived: Base + Marker {}\n";
    let (_d, corpus) = one_source(src, "sample.rs", Language::Rust);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let derived = out.classes.iter().find(|c| c.name == "Derived").expect("derived trait");
    assert!(derived.parents.contains(&"Base".to_string()), "a supertrait bound is a parent; got {:?}", derived.parents);
    assert!(derived.parents.contains(&"Marker".to_string()), "every supertrait bound is recorded");
}

#[test]
fn rust_impl_for_records_the_trait_as_a_parent() {
    let src = "struct Repo;\ntrait Drawable {}\nimpl Drawable for Repo {}\n";
    let (_d, corpus) = one_source(src, "sample.rs", Language::Rust);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let with_trait = out.classes.iter().find(|c| c.parents.contains(&"Drawable".to_string()));
    assert!(with_trait.is_some(), "the impl-of-trait records Drawable as a parent; classes: {:?}", out.classes);
}

#[test]
fn go_var_block_with_multiple_names_binds_each() {
    let src = "package p\nfunc f() {\n\tvar a, b Worker\n\tvar n int\n\ta.Use()\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = out.method_envs.iter().find(|(id, _)| id.name == "f").map(|(_, e)| e).expect("f env");
    assert_eq!(f.get("a").map(String::as_str), Some("Worker"), "the first name in a var block binds");
    assert_eq!(f.get("b").map(String::as_str), Some("Worker"), "the second name in a var block binds");
    assert!(f.get("n").is_none(), "a primitive-typed var is not bound");
}

#[test]
fn go_qualified_and_pointer_param_types_resolve() {
    let src = "package p\nfunc f(svc *Service, q pkg.Conn, n int) {\n\tsvc.Run()\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = out.method_envs.iter().find(|(id, _)| id.name == "f").map(|(_, e)| e).expect("f env");
    assert_eq!(f.get("svc").map(String::as_str), Some("Service"), "a pointer parameter peels to its base type");
    assert_eq!(f.get("q").map(String::as_str), Some("Conn"), "a qualified parameter type binds to its last segment");
    assert!(f.get("n").is_none(), "a primitive parameter is not bound");
}

#[test]
fn go_function_with_no_receiver_and_no_params_yields_empty_env() {
    let src = "package p\nfunc Standalone() {\n\tx := 1\n\t_ = x\n}\n";
    let (_d, corpus) = one_source(src, "sample.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env = out.method_envs.iter().find(|(id, _)| id.name == "Standalone").map(|(_, e)| e);
    assert!(env.is_none() || env.unwrap().is_empty(), "a receiverless, paramless func contributes no bindings");
}
