use std::path::PathBuf;

use super::{CompDb, CompDbEntry};

pub(super) fn parse(source: &str) -> Option<CompDb> {
    let doc: serde_json::Value = serde_json::from_str(source).ok()?;
    let entries: Vec<CompDbEntry> = doc.as_array()?.iter().filter_map(parse_entry).collect();
    (!entries.is_empty()).then_some(CompDb { entries })
}

fn parse_entry(entry: &serde_json::Value) -> Option<CompDbEntry> {
    let file = PathBuf::from(entry.get("file")?.as_str()?);
    let tokens = entry_tokens(entry)?;
    let (defines, includes) = extract_flags(&tokens);
    Some(CompDbEntry { file, defines, includes })
}

fn entry_tokens(entry: &serde_json::Value) -> Option<Vec<String>> {
    if let Some(cmd) = entry.get("command").and_then(|c| c.as_str()) {
        return Some(cmd.split_whitespace().map(String::from).collect());
    }
    let args = entry.get("arguments")?.as_array()?;
    Some(args.iter().filter_map(|v| v.as_str()).map(String::from).collect())
}

fn extract_flags(tokens: &[String]) -> (Vec<String>, Vec<PathBuf>) {
    let mut defines = Vec::new();
    let mut includes = Vec::new();
    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        if let Some(rest) = token.strip_prefix("-D") {
            let value = joined_or_next(rest, &mut iter);
            let name = value.split('=').next().unwrap_or(&value);
            if !name.is_empty() {
                defines.push(name.to_string());
            }
        } else if let Some(rest) = token.strip_prefix("-I") {
            let path = joined_or_next(rest, &mut iter);
            if !path.is_empty() {
                includes.push(PathBuf::from(path));
            }
        }
    }
    (defines, includes)
}

fn joined_or_next(rest: &str, iter: &mut std::iter::Peekable<std::slice::Iter<'_, String>>) -> String {
    if rest.is_empty() {
        iter.next().cloned().unwrap_or_default()
    } else {
        rest.to_string()
    }
}
