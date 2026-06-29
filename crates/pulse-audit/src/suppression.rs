use std::collections::HashSet;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::finding::{kind_slug, AuditFinding, AuditKind, PatternCategory};
use pulse_config::AuditConfig;

pub struct AuditSuppression {
    categories: HashSet<String>,
    smells: HashSet<String>,
    patterns: GlobSet,
}

impl AuditSuppression {
    pub fn new() -> Self {
        Self { categories: HashSet::new(), smells: HashSet::new(), patterns: GlobSet::empty() }
    }

    pub fn from_config(cfg: Option<&AuditConfig>) -> Self {
        let Some(cfg) = cfg else { return Self::new() };
        let mut builder = GlobSetBuilder::new();
        for raw in &cfg.hide_patterns {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(glob) = GlobBuilder::new(trimmed).literal_separator(false).build() {
                builder.add(glob);
            }
        }
        let patterns = builder.build().unwrap_or_else(|_| GlobSet::empty());
        Self {
            categories: cfg.hide_categories.iter().map(|s| s.trim().to_string()).collect(),
            smells: cfg.hide_smells.iter().map(|s| s.trim().to_string()).collect(),
            patterns,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.categories.is_empty() && self.smells.is_empty() && self.patterns.is_empty()
    }

    pub fn is_hidden(&self, f: &AuditFinding) -> bool {
        match f.kind {
            AuditKind::UncategorizedPattern { .. } => pattern_hidden(self, f),
            _ => self.smells.contains(kind_slug(&f.kind)),
        }
    }
}

fn pattern_hidden(s: &AuditSuppression, f: &AuditFinding) -> bool {
    let category_hit = f.pattern_category.is_some_and(|cat| s.categories.contains(PatternCategory::slug(cat)));
    category_hit || glob_matches_text(&s.patterns, &f.representative_snippet)
}

fn glob_matches_text(set: &GlobSet, candidate: &str) -> bool {
    !set.is_empty() && set.is_match(candidate)
}
