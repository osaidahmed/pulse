use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::{
    bucket_for_class, bucket_for_method, class_atfd, class_cc, class_fanout, class_method_count, class_tcc, class_wmc,
    ClassIndex, ClassRegistry,
};
use pulse::audit::definitions::DefinitionRecord;

#[allow(clippy::too_many_arguments)]
fn def_with(
    file: &str,
    class: Option<&str>,
    name: &str,
    line: u32,
    parent: Option<&str>,
    cc: u32,
    fields: Vec<&str>,
    foreign: Vec<(&str, &str)>,
    is_ctor: bool,
) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: class.map(String::from),
            name: name.to_string(),
            line,
        },
        cc,
        field_accesses: fields.into_iter().map(String::from).collect(),
        foreign_field_accesses: foreign.into_iter().map(|(r, f)| (r.to_string(), f.to_string())).collect(),
        parent_class: parent.map(String::from),
        is_constructor: is_ctor,
    }
}

fn build_registry(defs: Vec<DefinitionRecord>) -> (ClassRegistry, CallGraph) {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    (registry, graph)
}

#[test]
fn empty_defs_produce_empty_registry() {
    let (reg, _) = build_registry(Vec::new());
    assert_eq!(reg.count(), 0);
}

#[test]
fn class_with_two_methods_creates_one_class_entry() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m1", 1, None, 1, vec![], vec![], false),
        def_with("a.py", Some("Foo"), "m2", 5, None, 1, vec![], vec![], false),
    ];
    let (reg, _) = build_registry(defs);
    assert_eq!(reg.count(), 1);
    let foo = reg.get(ClassIndex(0)).unwrap();
    assert_eq!(foo.name, "Foo");
    assert_eq!(foo.method_indices.len(), 2);
}

#[test]
fn same_class_name_in_different_files_creates_two_entries() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m", 1, None, 1, vec![], vec![], false),
        def_with("b.py", Some("Foo"), "m", 1, None, 1, vec![], vec![], false),
    ];
    let (reg, _) = build_registry(defs);
    assert_eq!(reg.count(), 2);
    let by_name = reg.by_name.get("Foo").unwrap();
    assert_eq!(by_name.len(), 2);
}

#[test]
fn parent_class_propagates_to_class_identity() {
    let defs = vec![def_with("a.py", Some("Bar"), "m", 1, Some("Foo"), 1, vec![], vec![], false)];
    let (reg, _) = build_registry(defs);
    let bar = reg.get(ClassIndex(0)).unwrap();
    assert_eq!(bar.parent_class.as_deref(), Some("Foo"));
}

#[test]
fn lookup_parent_returns_parent_class_index() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m", 1, None, 1, vec![], vec![], false),
        def_with("b.py", Some("Bar"), "m", 1, Some("Foo"), 1, vec![], vec![], false),
    ];
    let (reg, _) = build_registry(defs);
    let foo_idx = *reg.by_name.get("Foo").unwrap().first().unwrap();
    let bar_idx = *reg.by_name.get("Bar").unwrap().first().unwrap();
    let parents = reg.lookup_parent(bar_idx);
    assert_eq!(parents, vec![foo_idx]);
}

#[test]
fn class_method_count_matches() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m1", 1, None, 1, vec![], vec![], false),
        def_with("a.py", Some("Foo"), "m2", 5, None, 1, vec![], vec![], false),
        def_with("a.py", Some("Foo"), "m3", 10, None, 1, vec![], vec![], false),
    ];
    let (reg, _) = build_registry(defs);
    assert_eq!(class_method_count(&reg, ClassIndex(0)), 3);
}

#[test]
fn class_wmc_sums_per_method_cc() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m1", 1, None, 5, vec![], vec![], false),
        def_with("a.py", Some("Foo"), "m2", 5, None, 7, vec![], vec![], false),
        def_with("a.py", Some("Foo"), "m3", 10, None, 3, vec![], vec![], false),
    ];
    let (reg, graph) = build_registry(defs);
    let cc_for = |m| {
        graph.registry.get(m).map_or(0, |id| {
            if id.name == "m1" {
                5
            } else if id.name == "m2" {
                7
            } else {
                3
            }
        })
    };
    assert_eq!(class_wmc(&reg, ClassIndex(0), &cc_for), 15);
}

#[test]
fn class_atfd_dedupes_distinct_pairs() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m1", 1, None, 1, vec![], vec![("obj", "x")], false),
        def_with("a.py", Some("Foo"), "m2", 5, None, 1, vec![], vec![("obj", "x"), ("obj", "y")], false),
    ];
    let (reg, graph) = build_registry(defs.clone());
    let foreign_for = |m: pulse::audit::call_graph::MethodIndex| -> Vec<(String, String)> {
        let id = graph.registry.get(m).unwrap();
        defs.iter().find(|d| &d.identity == id).map(|d| d.foreign_field_accesses.clone()).unwrap_or_default()
    };
    let atfd = class_atfd(&reg, ClassIndex(0), &foreign_for);
    assert_eq!(atfd, 2);
}

#[test]
fn class_tcc_three_methods_all_share_field() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m1", 1, None, 1, vec!["x"], vec![], false),
        def_with("a.py", Some("Foo"), "m2", 5, None, 1, vec!["x"], vec![], false),
        def_with("a.py", Some("Foo"), "m3", 10, None, 1, vec!["x"], vec![], false),
    ];
    let (reg, graph) = build_registry(defs.clone());
    let fields_for = |m: pulse::audit::call_graph::MethodIndex| -> (Vec<String>, bool) {
        let id = graph.registry.get(m).unwrap();
        let d = defs.iter().find(|d| &d.identity == id).unwrap();
        (d.field_accesses.clone(), d.is_constructor)
    };
    let tcc = class_tcc(&reg, ClassIndex(0), &fields_for);
    assert!((tcc - 1.0).abs() < 0.001);
}

#[test]
fn class_tcc_no_shared_fields_yields_zero() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "m1", 1, None, 1, vec!["a"], vec![], false),
        def_with("a.py", Some("Foo"), "m2", 5, None, 1, vec!["b"], vec![], false),
        def_with("a.py", Some("Foo"), "m3", 10, None, 1, vec!["c"], vec![], false),
    ];
    let (reg, graph) = build_registry(defs.clone());
    let fields_for = |m: pulse::audit::call_graph::MethodIndex| -> (Vec<String>, bool) {
        let id = graph.registry.get(m).unwrap();
        let d = defs.iter().find(|d| &d.identity == id).unwrap();
        (d.field_accesses.clone(), d.is_constructor)
    };
    let tcc = class_tcc(&reg, ClassIndex(0), &fields_for);
    assert!(tcc.abs() < 1e-6);
}

#[test]
fn class_tcc_excludes_constructors() {
    let defs = vec![
        def_with("a.py", Some("Foo"), "__init__", 1, None, 1, vec!["x", "y"], vec![], true),
        def_with("a.py", Some("Foo"), "m1", 5, None, 1, vec!["a"], vec![], false),
        def_with("a.py", Some("Foo"), "m2", 10, None, 1, vec!["b"], vec![], false),
    ];
    let (reg, graph) = build_registry(defs.clone());
    let fields_for = |m: pulse::audit::call_graph::MethodIndex| -> (Vec<String>, bool) {
        let id = graph.registry.get(m).unwrap();
        let d = defs.iter().find(|d| &d.identity == id).unwrap();
        (d.field_accesses.clone(), d.is_constructor)
    };
    let tcc = class_tcc(&reg, ClassIndex(0), &fields_for);
    assert!(tcc.abs() < 1e-6, "constructors excluded; m1/m2 share no fields");
}

#[test]
fn class_tcc_one_method_returns_zero() {
    let defs = vec![def_with("a.py", Some("Foo"), "m", 1, None, 1, vec!["x"], vec![], false)];
    let (reg, graph) = build_registry(defs.clone());
    let fields_for = |m: pulse::audit::call_graph::MethodIndex| -> (Vec<String>, bool) {
        let id = graph.registry.get(m).unwrap();
        let d = defs.iter().find(|d| &d.identity == id).unwrap();
        (d.field_accesses.clone(), d.is_constructor)
    };
    let tcc = class_tcc(&reg, ClassIndex(0), &fields_for);
    assert!(tcc.abs() < 1e-6);
}

#[test]
fn bucket_for_method_with_class_includes_file() {
    let m =
        MethodIdentity { file: PathBuf::from("a.py"), class: Some("Foo".to_string()), name: "m".to_string(), line: 1 };
    assert_eq!(bucket_for_method(&m), "a.py::Foo");
}

#[test]
fn bucket_for_method_without_class_uses_free_prefix() {
    let m = MethodIdentity { file: PathBuf::from("a.py"), class: None, name: "f".to_string(), line: 1 };
    assert_eq!(bucket_for_method(&m), "__free::a.py");
}

#[test]
fn bucket_for_class_uses_file_and_name() {
    let defs = vec![def_with("a.py", Some("Foo"), "m", 1, None, 1, vec![], vec![], false)];
    let (reg, _) = build_registry(defs);
    let foo = reg.get(ClassIndex(0));
    assert_eq!(bucket_for_class(foo), "a.py::Foo");
}

#[test]
fn class_cc_zero_with_no_callers() {
    let defs = vec![def_with("a.py", Some("Foo"), "m", 1, None, 1, vec![], vec![], false)];
    let (reg, graph) = build_registry(defs);
    assert_eq!(class_cc(&reg, &graph, ClassIndex(0)), 0);
}

#[test]
fn determinism_two_builds_yield_same_class_count() {
    let defs1 = vec![
        def_with("a.py", Some("Foo"), "m", 1, None, 1, vec![], vec![], false),
        def_with("b.py", Some("Bar"), "m", 1, None, 1, vec![], vec![], false),
    ];
    let defs2 = defs1.clone();
    let (r1, _) = build_registry(defs1);
    let (r2, _) = build_registry(defs2);
    assert_eq!(r1.count(), r2.count());
}

#[test]
fn free_function_not_in_registry() {
    let defs = vec![def_with("a.py", None, "free_fn", 1, None, 1, vec![], vec![], false)];
    let (reg, _) = build_registry(defs);
    assert_eq!(reg.count(), 0);
}

#[test]
fn class_fanout_zero_with_no_outgoing() {
    let defs = vec![def_with("a.py", Some("Foo"), "m", 1, None, 1, vec![], vec![], false)];
    let (reg, graph) = build_registry(defs);
    assert_eq!(class_fanout(&reg, &graph, ClassIndex(0)), 0);
}
