use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::call_graph::MethodIndex;
use super::class_registry::{class_atfd, class_tcc, class_wmc, ClassIndex, ClassRegistry};
use super::definitions::DefinitionRecord;
use super::finding::{AuditFinding, AuditKind, ImportConfidence};
use pulse_thresholds::AuditThresholds;
use serde::{Deserialize, Serialize};

const DIMS: usize = 4;
const C_STEPS: usize = 20;
const STARTS: usize = 10;

type Vec4 = [f64; DIMS];
type Mat4 = [[f64; DIMS]; DIMS];
type Aug4 = [[f64; DIMS * 2]; DIMS];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultivariateAnomalyEvidence {
    pub class_file: PathBuf,
    pub class_name: String,
    pub wmc: u32,
    pub tcc: f64,
    pub atfd: u32,
    pub n4: u32,
    pub mahalanobis_sq: f64,
    pub confidence: ImportConfidence,
}

struct ClassVec {
    class_idx: ClassIndex,
    fv: Vec4,
    wmc: u32,
    tcc: f64,
    atfd: u32,
    n4: u32,
}

pub fn detect(
    registry: &ClassRegistry,
    defs: &[DefinitionRecord],
    method_idx_lookup: &HashMap<MethodIndex, usize>,
    t: &AuditThresholds,
) -> Vec<AuditFinding> {
    let mcd_t = t.named_smells.multivariate_anomaly;
    if !mcd_t.enabled {
        return Vec::new();
    }
    let n = registry.count();
    if n < mcd_t.min_classes as usize {
        return Vec::new();
    }
    let cc_lookup =
        |m: MethodIndex| -> u32 { method_idx_lookup.get(&m).and_then(|i| defs.get(*i)).map_or(0, |d| d.cc) };
    let foreign_lookup = |m: MethodIndex| -> Vec<(String, String)> {
        method_idx_lookup
            .get(&m)
            .and_then(|i| defs.get(*i))
            .map(|d| d.foreign_field_accesses.clone())
            .unwrap_or_default()
    };
    let fields_lookup = |m: MethodIndex| -> (Vec<String>, bool) {
        method_idx_lookup
            .get(&m)
            .and_then(|i| defs.get(*i))
            .map(|d| (d.field_accesses.clone(), d.is_constructor))
            .unwrap_or_default()
    };
    let cvecs: Vec<ClassVec> = (0..n)
        .map(|i| {
            let class_idx = ClassIndex(i as u32);
            let wmc = class_wmc(registry, class_idx, &cc_lookup);
            let tcc = class_tcc(registry, class_idx, &fields_lookup);
            let atfd = class_atfd(registry, class_idx, &foreign_lookup);
            let n4 = class_n4(registry, class_idx, defs, method_idx_lookup);
            let fv: Vec4 = [(f64::from(wmc) + 1.0).ln(), tcc, (f64::from(atfd) + 1.0).ln(), (f64::from(n4) + 1.0).ln()];
            ClassVec { class_idx, fv, wmc, tcc, atfd, n4 }
        })
        .collect();
    let raw: Vec<Vec4> = cvecs.iter().map(|c| c.fv).collect();
    let Some((mu, sigma_inv)) = run_mcd(&raw) else { return Vec::new() };
    let mut findings: Vec<AuditFinding> = cvecs
        .into_iter()
        .filter_map(|cv| {
            let d2 = mahal_sq(&cv.fv, &mu, &sigma_inv);
            if d2 <= mcd_t.distance_quantile {
                return None;
            }
            let ci = registry.get(cv.class_idx)?;
            let evidence = MultivariateAnomalyEvidence {
                class_file: ci.file.clone(),
                class_name: ci.name.clone(),
                wmc: cv.wmc,
                tcc: cv.tcc,
                atfd: cv.atfd,
                n4: cv.n4,
                mahalanobis_sq: d2,
                confidence: ImportConfidence::Low,
            };
            Some(make_finding(evidence))
        })
        .collect();
    findings.sort_by(|a, b| anomaly_d2(b).partial_cmp(&anomaly_d2(a)).unwrap_or(std::cmp::Ordering::Equal));
    let max = mcd_t.max_findings as usize;
    if findings.len() > max {
        findings.truncate(max);
    }
    findings
}

fn anomaly_d2(f: &AuditFinding) -> f64 {
    if let AuditKind::MultivariateAnomaly(e) = &f.kind {
        e.mahalanobis_sq
    } else {
        0.0
    }
}

fn make_finding(evidence: MultivariateAnomalyEvidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::MultivariateAnomaly(evidence),
        representative_snippet: String::new(),
        support: 0,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}

fn class_n4(
    registry: &ClassRegistry,
    idx: ClassIndex,
    defs: &[DefinitionRecord],
    method_idx_lookup: &HashMap<MethodIndex, usize>,
) -> u32 {
    let methods: Vec<&DefinitionRecord> = registry
        .methods_in(idx)
        .iter()
        .filter_map(|m| method_idx_lookup.get(m).and_then(|i| defs.get(*i)))
        .filter(|d| !d.is_constructor)
        .collect();
    let n = methods.len();
    if n == 0 {
        return 0;
    }
    let field_sets: Vec<HashSet<&str>> =
        methods.iter().map(|m| m.field_accesses.iter().map(String::as_str).collect()).collect();
    let mut parent: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            if !field_sets[i].is_disjoint(&field_sets[j]) {
                uf_union(&mut parent, i, j);
            }
        }
    }
    (0..n).filter(|&i| uf_find(&parent, i) == i).count() as u32
}

fn uf_find(parent: &[usize], mut x: usize) -> usize {
    while parent[x] != x {
        x = parent[x];
    }
    x
}

fn uf_union(parent: &mut [usize], a: usize, b: usize) {
    let ra = uf_find(parent, a);
    let rb = uf_find(parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

fn run_mcd(data: &[Vec4]) -> Option<(Vec4, Mat4)> {
    let n = data.len();
    let h = (n / 2 + 1).max(DIMS + 1);
    if n < h {
        return None;
    }
    let mut best_det = f64::INFINITY;
    let mut best_mean = [0.0f64; DIMS];
    let mut best_inv: Mat4 = [[0.0; DIMS]; DIMS];
    for subset in build_starts(n, h) {
        let subset = converge_subset(data, subset, h);
        if let Some((mean, inv, det)) = eval_subset(data, &subset) {
            if det < best_det {
                best_det = det;
                best_mean = mean;
                best_inv = inv;
            }
        }
    }
    if best_det == f64::INFINITY {
        return None;
    }
    Some(reweighted_estimate(data, &best_mean, &best_inv).unwrap_or((best_mean, best_inv)))
}

fn converge_subset(data: &[Vec4], mut subset: Vec<usize>, h: usize) -> Vec<usize> {
    for _ in 0..C_STEPS {
        match c_step(data, &subset, h) {
            Some(next) if next != subset => subset = next,
            _ => break,
        }
    }
    subset
}

fn reweighted_estimate(data: &[Vec4], mean: &Vec4, inv: &Mat4) -> Option<(Vec4, Mat4)> {
    let inliers: Vec<usize> = (0..data.len()).filter(|&i| mahal_sq(&data[i], mean, inv) <= 11.143).collect();
    if inliers.len() < DIMS + 1 {
        return None;
    }
    eval_subset(data, &inliers).filter(|(_, _, det)| *det > 0.0).map(|(m, inv, _)| (m, inv))
}

fn build_starts(n: usize, h: usize) -> Vec<Vec<usize>> {
    let k = STARTS.min(n);
    (0..k).map(|i| (0..h).map(|j| (i * n / k + j) % n).collect()).collect()
}

fn c_step(data: &[Vec4], subset: &[usize], h: usize) -> Option<Vec<usize>> {
    let mean = subset_mean(data, subset);
    let cov = subset_cov(data, subset, &mean);
    let inv = mat4_inv(&cov)?;
    let mut scored: Vec<(f64, usize)> = data.iter().enumerate().map(|(i, x)| (mahal_sq(x, &mean, &inv), i)).collect();
    scored.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Some(scored[..h].iter().map(|(_, i)| *i).collect())
}

fn eval_subset(data: &[Vec4], subset: &[usize]) -> Option<(Vec4, Mat4, f64)> {
    let mean = subset_mean(data, subset);
    let cov = subset_cov(data, subset, &mean);
    let det = mat4_det(&cov);
    if det <= 0.0 {
        return None;
    }
    let inv = mat4_inv(&cov)?;
    Some((mean, inv, det))
}

fn subset_mean(data: &[Vec4], subset: &[usize]) -> Vec4 {
    let mut m = [0.0f64; DIMS];
    for &i in subset {
        for d in 0..DIMS {
            m[d] += data[i][d];
        }
    }
    let n = subset.len() as f64;
    for v in &mut m {
        *v /= n;
    }
    m
}

fn subset_cov(data: &[Vec4], subset: &[usize], mean: &Vec4) -> Mat4 {
    let mut cov = [[0.0f64; DIMS]; DIMS];
    for &i in subset {
        let d = [data[i][0] - mean[0], data[i][1] - mean[1], data[i][2] - mean[2], data[i][3] - mean[3]];
        for r in 0..DIMS {
            for c in 0..DIMS {
                cov[r][c] += d[r] * d[c];
            }
        }
    }
    let denom = subset.len().saturating_sub(1).max(1) as f64;
    for row in &mut cov {
        for v in row {
            *v /= denom;
        }
    }
    cov
}

fn mahal_sq(x: &Vec4, mu: &Vec4, inv: &Mat4) -> f64 {
    let d = [x[0] - mu[0], x[1] - mu[1], x[2] - mu[2], x[3] - mu[3]];
    let mut tmp = [0.0f64; DIMS];
    for i in 0..DIMS {
        for j in 0..DIMS {
            tmp[i] += inv[i][j] * d[j];
        }
    }
    tmp[0] * d[0] + tmp[1] * d[1] + tmp[2] * d[2] + tmp[3] * d[3]
}

fn eliminate_below_sq(a: &mut Mat4, col: usize) {
    let pivot = a[col][col];
    let pivot_row = a[col];
    for target_row in a.iter_mut().skip(col + 1) {
        let f = target_row[col] / pivot;
        for (target, &src) in target_row.iter_mut().zip(&pivot_row).skip(col) {
            *target -= f * src;
        }
    }
}

fn mat4_det(m: &Mat4) -> f64 {
    let mut a = *m;
    let mut det = 1.0f64;
    for col in 0..DIMS {
        let mut max_row = col;
        for row in (col + 1)..DIMS {
            if a[row][col].abs() > a[max_row][col].abs() {
                max_row = row;
            }
        }
        if a[max_row][col].abs() < 1e-12 {
            return 0.0;
        }
        if max_row != col {
            a.swap(max_row, col);
            det = -det;
        }
        det *= a[col][col];
        eliminate_below_sq(&mut a, col);
    }
    det
}

fn aug_pivot_row(a: &Aug4, col: usize) -> usize {
    let mut max_row = col;
    for row in (col + 1)..DIMS {
        if a[row][col].abs() > a[max_row][col].abs() {
            max_row = row;
        }
    }
    max_row
}

fn aug_eliminate_row(a: &mut Aug4, col: usize, row: usize) {
    let f = a[row][col];
    let pivot_row = a[col];
    for (target, &src) in a[row].iter_mut().zip(&pivot_row) {
        *target -= f * src;
    }
}

fn mat4_inv(m: &Mat4) -> Option<Mat4> {
    let mut a: Aug4 = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if j < DIMS {
                m[i][j]
            } else if j == DIMS + i {
                1.0
            } else {
                0.0
            }
        })
    });
    for col in 0..DIMS {
        let max_row = aug_pivot_row(&a, col);
        if a[max_row][col].abs() < 1e-12 {
            return None;
        }
        a.swap(max_row, col);
        let pivot = a[col][col];
        for v in &mut a[col] {
            *v /= pivot;
        }
        for row in 0..DIMS {
            if row == col {
                continue;
            }
            aug_eliminate_row(&mut a, col, row);
        }
    }
    Some(std::array::from_fn(|i| std::array::from_fn(|j| a[i][DIMS + j])))
}
