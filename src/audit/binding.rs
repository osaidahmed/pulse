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

type EnvExtractor = fn(Node, &str) -> TypeEnv;
type ParentsExtractor = fn(Node, &str) -> Vec<String>;

struct LangBinder {
    lang: Language,
    method_var_types: EnvExtractor,
    class_field_types: EnvExtractor,
    class_parents: ParentsExtractor,
}

const BINDERS: &[LangBinder] = &[
    LangBinder {
        lang: Language::Java,
        method_var_types: super::binding_java::method_var_types,
        class_field_types: super::binding_java::class_field_types,
        class_parents: super::binding_java::class_parents,
    },
    LangBinder {
        lang: Language::Kotlin,
        method_var_types: super::binding_kotlin::method_var_types,
        class_field_types: super::binding_kotlin::class_field_types,
        class_parents: super::binding_kotlin::class_parents,
    },
    LangBinder {
        lang: Language::TypeScript,
        method_var_types: super::binding_typescript::method_var_types,
        class_field_types: super::binding_typescript::class_field_types,
        class_parents: super::binding_typescript::class_parents,
    },
    LangBinder {
        lang: Language::Swift,
        method_var_types: super::binding_swift::method_var_types,
        class_field_types: super::binding_swift::class_field_types,
        class_parents: super::binding_swift::class_parents,
    },
    LangBinder {
        lang: Language::CSharp,
        method_var_types: super::binding_csharp::method_var_types,
        class_field_types: super::binding_csharp::class_field_types,
        class_parents: super::binding_csharp::class_parents,
    },
    LangBinder {
        lang: Language::Rust,
        method_var_types: super::binding_rust::method_var_types,
        class_field_types: super::binding_rust::class_field_types,
        class_parents: super::binding_rust::class_parents,
    },
    LangBinder {
        lang: Language::D,
        method_var_types: super::binding_d::method_var_types,
        class_field_types: super::binding_d::class_field_types,
        class_parents: super::binding_d::class_parents,
    },
    LangBinder {
        lang: Language::Go,
        method_var_types: super::binding_go::method_var_types,
        class_field_types: super::binding_go::class_field_types,
        class_parents: super::binding_go::class_parents,
    },
    LangBinder {
        lang: Language::Cpp,
        method_var_types: super::binding_cpp::method_var_types,
        class_field_types: super::binding_cpp::class_field_types,
        class_parents: super::binding_cpp::class_parents,
    },
];

fn binder_for(lang: Language) -> Option<&'static LangBinder> {
    BINDERS.iter().find(|b| b.lang == lang)
}

pub fn method_var_types(method_node: Node, source: &str, lang: Language) -> TypeEnv {
    binder_for(lang).map_or_else(TypeEnv::new, |b| (b.method_var_types)(method_node, source))
}

pub fn class_field_types(class_node: Node, source: &str, lang: Language) -> TypeEnv {
    binder_for(lang).map_or_else(TypeEnv::new, |b| (b.class_field_types)(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str, lang: Language) -> Vec<String> {
    binder_for(lang).map_or_else(Vec::new, |b| (b.class_parents)(class_node, source))
}

pub fn supports(lang: Language) -> bool {
    binder_for(lang).is_some()
}
