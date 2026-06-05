use std::collections::{HashMap, HashSet};

use crate::cpg::cfg::Cfg;
use crate::cpg::defuse::{DefUse, DefUseRecord};

pub struct Flow {
    pub reaching_in: Vec<HashSet<usize>>,
    pub reachable: HashSet<u32>,
}

pub fn analyze(cfg: &Cfg, def_use: &[DefUseRecord]) -> Flow {
    let count = cfg.nodes.len();
    let gen = gen_per_node(count, def_use);
    let kill = kill_per_node(count, def_use, &gen);
    let preds = preds_per_node(cfg);
    Flow {
        reaching_in: fixpoint(count, &gen, &kill, &preds),
        reachable: reachable_from(cfg),
    }
}

fn gen_per_node(count: usize, def_use: &[DefUseRecord]) -> Vec<Vec<usize>> {
    let mut gen = vec![Vec::new(); count];
    for (i, r) in def_use.iter().enumerate() {
        if r.kind == DefUse::Def && (r.block as usize) < count {
            gen[r.block as usize].push(i);
        }
    }
    gen
}

fn kill_per_node(count: usize, def_use: &[DefUseRecord], gen: &[Vec<usize>]) -> Vec<HashSet<usize>> {
    let mut by_var: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, r) in def_use.iter().enumerate() {
        if r.kind == DefUse::Def {
            by_var.entry(r.name.as_str()).or_default().push(i);
        }
    }
    let mut kill = vec![HashSet::new(); count];
    for (node, gens) in gen.iter().enumerate() {
        for &di in gens {
            if let Some(all) = by_var.get(def_use[di].name.as_str()) {
                kill[node].extend(all.iter().copied());
            }
        }
    }
    kill
}

fn preds_per_node(cfg: &Cfg) -> Vec<Vec<u32>> {
    let mut preds = vec![Vec::new(); cfg.nodes.len()];
    for e in &cfg.edges {
        if (e.to as usize) < preds.len() {
            preds[e.to as usize].push(e.from);
        }
    }
    preds
}

fn fixpoint(
    count: usize,
    gen: &[Vec<usize>],
    kill: &[HashSet<usize>],
    preds: &[Vec<u32>],
) -> Vec<HashSet<usize>> {
    let mut in_sets = vec![HashSet::new(); count];
    let mut out_sets: Vec<HashSet<usize>> = vec![HashSet::new(); count];
    for _ in 0..=count {
        let mut changed = false;
        for node in 0..count {
            let new_in = union_preds(&preds[node], &out_sets);
            let mut new_out: HashSet<usize> =
                new_in.iter().copied().filter(|d| !kill[node].contains(d)).collect();
            new_out.extend(gen[node].iter().copied());
            if new_in != in_sets[node] || new_out != out_sets[node] {
                changed = true;
                in_sets[node] = new_in;
                out_sets[node] = new_out;
            }
        }
        if !changed {
            break;
        }
    }
    in_sets
}

fn union_preds(preds: &[u32], out_sets: &[HashSet<usize>]) -> HashSet<usize> {
    let mut acc = HashSet::new();
    for &p in preds {
        if let Some(s) = out_sets.get(p as usize) {
            acc.extend(s.iter().copied());
        }
    }
    acc
}

fn reachable_from(cfg: &Cfg) -> HashSet<u32> {
    let mut succ: HashMap<u32, Vec<u32>> = HashMap::new();
    for e in &cfg.edges {
        succ.entry(e.from).or_default().push(e.to);
    }
    let mut seen: HashSet<u32> = HashSet::new();
    let mut stack = vec![cfg.entry];
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        if let Some(s) = succ.get(&node) {
            stack.extend(s.iter().copied());
        }
    }
    seen
}
