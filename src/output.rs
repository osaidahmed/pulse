use std::fmt::Write;

use crate::smells::{Finding, Location};

pub fn format(findings: &[Finding], filename: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "pulse: {} issue{} in {}",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" },
        filename
    );

    for f in findings {
        match &f.location {
            Location::Function {
                name,
                start_line,
                end_line,
            } => {
                let _ = writeln!(
                    out,
                    "  {} (L{}-{}): {} — {}",
                    name, start_line, end_line, f.smell, f.detail
                );
            }
            Location::Module => {
                let _ = writeln!(out, "  Module: {} — {}", f.smell, f.detail);
            }
        }
    }

    out
}

pub fn format_stop(regressions: &[(String, Vec<Finding>)]) -> String {
    let mut out = String::new();

    let actionable: Vec<String> = regressions
        .iter()
        .flat_map(|(filename, findings)| {
            findings
                .iter()
                .filter(|f| is_actionable(f.smell))
                .map(move |f| format!("{} in {} ({})", f.smell, filename, f.detail))
        })
        .collect();

    if !actionable.is_empty() {
        let _ = writeln!(
            out,
            "pulse: {} regression{} — {}",
            actionable.len(),
            if actionable.len() == 1 { "" } else { "s" },
            actionable.join(", ")
        );
    }

    let notes: Vec<String> = regressions
        .iter()
        .flat_map(|(filename, findings)| {
            findings
                .iter()
                .filter(|f| !is_actionable(f.smell))
                .map(move |f| format!("{} crossed {} ({})", filename, f.smell, f.detail))
        })
        .collect();

    if !notes.is_empty() {
        let _ = writeln!(out, "pulse: note — {}", notes.join(", "));
    }

    out
}

fn is_actionable(smell: &str) -> bool {
    matches!(
        smell,
        "Code Duplication"
            | "Global Conditionals"
            | "Deep Global Nesting"
            | "Duplicated Assertion Blocks"
    )
}

pub fn format_compact(findings: &[Finding], filename: &str) -> String {
    let parts: Vec<String> = findings
        .iter()
        .map(|f| match &f.location {
            Location::Function {
                name, start_line, ..
            } => format!("{} at {}:{} ({})", f.smell, name, start_line, f.detail),
            Location::Module => format!("{} ({})", f.smell, f.detail),
        })
        .collect();

    format!(
        "pulse: {} issue{} in {} — {}\n",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" },
        filename,
        parts.join(", ")
    )
}
