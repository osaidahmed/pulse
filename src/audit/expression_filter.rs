use super::categorize;
use super::discovery::RawCluster;
use super::finding::PatternCategory;
use super::walker::KindIndex;

pub fn keep_expression_clusters(
    clusters: Vec<RawCluster>,
    kinds_by_fp: &KindIndex,
) -> Vec<RawCluster> {
    clusters
        .into_iter()
        .filter(|c| is_expression_level(c.fingerprint, kinds_by_fp))
        .collect()
}

pub fn is_expression_level(fp: u64, kinds_by_fp: &KindIndex) -> bool {
    let Some(kinds) = kinds_by_fp.get(&fp) else {
        return false;
    };
    matches!(
        categorize::categorize(kinds),
        PatternCategory::PrimitiveObsession
            | PatternCategory::ChainedDictAccess
            | PatternCategory::EnumValueAccess
            | PatternCategory::AttributeChain
            | PatternCategory::LiteralRepetition
            | PatternCategory::MethodCall
            | PatternCategory::Comparison
            | PatternCategory::Assignment
            | PatternCategory::DictLiteral
            | PatternCategory::ListLiteral
    )
}
