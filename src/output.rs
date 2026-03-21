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
    for (filename, findings) in regressions {
        for f in findings {
            format_stop_finding(&mut out, f, filename);
        }
    }
    let _ = writeln!(out, "Fix all issues above before proceeding.");
    out
}

fn format_stop_finding(out: &mut String, f: &Finding, filename: &str) {
    let label = if is_actionable(f.smell) {
        "regression"
    } else {
        "threshold crossed"
    };
    let _ = writeln!(
        out,
        "error[pulse]: {}: {} {} — {}. {}",
        filename,
        f.smell.to_lowercase(),
        label,
        f.detail,
        action_for(f.smell)
    );
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
    let mut out: String = findings
        .iter()
        .map(|f| format_compact_line(f, filename))
        .collect();
    let _ = writeln!(out, "Fix all issues above before proceeding.");
    out
}

fn format_compact_line(f: &Finding, filename: &str) -> String {
    let action = action_for(f.smell);
    let smell = f.smell.to_lowercase();
    match &f.location {
        Location::Function {
            name, start_line, ..
        } => {
            format!(
                "error[pulse]: {}:{}: {} in `{}` — {}. {}\n",
                filename, start_line, smell, name, f.detail, action
            )
        }
        Location::Module => {
            format!(
                "error[pulse]: {}: {} — {}. {}\n",
                filename, smell, f.detail, action
            )
        }
    }
}

const ACTIONS: &[(&str, &str)] = &[
    ("God Method", "extract smaller functions to reduce size and complexity"),
    ("Complex Method", "reduce branching or extract helper functions"),
    ("Large Method", "break into smaller functions"),
    ("Nested Conditional Chunks", "extract nested blocks into named functions"),
    ("Deep Nested Complexity", "flatten nesting with early returns or guard clauses"),
    ("Complex Conditional", "extract condition into a named boolean or function"),
    ("Excess Arguments", "group related parameters into a struct or config object"),
    ("Constructor Over-Injection", "use a builder pattern or reduce dependencies"),
    ("Large Embedded Block", "move embedded content to a separate file or constant"),
    ("Primitive Obsession", "introduce domain types to replace primitive parameters"),
    ("Large Assertion Block", "extract shared setup into a helper function"),
    ("Empty Error Handler", "add error handling logic or propagate the error"),
    ("File Too Large", "split into smaller, focused modules"),
    ("Too Many Functions", "group related functions into separate modules"),
    ("Overall Code Complexity", "simplify control flow across the module"),
    ("God Class", "split into smaller, single-responsibility classes"),
    ("Excessive Declarations", "reduce type declarations or split the module"),
    ("Global Conditionals", "move conditional logic into functions"),
    ("Deep Global Nesting", "flatten top-level nesting into function calls"),
    ("Code Duplication", "extract shared logic into a reusable function"),
    ("Duplicated Assertion Blocks", "extract common assertions into a helper"),
    ("Low Cohesion", "split class into smaller classes with focused responsibilities"),
    ("Overall Function Size", "break large functions into smaller units"),
];

fn action_for(smell: &str) -> &'static str {
    ACTIONS
        .iter()
        .find(|(s, _)| *s == smell)
        .map_or("address this finding", |(_, a)| a)
}
