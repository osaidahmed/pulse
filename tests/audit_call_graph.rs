use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::call_walker::LocatedCall;
use pulse::audit::calls::RawCall;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::finding::ImportConfidence;

fn def(file: &str, class: Option<&str>, name: &str, line: u32) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: class.map(String::from),
            name: name.to_string(),
            line,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: None,
        is_constructor: false,
    }
}

fn call_to(
    caller_def: &DefinitionRecord,
    callee: &str,
    receiver: Option<&str>,
) -> LocatedCall {
    LocatedCall {
        call: RawCall {
            callee_name: callee.to_string(),
            receiver_hint: receiver.map(String::from),
            line: caller_def.identity.line + 1,
        },
        caller: Some(caller_def.identity.clone()),
        file: caller_def.identity.file.clone(),
    }
}

#[test]
fn build_with_no_calls_produces_empty_adjacency() {
    let defs = vec![def("a.py", None, "f", 1), def("b.py", None, "g", 2)];
    let graph = CallGraph::build(defs, Vec::new());
    assert_eq!(graph.registry.count(), 2);
    let zero = pulse::audit::call_graph::MethodIndex(0);
    assert_eq!(graph.adjacency.outgoing(zero).len(), 0);
}

#[test]
fn build_resolves_class_hint_to_high_confidence() {
    let target = def("a.py", Some("Foo"), "method", 1);
    let caller = def("b.py", Some("Bar"), "caller", 2);
    let calls = vec![call_to(&caller, "method", Some("Foo"))];
    let graph = CallGraph::build(vec![target, caller], calls);
    let foo_idx = graph.registry.lookup_by_class_and_name("Foo", "method")[0];
    let incoming = graph.adjacency.incoming(foo_idx);
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].confidence, ImportConfidence::High);
}

#[test]
fn build_resolves_unique_name_to_medium_confidence() {
    let target = def("a.py", None, "unique_helper", 1);
    let caller = def("b.py", None, "caller", 2);
    let calls = vec![call_to(&caller, "unique_helper", None)];
    let graph = CallGraph::build(vec![target, caller], calls);
    let target_idx = graph.registry.lookup_by_name("unique_helper")[0];
    let incoming = graph.adjacency.incoming(target_idx);
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].confidence, ImportConfidence::Medium);
}

#[test]
fn build_resolves_ambiguous_name_to_low_confidence() {
    let t1 = def("a.py", Some("A"), "shared", 1);
    let t2 = def("b.py", Some("B"), "shared", 1);
    let caller = def("c.py", None, "caller", 2);
    let calls = vec![call_to(&caller, "shared", None)];
    let graph = CallGraph::build(vec![t1, t2, caller], calls);
    let candidates = graph.registry.lookup_by_name("shared");
    assert_eq!(candidates.len(), 2);
    let total_incoming: usize = candidates
        .iter()
        .map(|i| graph.adjacency.incoming(*i).len())
        .sum();
    assert_eq!(total_incoming, 2);
    for c in candidates {
        for edge in graph.adjacency.incoming(*c) {
            assert_eq!(edge.confidence, ImportConfidence::Low);
        }
    }
}

#[test]
fn build_drops_calls_with_no_target() {
    let caller = def("a.py", None, "caller", 1);
    let calls = vec![call_to(&caller, "nonexistent", None)];
    let graph = CallGraph::build(vec![caller], calls);
    let caller_idx = graph.registry.lookup_by_name("caller")[0];
    assert_eq!(graph.adjacency.outgoing(caller_idx).len(), 0);
}

#[test]
fn build_drops_calls_with_no_caller_method() {
    let target = def("a.py", None, "target", 1);
    let bogus = def("any.py", None, "anyone", 99);
    let mut call = call_to(&bogus, "target", None);
    call.caller = None;
    let graph = CallGraph::build(vec![target], vec![call]);
    let target_idx = graph.registry.lookup_by_name("target")[0];
    assert_eq!(graph.adjacency.incoming(target_idx).len(), 0);
}

#[test]
fn self_receiver_resolves_to_caller_class() {
    let foo_helper = def("a.py", Some("Foo"), "helper", 1);
    let foo_caller = def("a.py", Some("Foo"), "caller", 5);
    let bar_helper = def("b.py", Some("Bar"), "helper", 1);
    let calls = vec![call_to(&foo_caller, "helper", Some("self"))];
    let graph = CallGraph::build(vec![foo_helper, foo_caller, bar_helper], calls);
    let foo_idx = graph.registry.lookup_by_class_and_name("Foo", "helper")[0];
    let bar_idx = graph.registry.lookup_by_class_and_name("Bar", "helper")[0];
    assert_eq!(graph.adjacency.incoming(foo_idx).len(), 1);
    assert_eq!(graph.adjacency.incoming(bar_idx).len(), 0);
}

#[test]
fn this_receiver_resolves_to_caller_class() {
    let helper = def("a.java", Some("Foo"), "helper", 1);
    let caller = def("a.java", Some("Foo"), "caller", 5);
    let calls = vec![call_to(&caller, "helper", Some("this"))];
    let graph = CallGraph::build(vec![helper, caller], calls);
    let helper_idx = graph.registry.lookup_by_class_and_name("Foo", "helper")[0];
    assert_eq!(graph.adjacency.incoming(helper_idx).len(), 1);
}

#[test]
fn self_call_emits_self_loop_edge() {
    let recursive = def("a.py", Some("Foo"), "recursive", 1);
    let calls = vec![call_to(&recursive, "recursive", Some("self"))];
    let graph = CallGraph::build(vec![recursive], calls);
    let idx = graph.registry.lookup_by_class_and_name("Foo", "recursive")[0];
    assert_eq!(graph.adjacency.outgoing(idx).len(), 1);
    assert_eq!(graph.adjacency.outgoing(idx)[0].target, idx);
}

#[test]
fn registry_methods_sorted_deterministically() {
    let defs1 = vec![
        def("z.py", None, "x", 1),
        def("a.py", None, "y", 1),
        def("m.py", None, "z", 1),
    ];
    let defs2 = defs1.clone();
    let g1 = CallGraph::build(defs1, Vec::new());
    let g2 = CallGraph::build(defs2, Vec::new());
    let n1: Vec<_> = g1.registry.methods.iter().map(|m| m.file.clone()).collect();
    let n2: Vec<_> = g2.registry.methods.iter().map(|m| m.file.clone()).collect();
    assert_eq!(n1, n2);
}

#[test]
fn empty_graph_has_zero_methods() {
    let graph = CallGraph::build(Vec::new(), Vec::new());
    assert_eq!(graph.registry.count(), 0);
}

#[test]
fn high_confidence_takes_precedence_over_name_match() {
    let foo = def("a.py", Some("Foo"), "method", 1);
    let bar = def("b.py", Some("Bar"), "method", 1);
    let caller = def("c.py", Some("Caller"), "caller", 5);
    let calls = vec![call_to(&caller, "method", Some("Foo"))];
    let graph = CallGraph::build(vec![foo, bar, caller], calls);
    let foo_idx = graph.registry.lookup_by_class_and_name("Foo", "method")[0];
    let bar_idx = graph.registry.lookup_by_class_and_name("Bar", "method")[0];
    assert_eq!(graph.adjacency.incoming(foo_idx).len(), 1);
    assert_eq!(graph.adjacency.incoming(bar_idx).len(), 0);
    assert_eq!(graph.adjacency.incoming(foo_idx)[0].confidence, ImportConfidence::High);
}

#[test]
fn ambiguous_class_hint_falls_through_to_name_match() {
    let x = def("a.py", Some("X"), "method", 1);
    let y = def("b.py", Some("Y"), "method", 1);
    let caller = def("c.py", None, "caller", 5);
    let calls = vec![call_to(&caller, "method", Some("Z"))];
    let graph = CallGraph::build(vec![x, y, caller], calls);
    let candidates = graph.registry.lookup_by_name("method");
    let total: usize = candidates.iter().map(|i| graph.adjacency.incoming(*i).len()).sum();
    assert_eq!(total, 2);
}

#[test]
fn many_callers_to_same_target_all_resolve() {
    let target_def = def("target.py", Some("T"), "h", 1);
    let mut defs = vec![target_def];
    let mut calls = Vec::new();
    for i in 0..50 {
        let caller_def = def(&format!("c{i}.py"), Some(&format!("C{i}")), &format!("m{i}"), 1);
        calls.push(call_to(&caller_def, "h", Some("T")));
        defs.push(caller_def);
    }
    let graph = CallGraph::build(defs, calls);
    let t = graph.registry.lookup_by_class_and_name("T", "h")[0];
    assert_eq!(graph.adjacency.incoming(t).len(), 50);
}

#[test]
fn class_hint_with_two_same_named_classes_drops_to_medium() {
    let foo_a = def("a.py", Some("Foo"), "method", 1);
    let foo_b = def("b.py", Some("Foo"), "method", 1);
    let caller = def("c.py", Some("Caller"), "trigger", 5);
    let calls = vec![call_to(&caller, "method", Some("Foo"))];
    let graph = CallGraph::build(vec![foo_a, foo_b, caller], calls);
    let candidates = graph.registry.lookup_by_class_and_name("Foo", "method");
    assert_eq!(candidates.len(), 2, "both Foo classes should be candidates");
    for c in candidates {
        let incoming = graph.adjacency.incoming(*c);
        assert_eq!(incoming.len(), 1);
        assert_eq!(
            incoming[0].confidence,
            ImportConfidence::Medium,
            "two same-named class candidates should drop confidence to Medium"
        );
    }
}

#[test]
fn class_hint_with_unique_match_stays_high() {
    let target = def("a.py", Some("Bar"), "method", 1);
    let caller = def("b.py", Some("Caller"), "trigger", 5);
    let calls = vec![call_to(&caller, "method", Some("Bar"))];
    let graph = CallGraph::build(vec![target, caller], calls);
    let target_idx = graph.registry.lookup_by_class_and_name("Bar", "method")[0];
    let incoming = graph.adjacency.incoming(target_idx);
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].confidence, ImportConfidence::High);
}

#[test]
fn three_same_named_classes_all_get_medium() {
    let a = def("a.py", Some("Util"), "run", 1);
    let b = def("b.py", Some("Util"), "run", 1);
    let c = def("c.py", Some("Util"), "run", 1);
    let caller = def("d.py", Some("Caller"), "trigger", 5);
    let calls = vec![call_to(&caller, "run", Some("Util"))];
    let graph = CallGraph::build(vec![a, b, c, caller], calls);
    let candidates = graph.registry.lookup_by_class_and_name("Util", "run");
    assert_eq!(candidates.len(), 3);
    for cand in candidates {
        for edge in graph.adjacency.incoming(*cand) {
            assert_eq!(edge.confidence, ImportConfidence::Medium);
        }
    }
}
