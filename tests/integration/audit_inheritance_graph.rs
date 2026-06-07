use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::{ClassIndex, ClassRegistry};
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::inheritance::{ancestors_of, build_inheritance_graph, descendants_of};

fn def(file: &str, class: &str, parent: Option<&str>) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: Some(class.to_string()),
            name: "m".to_string(),
            line: 1,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: parent.map(String::from),
        is_constructor: false,
    }
}

fn build(defs: Vec<DefinitionRecord>) -> (ClassRegistry, pulse::audit::inheritance::InheritanceGraph) {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    (registry, inh)
}

#[test]
fn empty_registry_yields_empty_graph() {
    let (_, inh) = build(Vec::new());
    assert!(inh.parent_of.is_empty());
    assert!(inh.children_of.is_empty());
}

#[test]
fn single_class_has_no_parent() {
    let (reg, inh) = build(vec![def("a.py", "Foo", None)]);
    let foo = *reg.by_name.get("Foo").unwrap().first().unwrap();
    assert!(inh.parents(foo).is_empty());
    assert!(inh.children(foo).is_empty());
}

#[test]
fn child_with_parent_has_parent_edge() {
    let defs = vec![def("a.py", "Foo", None), def("b.py", "Bar", Some("Foo"))];
    let (reg, inh) = build(defs);
    let foo = *reg.by_name.get("Foo").unwrap().first().unwrap();
    let bar = *reg.by_name.get("Bar").unwrap().first().unwrap();
    assert_eq!(inh.parents(bar), &[foo]);
    assert_eq!(inh.children(foo), &[bar]);
}

#[test]
fn three_level_chain_descendants() {
    let defs = vec![def("a.py", "A", None), def("b.py", "B", Some("A")), def("c.py", "C", Some("B"))];
    let (reg, inh) = build(defs);
    let a = *reg.by_name.get("A").unwrap().first().unwrap();
    let b = *reg.by_name.get("B").unwrap().first().unwrap();
    let c = *reg.by_name.get("C").unwrap().first().unwrap();
    let descendants = descendants_of(&inh, a);
    assert!(descendants.contains(&b));
    assert!(descendants.contains(&c));
    assert_eq!(descendants.len(), 2);
}

#[test]
fn three_level_chain_ancestors() {
    let defs = vec![def("a.py", "A", None), def("b.py", "B", Some("A")), def("c.py", "C", Some("B"))];
    let (reg, inh) = build(defs);
    let a = *reg.by_name.get("A").unwrap().first().unwrap();
    let b = *reg.by_name.get("B").unwrap().first().unwrap();
    let c = *reg.by_name.get("C").unwrap().first().unwrap();
    let ancestors = ancestors_of(&inh, c);
    assert!(ancestors.contains(&a));
    assert!(ancestors.contains(&b));
    assert_eq!(ancestors.len(), 2);
}

#[test]
fn cyclic_inheritance_does_not_loop_forever() {
    let defs = vec![def("a.py", "A", Some("B")), def("b.py", "B", Some("A"))];
    let (reg, inh) = build(defs);
    let a = *reg.by_name.get("A").unwrap().first().unwrap();
    let descendants = descendants_of(&inh, a);
    assert!(descendants.len() <= 2);
}

#[test]
fn parent_in_different_file_resolves() {
    let defs = vec![def("foo.py", "Foo", None), def("bar.py", "Bar", Some("Foo"))];
    let (reg, inh) = build(defs);
    let foo = *reg.by_name.get("Foo").unwrap().first().unwrap();
    let bar = *reg.by_name.get("Bar").unwrap().first().unwrap();
    assert!(inh.parents(bar).contains(&foo));
}

#[test]
fn parent_resolves_to_multiple_when_ambiguous() {
    let defs = vec![def("a.py", "Foo", None), def("b.py", "Foo", None), def("c.py", "Bar", Some("Foo"))];
    let (reg, inh) = build(defs);
    let bar = *reg.by_name.get("Bar").unwrap().first().unwrap();
    assert_eq!(inh.parents(bar).len(), 2);
}

#[test]
fn external_parent_yields_no_edge() {
    let defs = vec![def("a.py", "Bar", Some("ExternalLib"))];
    let (reg, inh) = build(defs);
    let bar = *reg.by_name.get("Bar").unwrap().first().unwrap();
    assert!(inh.parents(bar).is_empty());
}

#[test]
fn descendants_of_isolated_class_is_empty() {
    let defs = vec![def("a.py", "Foo", None)];
    let (reg, inh) = build(defs);
    let foo = *reg.by_name.get("Foo").unwrap().first().unwrap();
    assert!(descendants_of(&inh, foo).is_empty());
}

#[test]
fn ancestors_of_root_is_empty() {
    let defs = vec![def("a.py", "Foo", None)];
    let (reg, inh) = build(defs);
    let foo = *reg.by_name.get("Foo").unwrap().first().unwrap();
    assert!(ancestors_of(&inh, foo).is_empty());
}

#[test]
fn nonexistent_class_index_returns_empty() {
    let (_, inh) = build(vec![def("a.py", "Foo", None)]);
    let bogus = ClassIndex(999);
    assert!(inh.parents(bogus).is_empty());
    assert!(inh.children(bogus).is_empty());
    assert!(descendants_of(&inh, bogus).is_empty());
    assert!(ancestors_of(&inh, bogus).is_empty());
}

#[test]
fn determinism_two_builds_same_edges() {
    let defs = vec![def("a.py", "A", None), def("b.py", "B", Some("A"))];
    let (_, inh1) = build(defs.clone());
    let (_, inh2) = build(defs);
    assert_eq!(inh1.parent_of, inh2.parent_of);
    assert_eq!(inh1.children_of, inh2.children_of);
}

#[test]
fn diamond_hierarchy_descendants() {
    let defs = vec![
        def("a.py", "Top", None),
        def("b.py", "Left", Some("Top")),
        def("c.py", "Right", Some("Top")),
        def("d.py", "Bottom", Some("Left")),
    ];
    let (reg, inh) = build(defs);
    let top = *reg.by_name.get("Top").unwrap().first().unwrap();
    let descendants = descendants_of(&inh, top);
    assert_eq!(descendants.len(), 3);
}
