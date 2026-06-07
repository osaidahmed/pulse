use pulse::parse::{parse_and_walk_guarded, parse_guarded, Language, MAX_INPUT_LINES, MAX_LINE_BYTES};

fn adversarial_inputs() -> Vec<(&'static str, String)> {
    vec![
        ("empty", String::new()),
        ("one_byte", "x".to_string()),
        ("whitespace_only", "   \n\t  \n   ".to_string()),
        ("nul_bytes", "\0\0\0\0".to_string()),
        ("bom_prefix", "\u{FEFF}fn main() {}".to_string()),
        ("unicode_mix", "λ π ☃ 🦀 \u{1F600} café 日本語".to_string()),
        ("mixed_eol", "a\r\nb\nc\rd".to_string()),
        ("control_chars", "\u{1}\u{2}\u{7}\u{1b}[0m\u{7f}".to_string()),
        ("truncated_decl", "fn f(".to_string()),
        ("unbalanced_open", "{".repeat(200)),
        ("unbalanced_close", ")".repeat(200)),
        ("deeply_nested", "(".repeat(200)),
        ("oversized_line", "a".repeat(MAX_LINE_BYTES + 1)),
        ("too_many_lines", "x\n".repeat(MAX_INPUT_LINES + 1)),
    ]
}

#[test]
fn guarded_walk_fails_open_on_adversarial_input_across_all_languages() {
    let inputs = adversarial_inputs();
    let mut panics = Vec::new();
    for lang in Language::ALL {
        for (label, src) in &inputs {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_and_walk_guarded(src, lang)));
            if outcome.is_err() {
                panics.push(format!("{lang:?}/{label}"));
            }
        }
    }
    assert!(panics.is_empty(), "guarded walk must fail open (never panic); panicked on: {panics:?}");
}

#[test]
fn wall_clock_guard_bounds_a_non_ticking_parser_hang() {
    assert!(
        parse_guarded("x", Language::Cobol).is_none(),
        "parse_guarded must bound a non-ticking parser hang via the wall-clock backstop, not run forever"
    );
}
