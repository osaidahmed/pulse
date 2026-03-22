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
    Kotlin,
    Haskell,
    Lua,
    R,
    Php,
    Cobol,
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
    (&["kt", "kts"], Language::Kotlin),
    (&["hs", "lhs"], Language::Haskell),
    (&["lua"], Language::Lua),
    (&["r", "R"], Language::R),
    (&["php", "php5"], Language::Php),
    (&["cob", "cbl", "cobol"], Language::Cobol),
];

macro_rules! lang_init {
    ($crate_name:ident :: $constant:ident) => {
        (|| -> tree_sitter::Language { $crate_name::$constant.into() }) as fn() -> tree_sitter::Language
    };
}

type LangInit = fn() -> tree_sitter::Language;

static DISPATCH: [(LangInit, WalkFn); 20] = [
    (lang_init!(tree_sitter_python::LANGUAGE), walk::python::walk as WalkFn),
    (lang_init!(tree_sitter_typescript::LANGUAGE_TYPESCRIPT), |t, s| walk::typescript::walk(t, s, true)),
    (lang_init!(tree_sitter_javascript::LANGUAGE), walk::javascript::walk as WalkFn),
    (lang_init!(tree_sitter_rust::LANGUAGE), walk::rust::walk as WalkFn),
    (lang_init!(tree_sitter_c::LANGUAGE), walk::c::walk as WalkFn),
    (lang_init!(tree_sitter_cpp::LANGUAGE), walk::cpp::walk as WalkFn),
    (lang_init!(tree_sitter_java::LANGUAGE), walk::java::walk as WalkFn),
    (lang_init!(tree_sitter_c_sharp::LANGUAGE), walk::csharp::walk as WalkFn),
    (lang_init!(tree_sitter_go::LANGUAGE), walk::go::walk as WalkFn),
    (lang_init!(tree_sitter_swift::LANGUAGE), walk::swift::walk as WalkFn),
    (lang_init!(tree_sitter_zig::LANGUAGE), walk::zig::walk as WalkFn),
    (lang_init!(tree_sitter_ruby::LANGUAGE), walk::ruby::walk as WalkFn),
    (lang_init!(tree_sitter_objc::LANGUAGE), walk::objc::walk as WalkFn),
    (lang_init!(tree_sitter_tcl::LANGUAGE), walk::tcl::walk as WalkFn),
    (lang_init!(tree_sitter_kotlin_ng::LANGUAGE), walk::kotlin::walk as WalkFn),
    (lang_init!(tree_sitter_haskell::LANGUAGE), walk::haskell::walk as WalkFn),
    (lang_init!(tree_sitter_lua::LANGUAGE), walk::lua::walk as WalkFn),
    (lang_init!(tree_sitter_r::LANGUAGE), walk::r::walk as WalkFn),
    (lang_init!(tree_sitter_php::LANGUAGE_PHP), walk::php::walk as WalkFn),
    (lang_init!(tree_sitter_cobol::LANGUAGE), walk::cobol::walk as WalkFn),
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
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&ts_lang_fn()).ok()?;
    let tree = parser.parse(source, None)?;
    Some(walk_fn(&tree, source))
}
