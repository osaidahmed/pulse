use pulse_syntax::parse::Language;

pub struct LanguageCoverage {
    pub languages: usize,
    pub imports: usize,
    pub abstractness: usize,
    pub taint: usize,
    pub cpg: usize,
}

pub fn language_coverage() -> LanguageCoverage {
    LanguageCoverage {
        languages: Language::COUNT,
        imports: count(super::imports::has_extractor),
        abstractness: count(super::abstractness::covers),
        taint: count(super::taint::covers),
        cpg: count(pulse_syntax::cpg::covers),
    }
}

fn count(pred: fn(Language) -> bool) -> usize {
    Language::ALL.iter().filter(|&&l| pred(l)).count()
}

pub fn disclosure(cpg_enabled: bool) -> String {
    let c = language_coverage();
    let n = c.languages;
    let cpg = if cpg_enabled { format!("cpg {}/{n}", c.cpg) } else { "cpg off".to_string() };
    format!("imports {}/{n} · abstractness {}/{n} · taint {}/{n} · {cpg}", c.imports, c.abstractness, c.taint)
}

pub fn summary_json(cpg_enabled: bool) -> serde_json::Value {
    let c = language_coverage();
    serde_json::json!({
        "languages": c.languages,
        "imports": c.imports,
        "abstractness": c.abstractness,
        "taint": c.taint,
        "cpg": c.cpg,
        "cpg_enabled": cpg_enabled,
    })
}
