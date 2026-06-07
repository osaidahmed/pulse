use super::finding::PatternCategory;

type RootMatcher = fn(&str) -> bool;
type Predicate = fn(&[Box<str>]) -> bool;

struct Rule {
    matches_root: RootMatcher,
    predicate: Predicate,
    primary: PatternCategory,
    fallback: PatternCategory,
}

const RULES: &[Rule] = &[
    Rule {
        matches_root: matches_subscript_root,
        predicate: has_chained_subscripts,
        primary: PatternCategory::ChainedDictAccess,
        fallback: PatternCategory::PrimitiveObsession,
    },
    Rule {
        matches_root: matches_attribute_root,
        predicate: has_value_field,
        primary: PatternCategory::EnumValueAccess,
        fallback: PatternCategory::AttributeChain,
    },
    Rule {
        matches_root: matches_literal_root,
        predicate: always,
        primary: PatternCategory::LiteralRepetition,
        fallback: PatternCategory::LiteralRepetition,
    },
    Rule {
        matches_root: matches_call_root,
        predicate: always,
        primary: PatternCategory::MethodCall,
        fallback: PatternCategory::MethodCall,
    },
    Rule {
        matches_root: matches_comparison_root,
        predicate: always,
        primary: PatternCategory::Comparison,
        fallback: PatternCategory::Comparison,
    },
    Rule {
        matches_root: matches_assignment_root,
        predicate: always,
        primary: PatternCategory::Assignment,
        fallback: PatternCategory::Assignment,
    },
    Rule {
        matches_root: matches_dict_root,
        predicate: always,
        primary: PatternCategory::DictLiteral,
        fallback: PatternCategory::DictLiteral,
    },
    Rule {
        matches_root: matches_list_root,
        predicate: always,
        primary: PatternCategory::ListLiteral,
        fallback: PatternCategory::ListLiteral,
    },
];

pub fn categorize(kinds: &[Box<str>]) -> PatternCategory {
    let Some(root) = kinds.first().map(AsRef::as_ref) else {
        return PatternCategory::Other;
    };
    for rule in RULES {
        if (rule.matches_root)(root) {
            return if (rule.predicate)(kinds) { rule.primary } else { rule.fallback };
        }
    }
    PatternCategory::Other
}

fn matches_subscript_root(kind: &str) -> bool {
    matches!(kind, "subscript" | "subscript_expression" | "index_expression")
}

fn matches_attribute_root(kind: &str) -> bool {
    matches!(kind, "attribute" | "member_expression" | "field_expression" | "field_access")
}

fn matches_literal_root(kind: &str) -> bool {
    matches!(
        kind,
        "string"
            | "string_literal"
            | "concatenated_string"
            | "raw_string_literal"
            | "integer"
            | "integer_literal"
            | "float"
            | "float_literal"
            | "number"
    )
}

fn matches_call_root(kind: &str) -> bool {
    matches!(kind, "call" | "call_expression" | "method_invocation" | "function_call")
}

fn matches_comparison_root(kind: &str) -> bool {
    matches!(kind, "comparison_operator" | "binary_expression" | "binary_operator")
}

fn matches_assignment_root(kind: &str) -> bool {
    matches!(kind, "assignment" | "assignment_expression" | "let_declaration")
}

fn matches_dict_root(kind: &str) -> bool {
    matches!(kind, "dictionary" | "object" | "object_expression" | "hash_literal")
}

fn matches_list_root(kind: &str) -> bool {
    matches!(kind, "list" | "array" | "array_expression" | "array_literal")
}

fn always(_kinds: &[Box<str>]) -> bool {
    true
}

fn has_chained_subscripts(kinds: &[Box<str>]) -> bool {
    kinds.iter().filter(|k| matches_subscript_root(k.as_ref())).count() >= 2
}

fn has_value_field(kinds: &[Box<str>]) -> bool {
    kinds.iter().any(|k| matches!(k.as_ref(), "value" | "Value"))
}
