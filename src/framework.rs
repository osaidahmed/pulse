use std::collections::HashSet;
use std::path::Path;

use crate::buildmeta::declared;
use crate::import_check::{ecosystem_for, normalize};
use crate::parse::Language;

const DI_FRAMEWORKS: &[&str] = &[
    "@nestjs/common",
    "@nestjs/core",
    "@angular/core",
    "inversify",
    "tsyringe",
    "typedi",
    "dependency-injector",
    "injector",
    "dry-auto_inject",
    "dry-container",
];

pub fn di_framework_declared(file_path: &str, lang: Language) -> bool {
    let Some(eco) = ecosystem_for(lang) else { return false };
    let Some(root) = declared::nearest_manifest_root(Path::new(file_path), eco) else { return false };
    let declared: HashSet<String> = declared::declared_names_for(&root).iter().map(|n| normalize(n)).collect();
    DI_FRAMEWORKS.iter().any(|f| declared.contains(&normalize(f)))
}
