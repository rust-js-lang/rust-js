//! What the crate's modules import of each other where their JS loads, and
//! so which may load before another: each one's JS imports of another's,
//! and what a JS file of the program's own that it imports loads of them,
//! as files it imports do in turn (`program::JsModules`, ADR 0031, amended).
//! The one graph a module's load asks of: what it reads of another's
//! constants (`pipeline::load_cycles`, ADR 0031, amended) and a dictionary
//! it makes (ADR 0592).

use std::collections::{HashMap, HashSet};

use rustc_span::def_id::LocalModId;

use crate::program::{JsModules, ModuleAt};

pub(super) struct ModuleImports {
    edges: HashMap<LocalModId, HashSet<LocalModId>>,
}

impl ModuleImports {
    /// Of `modules`, each where its JS is laid out: `direct`, each module and
    /// one of the crate's it imports; `bound`, each module and a JS module it
    /// imports, as its binding names it, `./src/hook.js`, `react`, which
    /// imports what `js_modules` says it does; one it can't say may import
    /// any.
    pub(super) fn new<'s>(
        modules: &[(LocalModId, ModuleAt<'_>)],
        direct: impl IntoIterator<Item = (LocalModId, LocalModId)>,
        bound: impl IntoIterator<Item = (LocalModId, &'s str)>,
        js_modules: &dyn JsModules,
    ) -> Self {
        let mut edges: HashMap<LocalModId, HashSet<LocalModId>> = HashMap::new();
        for (from, to) in direct {
            edges.entry(from).or_default().insert(to);
        }
        let bound: Vec<(LocalModId, &str)> = bound.into_iter().collect();
        let laid_out: Vec<ModuleAt<'_>> = modules.iter().map(|&(_, at)| at).collect();
        let specifiers: HashSet<&str> = bound.iter().map(|&(_, specifier)| specifier).collect();
        let loaded = js_modules.loaded(&specifiers.into_iter().collect::<Vec<_>>(), &laid_out);
        for (from, specifier) in bound {
            let to = edges.entry(from).or_default();
            match &loaded[specifier] {
                Some(reached) => to.extend(reached.iter().map(|&at| modules[at].0)),
                None => to.extend(modules.iter().map(|&(module, _)| module)),
            }
        }
        Self { edges }
    }

    /// Whether `from`'s JS imports `to`'s, or imports what does.
    pub(super) fn reaches(&self, from: LocalModId, to: LocalModId) -> bool {
        let mut seen = HashSet::from([from]);
        let mut pending = vec![from];
        while let Some(at) = pending.pop() {
            for &next in self.edges.get(&at).into_iter().flatten() {
                if next == to {
                    return true;
                }
                if seen.insert(next) {
                    pending.push(next);
                }
            }
        }
        false
    }
}
