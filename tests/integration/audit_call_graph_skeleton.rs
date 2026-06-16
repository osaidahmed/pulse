use std::path::PathBuf;

use pulse::audit::call_graph::{CallAdjacency, CallEdge, CallGraph, MethodIdentity, MethodIndex, MethodRegistry};
use pulse::audit::finding::ImportConfidence;

fn mk_id(file: &str, class: Option<&str>, name: &str, line: u32) -> MethodIdentity {
    MethodIdentity { file: PathBuf::from(file), class: class.map(String::from), name: name.to_string(), line }
}

#[test]
fn method_identity_eq_uses_all_fields() {
    let a = mk_id("a.py", Some("Foo"), "bar", 10);
    let b = mk_id("a.py", Some("Foo"), "bar", 10);
    let different = mk_id("a.py", Some("Foo"), "bar", 11);
    assert_eq!(a, b);
    assert_ne!(a, different);
}

#[test]
fn method_identity_orders_by_field_tuple() {
    let a = mk_id("a.py", None, "x", 1);
    let b = mk_id("a.py", None, "x", 2);
    let c = mk_id("b.py", None, "x", 1);
    assert!(a < b);
    assert!(b < c);
}

#[test]
fn registry_intern_assigns_sequential_indices() {
    let mut reg = MethodRegistry::default();
    let i0 = reg.intern(mk_id("a.py", None, "f", 1));
    let i1 = reg.intern(mk_id("b.py", None, "g", 2));
    assert_eq!(i0, MethodIndex(0));
    assert_eq!(i1, MethodIndex(1));
}

#[test]
fn registry_get_returns_identity() {
    let mut reg = MethodRegistry::default();
    let id = mk_id("a.py", Some("Foo"), "bar", 10);
    let idx = reg.intern(id.clone());
    assert_eq!(reg.get(idx), Some(&id));
}

#[test]
fn registry_lookup_by_name_returns_all_definitions() {
    let mut reg = MethodRegistry::default();
    reg.intern(mk_id("a.py", Some("Foo"), "shared", 1));
    reg.intern(mk_id("b.py", Some("Bar"), "shared", 2));
    reg.intern(mk_id("c.py", None, "other", 3));
    let by_name = reg.lookup_by_name("shared");
    assert_eq!(by_name.len(), 2);
}

#[test]
fn registry_lookup_by_class_and_name_filters_correctly() {
    let mut reg = MethodRegistry::default();
    reg.intern(mk_id("a.py", Some("Foo"), "method", 1));
    reg.intern(mk_id("b.py", Some("Bar"), "method", 2));
    let foo_method = reg.lookup_by_class_and_name("Foo", "method");
    let bar_method = reg.lookup_by_class_and_name("Bar", "method");
    assert_eq!(foo_method.len(), 1);
    assert_eq!(bar_method.len(), 1);
    assert_ne!(foo_method[0], bar_method[0]);
}

#[test]
fn registry_lookup_missing_name_returns_empty() {
    let reg = MethodRegistry::default();
    assert_eq!(reg.lookup_by_name("nonexistent").len(), 0);
}

#[test]
fn registry_lookup_missing_class_returns_empty() {
    let mut reg = MethodRegistry::default();
    reg.intern(mk_id("a.py", Some("Foo"), "method", 1));
    assert_eq!(reg.lookup_by_class_and_name("OtherClass", "method").len(), 0);
}

#[test]
fn registry_count_tracks_inserts() {
    let mut reg = MethodRegistry::default();
    assert_eq!(reg.count(), 0);
    reg.intern(mk_id("a.py", None, "f", 1));
    reg.intern(mk_id("b.py", None, "g", 2));
    assert_eq!(reg.count(), 2);
}

#[test]
fn registry_from_definitions_sorts_before_interning() {
    let defs = vec![mk_id("z.py", None, "x", 5), mk_id("a.py", None, "x", 1), mk_id("m.py", None, "x", 3)];
    let reg = MethodRegistry::from_definitions(defs);
    assert_eq!(reg.methods[0].file, PathBuf::from("a.py"));
    assert_eq!(reg.methods[1].file, PathBuf::from("m.py"));
    assert_eq!(reg.methods[2].file, PathBuf::from("z.py"));
}

#[test]
fn adjacency_with_capacity_pre_sizes_vectors() {
    let a = CallAdjacency::with_capacity(5);
    assert_eq!(a.outgoing.len(), 5);
    assert_eq!(a.incoming.len(), 5);
}

#[test]
fn adjacency_insert_populates_both_directions() {
    let mut a = CallAdjacency::with_capacity(2);
    a.insert(CallEdge { source: MethodIndex(0), target: MethodIndex(1), confidence: ImportConfidence::High });
    assert_eq!(a.outgoing(MethodIndex(0)).len(), 1);
    assert_eq!(a.incoming(MethodIndex(1)).len(), 1);
    assert_eq!(a.outgoing(MethodIndex(1)).len(), 0);
    assert_eq!(a.incoming(MethodIndex(0)).len(), 0);
}

#[test]
fn adjacency_outgoing_for_unknown_index_is_empty() {
    let a = CallAdjacency::with_capacity(2);
    assert_eq!(a.outgoing(MethodIndex(99)).len(), 0);
}

#[test]
fn adjacency_incoming_for_unknown_index_is_empty() {
    let a = CallAdjacency::with_capacity(2);
    assert_eq!(a.incoming(MethodIndex(99)).len(), 0);
}

#[test]
fn adjacency_self_loop_emitted_correctly() {
    let mut a = CallAdjacency::with_capacity(1);
    a.insert(CallEdge { source: MethodIndex(0), target: MethodIndex(0), confidence: ImportConfidence::High });
    assert_eq!(a.outgoing(MethodIndex(0)).len(), 1);
    assert_eq!(a.incoming(MethodIndex(0)).len(), 1);
}

#[test]
fn adjacency_multiple_edges_to_same_target() {
    let mut a = CallAdjacency::with_capacity(3);
    for src in 0..2u32 {
        a.insert(CallEdge { source: MethodIndex(src), target: MethodIndex(2), confidence: ImportConfidence::Medium });
    }
    assert_eq!(a.incoming(MethodIndex(2)).len(), 2);
}

#[test]
fn empty_call_graph_has_zero_methods() {
    let g = CallGraph::default();
    assert_eq!(g.registry.count(), 0);
}

#[test]
fn call_edge_carries_confidence_label() {
    let e = CallEdge { source: MethodIndex(0), target: MethodIndex(1), confidence: ImportConfidence::Low };
    assert_eq!(e.confidence, ImportConfidence::Low);
}

#[test]
fn registry_handles_free_function_with_no_class() {
    let mut reg = MethodRegistry::default();
    reg.intern(mk_id("a.py", None, "free_fn", 1));
    assert_eq!(reg.lookup_by_name("free_fn").len(), 1);
    assert_eq!(reg.lookup_by_class_and_name("", "free_fn").len(), 0);
}

#[test]
fn registry_determinism_same_input_produces_same_output() {
    let defs1 = vec![mk_id("a.py", Some("Foo"), "m", 1), mk_id("b.py", Some("Bar"), "m", 2)];
    let defs2 = defs1.clone();
    let r1 = MethodRegistry::from_definitions(defs1);
    let r2 = MethodRegistry::from_definitions(defs2);
    assert_eq!(r1.methods, r2.methods);
}

#[test]
fn registry_sort_is_stable_for_identical_keys_in_input_order() {
    let defs = vec![mk_id("a.py", None, "x", 1), mk_id("a.py", None, "y", 2)];
    let reg = MethodRegistry::from_definitions(defs);
    assert_eq!(reg.methods[0].name, "x");
    assert_eq!(reg.methods[1].name, "y");
}

#[test]
fn adjacency_insert_preserves_edge_confidence() {
    let mut a = CallAdjacency::with_capacity(2);
    let edge = CallEdge { source: MethodIndex(0), target: MethodIndex(1), confidence: ImportConfidence::BestEffort };
    a.insert(edge);
    assert_eq!(a.outgoing(MethodIndex(0))[0].confidence, ImportConfidence::BestEffort);
}
