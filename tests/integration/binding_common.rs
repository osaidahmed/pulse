#![allow(dead_code)]

use pulse::audit::binding::BindingTable;
use pulse::audit::call_graph::{CallGraph, MethodIdentity, MethodIndex};
use pulse::audit::call_walker::{calls_and_bindings_from, LocatedCall};
use pulse::audit::corpus::{Corpus, CorpusFile};
use pulse::audit::definitions::{definitions_from, DefinitionRecord};
use pulse::parse::Language;

pub fn corpus_of(files: &[(&str, &str)], lang: Language) -> (tempfile::TempDir, Corpus) {
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

pub fn one_source(content: &str, name: &str, lang: Language) -> (tempfile::TempDir, Corpus) {
    corpus_of(&[(name, content)], lang)
}

pub fn only_file(corpus: &Corpus) -> &CorpusFile {
    corpus.files.first().expect("one file")
}

pub struct Analyzed {
    pub defs: Vec<DefinitionRecord>,
    pub calls: Vec<LocatedCall>,
    pub table: BindingTable,
}

pub fn analyze(file: &CorpusFile) -> Analyzed {
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

pub fn analyze_corpus(corpus: &Corpus) -> Analyzed {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    let mut table = BindingTable::default();
    for file in &corpus.files {
        defs.extend(definitions_from(file));
        let out = calls_and_bindings_from(file);
        calls.extend(out.calls);
        for (id, env) in out.method_envs {
            table.insert_method(id, env);
        }
        for class in out.classes {
            table.insert_class(class);
        }
    }
    Analyzed { defs, calls, table }
}

pub fn caller_of(calls: &[LocatedCall], callee: &str) -> MethodIdentity {
    calls.iter().find(|c| c.call.callee_name == callee).and_then(|c| c.caller.clone()).expect("caller present")
}

pub fn outgoing_targets(graph: &CallGraph, caller: &MethodIdentity) -> Vec<MethodIdentity> {
    let idx = graph
        .registry
        .methods
        .iter()
        .position(|m| m == caller)
        .map(|i| MethodIndex(i as u32))
        .expect("caller in registry");
    graph.adjacency.outgoing(idx).iter().filter_map(|e| graph.registry.get(e.target).cloned()).collect()
}

pub fn targets_named(graph: &CallGraph, caller: &MethodIdentity, method: &str) -> Vec<MethodIdentity> {
    outgoing_targets(graph, caller).into_iter().filter(|m| m.name == method).collect()
}

pub fn method_env<'a>(
    out: &'a pulse::audit::call_walker::CallWalkOut,
    name: &str,
) -> Option<&'a pulse::audit::binding::TypeEnv> {
    out.method_envs.iter().find(|(id, _)| id.name == name).map(|(_, e)| e)
}

pub fn class_binding<'a>(
    out: &'a pulse::audit::call_walker::CallWalkOut,
    name: &str,
) -> Option<&'a pulse::audit::binding::ClassBinding> {
    out.classes.iter().find(|c| c.name == name)
}
