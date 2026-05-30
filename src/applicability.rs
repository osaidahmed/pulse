use crate::smells::{Finding, Smell};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Hook,
    Check,
}

pub fn smell_applies(smell: Smell, surface: Surface) -> bool {
    match smell {
        Smell::ShortVariableNames => surface != Surface::Hook,
        _ => true,
    }
}

pub fn filter_by_applicability(findings: &mut Vec<Finding>, surface: Surface) {
    findings.retain(|f| smell_applies(f.smell, surface));
}
