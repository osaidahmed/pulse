use crate::walk::{self, FileMetrics};

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Python,
}

pub fn detect_language(path: &std::path::Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "py" => Some(Language::Python),
        _ => None,
    }
}

pub fn parse_and_walk(source: &str, lang: Language) -> Option<FileMetrics> {
    match lang {
        Language::Python => parse_python(source),
    }
}

fn parse_python(source: &str) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_python::LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk::walk_python(&tree, source))
}
