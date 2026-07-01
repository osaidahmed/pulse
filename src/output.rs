use std::fmt::Write;

use serde::Serialize;

use crate::smells::{Finding, Location, Smell};

#[derive(Serialize)]
pub struct CheckFinding {
    pub file: String,
    pub smell: String,
    pub scope: &'static str,
    pub function: Option<String>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub detail: String,
}

pub fn to_check_finding(file: &str, f: &Finding) -> CheckFinding {
    let (scope, function, start_line, end_line) = match &f.location {
        Location::Function { name, start_line, end_line } => {
            ("function", Some(name.clone()), Some(*start_line), Some(*end_line))
        }
        Location::Module => ("module", None, None, None),
    };
    CheckFinding {
        file: file.to_string(),
        smell: f.smell.to_string(),
        scope,
        function,
        start_line,
        end_line,
        detail: f.detail.clone(),
    }
}

pub fn format_check_json(records: &[CheckFinding]) -> String {
    serde_json::to_string_pretty(records).unwrap_or_else(|_| "[]".to_string())
}

pub fn format(findings: &[Finding], filename: &str) -> String {
    let mut out = String::new();
    let _ =
        writeln!(out, "pulse: {} issue{} in {}", findings.len(), if findings.len() == 1 { "" } else { "s" }, filename);

    for f in findings {
        match &f.location {
            Location::Function { name, start_line, end_line } => {
                let _ = writeln!(out, "  {} (L{}-{}): {} — {}", name, start_line, end_line, f.smell, f.detail);
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
    let label = if is_actionable(f.smell) { "regression" } else { "threshold crossed" };
    let smell_lower = f.smell.as_str().to_lowercase();
    let _ = writeln!(
        out,
        "error[pulse]: {}: {} {} — {}. {}",
        filename,
        smell_lower,
        label,
        f.detail,
        action_for(f.smell, &f.detail)
    );
}

fn is_actionable(smell: Smell) -> bool {
    matches!(
        smell,
        Smell::CodeDuplication
            | Smell::GlobalConditionals
            | Smell::DeepGlobalNesting
            | Smell::DuplicatedAssertionBlocks
    )
}

pub fn format_compact(findings: &[Finding], filename: &str) -> String {
    let mut out: String = findings.iter().map(|f| format_compact_line(f, filename, "error[pulse]")).collect();
    let _ = writeln!(out, "Fix all issues above before proceeding.");
    out
}

fn push_advisory_lines(out: &mut String, filename: &str, findings: &[Finding]) {
    for f in findings {
        out.push_str(&format_compact_line(f, filename, "advisory[pulse]"));
    }
}

pub fn format_advisory(findings: &[Finding], filename: &str) -> String {
    let mut out = String::from("pulse advisory (non-blocking) — consider addressing:\n");
    push_advisory_lines(&mut out, filename, findings);
    out
}

pub fn format_unused(groups: &[(String, Vec<Finding>)]) -> String {
    let mut out =
        String::from("pulse advisory (non-blocking) — functions added this session that are referenced nowhere:\n");
    for (filename, findings) in groups {
        push_advisory_lines(&mut out, filename, findings);
    }
    out
}

fn format_compact_line(f: &Finding, filename: &str, tag: &str) -> String {
    let action = action_for(f.smell, &f.detail);
    let smell_lower = f.smell.as_str().to_lowercase();
    match &f.location {
        Location::Function { name, start_line, .. } => {
            format!("{}: {}:{}: {} in `{}` — {}. {}\n", tag, filename, start_line, smell_lower, name, f.detail, action)
        }
        Location::Module => {
            format!("{}: {}: {} — {}. {}\n", tag, filename, smell_lower, f.detail, action)
        }
    }
}

const ACTIONS: &[&str] = &[
    "extract smaller functions to reduce size and complexity",
    "reduce branching or extract helper functions",
    "break into smaller functions",
    "extract nested blocks into named functions",
    "flatten nesting with early returns or guard clauses",
    "extract condition into a named boolean or function",
    "group related parameters into a struct or config object",
    "use a builder pattern or reduce dependencies",
    "move embedded content to a separate file or constant",
    "introduce domain types to replace primitive parameters",
    "extract shared setup into a helper function",
    "add error handling logic or propagate the error",
    "split into smaller, focused modules",
    "group related functions into separate modules",
    "simplify control flow across the module",
    "split into smaller, single-responsibility classes",
    "reduce type declarations or split the module",
    "move conditional logic into functions",
    "flatten top-level nesting into function calls",
    "extract shared logic into a reusable function",
    "extract common assertions into a helper",
    "split class into smaller classes with focused responsibilities",
    "break large functions into smaller units",
    "group related fields into smaller structs or use composition",
    "use descriptive names that convey purpose",
    "replace string matching with an enum for type safety",
    "remove the dead assignment or use its value before reassigning",
    "define the variable before it is read on every path",
    "remove the unreachable code or fix the control flow that skips it",
    "verify the package name against the manifest or declare the dependency",
    "consolidate the duplicated logic into a shared function instead of copying it across files",
    "remove the unused function, or call it where it is needed",
];

pub fn action_for(smell: Smell, detail: &str) -> &'static str {
    if smell == Smell::CodeDuplication && detail.contains("similar structure") {
        return "restructure to reduce structural similarity";
    }
    ACTIONS[smell as usize]
}
