use pulse::audit::binding::{self, BindingTable, ClassBinding};
use pulse::audit::call_graph::{CallGraph, MethodIdentity, MethodIndex};
use pulse::audit::call_walker::{calls_and_bindings_from, LocatedCall};
use pulse::audit::corpus::{Corpus, CorpusFile};
use pulse::audit::definitions::{definitions_from, DefinitionRecord};
use pulse::parse::Language;

fn corpus_of(files: &[(&str, &str)], lang: Language) -> (tempfile::TempDir, Corpus) {
    let dir = tempfile::tempdir().unwrap();
    let mut typed = Vec::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        typed.push((path, lang));
    }
    let corpus = Corpus::load(&typed);
    (dir, corpus)
}

fn java(content: &str) -> (tempfile::TempDir, Corpus) {
    corpus_of(&[("Sample.java", content)], Language::Java)
}

fn only_file(corpus: &Corpus) -> &CorpusFile {
    corpus.files.first().expect("one file")
}

struct Analyzed {
    defs: Vec<DefinitionRecord>,
    calls: Vec<LocatedCall>,
    table: BindingTable,
}

fn analyze(file: &CorpusFile) -> Analyzed {
    let out = calls_and_bindings_from(file);
    let defs = definitions_from(file);
    let mut table = BindingTable::default();
    for (id, env) in out.method_envs {
        table.insert_method(id, env);
    }
    for class in out.classes {
        table.insert_class(class);
    }
    Analyzed { defs, calls: out.calls, table }
}

fn caller_of(calls: &[LocatedCall], callee: &str) -> MethodIdentity {
    calls.iter().find(|c| c.call.callee_name == callee).and_then(|c| c.caller.clone()).expect("caller present")
}

fn outgoing_targets(graph: &CallGraph, caller: &MethodIdentity) -> Vec<MethodIdentity> {
    let idx = graph
        .registry
        .methods
        .iter()
        .position(|m| m == caller)
        .map(|i| MethodIndex(i as u32))
        .expect("caller in registry");
    graph.adjacency.outgoing(idx).iter().filter_map(|e| graph.registry.get(e.target).cloned()).collect()
}

fn targets_named(graph: &CallGraph, caller: &MethodIdentity, method: &str) -> Vec<MethodIdentity> {
    outgoing_targets(graph, caller).into_iter().filter(|m| m.name == method).collect()
}

#[test]
fn method_var_types_capture_locals_and_object_params() {
    let (_d, corpus) = java(
        "class Caller {\n  void run() {\n    Bar b = new Bar();\n    Baz c = make();\n  }\n  void take(Foo f, int n) {}\n  Bar make() { return null; }\n}\n",
    );
    let out = calls_and_bindings_from(only_file(&corpus));
    let run_env = out.method_envs.iter().find(|(id, _)| id.name == "run").map(|(_, e)| e).expect("run env");
    assert_eq!(run_env.get("b").map(String::as_str), Some("Bar"));
    assert_eq!(run_env.get("c").map(String::as_str), Some("Baz"));
    let take_env = out.method_envs.iter().find(|(id, _)| id.name == "take").map(|(_, e)| e).expect("take env");
    assert_eq!(take_env.get("f").map(String::as_str), Some("Foo"));
    assert!(take_env.get("n").is_none(), "primitive-typed params are not receivers");
}

#[test]
fn class_fields_and_parents_are_captured() {
    let (_d, corpus) = java(
        "class Widget extends Base implements Drawable {\n  private Bar helper;\n  Foo other;\n  void run() {}\n}\n",
    );
    let out = calls_and_bindings_from(only_file(&corpus));
    let widget = out.classes.iter().find(|c| c.name == "Widget").expect("widget class");
    assert_eq!(widget.fields.get("helper").map(String::as_str), Some("Bar"));
    assert_eq!(widget.fields.get("other").map(String::as_str), Some("Foo"));
    assert!(widget.parents.contains(&"Base".to_string()));
    assert!(widget.parents.contains(&"Drawable".to_string()));
}

#[test]
fn typed_receiver_resolves_precisely_not_by_fanout() {
    let (_d, corpus) = java(
        "class Bar { void shared() {} }\nclass Foo { void shared() {} }\nclass Caller {\n  void run() {\n    Bar b = new Bar();\n    b.shared();\n  }\n}\n",
    );
    let a = analyze(only_file(&corpus));
    let caller = caller_of(&a.calls, "shared");
    let plain = CallGraph::build(a.defs.clone(), a.calls.clone());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    assert_eq!(targets_named(&plain, &caller, "shared").len(), 2, "name fan-out reaches both classes");
    let bound_targets = targets_named(&bound, &caller, "shared");
    assert_eq!(bound_targets.len(), 1, "binding resolves to exactly the declared type");
    assert_eq!(bound_targets[0].class.as_deref(), Some("Bar"));
}

#[test]
fn inherited_method_resolves_through_ancestor() {
    let (_d, corpus) = java(
        "class Base { void greet() {} }\nclass Unrelated { void greet() {} }\nclass Derived extends Base {}\nclass Caller {\n  void run() {\n    Derived d = new Derived();\n    d.greet();\n  }\n}\n",
    );
    let a = analyze(only_file(&corpus));
    let caller = caller_of(&a.calls, "greet");
    let plain = CallGraph::build(a.defs.clone(), a.calls.clone());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    assert_eq!(targets_named(&plain, &caller, "greet").len(), 2, "name fan-out reaches base and unrelated");
    let bound_targets = targets_named(&bound, &caller, "greet");
    assert_eq!(bound_targets.len(), 1, "binding resolves up the hierarchy to the base only");
    assert_eq!(bound_targets[0].class.as_deref(), Some("Base"));
}

#[test]
fn typed_receiver_with_absent_method_is_suppressed() {
    let (_d, corpus) = java(
        "class Bar { void shared() {} }\nclass Other { void onlyHere() {} }\nclass Caller {\n  void run() {\n    Bar b = new Bar();\n    b.onlyHere();\n  }\n}\n",
    );
    let a = analyze(only_file(&corpus));
    let caller = caller_of(&a.calls, "onlyHere");
    let plain = CallGraph::build(a.defs.clone(), a.calls.clone());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    assert_eq!(targets_named(&plain, &caller, "onlyHere").len(), 1, "name fallback fabricates a wrong edge");
    assert_eq!(
        targets_named(&bound, &caller, "onlyHere").len(),
        0,
        "a known receiver type without the method emits no edge"
    );
}

#[test]
fn field_typed_receiver_resolves_precisely() {
    let (_d, corpus) = java(
        "class Bar { void shared() {} }\nclass Foo { void shared() {} }\nclass Caller {\n  private Bar helper;\n  void run() {\n    helper.shared();\n  }\n}\n",
    );
    let a = analyze(only_file(&corpus));
    let caller = caller_of(&a.calls, "shared");
    let plain = CallGraph::build(a.defs.clone(), a.calls.clone());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    assert_eq!(targets_named(&plain, &caller, "shared").len(), 2);
    let bound_targets = targets_named(&bound, &caller, "shared");
    assert_eq!(bound_targets.len(), 1);
    assert_eq!(bound_targets[0].class.as_deref(), Some("Bar"));
}

#[test]
fn non_java_languages_yield_no_bindings() {
    assert!(binding::supports(Language::Java));
    assert!(!binding::supports(Language::Python));
    let (_d, corpus) = corpus_of(
        &[("sample.py", "class Caller:\n    def run(self):\n        b = Bar()\n        b.shared()\n")],
        Language::Python,
    );
    let out = calls_and_bindings_from(only_file(&corpus));
    assert!(out.method_envs.is_empty(), "unsupported languages contribute no type env");
    assert!(out.classes.is_empty(), "unsupported languages contribute no class bindings");
}

#[test]
fn ancestor_walk_terminates_and_dedups_on_cycles() {
    let mut table = BindingTable::default();
    let class = |name: &str, parent: &str| ClassBinding {
        name: name.to_string(),
        parents: vec![parent.to_string()],
        ..Default::default()
    };
    table.insert_class(class("A", "B"));
    table.insert_class(class("B", "C"));
    table.insert_class(class("C", "A"));
    assert_eq!(table.ancestors("A"), vec!["B".to_string(), "C".to_string()]);
}
