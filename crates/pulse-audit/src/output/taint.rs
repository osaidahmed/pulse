use std::fmt::Write;
use std::path::Path;

use crate::finding::InjectionEvidence;
use crate::output::helpers::{confidence_str, display_path};

pub fn write_injection(out: &mut String, e: &InjectionEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: injection shape — {}", e.function);
    let _ = writeln!(out, "  file:          {}", display_path(&e.file, root));
    let _ = writeln!(out, "  source:        {}() at line {}", e.source_name, e.source_line);
    let _ = writeln!(out, "  sink:          {}() at line {}", e.sink_name, e.sink_line);
    let _ = writeln!(out, "  tainted:       {}", e.tainted_var);
    if e.crossed_opacity {
        let _ = writeln!(out, "  note: flow crosses dynamic dispatch — treated as unknown, not clean");
    }
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

pub fn injection_json(e: &InjectionEvidence, root: Option<&Path>) -> serde_json::Value {
    serde_json::json!({
        "kind": "InjectionShape",
        "file": display_path(&e.file, root),
        "function": e.function,
        "source": e.source_name,
        "source_line": e.source_line,
        "sink": e.sink_name,
        "sink_line": e.sink_line,
        "tainted_var": e.tainted_var,
        "crossed_opacity": e.crossed_opacity,
        "confidence": confidence_str(e.confidence),
    })
}
