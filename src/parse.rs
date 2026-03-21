use crate::walk::{self, FileMetrics};

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Python,
    TypeScript,
    JavaScript,
    Rust,
    C,
    Cpp,
    Java,
    CSharp,
}

pub fn detect_language(path: &std::path::Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "py" => Some(Language::Python),
        "ts" | "tsx" => Some(Language::TypeScript),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "rs" => Some(Language::Rust),
        "c" | "h" => Some(Language::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some(Language::Cpp),
        "java" => Some(Language::Java),
        "cs" => Some(Language::CSharp),
        _ => None,
    }
}

pub fn parse_and_walk(source: &str, lang: Language) -> Option<FileMetrics> {
    match lang {
        Language::Python => parse_python(source),
        Language::TypeScript => parse_typescript(source),
        Language::JavaScript => parse_javascript(source),
        Language::Rust => parse_rust(source),
        Language::C => parse_c(source),
        Language::Cpp => parse_cpp(source),
        Language::Java => parse_java(source),
        Language::CSharp => parse_csharp(source),
    }
}

fn parse_python(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_python::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::python::walk(&tree, source))
}

fn parse_typescript(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::typescript::walk(&tree, source, true))
}

fn parse_javascript(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_javascript::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::javascript::walk(&tree, source))
}

fn parse_rust(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::rust::walk(&tree, source))
}

fn parse_c(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_c::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::c::walk(&tree, source))
}

fn parse_cpp(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_cpp::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::cpp::walk(&tree, source))
}

fn parse_java(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::java::walk(&tree, source))
}

fn parse_csharp(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_c_sharp::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::csharp::walk(&tree, source))
}
