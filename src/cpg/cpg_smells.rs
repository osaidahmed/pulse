use std::collections::HashSet;

use crate::cpg::cfg::{Cfg, NodeKind};
use crate::cpg::defuse::DefUse;
use crate::cpg::reaching::{self, Flow};
use crate::cpg::CpgMetrics;
use crate::smells::{func_loc, Finding, Smell};
use crate::thresholds::Thresholds;
use crate::walk::FunctionMetrics;

pub fn detect_all(functions: &[FunctionMetrics], t: &Thresholds, out: &mut Vec<Finding>) {
    for f in functions {
        if let Some(cpg) = &f.cpg {
            detect(cpg, f, t, out);
        }
    }
}

fn detect(cpg: &CpgMetrics, func: &FunctionMetrics, t: &Thresholds, out: &mut Vec<Finding>) {
    let flow = reaching::analyze(&cpg.cfg, &cpg.def_use);
    if t.cpg.unreachable_code {
        unreachable_code(&cpg.cfg, &flow, func, out);
    }
    if t.cpg.dead_store {
        dead_stores(cpg, &flow, func, out);
    }
    if t.cpg.use_before_def {
        use_before_def(cpg, &flow, func, out);
    }
}

fn finding(smell: Smell, func: &FunctionMetrics, detail: String) -> Finding {
    Finding { smell, location: func_loc(func), detail }
}

fn is_real(kind: NodeKind) -> bool {
    matches!(kind, NodeKind::Stmt | NodeKind::Predicate | NodeKind::LoopHead | NodeKind::Handler)
}

fn unreachable_code(cfg: &Cfg, flow: &Flow, func: &FunctionMetrics, out: &mut Vec<Finding>) {
    let mut lines: Vec<u32> = cfg
        .nodes
        .iter()
        .filter(|n| n.id != cfg.entry && n.id != cfg.exit && is_real(n.kind) && !flow.reachable.contains(&n.id))
        .map(|n| n.line)
        .collect();
    lines.sort_unstable();
    lines.dedup();
    if let Some(first) = lines.first() {
        out.push(finding(Smell::UnreachableCode, func, format!("unreachable code starting at line {first}")));
    }
}

fn reaches_a_use(def_idx: usize, var: &str, cpg: &CpgMetrics, flow: &Flow) -> bool {
    cpg.def_use.iter().any(|u| {
        u.kind == DefUse::Use
            && u.name == var
            && flow.reachable.contains(&u.block)
            && flow.reaching_in[u.block as usize].contains(&def_idx)
    })
}

fn dead_stores(cpg: &CpgMetrics, flow: &Flow, func: &FunctionMetrics, out: &mut Vec<Finding>) {
    for (i, r) in cpg.def_use.iter().enumerate() {
        if r.kind != DefUse::Def || r.block == cpg.cfg.entry || !flow.reachable.contains(&r.block) {
            continue;
        }
        if !reaches_a_use(i, &r.name, cpg, flow) {
            out.push(finding(
                Smell::DeadStore,
                func,
                format!("`{}` assigned at line {} is never read", r.name, r.line),
            ));
        }
    }
}

fn use_before_def(cpg: &CpgMetrics, flow: &Flow, func: &FunctionMetrics, out: &mut Vec<Finding>) {
    let local: HashSet<&str> = cpg.def_use.iter().filter(|r| r.kind == DefUse::Def).map(|r| r.name.as_str()).collect();
    for u in &cpg.def_use {
        if u.kind != DefUse::Use || !flow.reachable.contains(&u.block) {
            continue;
        }
        if !local.contains(u.name.as_str()) {
            continue;
        }
        let reached = flow.reaching_in[u.block as usize].iter().any(|&d| cpg.def_use[d].name == u.name);
        if !reached {
            out.push(finding(
                Smell::UseBeforeDef,
                func,
                format!("`{}` read at line {} before any definition reaches it", u.name, u.line),
            ));
        }
    }
}
