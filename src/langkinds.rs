use crate::parse::Language;

const EMPTY: &[&str] = &[];

struct LangKinds {
    lang: Language,
    functions: &'static [&'static str],
    control_flow: &'static [&'static str],
    blocks: &'static [&'static str],
}

const fn lk(
    lang: Language,
    functions: &'static [&'static str],
    control_flow: &'static [&'static str],
    blocks: &'static [&'static str],
) -> LangKinds {
    LangKinds { lang, functions, control_flow, blocks }
}

const TABLE: &[LangKinds] = &[
    lk(
        Language::Python,
        &["function_definition"],
        &["if_statement", "for_statement", "while_statement", "with_statement"],
        &["block"],
    ),
    lk(
        Language::TypeScript,
        &["function_declaration", "generator_function_declaration", "arrow_function", "method_definition"],
        &["if_statement", "for_statement", "for_in_statement", "while_statement", "switch_statement"],
        &["statement_block"],
    ),
    lk(
        Language::JavaScript,
        &["function_declaration", "generator_function_declaration", "arrow_function", "method_definition"],
        &["if_statement", "for_statement", "for_in_statement", "while_statement", "switch_statement"],
        &["statement_block"],
    ),
    lk(
        Language::Rust,
        &["function_item"],
        &["if_expression", "for_expression", "while_expression", "loop_expression", "match_expression"],
        &["block"],
    ),
    lk(
        Language::C,
        &["function_definition"],
        &["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement"],
        &["compound_statement"],
    ),
    lk(
        Language::Cpp,
        &["function_definition"],
        &["if_statement", "for_statement", "for_range_loop", "while_statement", "do_statement", "switch_statement"],
        &["compound_statement"],
    ),
    lk(
        Language::Java,
        &["method_declaration", "constructor_declaration"],
        &[
            "if_statement",
            "for_statement",
            "enhanced_for_statement",
            "while_statement",
            "do_statement",
            "switch_expression",
        ],
        &["block", "constructor_body"],
    ),
    lk(
        Language::CSharp,
        &["method_declaration", "constructor_declaration", "local_function_statement"],
        &["if_statement", "for_statement", "foreach_statement", "while_statement", "do_statement", "switch_statement"],
        &["block"],
    ),
    lk(
        Language::Go,
        &["function_declaration", "method_declaration"],
        &["if_statement", "for_statement", "expression_switch_statement", "type_switch_statement", "select_statement"],
        &["block"],
    ),
    lk(
        Language::Swift,
        &["function_declaration", "init_declaration"],
        &[
            "if_statement",
            "guard_statement",
            "for_statement",
            "while_statement",
            "repeat_while_statement",
            "switch_statement",
        ],
        &["statements"],
    ),
    lk(
        Language::Zig,
        &["function_declaration"],
        &["if_statement", "for_statement", "while_statement", "switch_expression"],
        &["block", "block_expression"],
    ),
    lk(
        Language::Ruby,
        &["method", "singleton_method"],
        &["if", "unless", "while", "until", "for", "case"],
        &["body_statement", "do_block", "block"],
    ),
    lk(
        Language::ObjectiveC,
        &["function_definition", "method_definition"],
        &["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement"],
        &["compound_statement"],
    ),
    lk(Language::Tcl, &["procedure"], &["if", "while", "foreach"], &["braced_word"]),
    lk(
        Language::Kotlin,
        &["function_declaration", "secondary_constructor"],
        &["if_expression", "for_statement", "while_statement", "do_while_statement", "when_expression"],
        &["block"],
    ),
    lk(Language::Haskell, &["function", "bind"], &["conditional", "case"], EMPTY),
    lk(
        Language::Lua,
        &["function_declaration", "function_definition"],
        &["if_statement", "for_statement", "while_statement", "repeat_statement"],
        &["block"],
    ),
    lk(
        Language::R,
        &["function_definition"],
        &["if_statement", "for_statement", "while_statement", "repeat_statement"],
        EMPTY,
    ),
    lk(
        Language::Php,
        &["function_definition", "method_declaration"],
        &[
            "if_statement",
            "for_statement",
            "foreach_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "match_expression",
        ],
        &["compound_statement", "colon_block"],
    ),
    lk(Language::Cobol, EMPTY, &["if_header", "evaluate_header"], EMPTY),
    lk(
        Language::D,
        &["function_declaration", "constructor"],
        &["if_statement", "for_statement", "foreach_statement", "while_statement", "do_statement", "switch_statement"],
        &["block_statement"],
    ),
    lk(
        Language::Groovy,
        &["method_declaration", "constructor_declaration", "function_definition"],
        &[
            "if_statement",
            "for_statement",
            "enhanced_for_statement",
            "while_statement",
            "do_statement",
            "switch_expression",
        ],
        &["block", "closure"],
    ),
];

fn lookup(lang: Language) -> Option<&'static LangKinds> {
    TABLE.iter().find(|e| e.lang == lang)
}

pub fn function_kinds(lang: Language) -> &'static [&'static str] {
    lookup(lang).map_or(EMPTY, |e| e.functions)
}

pub fn control_flow_kinds(lang: Language) -> &'static [&'static str] {
    lookup(lang).map_or(EMPTY, |e| e.control_flow)
}

pub fn block_kinds(lang: Language) -> &'static [&'static str] {
    lookup(lang).map_or(EMPTY, |e| e.blocks)
}
