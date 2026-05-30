use crate::walk::{self, FileMetrics};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    D,
    Groovy,
}

const CONFIG_KEYS: &[&str] = &[
    "python", "typescript", "javascript", "rust", "c", "cpp",
    "java", "csharp", "go", "swift", "zig", "ruby", "objc",
    "tcl", "kotlin", "haskell", "lua", "r", "php", "cobol",
    "d", "groovy",
];

impl Language {
    pub fn to_config_key(self) -> &'static str {
        CONFIG_KEYS[self as usize]
    }
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
    (&["d", "di"], Language::D),
    (&["groovy"], Language::Groovy),
];

macro_rules! lang_init {
    ($crate_name:ident :: $constant:ident) => {
        (|| -> tree_sitter::Language { $crate_name::$constant.into() }) as fn() -> tree_sitter::Language
    };
}

type LangInit = fn() -> tree_sitter::Language;

static DISPATCH: [(LangInit, WalkFn); 22] = [
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
    (lang_init!(tree_sitter_d::LANGUAGE), walk::d::walk as WalkFn),
    (lang_init!(tree_sitter_groovy::LANGUAGE), walk::groovy::walk as WalkFn),
];

pub fn detect_language(path: &std::path::Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    EXTENSION_MAP
        .iter()
        .find(|(exts, _)| exts.contains(&ext))
        .map(|(_, lang)| *lang)
}

pub fn parse_only(source: &str, lang: Language) -> Option<tree_sitter::Tree> {
    let (ts_lang_fn, _) = DISPATCH[lang as usize];
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&ts_lang_fn()).ok()?;
    parser.parse(source, None)
}

pub fn walk_only(tree: &tree_sitter::Tree, source: &str, lang: Language) -> FileMetrics {
    let (_, walk_fn) = DISPATCH[lang as usize];
    walk_fn(tree, source)
}

pub fn parse_and_walk(source: &str, lang: Language) -> Option<FileMetrics> {
    parse_only(source, lang).map(|tree| walk_only(&tree, source, lang))
}

pub const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_INPUT_LINES: usize = 50_000;
pub const MAX_LINE_BYTES: usize = 200_000;
const ANALYZE_STACK_BYTES: usize = 32 * 1024 * 1024;

fn exceeds_size_caps(source: &str) -> bool {
    source.len() > MAX_INPUT_BYTES
        || memchr::memchr_iter(b'\n', source.as_bytes()).count() > MAX_INPUT_LINES
        || source.split('\n').any(|line| line.len() > MAX_LINE_BYTES)
}

fn run_guarded(source: &str, work: impl FnOnce() -> Option<FileMetrics> + Send) -> Option<FileMetrics> {
    if exceeds_size_caps(source) {
        return None;
    }
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .stack_size(ANALYZE_STACK_BYTES)
            .spawn_scoped(scope, || std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)))
            .ok()?;
        handle.join().ok().and_then(Result::ok).flatten()
    })
}

pub fn parse_and_walk_guarded(source: &str, lang: Language) -> Option<FileMetrics> {
    run_guarded(source, || parse_and_walk(source, lang))
}

pub fn parse_and_walk_scoped(
    source: &str,
    lang: Language,
    edit_byte_range: Option<(usize, usize)>,
) -> Option<FileMetrics> {
    match edit_byte_range {
        None => parse_and_walk_guarded(source, lang),
        scope => run_guarded(source, || {
            walk::with_edit_scope(scope, || parse_and_walk(source, lang))
        }),
    }
}
