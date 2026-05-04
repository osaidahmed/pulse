use std::fmt::Write;
use std::path::Path;

use super::finding::ImportConfidence;

pub struct ListLayout<'a> {
    pub prefix_first: &'a str,
    pub prefix_rest: &'a str,
    pub cap: usize,
}

pub fn write_capped_list<F: Fn(usize) -> String>(
    out: &mut String,
    layout: &ListLayout,
    total: usize,
    render: F,
) {
    let shown = total.min(layout.cap);
    for i in 0..shown {
        let prefix = if i == 0 { layout.prefix_first } else { layout.prefix_rest };
        let _ = writeln!(out, "{}{}", prefix, render(i));
    }
    if total > shown {
        let _ = writeln!(out, "{}... ({} more)", layout.prefix_rest, total - shown);
    }
}

pub fn display_path(file: &Path, root: Option<&Path>) -> String {
    let rel = root.and_then(|r| file.strip_prefix(r).ok()).map_or_else(
        || file.to_path_buf(),
        Path::to_path_buf,
    );
    rel.display().to_string()
}

pub fn confidence_str(c: ImportConfidence) -> &'static str {
    match c {
        ImportConfidence::High => "high",
        ImportConfidence::Medium => "medium",
        ImportConfidence::Low => "low",
        ImportConfidence::BestEffort => "best-effort",
        ImportConfidence::NaAbstraction => "n/a-abstraction",
    }
}
