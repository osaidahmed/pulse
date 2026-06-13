use crate::smells::{Finding, Location, Smell};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingTier {
    Blocking,
    Advisory,
    AuditOnly,
}

const SUBSUMED_BY_GOD_METHOD: &[Smell] =
    &[Smell::NestedConditionalChunks, Smell::DeepNestedComplexity, Smell::LargeEmbeddedBlock];

pub fn suppress_subsumed(findings: Vec<Finding>) -> Vec<Finding> {
    let god_functions: Vec<(String, u32)> = findings
        .iter()
        .filter(|f| f.smell == Smell::GodMethod)
        .filter_map(|f| match &f.location {
            Location::Function { name, start_line, .. } => Some((name.clone(), *start_line)),
            Location::Module => None,
        })
        .collect();
    if god_functions.is_empty() {
        return findings;
    }
    findings
        .into_iter()
        .filter(|f| {
            if !SUBSUMED_BY_GOD_METHOD.contains(&f.smell) {
                return true;
            }
            let Location::Function { name, start_line, .. } = &f.location else { return true };
            !god_functions.iter().any(|(n, s)| n == name && s == start_line)
        })
        .collect()
}

impl FindingTier {
    pub fn as_str(self) -> &'static str {
        match self {
            FindingTier::Blocking => "blocking",
            FindingTier::Advisory => "advisory",
            FindingTier::AuditOnly => "audit_only",
        }
    }
}

const BLOCKING: &[Smell] = &[
    Smell::GodMethod,
    Smell::ComplexMethod,
    Smell::LargeMethod,
    Smell::NestedConditionalChunks,
    Smell::DeepNestedComplexity,
    Smell::ComplexConditional,
    Smell::ExcessArguments,
    Smell::ConstructorOverInjection,
    Smell::LargeEmbeddedBlock,
    Smell::EmptyErrorHandler,
    Smell::DeadStore,
    Smell::UseBeforeDef,
    Smell::UnreachableCode,
    Smell::HallucinatedImport,
];

const ADVISORY: &[Smell] = &[
    Smell::PrimitiveObsession,
    Smell::LargeAssertionBlock,
    Smell::ShortVariableNames,
    Smell::StringlyTypedSwitch,
    Smell::CrossFileDuplication,
    Smell::UnusedFunction,
];

pub fn tier_for(smell: Smell) -> FindingTier {
    if BLOCKING.contains(&smell) {
        FindingTier::Blocking
    } else if ADVISORY.contains(&smell) {
        FindingTier::Advisory
    } else {
        FindingTier::AuditOnly
    }
}
