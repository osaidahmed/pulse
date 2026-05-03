use crate::parse::Language;

const TABLE: [&[&str]; 22] = [
    &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[],
    &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[],
];

pub fn skippable_root_kinds(lang: Language) -> &'static [&'static str] {
    TABLE[lang as usize]
}

pub fn is_skippable_root(lang: Language, kind: &str) -> bool {
    skippable_root_kinds(lang).contains(&kind)
}
