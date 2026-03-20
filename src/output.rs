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
