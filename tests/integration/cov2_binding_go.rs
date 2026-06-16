use pulse::audit::call_walker::calls_and_bindings_from;
use pulse::parse::Language;

use crate::binding_common::{class_binding, method_env, one_source};

#[test]
fn go_interface_type_spec_has_no_struct_fields_or_parents() {
    let src = "package p\ntype Reader interface {\n\tRead() int\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let reader = class_binding(&out, "Reader").expect("interface type_spec is still a class binding");
    assert!(
        reader.fields.is_empty(),
        "an interface type_spec has no struct_type so class_field_types returns the empty env"
    );
    assert!(reader.parents.is_empty(), "an interface type_spec has no struct_type so class_parents returns no parents");
}

#[test]
fn go_non_struct_alias_type_spec_yields_empty_field_and_parent_sets() {
    let src = "package p\ntype Celsius float64\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let alias = class_binding(&out, "Celsius").expect("alias type_spec is a class binding");
    assert!(alias.fields.is_empty(), "a non-struct alias has no struct_fields list");
    assert!(alias.parents.is_empty(), "a non-struct alias contributes no embedded parents");
}

#[test]
fn go_struct_pointer_qualified_and_generic_field_heads_resolve() {
    let src = "package p\ntype Server struct {\n\tconn *Conn\n\tclient pkg.Client\n\tcache Box[Item]\n\tcount int\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let server = class_binding(&out, "Server").expect("Server class");
    assert_eq!(
        server.fields.get("conn").map(String::as_str),
        Some("Conn"),
        "a pointer_type field head peels to its pointee type identifier"
    );
    assert_eq!(
        server.fields.get("client").map(String::as_str),
        Some("Client"),
        "a qualified_type field head resolves to the last segment"
    );
    assert_eq!(
        server.fields.get("cache").map(String::as_str),
        Some("Box"),
        "a generic_type field head resolves to the template name identifier"
    );
    assert!(server.fields.get("count").is_none(), "a primitive int field is skipped by simple_head");
}

#[test]
fn go_pointer_and_qualified_embeds_are_extracted_as_parents() {
    let src = "package p\ntype Server struct {\n\t*Base\n\tpkg.Mixin\n\tname string\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let server = class_binding(&out, "Server").expect("Server class");
    assert!(
        server.parents.contains(&"Base".to_string()),
        "a pointer embedded type contributes its pointee head as a parent"
    );
    assert!(
        server.parents.contains(&"Mixin".to_string()),
        "a qualified embedded type contributes its last-segment head as a parent"
    );
    assert!(!server.parents.contains(&"name".to_string()), "a named field is not an embedded parent");
}

#[test]
fn go_var_spec_with_explicit_type_binds_inferred_var_does_not() {
    let src = "package p\nfunc Run() {\n\tvar w Worker\n\tvar lazy = make()\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let run = method_env(&out, "Run").expect("Run env");
    assert_eq!(run.get("w").map(String::as_str), Some("Worker"), "a var_spec carrying an explicit type binds the name");
    assert!(run.get("lazy").is_none(), "a var_spec with no explicit type node hits the early return and binds nothing");
}

#[test]
fn go_generic_function_type_parameter_param_is_dropped() {
    let src = "package p\nfunc Map[T any](item T, dep Service) {\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let map = method_env(&out, "Map").expect("Map env");
    assert!(map.get("item").is_none(), "a parameter typed by a method-level type parameter is filtered out of the env");
    assert_eq!(
        map.get("dep").map(String::as_str),
        Some("Service"),
        "a concrete parameter alongside the generic still binds"
    );
}

#[test]
fn go_pointer_receiver_generic_unwraps_to_collect_its_type_arguments() {
    let src = "package p\ntype Box struct{}\nfunc (b *Box[T]) Store(v T, real Concrete) {\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let store = method_env(&out, "Store").expect("Store env");
    assert!(
        store.get("v").is_none(),
        "a pointer receiver generic type arg is collected and the param typed by it is dropped"
    );
    assert_eq!(
        store.get("real").map(String::as_str),
        Some("Concrete"),
        "a concrete param on the same generic-receiver method still binds"
    );
    assert_eq!(store.get("b").map(String::as_str), Some("Box"), "the pointer receiver itself binds to its base type");
}

#[test]
fn go_receiver_with_non_type_kind_is_skipped_without_panicking() {
    let src = "package p\nfunc (s func()) Weird(real Concrete) {\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env = out.method_envs.iter().find(|(id, _)| id.name == "Weird").map(|(_, e)| e);
    if let Some(env) = env {
        assert_eq!(
            env.get("real").map(String::as_str),
            Some("Concrete"),
            "the body param still binds even when the receiver type is not a collectable type kind"
        );
    }
}

#[test]
fn go_pointer_receiver_to_non_type_kind_is_skipped() {
    let src = "package p\nfunc (s *func()) Odd(real Concrete) {\n}\n";
    let (_d, corpus) = one_source(src, "x.go", Language::Go);
    let out = calls_and_bindings_from(corpus.files.first().unwrap());
    let env = out.method_envs.iter().find(|(id, _)| id.name == "Odd").map(|(_, e)| e);
    if let Some(env) = env {
        assert_eq!(
            env.get("real").map(String::as_str),
            Some("Concrete"),
            "a pointer receiver with no inner type kind continues without collecting type args"
        );
    }
}
