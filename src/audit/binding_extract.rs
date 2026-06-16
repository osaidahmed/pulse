use std::collections::BTreeSet;

use super::binding::TypeEnv;

#[derive(Default)]
pub struct EnvBuilder {
    env: TypeEnv,
    conflicts: BTreeSet<String>,
}

impl EnvBuilder {
    pub fn bind(&mut self, name: String, ty: String) {
        if self.conflicts.contains(&name) {
            return;
        }
        match self.env.get(&name) {
            Some(existing) if *existing != ty => {
                self.env.remove(&name);
                self.conflicts.insert(name);
            }
            Some(_) => {}
            None => {
                self.env.insert(name, ty);
            }
        }
    }

    pub fn into_env(mut self, type_vars: &BTreeSet<String>) -> TypeEnv {
        self.env.retain(|_, ty| !type_vars.contains(ty.as_str()));
        self.env
    }
}

pub fn head_of(text: &str) -> Option<String> {
    let before_generic = text.split('<').next()?.trim();
    let unqualified = before_generic.rsplit("::").next()?.rsplit('.').next()?.trim();
    let simple = unqualified.trim_end_matches('?').trim();
    if simple.is_empty() {
        None
    } else {
        Some(simple.to_string())
    }
}
