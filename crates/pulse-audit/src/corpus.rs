use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pulse_syntax::parse::{self, Language};

pub struct CorpusFile {
    pub path: PathBuf,
    pub lang: Language,
    pub source: Option<Arc<str>>,
    pub tree: Option<tree_sitter::Tree>,
}

impl CorpusFile {
    pub fn parsed(&self) -> Option<(&str, &tree_sitter::Tree)> {
        Some((self.source.as_deref()?, self.tree.as_ref()?))
    }
}

pub struct Corpus {
    pub files: Vec<CorpusFile>,
    by_path: HashMap<PathBuf, usize>,
}

impl Corpus {
    pub fn load(typed_files: &[(PathBuf, Language)]) -> Self {
        use rayon::prelude::*;
        let files: Vec<CorpusFile> = typed_files.par_iter().map(|(path, lang)| load_file(path, *lang)).collect();
        let by_path = files.iter().enumerate().map(|(i, f)| (f.path.clone(), i)).collect();
        Self { files, by_path }
    }

    pub fn get(&self, path: &Path) -> Option<&CorpusFile> {
        self.by_path.get(path).map(|&i| &self.files[i])
    }
}

fn load_file(path: &Path, lang: Language) -> CorpusFile {
    let source: Option<Arc<str>> = std::fs::read_to_string(path).ok().map(Arc::from);
    let tree = source.as_ref().and_then(|s| parse::parse_guarded_shared(s, lang));
    CorpusFile { path: path.to_path_buf(), lang, source, tree }
}
