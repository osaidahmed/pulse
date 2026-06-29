use std::path::Path;

use pulse_syntax::parse::Language;

use super::finding::ImportConfidence;
use super::imports;

#[derive(Debug, Clone, Copy)]
pub enum EvidenceQuality {
    ResolvedInFile,
    NameKeyedUnique,
    NameKeyedAmbiguous,
    Heuristic,
}

pub fn named_smell_confidence(lang: Language, quality: EvidenceQuality) -> ImportConfidence {
    downgrade(imports::confidence_for(lang), penalty(quality))
}

pub fn confidence_for_file(
    file_lang: &impl Fn(&Path) -> Option<Language>,
    file: &Path,
    quality: EvidenceQuality,
) -> ImportConfidence {
    file_lang(file).map_or(ImportConfidence::BestEffort, |lang| named_smell_confidence(lang, quality))
}

fn penalty(quality: EvidenceQuality) -> u8 {
    match quality {
        EvidenceQuality::ResolvedInFile => 0,
        EvidenceQuality::NameKeyedUnique => 1,
        EvidenceQuality::NameKeyedAmbiguous => 2,
        EvidenceQuality::Heuristic => 3,
    }
}

fn downgrade(base: ImportConfidence, steps: u8) -> ImportConfidence {
    let floor = ImportConfidence::BestEffort as u8;
    let level = (base as u8).saturating_sub(steps).max(floor);
    match level {
        4 => ImportConfidence::High,
        3 => ImportConfidence::Medium,
        2 => ImportConfidence::Low,
        _ => ImportConfidence::BestEffort,
    }
}
