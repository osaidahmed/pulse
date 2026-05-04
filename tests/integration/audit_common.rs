#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub use pulse::thresholds::Thresholds;

pub fn t() -> Thresholds {
    Thresholds::default()
}

pub fn audit_fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("audit")
}

pub fn scenario_path(scenario: &str, lang: &str) -> PathBuf {
    audit_fixtures_root().join(scenario).join(lang)
}

pub fn list_scenarios() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(audit_fixtures_root())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    out.sort();
    out
}

pub fn list_scenario_languages(scenario: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(audit_fixtures_root().join(scenario))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    out.sort();
    out
}

#[derive(Debug, Clone, Default)]
pub struct ExpectedFinding {
    pub kind: String,
    pub file_count_min: u32,
    pub file_count_max: Option<u32>,
    pub support_min: u32,
    pub support_max: Option<u32>,
    pub representative_substr: Option<String>,
    pub must_include_files: Vec<String>,
    pub forbid_files: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Expected {
    pub scenario: String,
    pub language: String,
    pub no_findings: bool,
    pub findings: Vec<ExpectedFinding>,
}

pub fn load_expected(path: &Path) -> Expected {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()));
    let mut exp = Expected::default();
    let mut current: Option<ExpectedFinding> = None;
    let mut in_must_include = false;
    let mut in_forbid_files = false;
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let key_value = parse_key_value(line);
        match key_value {
            ParsedLine::Top("scenario", v) => exp.scenario = v.to_string(),
            ParsedLine::Top("language", v) => exp.language = v.to_string(),
            ParsedLine::Top("no_findings", v) => exp.no_findings = v == "true",
            ParsedLine::Top(k, _) => panic!("unknown top-level key: {k}"),
            ParsedLine::FindingsHeader => {}
            ParsedLine::FindingDash => {
                push_current(&mut current, &mut exp);
                current = Some(ExpectedFinding::default());
                in_must_include = false;
                in_forbid_files = false;
            }
            ParsedLine::FieldEntry(k, v) => apply_field(&mut current, k, v, &mut in_must_include, &mut in_forbid_files),
            ParsedLine::ListItem(item) => apply_list_item(&mut current, item, in_must_include, in_forbid_files),
        }
    }
    push_current(&mut current, &mut exp);
    exp
}

#[derive(Debug)]
enum ParsedLine<'a> {
    Top(&'a str, &'a str),
    FindingsHeader,
    FindingDash,
    FieldEntry(&'a str, &'a str),
    ListItem(&'a str),
}

fn parse_key_value(line: &str) -> ParsedLine<'_> {
    let trimmed = line.trim_start();
    if trimmed == "findings:" {
        return ParsedLine::FindingsHeader;
    }
    if let Some(rest) = trimmed.strip_prefix("- kind:") {
        return ParsedLine::FieldEntry("__dash_kind", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("- ") {
        return ParsedLine::ListItem(rest.trim());
    }
    if let Some((k, v)) = trimmed.split_once(':') {
        let k = k.trim();
        let v = v.trim();
        if line.starts_with(' ') || line.starts_with('\t') {
            return ParsedLine::FieldEntry(k, v);
        }
        return ParsedLine::Top(k, v);
    }
    panic!("unparsable line: {line:?}");
}

fn push_current(current: &mut Option<ExpectedFinding>, exp: &mut Expected) {
    if let Some(f) = current.take() {
        exp.findings.push(f);
    }
}

fn apply_field(
    current: &mut Option<ExpectedFinding>,
    key: &str,
    val: &str,
    in_must_include: &mut bool,
    in_forbid_files: &mut bool,
) {
    if key == "__dash_kind" {
        *current = Some(ExpectedFinding {
            kind: strip_quotes(val).to_string(),
            ..ExpectedFinding::default()
        });
        *in_must_include = false;
        *in_forbid_files = false;
        return;
    }
    let f = current.as_mut().unwrap_or_else(|| panic!("field outside finding: {key}={val}"));
    *in_must_include = key == "must_include_files";
    *in_forbid_files = key == "forbid_files";
    if *in_must_include || *in_forbid_files {
        return;
    }
    assign_scalar(f, key, val);
}

fn assign_scalar(f: &mut ExpectedFinding, key: &str, val: &str) {
    match key {
        "kind" => f.kind = strip_quotes(val).to_string(),
        "file_count_min" => f.file_count_min = val.parse().unwrap(),
        "file_count_max" => f.file_count_max = Some(val.parse().unwrap()),
        "support_min" => f.support_min = val.parse().unwrap(),
        "support_max" => f.support_max = Some(val.parse().unwrap()),
        "representative_substr" => f.representative_substr = Some(strip_quotes(val).to_string()),
        other => panic!("unknown finding field: {other}"),
    }
}

fn apply_list_item(
    current: &mut Option<ExpectedFinding>,
    item: &str,
    in_must_include: bool,
    in_forbid_files: bool,
) {
    let f = current.as_mut().expect("list item outside finding");
    let cleaned = strip_quotes(item).to_string();
    if in_must_include {
        f.must_include_files.push(cleaned);
    } else if in_forbid_files {
        f.forbid_files.push(cleaned);
    }
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s)
}

pub fn copy_scenario_to_tempdir(scenario: &str, lang: &str) -> tempfile::TempDir {
    let src = scenario_path(scenario, lang);
    let dir = tempfile::tempdir().unwrap();
    copy_dir_recursive(&src, dir.path());
    dir
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap().flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to);
        } else {
            std::fs::copy(&from, &to).unwrap();
        }
    }
}
