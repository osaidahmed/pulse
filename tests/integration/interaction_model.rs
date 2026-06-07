use pulse::interaction::{tier_for, FindingTier};
use pulse::smells::Smell;

#[test]
fn function_correctness_smells_block() {
    for smell in [
        Smell::GodMethod,
        Smell::ComplexMethod,
        Smell::LargeMethod,
        Smell::DeepNestedComplexity,
        Smell::ComplexConditional,
        Smell::ExcessArguments,
        Smell::ConstructorOverInjection,
        Smell::LargeEmbeddedBlock,
        Smell::EmptyErrorHandler,
        Smell::NestedConditionalChunks,
        Smell::DeadStore,
        Smell::UseBeforeDef,
        Smell::UnreachableCode,
    ] {
        assert_eq!(tier_for(smell), FindingTier::Blocking, "{smell:?} should block");
    }
}

#[test]
fn false_positive_prone_smells_are_advisory() {
    for smell in [
        Smell::PrimitiveObsession,
        Smell::LargeAssertionBlock,
        Smell::ShortVariableNames,
        Smell::StringlyTypedSwitch,
    ] {
        assert_eq!(tier_for(smell), FindingTier::Advisory, "{smell:?} should be advisory, not blocking");
    }
}

#[test]
fn module_level_smells_are_audit_only() {
    for smell in [
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
    ] {
        assert_eq!(tier_for(smell), FindingTier::AuditOnly, "{smell:?} is reported by the stop hook, not blocking");
    }
}

#[test]
fn god_component_is_never_blocking_via_a_class_smell() {
    assert_ne!(tier_for(Smell::GodClass), FindingTier::Blocking);
    assert_ne!(tier_for(Smell::CodeDuplication), FindingTier::Blocking);
}
