use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;

use tree_sitter::Node;

use crate::parse::Language;

use super::call_graph::MethodIdentity;

pub type TypeEnv = BTreeMap<String, String>;

#[derive(Debug, Default, Clone)]
pub struct ClassBinding {
    pub file: PathBuf,
    pub name: String,
    pub parents: Vec<String>,
    pub fields: TypeEnv,
}

#[derive(Debug, Default)]
pub struct BindingTable {
    methods: BTreeMap<MethodIdentity, TypeEnv>,
    fields: BTreeMap<(PathBuf, String), TypeEnv>,
    parents: BTreeMap<String, Vec<String>>,
    poisoned: BTreeSet<String>,
    known: BTreeSet<String>,
}

const MAX_ANCESTOR_WALK: usize = 64;

impl BindingTable {
    pub fn insert_method(&mut self, identity: MethodIdentity, env: TypeEnv) {
        if !env.is_empty() {
            self.methods.insert(identity, env);
        }
    }

    pub fn insert_class(&mut self, class: ClassBinding) {
        self.known.insert(class.name.clone());
        self.record_parents(&class.name, class.parents);
        if !class.fields.is_empty() {
            self.fields.insert((class.file, class.name), class.fields);
        }
    }

    fn record_parents(&mut self, name: &str, parents: Vec<String>) {
        if parents.is_empty() {
            return;
        }
        match self.parents.get(name) {
            Some(existing) if *existing != parents => {
                self.poisoned.insert(name.to_string());
            }
            Some(_) => {}
            None => {
                self.parents.insert(name.to_string(), parents);
            }
        }
    }

    pub fn var_type(&self, caller: &MethodIdentity, name: &str) -> Option<&str> {
        self.methods.get(caller)?.get(name).map(String::as_str)
    }

    pub fn field_type(&self, caller: &MethodIdentity, name: &str) -> Option<&str> {
        let class = caller.class.as_ref()?;
        self.fields.get(&(caller.file.clone(), class.clone()))?.get(name).map(String::as_str)
    }

    pub fn is_known_class(&self, name: &str) -> bool {
        self.known.contains(name)
    }

    pub fn ancestors(&self, class: &str) -> Vec<String> {
        if self.poisoned.contains(class) {
            return Vec::new();
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut out: Vec<String> = Vec::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        seen.insert(class);
        queue.push_back(class);
        while let Some(cur) = queue.pop_front() {
            if self.poisoned.contains(cur) {
                continue;
            }
            let Some(parents) = self.parents.get(cur) else { continue };
            for parent in parents {
                if seen.insert(parent.as_str()) {
                    out.push(parent.clone());
                    queue.push_back(parent.as_str());
                }
            }
            if out.len() >= MAX_ANCESTOR_WALK {
                break;
            }
        }
        out
    }
}

fn for_java<T: Default>(lang: Language, extract: impl FnOnce() -> T) -> T {
    if matches!(lang, Language::Java) {
        extract()
    } else {
        T::default()
    }
}

pub fn method_var_types(method_node: Node, source: &str, lang: Language) -> TypeEnv {
    for_java(lang, || super::binding_java::method_var_types(method_node, source))
}

pub fn class_field_types(class_node: Node, source: &str, lang: Language) -> TypeEnv {
    for_java(lang, || super::binding_java::class_field_types(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str, lang: Language) -> Vec<String> {
    for_java(lang, || super::binding_java::class_parents(class_node, source))
}

pub fn supports(lang: Language) -> bool {
    matches!(lang, Language::Java)
}
