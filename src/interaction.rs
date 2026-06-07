use crate::smells::Smell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingTier {
    Blocking,
    Advisory,
    AuditOnly,
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
];

const ADVISORY: &[Smell] =
    &[Smell::PrimitiveObsession, Smell::LargeAssertionBlock, Smell::ShortVariableNames, Smell::StringlyTypedSwitch];

pub fn tier_for(smell: Smell) -> FindingTier {
    if BLOCKING.contains(&smell) {
        FindingTier::Blocking
    } else if ADVISORY.contains(&smell) {
        FindingTier::Advisory
    } else {
        FindingTier::AuditOnly
    }
}
