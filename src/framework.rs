use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::buildmeta::{declared, discover, DepScope, Ecosystem};
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

thread_local! {
    static DI_CACHE: RefCell<HashMap<(PathBuf, Ecosystem), bool>> = RefCell::new(HashMap::new());
}

pub fn di_framework_declared(file_path: &str, lang: Language) -> bool {
    let Some(eco) = ecosystem_for(lang) else { return false };
    let Some(root) = declared::nearest_manifest_root(Path::new(file_path), eco) else { return false };
    DI_CACHE.with(|cache| {
        if let Some(&hit) = cache.borrow().get(&(root.clone(), eco)) {
            return hit;
        }
        let present = detect(&root, eco);
        cache.borrow_mut().insert((root, eco), present);
        present
    })
}

fn detect(root: &Path, eco: Ecosystem) -> bool {
    let direct: HashSet<String> = discover(root)
        .manifests
        .iter()
        .filter(|m| m.ecosystem == eco)
        .flat_map(|m| m.deps.iter())
        .filter(|d| d.scope == DepScope::Deployed && !d.own)
        .map(|d| normalize(&d.name))
        .collect();
    DI_FRAMEWORKS.iter().any(|f| direct.contains(&normalize(f)))
}
