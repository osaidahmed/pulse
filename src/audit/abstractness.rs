use std::path::Path;

use tree_sitter::Node;

use crate::parse::{self, Language};

use super::finding::ImportConfidence;
use super::martin::AbstractnessRecord;

#[derive(Debug, Clone, Copy)]
struct AbstractnessProfile {
    abstract_kinds: &'static [&'static str],
    concrete_kinds: &'static [&'static str],
    confidence: ImportConfidence,
}

const PROFILE_TABLE: &[(Language, AbstractnessProfile)] = &[
    (
        Language::Java,
        AbstractnessProfile {
            abstract_kinds: &["interface_declaration"],
            concrete_kinds: &["class_declaration", "enum_declaration", "record_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::CSharp,
        AbstractnessProfile {
            abstract_kinds: &["interface_declaration"],
            concrete_kinds: &["class_declaration", "struct_declaration", "enum_declaration", "record_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::Kotlin,
        AbstractnessProfile {
            abstract_kinds: &["interface_declaration"],
            concrete_kinds: &["class_declaration", "object_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::Swift,
        AbstractnessProfile {
            abstract_kinds: &["protocol_declaration"],
            concrete_kinds: &["class_declaration", "struct_declaration", "enum_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::Rust,
        AbstractnessProfile {
            abstract_kinds: &["trait_item"],
            concrete_kinds: &["struct_item", "enum_item", "union_item"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::Go,
        AbstractnessProfile {
            abstract_kinds: &["interface_type"],
            concrete_kinds: &["struct_type", "type_declaration"],
            confidence: ImportConfidence::Medium,
        },
    ),
    (
        Language::Haskell,
        AbstractnessProfile {
            abstract_kinds: &["class_declaration"],
            concrete_kinds: &["data_type_declaration", "newtype_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::TypeScript,
        AbstractnessProfile {
            abstract_kinds: &["interface_declaration", "abstract_class_declaration"],
            concrete_kinds: &["class_declaration", "type_alias_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::JavaScript,
        AbstractnessProfile {
            abstract_kinds: &[],
            concrete_kinds: &["class_declaration"],
            confidence: ImportConfidence::BestEffort,
        },
    ),
    (
        Language::Php,
        AbstractnessProfile {
            abstract_kinds: &["interface_declaration"],
            concrete_kinds: &["class_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::Ruby,
        AbstractnessProfile {
            abstract_kinds: &["module"],
            concrete_kinds: &["class"],
            confidence: ImportConfidence::Medium,
        },
    ),
    (
        Language::Python,
        AbstractnessProfile {
            abstract_kinds: &[],
            concrete_kinds: &["class_definition"],
            confidence: ImportConfidence::Medium,
        },
    ),
    (
        Language::ObjectiveC,
        AbstractnessProfile {
            abstract_kinds: &["protocol_declaration"],
            concrete_kinds: &["class_implementation", "class_interface"],
            confidence: ImportConfidence::Medium,
        },
    ),
    (
        Language::D,
        AbstractnessProfile {
            abstract_kinds: &["interface_declaration"],
            concrete_kinds: &["class_declaration", "struct_declaration", "enum_declaration"],
            confidence: ImportConfidence::High,
        },
    ),
    (
        Language::Groovy,
        AbstractnessProfile {
            abstract_kinds: &["interface_declaration"],
            concrete_kinds: &["class_declaration", "enum_declaration"],
            confidence: ImportConfidence::Medium,
        },
    ),
    (
        Language::Cpp,
        AbstractnessProfile {
            abstract_kinds: &[],
            concrete_kinds: &["class_specifier", "struct_specifier"],
            confidence: ImportConfidence::BestEffort,
        },
    ),
    (
        Language::C,
        AbstractnessProfile {
            abstract_kinds: &[],
            concrete_kinds: &["struct_specifier", "union_specifier", "enum_specifier"],
            confidence: ImportConfidence::BestEffort,
        },
    ),
    (
        Language::Zig,
        AbstractnessProfile {
            abstract_kinds: &[],
            concrete_kinds: &["struct_declaration"],
            confidence: ImportConfidence::BestEffort,
        },
    ),
];

pub fn abstractness_for_file(path: &Path, lang: Language) -> AbstractnessRecord {
    let Ok(source) = std::fs::read_to_string(path) else {
        return unmeasured();
    };
    let tree = parse::parse_guarded(&source, lang);
    abstractness_from_parsed(tree.as_ref(), lang)
}

pub fn abstractness_from_parsed(tree: Option<&tree_sitter::Tree>, lang: Language) -> AbstractnessRecord {
    let Some(profile) = profile_for_lang(lang) else {
        return unmeasured();
    };
    let Some(tree) = tree else {
        return unmeasured();
    };
    let mut counts = TypeCounts::default();
    visit(tree.root_node(), &profile, &mut counts);
    if counts.total() == 0 {
        return unmeasured();
    }
    AbstractnessRecord { abstractness: Some(counts.ratio()), confidence: profile.confidence }
}

fn profile_for_lang(lang: Language) -> Option<AbstractnessProfile> {
    PROFILE_TABLE.iter().find(|(l, _)| *l == lang).map(|(_, p)| *p)
}

pub fn covers(lang: Language) -> bool {
    profile_for_lang(lang).is_some()
}

fn unmeasured() -> AbstractnessRecord {
    AbstractnessRecord { abstractness: None, confidence: ImportConfidence::NaAbstraction }
}

#[derive(Default)]
struct TypeCounts {
    abstract_count: u32,
    concrete_count: u32,
}

impl TypeCounts {
    fn total(&self) -> u32 {
        self.abstract_count + self.concrete_count
    }

    fn ratio(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        f64::from(self.abstract_count) / f64::from(total)
    }
}

fn visit(node: Node, profile: &AbstractnessProfile, counts: &mut TypeCounts) {
    let kind = node.kind();
    if profile.abstract_kinds.contains(&kind) {
        counts.abstract_count += 1;
    } else if profile.concrete_kinds.contains(&kind) {
        counts.concrete_count += 1;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, profile, counts);
    }
}
