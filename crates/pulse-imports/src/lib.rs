use pulse_buildmeta::Ecosystem;
use pulse_syntax::parse::Language;

pub const PYTHON_IMPORT_ALIASES: &[(&str, &str)] = &[
    ("attr", "attrs"),
    ("bs4", "beautifulsoup4"),
    ("cv2", "opencv-python"),
    ("crypto", "pycryptodome"),
    ("dateutil", "python-dateutil"),
    ("docx", "python-docx"),
    ("dotenv", "python-dotenv"),
    ("fitz", "pymupdf"),
    ("github", "pygithub"),
    ("jose", "python-jose"),
    ("jwt", "pyjwt"),
    ("magic", "python-magic"),
    ("multipart", "python-multipart"),
    ("mysqldb", "mysqlclient"),
    ("openssl", "pyopenssl"),
    ("pil", "pillow"),
    ("psycopg2", "psycopg2-binary"),
    ("serial", "pyserial"),
    ("sklearn", "scikit-learn"),
    ("slugify", "python-slugify"),
    ("telegram", "python-telegram-bot"),
    ("usb", "pyusb"),
    ("yaml", "pyyaml"),
];
pub fn ecosystem_for(lang: Language) -> Option<Ecosystem> {
    match lang {
        Language::Python => Some(Ecosystem::Pip),
        Language::TypeScript | Language::JavaScript => Some(Ecosystem::Npm),
        Language::Rust => Some(Ecosystem::Cargo),
        Language::Go => Some(Ecosystem::Go),
        Language::Ruby => Some(Ecosystem::RubyGems),
        Language::CSharp => Some(Ecosystem::NuGet),
        Language::Swift => Some(Ecosystem::Swift),
        _ => None,
    }
}

pub fn external_root(eco: Ecosystem, target: &str) -> Option<String> {
    if is_local_path(target) {
        return None;
    }
    match eco {
        Ecosystem::Npm => npm_root(target),
        Ecosystem::Cargo => Some(target.split("::").next().unwrap_or(target).trim_start_matches("r#").to_string()),
        _ => Some(root_before(root_separator(eco), target)),
    }
}

fn root_before(sep: Option<char>, target: &str) -> String {
    match sep {
        Some(c) => target.split(c).next().unwrap_or(target).to_string(),
        None => target.to_string(),
    }
}

fn root_separator(eco: Ecosystem) -> Option<char> {
    match eco {
        Ecosystem::Pip | Ecosystem::Swift => Some('.'),
        Ecosystem::RubyGems => Some('/'),
        Ecosystem::Composer => Some('\\'),
        _ => None,
    }
}

fn is_local_path(target: &str) -> bool {
    target.is_empty() || target.starts_with('.') || target.starts_with('/')
}

fn npm_root(target: &str) -> Option<String> {
    if target.starts_with("node:") {
        return Some(target.trim_start_matches("node:").split('/').next().unwrap_or_default().to_string());
    }
    if target.starts_with("@/") || target.starts_with("~/") || target.starts_with('#') {
        return None;
    }
    if let Some(rest) = target.strip_prefix('@') {
        let mut parts = rest.splitn(3, '/');
        let scope = parts.next()?;
        let name = parts.next()?;
        return Some(format!("@{scope}/{name}"));
    }
    Some(target.split('/').next().unwrap_or(target).to_string())
}
pub fn normalize(name: &str) -> String {
    name.to_lowercase().replace('_', "-")
}
