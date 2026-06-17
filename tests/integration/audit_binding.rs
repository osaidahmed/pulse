use pulse::audit::binding::{self, BindingTable, ClassBinding};
use pulse::audit::call_graph::CallGraph;
use pulse::audit::call_walker::calls_and_bindings_from;
use pulse::audit::corpus::Corpus;
use pulse::parse::Language;

use crate::binding_common::{analyze, analyze_corpus, caller_of, corpus_of, only_file, targets_named};

fn java(content: &str) -> (tempfile::TempDir, Corpus) {
    corpus_of(&[("Sample.java", content)], Language::Java)
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
fn var_local_is_not_bound_and_recovers_via_name_fallback() {
    let (_d, corpus) = java(
        "class Bar { void shared() {} }\nclass Caller {\n  void run() {\n    var b = new Bar();\n    b.shared();\n  }\n}\n",
    );
    let file = only_file(&corpus);
    let out = calls_and_bindings_from(file);
    let run_env = out.method_envs.iter().find(|(id, _)| id.name == "run").map(|(_, e)| e).expect("run env");
    assert!(run_env.get("b").is_none(), "a `var` local is not bound to the placeholder type name");
    let a = analyze(file);
    let caller = caller_of(&a.calls, "shared");
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    assert_eq!(targets_named(&bound, &caller, "shared").len(), 1, "the var receiver still resolves via name fallback");
}

#[test]
fn varargs_parameter_is_bound() {
    let (_d, corpus) = java("class Caller {\n  void take(Bar... items) {}\n}\n");
    let out = calls_and_bindings_from(only_file(&corpus));
    let take_env = out.method_envs.iter().find(|(id, _)| id.name == "take").map(|(_, e)| e).expect("take env");
    assert_eq!(take_env.get("items").map(String::as_str), Some("Bar"));
}

#[test]
fn interface_list_has_no_phantom_comma_parent() {
    let (_d, corpus) = java("class Widget implements Drawable, Serializable {\n  void run() {}\n}\n");
    let out = calls_and_bindings_from(only_file(&corpus));
    let widget = out.classes.iter().find(|c| c.name == "Widget").expect("widget");
    assert_eq!(widget.parents, vec!["Drawable".to_string(), "Serializable".to_string()]);
}

#[test]
fn conflicting_same_name_locals_are_unbound() {
    let (_d, corpus) = java(
        "class Caller {\n  void m() {\n    { Foo x = new Foo(); x.run(); }\n    { Bar x = new Bar(); x.run(); }\n  }\n}\n",
    );
    let out = calls_and_bindings_from(only_file(&corpus));
    let m_env = out.method_envs.iter().find(|(id, _)| id.name == "m").map(|(_, e)| e).expect("m env");
    assert!(
        m_env.get("x").is_none(),
        "a name declared with two types in one method is poisoned, not arbitrarily bound"
    );
}

#[test]
fn class_type_parameter_does_not_bind_a_field() {
    let (_d, corpus) = java("class Box<Foo> {\n  Foo item;\n  void use() {}\n}\nclass Foo { void run() {} }\n");
    let out = calls_and_bindings_from(only_file(&corpus));
    let box_class = out.classes.iter().find(|c| c.name == "Box").expect("box");
    assert!(
        !box_class.fields.contains_key("item"),
        "a field typed by a class type parameter is not bound to the same-named concrete class"
    );
}

#[test]
fn object_returning_caller_resolves_its_internal_calls() {
    let (_d, corpus) = java(
        "class Helper { void process() {} }\nclass Repo {\n  Bar lookup() {\n    Helper h = new Helper();\n    h.process();\n    return null;\n  }\n}\n",
    );
    let a = analyze(only_file(&corpus));
    let caller = caller_of(&a.calls, "process");
    assert_eq!(caller.name, "lookup", "the caller is keyed by its method name, not its object return type");
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    let targets = targets_named(&bound, &caller, "process");
    assert_eq!(targets.len(), 1, "the call inside an object-returning method now resolves");
    assert_eq!(targets[0].class.as_deref(), Some("Helper"));
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
fn cross_file_same_name_target_is_suppressed() {
    let (_d, corpus) = corpus_of(
        &[
            ("A.java", "class Bar { void shared() {} }\n"),
            ("B.java", "class Bar { void shared() {} }\n"),
            ("C.java", "class Caller {\n  void run() {\n    Bar b = new Bar();\n    b.shared();\n  }\n}\n"),
        ],
        Language::Java,
    );
    let a = analyze_corpus(&corpus);
    let caller = caller_of(&a.calls, "shared");
    let plain = CallGraph::build(a.defs.clone(), a.calls.clone());
    let bound = CallGraph::build_with_bindings(a.defs, a.calls, &a.table);
    assert_eq!(targets_named(&plain, &caller, "shared").len(), 2, "name fan-out reaches both same-named Bars");
    assert_eq!(
        targets_named(&bound, &caller, "shared").len(),
        0,
        "a same-name class spanning two files is ambiguous and emits no edge"
    );
}

#[test]
fn divergent_same_name_parents_poison_the_ancestor_walk() {
    let widget =
        |parent: &str| ClassBinding { name: "Widget".into(), parents: vec![parent.into()], ..Default::default() };
    let mut poisoned = BindingTable::default();
    poisoned.insert_class(widget("Base"));
    poisoned.insert_class(widget("Other"));
    assert!(poisoned.ancestors("Widget").is_empty(), "divergent same-name parents suppress the ancestor walk");

    let mut agreed = BindingTable::default();
    agreed.insert_class(widget("Base"));
    agreed.insert_class(widget("Base"));
    assert_eq!(agreed.ancestors("Widget"), vec!["Base".to_string()], "identical parents are not poisoned");
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
