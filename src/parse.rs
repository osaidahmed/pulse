use crate::walk::{self, FileMetrics};

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum Language {
    Python,
    TypeScript,
    JavaScript,
    Rust,
    C,
    Cpp,
    Java,
    CSharp,
    Go,
    Swift,
    Zig,
    Ruby,
    ObjectiveC,
    Tcl,
}

type WalkFn = fn(&tree_sitter::Tree, &str) -> FileMetrics;

const EXTENSION_MAP: &[(&[&str], Language)] = &[
    (&["py"], Language::Python),
    (&["ts", "tsx"], Language::TypeScript),
    (&["js", "jsx", "mjs", "cjs"], Language::JavaScript),
    (&["rs"], Language::Rust),
    (&["c", "h"], Language::C),
    (&["cpp", "cc", "cxx", "hpp", "hxx", "hh"], Language::Cpp),
    (&["java"], Language::Java),
    (&["cs"], Language::CSharp),
    (&["go"], Language::Go),
    (&["swift"], Language::Swift),
    (&["zig"], Language::Zig),
    (&["rb"], Language::Ruby),
    (&["m"], Language::ObjectiveC),
    (&["tcl", "tk", "itcl"], Language::Tcl),
];

fn ts_python() -> tree_sitter::Language { tree_sitter_python::LANGUAGE.into() }
fn ts_typescript() -> tree_sitter::Language { tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into() }
fn ts_javascript() -> tree_sitter::Language { tree_sitter_javascript::LANGUAGE.into() }
fn ts_rust() -> tree_sitter::Language { tree_sitter_rust::LANGUAGE.into() }
fn ts_c() -> tree_sitter::Language { tree_sitter_c::LANGUAGE.into() }
fn ts_cpp() -> tree_sitter::Language { tree_sitter_cpp::LANGUAGE.into() }
fn ts_java() -> tree_sitter::Language { tree_sitter_java::LANGUAGE.into() }
fn ts_csharp() -> tree_sitter::Language { tree_sitter_c_sharp::LANGUAGE.into() }
fn ts_go() -> tree_sitter::Language { tree_sitter_go::LANGUAGE.into() }
fn ts_swift() -> tree_sitter::Language { tree_sitter_swift::LANGUAGE.into() }
fn ts_zig() -> tree_sitter::Language { tree_sitter_zig::LANGUAGE.into() }
fn ts_ruby() -> tree_sitter::Language { tree_sitter_ruby::LANGUAGE.into() }
fn ts_objc() -> tree_sitter::Language { tree_sitter_objc::LANGUAGE.into() }
fn ts_tcl() -> tree_sitter::Language { tree_sitter_tcl::LANGUAGE.into() }

fn walk_ts(tree: &tree_sitter::Tree, source: &str) -> FileMetrics {
    walk::typescript::walk(tree, source, true)
}

fn walk_js(tree: &tree_sitter::Tree, source: &str) -> FileMetrics {
    walk::javascript::walk(tree, source)
}

type LangInit = fn() -> tree_sitter::Language;

static DISPATCH: [(LangInit, WalkFn); 14] = [
    (ts_python, walk::python::walk as WalkFn),
    (ts_typescript, walk_ts as WalkFn),
    (ts_javascript, walk_js as WalkFn),
    (ts_rust, walk::rust::walk as WalkFn),
    (ts_c, walk::c::walk as WalkFn),
    (ts_cpp, walk::cpp::walk as WalkFn),
    (ts_java, walk::java::walk as WalkFn),
    (ts_csharp, walk::csharp::walk as WalkFn),
    (ts_go, walk::go::walk as WalkFn),
    (ts_swift, walk::swift::walk as WalkFn),
    (ts_zig, walk::zig::walk as WalkFn),
    (ts_ruby, walk::ruby::walk as WalkFn),
    (ts_objc, walk::objc::walk as WalkFn),
    (ts_tcl, walk::tcl::walk as WalkFn),
];

pub fn detect_language(path: &std::path::Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    EXTENSION_MAP
        .iter()
        .find(|(exts, _)| exts.contains(&ext))
        .map(|(_, lang)| *lang)
}

pub fn parse_and_walk(source: &str, lang: Language) -> Option<FileMetrics> {
    let (ts_lang_fn, walk_fn) = DISPATCH[lang as usize];
    parse_generic(source, ts_lang_fn(), walk_fn)
}

fn parse_generic(
    source: &str,
    ts_lang: tree_sitter::Language,
    walk_fn: WalkFn,
) -> Option<FileMetrics> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&ts_lang).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk_fn(&tree, source))
}
