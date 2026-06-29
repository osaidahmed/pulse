use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Smell {
    GodMethod,
    ComplexMethod,
    LargeMethod,
    NestedConditionalChunks,
    DeepNestedComplexity,
    ComplexConditional,
    ExcessArguments,
    ConstructorOverInjection,
    LargeEmbeddedBlock,
    PrimitiveObsession,
    LargeAssertionBlock,
    EmptyErrorHandler,
    FileTooLarge,
    TooManyFunctions,
    OverallCodeComplexity,
    GodClass,
    ExcessiveDeclarations,
    GlobalConditionals,
    DeepGlobalNesting,
    CodeDuplication,
    DuplicatedAssertionBlocks,
    LowCohesion,
    OverallFunctionSize,
    LargeStruct,
    ShortVariableNames,
    StringlyTypedSwitch,
    DeadStore,
    UseBeforeDef,
    UnreachableCode,
    HallucinatedImport,
    CrossFileDuplication,
    UnusedFunction,
}

const SMELL_NAMES: &[&str] = &[
    "God Method",
    "Complex Method",
    "Large Method",
    "Nested Conditional Chunks",
    "Deep Nested Complexity",
    "Complex Conditional",
    "Excess Arguments",
    "Constructor Over-Injection",
    "Large Embedded Block",
    "Primitive Obsession",
    "Large Assertion Block",
    "Empty Error Handler",
    "File Too Large",
    "Too Many Functions",
    "Overall Code Complexity",
    "God Class",
    "Excessive Declarations",
    "Global Conditionals",
    "Deep Global Nesting",
    "Code Duplication",
    "Duplicated Assertion Blocks",
    "Low Cohesion",
    "Overall Function Size",
    "Large Struct",
    "Short Variable Names",
    "Stringly-Typed Switch",
    "Dead Store",
    "Use Before Definition",
    "Unreachable Code",
    "Hallucinated Import",
    "Cross-File Duplication",
    "Unused Function",
];

const SMELL_SNAKE_NAMES: &[&str] = &[
    "god_method",
    "complex_method",
    "large_method",
    "nested_conditional_chunks",
    "deep_nested_complexity",
    "complex_conditional",
    "excess_arguments",
    "constructor_over_injection",
    "large_embedded_block",
    "primitive_obsession",
    "large_assertion_block",
    "empty_error_handler",
    "file_too_large",
    "too_many_functions",
    "overall_code_complexity",
    "god_class",
    "excessive_declarations",
    "global_conditionals",
    "deep_global_nesting",
    "code_duplication",
    "duplicated_assertion_blocks",
    "low_cohesion",
    "overall_function_size",
    "large_struct",
    "short_variable_names",
    "stringly_typed_switch",
    "dead_store",
    "use_before_def",
    "unreachable_code",
    "hallucinated_import",
    "cross_file_duplication",
    "unused_function",
];

pub const ALL_SMELLS: &[Smell] = &[
    Smell::GodMethod,
    Smell::ComplexMethod,
    Smell::LargeMethod,
    Smell::NestedConditionalChunks,
    Smell::DeepNestedComplexity,
    Smell::ComplexConditional,
    Smell::ExcessArguments,
    Smell::ConstructorOverInjection,
    Smell::LargeEmbeddedBlock,
    Smell::PrimitiveObsession,
    Smell::LargeAssertionBlock,
    Smell::EmptyErrorHandler,
    Smell::FileTooLarge,
    Smell::TooManyFunctions,
    Smell::OverallCodeComplexity,
    Smell::GodClass,
    Smell::ExcessiveDeclarations,
    Smell::GlobalConditionals,
    Smell::DeepGlobalNesting,
    Smell::CodeDuplication,
    Smell::DuplicatedAssertionBlocks,
    Smell::LowCohesion,
    Smell::OverallFunctionSize,
    Smell::LargeStruct,
    Smell::ShortVariableNames,
    Smell::StringlyTypedSwitch,
    Smell::DeadStore,
    Smell::UseBeforeDef,
    Smell::UnreachableCode,
    Smell::HallucinatedImport,
    Smell::CrossFileDuplication,
    Smell::UnusedFunction,
];

impl Smell {
    pub fn as_str(self) -> &'static str {
        SMELL_NAMES[self as usize]
    }

    pub fn snake_name(self) -> &'static str {
        SMELL_SNAKE_NAMES[self as usize]
    }
}

pub fn smell_from_snake_case(s: &str) -> Option<Smell> {
    SMELL_SNAKE_NAMES.iter().position(|&name| name == s).and_then(|i| ALL_SMELLS.get(i).copied())
}

impl fmt::Display for Smell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub smell: Smell,
    pub location: Location,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    Function { name: String, start_line: u32, end_line: u32 },
    Module,
}

pub fn baseline_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("PULSE_BASELINE_DIR") {
        return std::path::PathBuf::from(dir);
    }
    std::path::PathBuf::from("/tmp/pulse-baselines")
}

pub fn file_key(file_path: &str) -> String {
    format!("{:016x}", hash_path(file_path))
}

pub fn hash_path(file_path: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    hasher.finish()
}

pub fn analytics_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("PULSE_ANALYTICS_DIR") {
        return std::path::PathBuf::from(dir);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home).join(".local/share/pulse");
    }
    std::path::PathBuf::from("/tmp/pulse-analytics")
}
