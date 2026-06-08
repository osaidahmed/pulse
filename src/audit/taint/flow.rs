use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use crate::audit::finding::{AuditFinding, AuditKind, ImportConfidence, InjectionEvidence};
use crate::walk::{node_text, DepthGuard};

use super::nodes::{
    binding_left, binding_right, callee_name, idents_in, is_callee, is_field_or_index_target, is_in, line,
    prop_source_name,
};
use super::state::{resolve_taint, sanitized, source_rhs, Rhs, Taint};
use super::FileCtx;

#[derive(Clone, Copy)]
struct AssignMode {
    augmented: bool,
    opaque: bool,
}

pub(super) struct Analyzer<'a> {
    ctx: &'a FileCtx<'a>,
    function: String,
    tainted: HashMap<String, Taint>,
    visits: HashMap<String, u32>,
    reported: HashSet<(u32, u32, String)>,
    out: Vec<AuditFinding>,
}

impl<'a> Analyzer<'a> {
    pub(super) fn new(ctx: &'a FileCtx<'a>, function: String) -> Self {
        Self {
            ctx,
            function,
            tainted: HashMap::new(),
            visits: HashMap::new(),
            reported: HashSet::new(),
            out: Vec::new(),
        }
    }

    pub(super) fn into_findings(self) -> Vec<AuditFinding> {
        self.out
    }

    pub(super) fn visit(&mut self, node: Node) {
        let Some(_g) = DepthGuard::enter() else { return };
        let k = node.kind();
        if self.ctx.lang.assign_kinds.contains(&k) {
            self.record_assignment(node);
        } else if self.ctx.lang.call_kinds.contains(&k) {
            self.check_sink(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() && !self.ctx.lang.fn_kinds.contains(&child.kind()) {
                self.visit(child);
            }
        }
    }

    fn record_assignment(&mut self, node: Node) {
        let Some(lhs) = binding_left(node) else { return };
        let Some(rhs) = binding_right(node) else { return };
        let taint = resolve_taint(self.analyze(rhs, false));
        let aug = self.ctx.lang.aug_kinds.contains(&node.kind());
        self.apply_targets(lhs, taint, aug);
    }

    fn apply_targets(&mut self, lhs: Node, taint: Option<Taint>, aug: bool) {
        if self.ctx.lang.ident_kinds.contains(&lhs.kind()) {
            let name = node_text(lhs, self.ctx.source).to_string();
            self.apply_one(name, taint, AssignMode { augmented: aug, opaque: false });
            return;
        }
        if is_field_or_index_target(lhs.kind()) {
            return;
        }
        let names = idents_in(lhs, self.ctx.source, self.ctx.lang.ident_kinds);
        let opaque = names.len() > 1;
        for name in names {
            self.apply_one(name, taint.clone(), AssignMode { augmented: aug, opaque });
        }
    }

    fn apply_one(&mut self, name: String, taint: Option<Taint>, mode: AssignMode) {
        match taint {
            Some(mut t) => {
                if mode.opaque {
                    t.opaque = true;
                }
                self.assign_taint(name, t);
            }
            None => {
                if !mode.augmented {
                    self.tainted.remove(&name);
                }
            }
        }
    }

    fn assign_taint(&mut self, name: String, mut t: Taint) {
        t.depth += 1;
        if t.depth > self.ctx.caps.max_depth {
            self.tainted.remove(&name);
            return;
        }
        let count = self.visits.entry(name.clone()).or_insert(0);
        *count += 1;
        if *count > self.ctx.caps.visit_cap {
            return;
        }
        self.tainted.insert(name, t);
    }

    fn check_sink(&mut self, node: Node) {
        let Some(name) = callee_name(node, self.ctx.source) else { return };
        if !self.ctx.lang.sinks.contains(&name.as_str()) {
            return;
        }
        let Some(args) = node.child_by_field_name("arguments") else { return };
        if let Some(t) = resolve_taint(self.analyze(args, false)) {
            self.emit(&t, &name, line(node));
        }
    }

    fn emit(&mut self, t: &Taint, sink: &str, sink_line: u32) {
        let var = t.via_var.clone().unwrap_or_else(|| t.source_name.clone());
        if !self.reported.insert((t.source_line, sink_line, var.clone())) {
            return;
        }
        let confidence = if t.opaque { ImportConfidence::BestEffort } else { ImportConfidence::Low };
        self.out.push(AuditFinding {
            kind: AuditKind::InjectionShape(InjectionEvidence {
                file: self.ctx.path.to_path_buf(),
                function: self.function.clone(),
                source_name: t.source_name.clone(),
                source_line: t.source_line,
                sink_name: sink.to_string(),
                sink_line,
                tainted_var: var,
                crossed_opacity: t.opaque,
                confidence,
            }),
            representative_snippet: String::new(),
            support: 1,
            file_count: 1,
            idf_score: None,
            action_label: None,
            pattern_category: None,
            locality_entropy: None,
            p_value: None,
            locations: Vec::new(),
        });
    }

    fn analyze(&self, node: Node, in_sanitizer: bool) -> Rhs {
        let Some(_g) = DepthGuard::enter() else { return Rhs::default() };
        if self.ctx.lang.call_kinds.contains(&node.kind()) {
            return self.analyze_call(node, in_sanitizer);
        }
        if let Some(r) = self.prop_source(node, in_sanitizer) {
            return r;
        }
        let mut r = self.analyze_children(node, in_sanitizer);
        if self.ctx.lang.ident_kinds.contains(&node.kind()) {
            self.absorb_ident(node, in_sanitizer, &mut r);
        }
        if self.ctx.lang.opacity_kinds.contains(&node.kind()) {
            r.opaque = true;
        }
        r.finalize();
        r
    }

    fn prop_source(&self, node: Node, in_sanitizer: bool) -> Option<Rhs> {
        if !self.ctx.lang.source_kinds.contains(&node.kind()) || is_callee(node, self.ctx.lang.call_kinds) {
            return None;
        }
        let name = prop_source_name(node, self.ctx.source);
        self.ctx.lang.prop_sources.contains(&name.as_str()).then(|| source_rhs(&name, line(node), in_sanitizer))
    }

    fn analyze_call(&self, node: Node, in_sanitizer: bool) -> Rhs {
        let name = callee_name(node, self.ctx.source);
        let nm = name.as_deref();
        if is_in(nm, self.ctx.lang.sources) {
            return source_rhs(nm.unwrap_or_default(), line(node), in_sanitizer);
        }
        let sanitizing = is_in(nm, self.ctx.lang.sanitizers);
        let mut r = self.analyze_children(node, in_sanitizer || sanitizing);
        if sanitizing && !in_sanitizer {
            r = sanitized(r);
        }
        if is_in(nm, self.ctx.lang.opacity_calls) {
            r.opaque = true;
        }
        r.finalize();
        r
    }

    fn analyze_children(&self, node: Node, in_sanitizer: bool) -> Rhs {
        let mut r = Rhs::default();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                r.absorb(self.analyze(child, in_sanitizer));
            }
        }
        r
    }

    fn absorb_ident(&self, node: Node, in_sanitizer: bool, r: &mut Rhs) {
        let name = node_text(node, self.ctx.source);
        let Some(base) = self.tainted.get(name) else { return };
        let mut t = base.clone();
        t.via_var = Some(name.to_string());
        let add = Rhs { strict: if in_sanitizer { None } else { Some(t.clone()) }, opaque: t.opaque, loose: Some(t) };
        r.absorb(add);
    }
}
