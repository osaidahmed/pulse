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
    if t.cpg.dead_store && !cpg.suppress_dead_store {
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
    let def = &cpg.def_use[def_idx];
    cpg.def_use.iter().any(|u| {
        u.kind == DefUse::Use
            && u.name == var
            && flow.reachable.contains(&u.block)
            && (flow.reaching_in[u.block as usize].contains(&def_idx) || (u.block == def.block && def.line <= u.line))
    })
}

fn same_block_def_reaches(cpg: &CpgMetrics, u: &crate::cpg::defuse::DefUseRecord) -> bool {
    cpg.def_use.iter().any(|d| d.kind == DefUse::Def && d.name == u.name && d.block == u.block && d.line <= u.line)
}

fn survives_to_exit(idx: usize, cpg: &CpgMetrics, flow: &Flow) -> bool {
    flow.reaching_in.get(cpg.cfg.exit as usize).is_some_and(|s| s.contains(&idx))
}

fn is_dead_store(
    idx: usize,
    r: &crate::cpg::defuse::DefUseRecord,
    cpg: &CpgMetrics,
    flow: &Flow,
    declared: &HashSet<&str>,
) -> bool {
    if r.kind != DefUse::Def || r.block == cpg.cfg.entry || !flow.reachable.contains(&r.block) {
        return false;
    }
    if r.name.starts_with('_') {
        return false;
    }
    if cpg.captured.contains(r.name.as_str()) {
        return false;
    }
    if !cpg.flag_all_dead_stores && !declared.contains(r.name.as_str()) {
        return false;
    }
    !reaches_a_use(idx, &r.name, cpg, flow) && !survives_to_exit(idx, cpg, flow)
}

fn dead_stores(cpg: &CpgMetrics, flow: &Flow, func: &FunctionMetrics, out: &mut Vec<Finding>) {
    let declared: HashSet<&str> = cpg.def_use.iter().filter(|r| r.decl).map(|r| r.name.as_str()).collect();
    for (i, r) in cpg.def_use.iter().enumerate() {
        if is_dead_store(i, r, cpg, flow, &declared) {
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
        let reached = flow.reaching_in[u.block as usize].iter().any(|&d| cpg.def_use[d].name == u.name)
            || same_block_def_reaches(cpg, u);
        if !reached {
            out.push(finding(
                Smell::UseBeforeDef,
                func,
                format!("`{}` read at line {} before any definition reaches it", u.name, u.line),
            ));
        }
    }
}
