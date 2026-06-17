use pulse::audit::conceptual_cohesion::VocabModel;

fn v(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_string()).collect()
}

#[test]
fn cohesive_methods_score_higher_than_scattered_methods() {
    let invoice_a = v(&["invoice", "total", "amount", "tax"]);
    let invoice_b = v(&["invoice", "amount", "tax", "discount"]);
    let unrelated = v(&["socket", "buffer", "thread", "mutex"]);

    let all = vec![invoice_a.clone(), invoice_b.clone(), unrelated.clone()];
    let model = VocabModel::build(&all);

    let cohesive = model.cohesion(&[&invoice_a, &invoice_b]).expect("two methods");
    let scattered = model.cohesion(&[&invoice_a, &unrelated]).expect("two methods");

    assert!(cohesive > scattered, "shared-vocabulary methods are more cohesive: {cohesive} vs {scattered}");
    assert!(scattered.abs() < 1e-9, "disjoint vocabularies have zero conceptual overlap: {scattered}");
    assert!(cohesive > 0.3, "overlapping invoice methods are clearly cohesive: {cohesive}");
}

#[test]
fn identical_vocabularies_are_maximally_cohesive() {
    let a = v(&["render", "widget", "layout"]);
    let b = v(&["render", "widget", "layout"]);
    let model = VocabModel::build(&[a.clone(), b.clone()]);
    let score = model.cohesion(&[&a, &b]).expect("two methods");
    assert!((score - 1.0).abs() < 1e-9, "identical vocabularies cosine to 1.0 (no zero-vector degeneracy): {score}");
}

#[test]
fn fewer_than_two_nonempty_methods_yields_no_score() {
    let a = v(&["alpha", "beta"]);
    let model = VocabModel::build(std::slice::from_ref(&a));
    assert!(model.cohesion(&[&a]).is_none(), "a single method has no pairwise cohesion");
    let empty: Vec<String> = Vec::new();
    assert!(model.cohesion(&[&a, &empty]).is_none(), "an empty-vocabulary method is dropped, leaving one");
}
