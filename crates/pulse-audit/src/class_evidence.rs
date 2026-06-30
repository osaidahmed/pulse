use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::finding::{AuditLocation, ImportConfidence};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivergentChangeEvidence {
    pub class_file: PathBuf,
    pub class_name: String,
    pub changing_classes: u32,
    pub fanout: u32,
    pub method_count: u32,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureEnvyEvidence {
    pub method_file: PathBuf,
    pub method_class: Option<String>,
    pub method_name: String,
    pub method_line: u32,
    pub atfd: u32,
    pub foreign_call_count: u32,
    pub intra_call_count: u32,
    pub envied_class: Option<String>,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GodClassEvidence {
    pub class_file: PathBuf,
    pub class_name: String,
    pub wmc: u32,
    pub tcc: f64,
    pub atfd: u32,
    pub method_count: u32,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassIdentityRef {
    pub file: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParallelInheritanceEvidence {
    pub root_a: ClassIdentityRef,
    pub root_b: ClassIdentityRef,
    pub matched_descendants: Vec<(String, String)>,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusedBequestEvidence {
    pub subclass_file: PathBuf,
    pub subclass_name: String,
    pub parent_file: PathBuf,
    pub parent_name: String,
    pub override_count: u32,
    pub parent_method_count: u32,
    pub override_ratio: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShotgunSurgeryEvidence {
    pub method_file: PathBuf,
    pub method_class: Option<String>,
    pub method_name: String,
    pub method_line: u32,
    pub changing_classes: u32,
    pub changing_methods: u32,
    pub fanout: u32,
    pub confidence: ImportConfidence,
    pub caller_samples: Vec<AuditLocation>,
    pub name_collision_count: u32,
    pub additional_definitions: Vec<AuditLocation>,
}
