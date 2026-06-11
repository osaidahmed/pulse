use std::io::Read;

use crate::smells::{Finding, Location};

#[derive(Debug)]
pub struct HookInput {
    pub file_path: String,
    pub edit_range: Option<(u32, u32)>,
    pub edit_byte_range: Option<(usize, usize)>,
    pub old_string: Option<String>,
    pub new_string: Option<String>,
    pub original_file: Option<String>,
    pub session_id: Option<String>,
}

pub fn parse_hook_input() -> Option<HookInput> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok()?;
    let v: serde_json::Value = serde_json::from_str(&input).ok()?;

    let session_id = v.get("session_id").and_then(|s| s.as_str()).map(std::string::ToString::to_string);

    let tool_input = v.get("tool_input")?;
    let file_path = tool_input.get("file_path")?.as_str()?.to_string();

    let old_string = tool_input.get("old_string").and_then(|v| v.as_str()).map(std::string::ToString::to_string);
    let new_string = tool_input.get("new_string").and_then(|v| v.as_str()).map(std::string::ToString::to_string);

    let tool_response = v.get("tool_response");
    let original_file = tool_response.and_then(|r| r.get("originalFile")).and_then(|s| s.as_str()).map(String::from);
    let (edit_range, edit_byte_range) =
        patch_ranges(tool_response, &file_path).unwrap_or_else(|| compute_edit_ranges(tool_input, &file_path));

    Some(HookInput { file_path, edit_range, edit_byte_range, old_string, new_string, original_file, session_id })
}

fn patch_ranges(tool_response: Option<&serde_json::Value>, file_path: &str) -> Option<EditRanges> {
    let hunks = tool_response?.get("structuredPatch")?.as_array()?;
    let mut span: Option<(u32, u32)> = None;
    for hunk in hunks {
        let start = u32::try_from(hunk.get("newStart")?.as_u64()?).ok()?;
        let len = u32::try_from(hunk.get("newLines")?.as_u64()?).ok()?;
        let end = start + len.max(1) - 1;
        span = Some(span.map_or((start, end), |(s, e)| (s.min(start), e.max(end))));
    }
    let (start, end) = span?;
    Some((Some((start, end)), byte_range_of_lines(file_path, start, end)))
}

fn byte_range_of_lines(file_path: &str, start: u32, end: u32) -> Option<(usize, usize)> {
    let source = std::fs::read_to_string(file_path).ok()?;
    let bytes = source.as_bytes();
    let mut line = 1u32;
    let mut start_byte = (start == 1).then_some(0usize);
    let mut end_byte = bytes.len();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'\n' {
            continue;
        }
        line += 1;
        if line == start {
            start_byte = Some(i + 1);
        }
        if line == end + 1 {
            end_byte = i;
            break;
        }
    }
    let sb = start_byte?;
    Some((sb, end_byte.max(sb)))
}

type EditRanges = (Option<(u32, u32)>, Option<(usize, usize)>);

fn compute_edit_ranges(tool_input: &serde_json::Value, file_path: &str) -> EditRanges {
    let Some(new_string) = tool_input.get("new_string").and_then(|v| v.as_str()) else {
        return (None, None);
    };
    let Some(old_string) = tool_input.get("old_string").and_then(|v| v.as_str()) else {
        return (None, None);
    };
    let Ok(source) = std::fs::read_to_string(file_path) else {
        return (None, None);
    };
    let Some((start_byte, matched_len)) = source
        .find(new_string)
        .map(|s| (s, new_string.len()))
        .or_else(|| source.find(old_string).map(|s| (s, old_string.len())))
    else {
        return (None, None);
    };

    let start_line = source[..start_byte].matches('\n').count() as u32 + 1;
    let new_lines = new_string.matches('\n').count() as u32;
    (Some((start_line, start_line + new_lines)), Some((start_byte, start_byte + matched_len)))
}

pub fn filter_by_edit_range(findings: Vec<Finding>, range: Option<(u32, u32)>) -> Vec<Finding> {
    let Some((start, end)) = range else {
        return findings.into_iter().filter(|f| !matches!(f.location, Location::Module)).collect();
    };

    findings
        .into_iter()
        .filter(|f| match &f.location {
            Location::Function { start_line, end_line, .. } => *start_line <= end && *end_line >= start,
            Location::Module => false,
        })
        .collect()
}
