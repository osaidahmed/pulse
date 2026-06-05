#[derive(Clone)]
pub(super) struct Taint {
    pub source_name: String,
    pub source_line: u32,
    pub via_var: Option<String>,
    pub opaque: bool,
    pub depth: u32,
}

#[derive(Default)]
pub(super) struct Rhs {
    pub strict: Option<Taint>,
    pub loose: Option<Taint>,
    pub opaque: bool,
}

impl Rhs {
    pub(super) fn absorb(&mut self, other: Rhs) {
        self.opaque |= other.opaque;
        self.strict = merge(self.strict.take(), other.strict);
        self.loose = merge(self.loose.take(), other.loose);
    }

    pub(super) fn finalize(&mut self) {
        if self.opaque {
            mark_opaque(&mut self.strict);
            mark_opaque(&mut self.loose);
        }
    }
}

fn mark_opaque(slot: &mut Option<Taint>) {
    if let Some(t) = slot.as_mut() {
        t.opaque = true;
    }
}

fn earlier(a: &Taint, b: &Taint) -> bool {
    (a.source_line, a.source_name.as_str()) < (b.source_line, b.source_name.as_str())
}

pub(super) fn merge(a: Option<Taint>, b: Option<Taint>) -> Option<Taint> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if earlier(&y, &x) { y } else { x }),
        (Some(x), None) => Some(x),
        (None, other) => other,
    }
}

pub(super) fn source_rhs(name: &str, line: u32, in_sanitizer: bool) -> Rhs {
    let t = Taint {
        source_name: name.to_string(),
        source_line: line,
        via_var: None,
        opaque: false,
        depth: 0,
    };
    Rhs {
        strict: if in_sanitizer { None } else { Some(t.clone()) },
        loose: Some(t),
        opaque: false,
    }
}

pub(super) fn sanitized(r: Rhs) -> Rhs {
    if r.opaque {
        let mut kept = Rhs { strict: None, loose: r.loose, opaque: true };
        kept.finalize();
        kept
    } else {
        Rhs::default()
    }
}

pub(super) fn resolve_taint(r: Rhs) -> Option<Taint> {
    if let Some(t) = r.strict {
        return Some(t);
    }
    if r.opaque {
        return r.loose;
    }
    None
}
