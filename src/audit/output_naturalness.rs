use std::fmt::Write;
use std::path::Path;

use super::finding::NaturalnessEvidence;
use super::output_helpers::{confidence_str, display_path};

pub fn write_unnatural(out: &mut String, e: &NaturalnessEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: unnatural code — {}", e.function);
    let _ = writeln!(out, "  file:          {}:{}", display_path(&e.file, root), e.line);
    let _ = writeln!(out, "  surprisal:     {:.2} bits/token", e.surprisal);
    let _ = writeln!(out, "  z-score:       {:.2}", e.zscore);
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

pub fn unnatural_json(e: &NaturalnessEvidence, root: Option<&Path>) -> serde_json::Value {
    serde_json::json!({
        "kind": "UnnaturalCode",
        "file": display_path(&e.file, root),
        "function": e.function,
        "line": e.line,
        "surprisal": e.surprisal,
        "zscore": e.zscore,
        "confidence": confidence_str(e.confidence),
    })
}
