use std::path::{Path, PathBuf};

use tree_sitter::Node;

use pulse_syntax::parse::Language;

use super::imports::{line_of, node_text, RawImport};

struct DottedProfile {
    lang: Language,
    kind: &'static str,
    ext: &'static str,
    bases: &'static [&'static str],
}

const DOTTED_PROFILES: &[DottedProfile] = &[
    DottedProfile {
        lang: Language::Java,
        kind: "import_declaration",
        ext: "java",
        bases: &["", "src", "src/main/java", "src/main/kotlin"],
    },
    DottedProfile {
        lang: Language::Kotlin,
        kind: "import",
        ext: "kt",
        bases: &["", "src", "src/main/java", "src/main/kotlin"],
    },
    DottedProfile { lang: Language::CSharp, kind: "using_directive", ext: "cs", bases: &["", "src"] },
    DottedProfile { lang: Language::Swift, kind: "import_declaration", ext: "swift", bases: &["", "src", "Sources"] },
    DottedProfile { lang: Language::Haskell, kind: "import", ext: "hs", bases: &["", "src"] },
    DottedProfile { lang: Language::D, kind: "import_declaration", ext: "d", bases: &["", "src", "source"] },
    DottedProfile {
        lang: Language::Groovy,
        kind: "import_declaration",
        ext: "groovy",
        bases: &["", "src", "src/main/groovy", "src/main/java"],
    },
];

fn dotted_profile(lang: Language) -> Option<&'static DottedProfile> {
    DOTTED_PROFILES.iter().find(|p| p.lang == lang)
}

pub fn match_node(node: Node, source: &str, lang: Language) -> Option<RawImport> {
    if lang == Language::Rust {
        return match_rust(node, source);
    }
    if lang == Language::Go {
        return match_go(node, source);
    }
    if lang == Language::Zig {
        return match_zig(node, source);
    }
    let profile = dotted_profile(lang)?;
    if node.kind() != profile.kind {
        return None;
    }
    let target = first_dotted_text(node, source)?;
    Some(RawImport { target, line: line_of(node) })
}

fn match_zig(node: Node, source: &str) -> Option<RawImport> {
    if node.kind() != "builtin_function" {
        return None;
    }
    let mut cursor = node.walk();
    let is_import =
        node.children(&mut cursor).any(|c| c.kind() == "builtin_identifier" && node_text(c, source) == "@import");
    if !is_import {
        return None;
    }
    let arg_string = find_string_arg(node, source)?;
    Some(RawImport { target: arg_string, line: line_of(node) })
}

fn find_string_arg(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let arguments = node.children(&mut cursor).find(|c| c.is_named() && c.kind() == "arguments")?;
    let mut argc = arguments.walk();
    let string_node = arguments.children(&mut argc).find(|c| c.is_named() && c.kind() == "string")?;
    find_named_child(string_node, |c| (c.kind() == "string_content").then(|| node_text(c, source)))
}

fn find_named_child<R>(node: Node, mut predicate: impl FnMut(Node) -> Option<R>) -> Option<R> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if let Some(r) = predicate(child) {
            return Some(r);
        }
    }
    None
}

pub fn candidates(raw: &str, source_file: &Path, project_root: &Path, lang: Language) -> Vec<PathBuf> {
    if lang == Language::Rust {
        return rust_candidates(raw, source_file, project_root);
    }
    if lang == Language::Go {
        return go_candidates(raw, source_file, project_root);
    }
    let Some(profile) = dotted_profile(lang) else { return Vec::new() };
    dotted_candidates(raw, project_root, profile.ext, profile.bases)
}

pub fn path_suffix_for(raw: &str, lang: Language) -> Option<String> {
    let profile = dotted_profile(lang)?;
    let segments: Vec<&str> = raw.split('.').filter(|s| !s.is_empty()).collect();
    let joined = segments.join(std::path::MAIN_SEPARATOR_STR);
    Some(format!("{}{}.{}", std::path::MAIN_SEPARATOR_STR, joined, profile.ext))
}

fn match_rust(node: Node, source: &str) -> Option<RawImport> {
    if node.kind() == "use_declaration" {
        let arg = node.child_by_field_name("argument")?;
        let target = leftmost_path_rust(arg, source)?;
        return Some(RawImport { target, line: line_of(node) });
    }
    if node.kind() == "extern_crate_declaration" {
        let name = node.child_by_field_name("name")?;
        return Some(RawImport { target: node_text(name, source), line: line_of(node) });
    }
    None
}

fn match_go(node: Node, source: &str) -> Option<RawImport> {
    if node.kind() != "import_spec" {
        return None;
    }
    let path = node.child_by_field_name("path")?;
    let raw_text = node_text(path, source);
    let stripped = raw_text
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .or_else(|| raw_text.strip_prefix('`').and_then(|r| r.strip_suffix('`')))?;
    Some(RawImport { target: stripped.to_string(), line: line_of(node) })
}

fn leftmost_path_rust(node: Node, source: &str) -> Option<String> {
    let mut current = node;
    loop {
        if let Some(result) = path_for_kind(current, source) {
            return Some(result);
        }
        if current.kind() == "use_as_clause" {
            current = current.child_by_field_name("path")?;
            continue;
        }
        return None;
    }
}

fn path_for_kind(node: Node, source: &str) -> Option<String> {
    let kind = node.kind();
    if kind == "scoped_identifier" {
        let path = node.child_by_field_name("path")?;
        let name = node.child_by_field_name("name")?;
        return Some(format!("{}::{}", node_text(path, source), node_text(name, source)));
    }
    if kind == "scoped_use_list" {
        return node.child_by_field_name("path").map(|n| node_text(n, source));
    }
    if kind == "use_wildcard" {
        let mut cursor = node.walk();
        let child = node.children(&mut cursor).find(tree_sitter::Node::is_named)?;
        return Some(node_text(child, source));
    }
    if matches!(kind, "identifier" | "crate" | "self" | "super") {
        return Some(node_text(node, source));
    }
    None
}

fn first_dotted_text(node: Node, source: &str) -> Option<String> {
    find_named_child(node, |child| is_dotted_kind(child.kind()).then(|| node_text(child, source)))
}

fn is_dotted_kind(kind: &str) -> bool {
    matches!(
        kind,
        "scoped_identifier"
            | "qualified_name"
            | "qualified_identifier"
            | "identifier"
            | "type_identifier"
            | "package_identifier"
            | "module"
            | "module_fqn"
            | "imported"
            | "simple_identifier"
    )
}

fn rust_candidates(raw: &str, source_file: &Path, project_root: &Path) -> Vec<PathBuf> {
    let parts: Vec<&str> = raw.split("::").collect();
    if parts.is_empty() {
        return Vec::new();
    }
    let head = parts[0];
    let tail = &parts[1..];
    let bases: Vec<PathBuf> = if head == "crate" {
        vec![project_root.join("src")]
    } else if head == "self" {
        vec![source_file.parent().unwrap_or(project_root).to_path_buf()]
    } else if head == "super" {
        let parent = source_file.parent().unwrap_or(project_root);
        vec![parent.parent().unwrap_or(project_root).to_path_buf()]
    } else {
        vec![project_root.to_path_buf(), project_root.join("src")]
    };
    let mut out = Vec::new();
    for base in &bases {
        let mut cursor = base.clone();
        for segment in tail {
            cursor.push(segment);
        }
        out.push(cursor.with_extension("rs"));
        out.push(cursor.join("mod.rs"));
    }
    out
}

fn go_candidates(raw: &str, source_file: &Path, project_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if raw.starts_with("./") || raw.starts_with("../") {
        if let Some(parent) = source_file.parent() {
            out.extend(first_go_file_in_dir(&parent.join(raw)));
        }
    }
    out.extend(first_go_file_in_dir(&project_root.join(raw)));
    out
}

fn first_go_file_in_dir(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension().and_then(|s| s.to_str()) == Some("go")).then_some(p)
        })
        .collect();
    found.sort();
    found.into_iter().take(1).collect()
}

fn dotted_candidates(raw: &str, project_root: &Path, extension: &str, base_dirs: &[&str]) -> Vec<PathBuf> {
    let segments: Vec<&str> = raw.split('.').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for base in base_dirs {
        let mut p = project_root.to_path_buf();
        if !base.is_empty() {
            p.push(base);
        }
        for seg in &segments {
            p.push(seg);
        }
        out.push(p.with_extension(extension));
    }
    out
}
