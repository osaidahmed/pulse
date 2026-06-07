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

const CONFIG_KEYS: [&str; Language::COUNT] = [
    "python",
    "typescript",
    "javascript",
    "rust",
    "c",
    "cpp",
    "java",
    "csharp",
    "go",
    "swift",
    "zig",
    "ruby",
    "objc",
    "tcl",
    "kotlin",
    "haskell",
    "lua",
    "r",
    "php",
    "cobol",
    "d",
    "groovy",
];

impl Language {
    pub const COUNT: usize = 22;

    #[allow(dead_code)]
    pub const ALL: [Language; Self::COUNT] = [
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::Rust,
        Language::C,
        Language::Cpp,
        Language::Java,
        Language::CSharp,
        Language::Go,
        Language::Swift,
        Language::Zig,
        Language::Ruby,
        Language::ObjectiveC,
        Language::Tcl,
        Language::Kotlin,
        Language::Haskell,
        Language::Lua,
        Language::R,
        Language::Php,
        Language::Cobol,
        Language::D,
        Language::Groovy,
    ];

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

static DISPATCH: [(LangInit, WalkFn); Language::COUNT] = [
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
    EXTENSION_MAP.iter().find(|(exts, _)| exts.contains(&ext)).map(|(_, lang)| *lang)
}

pub fn parse_only(source: &str, lang: Language) -> Option<tree_sitter::Tree> {
    let (ts_lang_fn, _) = DISPATCH[lang as usize];
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&ts_lang_fn()).ok()?;
    let deadline = std::time::Instant::now() + PARSE_TIMEOUT;
    let mut cancel = move |_: &tree_sitter::ParseState| std::time::Instant::now() >= deadline;
    let options = tree_sitter::ParseOptions::new().progress_callback(&mut cancel);
    let bytes = source.as_bytes();
    let mut read = |byte: usize, _: tree_sitter::Point| -> &[u8] { bytes.get(byte..).unwrap_or(&[]) };
    parser.parse_with_options(&mut read, None, Some(options))
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
const PARSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const ANALYZE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn exceeds_size_caps(source: &str) -> bool {
    source.len() > MAX_INPUT_BYTES
        || memchr::memchr_iter(b'\n', source.as_bytes()).count() > MAX_INPUT_LINES
        || source.split('\n').any(|line| line.len() > MAX_LINE_BYTES)
}

fn run_guarded<T: Send + 'static>(source: &str, work: impl FnOnce() -> Option<T> + Send + 'static) -> Option<T> {
    if exceeds_size_caps(source) {
        return None;
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new().stack_size(ANALYZE_STACK_BYTES).spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).ok().flatten();
        let _ = tx.send(result);
    });
    if spawned.is_err() {
        return None;
    }
    rx.recv_timeout(ANALYZE_TIMEOUT).unwrap_or(None)
}

pub fn parse_guarded(source: &str, lang: Language) -> Option<tree_sitter::Tree> {
    let owned = source.to_string();
    run_guarded(source, move || parse_only(&owned, lang))
}

pub fn parse_and_walk_guarded(source: &str, lang: Language) -> Option<FileMetrics> {
    let owned = source.to_string();
    run_guarded(source, move || parse_and_walk(&owned, lang))
}

pub fn parse_and_walk_scoped(
    source: &str,
    lang: Language,
    edit_byte_range: Option<(usize, usize)>,
    cpg_enabled: bool,
) -> Option<FileMetrics> {
    match (edit_byte_range, cpg_enabled) {
        (None, false) => parse_and_walk_guarded(source, lang),
        (range, cpg) => {
            let owned = source.to_string();
            run_guarded(source, move || {
                walk::with_cpg_enabled(cpg, || walk::with_edit_scope(range, || parse_and_walk(&owned, lang)))
            })
        }
    }
}
