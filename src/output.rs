use crate::smells::{Finding, Location};

pub fn format(findings: &[Finding], filename: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "pulse: {} issue{} in {}\n",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" },
        filename
    ));

    for f in findings {
        match &f.location {
            Location::Function {
                name,
                start_line,
                end_line,
            } => {
                out.push_str(&format!(
                    "  {} (L{}-{}): {} — {}\n",
                    name, start_line, end_line, f.smell, f.detail
                ));
            }
            Location::Module => {
                out.push_str(&format!("  Module: {} — {}\n", f.smell, f.detail));
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
        out.push_str(&format!(
            "pulse: {} regression{} — {}\n",
            actionable.len(),
            if actionable.len() == 1 { "" } else { "s" },
            actionable.join(", ")
        ));
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
        out.push_str(&format!("pulse: note — {}\n", notes.join(", ")));
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
