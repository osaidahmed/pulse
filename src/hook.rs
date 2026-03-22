use std::io::Read;

use crate::smells::{Finding, Location};

#[derive(Debug)]
pub struct HookInput {
    pub file_path: String,
    pub edit_range: Option<(u32, u32)>,
    pub old_string: Option<String>,
    pub new_string: Option<String>,
    pub session_id: Option<String>,
}

pub fn parse_hook_input() -> Option<HookInput> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok()?;
    let v: serde_json::Value = serde_json::from_str(&input).ok()?;

    let session_id = v
        .get("session_id")
        .and_then(|s| s.as_str())
        .map(std::string::ToString::to_string);

    let tool_input = v.get("tool_input")?;
    let file_path = tool_input.get("file_path")?.as_str()?.to_string();

    let old_string = tool_input
        .get("old_string")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);
    let new_string = tool_input
        .get("new_string")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);

    let edit_range = compute_edit_range(tool_input, &file_path);

    Some(HookInput {
        file_path,
        edit_range,
        old_string,
        new_string,
        session_id,
    })
}

fn compute_edit_range(tool_input: &serde_json::Value, file_path: &str) -> Option<(u32, u32)> {
    let new_string = tool_input.get("new_string")?.as_str()?;
    let old_string = tool_input.get("old_string")?.as_str()?;

    let source = std::fs::read_to_string(file_path).ok()?;
    let start_byte = source
        .find(new_string)
        .or_else(|| source.find(old_string))?;

    let start_line = source[..start_byte].matches('\n').count() as u32 + 1;
    let new_lines = new_string.matches('\n').count() as u32;
    let end_line = start_line + new_lines;

    Some((start_line, end_line))
}

pub fn filter_by_edit_range(findings: Vec<Finding>, range: Option<(u32, u32)>) -> Vec<Finding> {
    let Some((start, end)) = range else {
        return findings
            .into_iter()
            .filter(|f| !matches!(f.location, Location::Module))
            .collect();
    };

    findings
        .into_iter()
        .filter(|f| match &f.location {
            Location::Function {
                start_line,
                end_line,
                ..
            } => *start_line <= end && *end_line >= start,
            Location::Module => false,
        })
        .collect()
}
