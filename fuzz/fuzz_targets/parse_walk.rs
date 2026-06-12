#![no_main]

use libfuzzer_sys::fuzz_target;
use pulse::parse::{parse_and_walk_guarded, Language};

fuzz_target!(|data: &[u8]| {
    let Some((&selector, body)) = data.split_first() else { return };
    let Ok(source) = std::str::from_utf8(body) else { return };
    let lang = Language::ALL[selector as usize % Language::COUNT];
    let Some(metrics) = parse_and_walk_guarded(source, lang) else { return };
    for f in &metrics.functions {
        assert!(f.cc >= 1, "{lang:?}: cc must never drop below 1");
        assert!(f.end_line >= f.start_line, "{lang:?}: inverted line span");
        assert!(f.loc <= f.end_line - f.start_line + 1, "{lang:?}: code lines exceed the raw span");
        assert!(f.max_nesting <= 2000, "{lang:?}: nesting beyond the walk depth cap");
    }
});
