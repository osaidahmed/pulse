use pulse_audit::binding;
use pulse_audit::binding::TypeEnv;
use pulse_audit::call_walker::calls_and_bindings_from;
use pulse_syntax::parse::{parse_only, Language};
use pulse_syntax::walk::{find_child_by_kind, node_text};

use crate::binding_common::{class_binding, method_env, one_source};

fn find_kind<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
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
fn objc_class_field_types_directly_returns_empty_env() {
    let tree = parse_only("@implementation Repo\n- (void)run {}\n@end\n", Language::ObjectiveC).expect("objc parse");
    let env = binding::objc::class_field_types(tree.root_node(), "@implementation Repo\n- (void)run {}\n@end\n");
    assert!(env.is_empty(), "objc class_field_types is an unconditional empty TypeEnv");
}

#[test]
fn objc_class_parents_directly_returns_empty_vec() {
    let src = "@interface Widget : Base\n@end\n";
    let tree = parse_only(src, Language::ObjectiveC).expect("objc parse");
    let parents = binding::objc::class_parents(tree.root_node(), src);
    assert!(parents.is_empty(), "objc class_parents is an unconditional empty Vec");
}

#[test]
fn objc_class_field_types_is_empty_for_any_node() {
    let tree = parse_only("@implementation A\n@end\n", Language::ObjectiveC).expect("objc parse");
    let nested = find_kind(tree.root_node(), "class_implementation").unwrap_or(tree.root_node());
    let env: TypeEnv = binding::objc::class_field_types(nested, "@implementation A\n@end\n");
    assert_eq!(env.len(), 0, "objc fields are never extracted regardless of which node is passed");
}

#[test]
fn d_class_field_binds_object_type_and_drops_primitive() {
    let (_d, corpus) = one_source("class Holder {\n  Bar item;\n  int count;\n}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let holder = class_binding(&out, "Holder").expect("holder class");
    assert_eq!(holder.fields.get("item").map(String::as_str), Some("Bar"), "object-typed field binds");
    assert!(!holder.fields.contains_key("count"), "int primitive field is not bound");
}

#[test]
fn d_template_class_field_drops_type_parameter_but_keeps_concrete() {
    let (_d, corpus) = one_source("class Box(T) {\n  T item;\n  Bar real;\n}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let b = class_binding(&out, "Box").expect("box class");
    assert!(!b.fields.contains_key("item"), "a field typed by the class template parameter T is dropped");
    assert_eq!(b.fields.get("real").map(String::as_str), Some("Bar"), "a concrete field still binds");
}

#[test]
fn d_template_method_drops_type_parameter_local() {
    let (_d, corpus) =
        one_source("class C {\n  void run(T)() {\n    T item;\n    Bar real;\n  }\n}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "run").expect("run env");
    assert!(run.get("item").is_none(), "a local typed by the method template parameter T is dropped");
    assert_eq!(run.get("real").map(String::as_str), Some("Bar"), "a concrete local still binds");
}

#[test]
fn d_type_constructor_local_unwraps_to_inner_type() {
    let (_d, corpus) =
        one_source("class C {\n  void f() {\n    const(Foo) wrapped;\n  }\n}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = method_env(&out, "f").expect("f env");
    assert_eq!(
        f.get("wrapped").map(String::as_str),
        Some("Foo"),
        "const(Foo) recurses through the nested type node to its inner identifier"
    );
}

#[test]
fn d_typeof_typed_local_is_not_bound() {
    let (_d, corpus) =
        one_source("class C {\n  void f() {\n    Bar src;\n    typeof(src) clone;\n  }\n}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = method_env(&out, "f").expect("f env");
    assert!(f.get("clone").is_none(), "a typeof-typed local resolves to no head type");
    assert_eq!(f.get("src").map(String::as_str), Some("Bar"), "the plain local alongside it still binds");
}

#[test]
fn d_typed_foreach_binds_and_untyped_foreach_does_not() {
    let src = "class C {\n  void f(Bar[] xs) {\n    foreach (Foo e; xs) {}\n    foreach (k; xs) {}\n  }\n}\n";
    let (_d, corpus) = one_source(src, "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let f = method_env(&out, "f").expect("f env");
    assert_eq!(f.get("e").map(String::as_str), Some("Foo"), "an explicitly typed foreach variable binds");
    assert!(f.get("k").is_none(), "an untyped foreach variable has no type and is not bound");
}

#[test]
fn d_nested_function_in_method_body_is_a_scope_boundary() {
    let src = "class C {\n  void outer() {\n    Bar visible;\n    void inner() {\n      Foo hidden;\n    }\n  }\n}\n";
    let (_d, corpus) = one_source(src, "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let outer = method_env(&out, "outer").expect("outer env");
    assert_eq!(outer.get("visible").map(String::as_str), Some("Bar"), "the outer local binds");
    assert!(outer.get("hidden").is_none(), "a local declared inside a nested function does not leak into outer");
}

#[test]
fn d_qualified_base_class_uses_last_segment() {
    let (_d, corpus) = one_source("class Widget : pkg.mod.Base {}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget class");
    assert!(w.parents.contains(&"Base".to_string()), "a dotted base resolves to its final segment");
}

#[test]
fn d_template_instance_base_class_uses_template_head() {
    let (_d, corpus) = one_source("class Widget : Container!(int) {}\n", "sample.d", Language::D);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let w = class_binding(&out, "Widget").expect("widget class");
    assert!(w.parents.contains(&"Container".to_string()), "a templated base resolves to the template head identifier");
}

#[test]
fn d_method_var_types_directly_on_class_node_returns_empty() {
    let src = "class Box(T) {}\n";
    let tree = parse_only(src, Language::D).expect("d parse");
    let class_node = find_kind(tree.root_node(), "class_declaration").expect("class_declaration node");
    let env = binding::d::method_var_types(class_node, src);
    assert!(env.is_empty(), "a class node carries no parameters node, so collect_params bails to an empty env");
}

#[test]
fn d_class_field_types_directly_extracts_object_field() {
    let src = "class Holder {\n  Bar item;\n}\n";
    let tree = parse_only(src, Language::D).expect("d parse");
    let class_node = find_kind(tree.root_node(), "class_declaration").expect("class_declaration node");
    let fields = binding::d::class_field_types(class_node, src);
    assert_eq!(fields.get("item").map(String::as_str), Some("Bar"), "the aggregate body field is extracted");

    let aggregate = find_child_by_kind(class_node, "aggregate_body").expect("aggregate body");
    let var_decl = find_kind(aggregate, "variable_declaration").expect("variable declaration");
    let ty = find_child_by_kind(var_decl, "type").expect("type node");
    assert_eq!(node_text(ty, src).trim(), "Bar", "the declared field type text is Bar");
}

#[test]
fn d_class_parents_directly_collects_base_class() {
    let src = "class Widget : Base {}\n";
    let tree = parse_only(src, Language::D).expect("d parse");
    let class_node = find_kind(tree.root_node(), "class_declaration").expect("class_declaration node");
    let parents = binding::d::class_parents(class_node, src);
    assert!(parents.contains(&"Base".to_string()), "a single base class is collected directly");
}
