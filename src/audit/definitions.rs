use std::path::Path;

use crate::parse::{self, Language};
use crate::walk::FunctionMetrics;

use super::call_graph::MethodIdentity;

#[derive(Debug, Clone)]
pub struct DefinitionRecord {
    pub identity: MethodIdentity,
    pub cc: u32,
    pub field_accesses: Vec<String>,
    pub foreign_field_accesses: Vec<(String, String)>,
    pub parent_class: Option<String>,
    pub is_constructor: bool,
}

pub fn definitions_for_file(path: &Path, lang: Language) -> Vec<DefinitionRecord> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Some(metrics) = parse::parse_and_walk_guarded(&source, lang) else {
        return Vec::new();
    };
    metrics.functions.into_iter().map(|f| record_from_metrics(path, f)).collect()
}

pub fn definitions_from(file: &super::corpus::CorpusFile) -> Vec<DefinitionRecord> {
    let (Some(source), Some(tree)) = (file.source.as_ref(), file.tree.as_ref()) else {
        return Vec::new();
    };
    let Some(metrics) = parse::walk_guarded_shared(tree, source, file.lang) else {
        return Vec::new();
    };
    metrics.functions.into_iter().map(|f| record_from_metrics(&file.path, f)).collect()
}

fn record_from_metrics(path: &Path, f: FunctionMetrics) -> DefinitionRecord {
    let bare_name = strip_class_prefix(&f.name, f.class_name.as_deref());
    DefinitionRecord {
        identity: MethodIdentity { file: path.to_path_buf(), class: f.class_name, name: bare_name, line: f.start_line },
        cc: f.cc,
        field_accesses: f.field_accesses,
        foreign_field_accesses: f.foreign_field_accesses,
        parent_class: f.parent_class,
        is_constructor: f.is_constructor,
    }
}

fn strip_class_prefix(name: &str, class: Option<&str>) -> String {
    let Some(class) = class else { return name.to_string() };
    let prefix = format!("{class}.");
    name.strip_prefix(&prefix).unwrap_or(name).to_string()
}
