use std::path::PathBuf;

use pulse_audit::class_registry::ClassIdentity;
use pulse_audit::detector::parallel_inheritance::unused_class_workaround;

#[test]
fn unused_class_workaround_accepts_class_identity() {
    let class = ClassIdentity {
        file: PathBuf::from("reader.py"),
        name: "Reader".to_string(),
        parent_class: None,
        method_indices: Vec::new(),
    };
    unused_class_workaround(&class);
}

#[test]
fn unused_class_workaround_accepts_class_with_parent() {
    let class = ClassIdentity {
        file: PathBuf::from("xml_reader.py"),
        name: "XmlReader".to_string(),
        parent_class: Some("Reader".to_string()),
        method_indices: Vec::new(),
    };
    unused_class_workaround(&class);
}
