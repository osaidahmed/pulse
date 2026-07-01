use pulse_core::{Finding, Location, Smell};

fn back<T: serde::de::DeserializeOwned + serde::Serialize>(v: &T) -> T {
    serde_json::from_str(&serde_json::to_string(v).unwrap()).unwrap()
}

// ── positive paths ──

#[test]
fn finding_with_function_location_round_trips() {
    let f = Finding {
        smell: Smell::GodMethod,
        location: Location::Function { name: "handle".into(), start_line: 12, end_line: 88 },
        detail: "cc 21".into(),
    };
    let r = back(&f);
    assert_eq!(r.smell, f.smell);
    assert_eq!(r.location, f.location);
    assert_eq!(r.detail, f.detail);
}

#[test]
fn finding_with_module_location_round_trips() {
    let f = Finding { smell: Smell::FileTooLarge, location: Location::Module, detail: "710 loc".into() };
    let r = back(&f);
    assert_eq!(r.location, Location::Module);
    assert_eq!(r.smell, Smell::FileTooLarge);
}

#[test]
fn every_smell_variant_round_trips() {
    let all = [
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
    for s in all {
        assert_eq!(back(&s), s);
    }
}

#[test]
fn location_variants_round_trip() {
    let fun = Location::Function { name: "f".into(), start_line: 1, end_line: 2 };
    assert_eq!(back(&fun), fun);
    assert_eq!(back(&Location::Module), Location::Module);
}

// ── negative paths ──

#[test]
fn malformed_json_is_an_error_not_a_panic() {
    assert!(serde_json::from_str::<Finding>("{not valid json").is_err());
    assert!(serde_json::from_str::<Smell>("42").is_err());
}

#[test]
fn missing_required_field_is_an_error() {
    // no `location`
    assert!(serde_json::from_str::<Finding>(r#"{"smell":"GodMethod","detail":"x"}"#).is_err());
    // no `smell`
    assert!(serde_json::from_str::<Finding>(r#"{"location":"Module","detail":"x"}"#).is_err());
}

#[test]
fn unknown_enum_variant_is_an_error() {
    assert!(serde_json::from_str::<Smell>(r#""NotARealSmell""#).is_err());
    assert!(serde_json::from_str::<Location>(r#""NotAVariant""#).is_err());
}

#[test]
fn wrong_field_type_is_an_error() {
    assert!(serde_json::from_str::<Location>(r#"{"Function":{"name":"f","start_line":"ten","end_line":2}}"#).is_err());
}

#[test]
fn unknown_extra_fields_are_tolerated_for_forward_compat() {
    let f: Finding =
        serde_json::from_str(r#"{"smell":"GodMethod","location":"Module","detail":"x","future_field":123}"#).unwrap();
    assert_eq!(f.smell, Smell::GodMethod);
    assert_eq!(f.location, Location::Module);
}
