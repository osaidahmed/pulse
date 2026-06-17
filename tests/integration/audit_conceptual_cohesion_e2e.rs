use std::path::PathBuf;

use pulse::audit::finding::AuditKind;
use pulse::audit::named_smells;
use pulse::parse::Language;
use pulse::thresholds::Thresholds;

const SCATTERED: &str = "class GrabBag {\n\
  void parseXmlDocument() { var documentNode = readXmlElement(); }\n\
  void computeTaxRate() { var taxAmount = grossSalary * bracketFactor; }\n\
  void renderHtmlWidget() { var widgetMarkup = buildScreenTemplate(); }\n\
  void openSocketConnection() { var socketHandle = connectRemoteHost(); }\n\
  void encryptPassword() { var cipherText = hashSecretKey(); }\n\
}\n";

const COHESIVE: &str = "class Invoice {\n\
  void addInvoiceLine() { var invoiceLine = createInvoiceItem(); }\n\
  void computeInvoiceTotal() { var invoiceTotal = sumInvoiceLines(); }\n\
  void applyInvoiceTax() { var invoiceTax = invoiceTotalValue * rateFactor; }\n\
  void printInvoiceDocument() { var invoiceMarkup = formatInvoiceLayout(); }\n\
  void voidInvoiceRecord() { var invoiceStatus = markInvoiceVoid(); }\n\
}\n";

fn scattered_classes(src: &str) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Sample.java");
    std::fs::write(&path, src).unwrap();
    let typed = vec![(path, Language::Java)];
    named_smells::run(&typed, dir.path(), &Thresholds::default().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::LowConceptualCohesion(e) => Some(e.class_name),
            _ => None,
        })
        .collect()
}

#[test]
fn a_semantically_scattered_class_is_flagged() {
    let flagged = scattered_classes(&format!("{SCATTERED}\n{COHESIVE}"));
    assert!(flagged.contains(&"GrabBag".to_string()), "disjoint-vocabulary class is flagged: {flagged:?}");
    assert!(!flagged.contains(&"Invoice".to_string()), "the shared-vocabulary class is not flagged: {flagged:?}");
}

#[test]
fn a_cohesive_only_project_yields_no_cohesion_findings() {
    let flagged = scattered_classes(COHESIVE);
    assert!(flagged.is_empty(), "a single cohesive class produces no low-cohesion finding: {flagged:?}");
}

#[test]
fn disabling_the_detector_suppresses_all_cohesion_findings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Sample.java");
    std::fs::write(&path, SCATTERED).unwrap();
    let typed: Vec<(PathBuf, Language)> = vec![(path, Language::Java)];
    let mut thresholds = Thresholds::default().audit;
    thresholds.named_smells.conceptual_cohesion.enabled = false;
    let findings = named_smells::run(&typed, dir.path(), &thresholds);
    assert!(
        !findings.iter().any(|f| matches!(f.kind, AuditKind::LowConceptualCohesion(_))),
        "the disabled detector emits nothing"
    );
}
