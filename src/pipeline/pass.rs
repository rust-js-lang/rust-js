//! What one lowering pass of the crate gathers of each function it keeps,
//! grouped for output.

use rust_js_program::collections::{IndexMap, IndexSet};
use rust_js_program::js;
use rustc_span::def_id::{DefId, LocalModId};

/// Retained functions and their dependencies, grouped for output.
#[derive(Default)]
pub(super) struct Pass {
    pub(super) functions: IndexMap<LocalModId, Vec<js::Function>>,
    pub(super) namespaces: IndexMap<LocalModId, Vec<js::Namespace>>,
    pub(super) jsx: IndexSet<LocalModId>,
    /// What each module runs when it's loaded, each `js::on_load!`'s
    /// (ADR 0267).
    pub(super) statements: IndexMap<LocalModId, Vec<Vec<js::Stmt>>>,
    /// `thread_local!`s' values, made from their lowered `init`s.
    pub(super) local_consts: IndexMap<LocalModId, Vec<js::Const>>,
    /// Each dictionary that's its module's `const`, or whose cache is, by
    /// its impl (ADR 0592), that the crate's own code reaches; and the
    /// names of the `const`s each dictionary makes.
    pub(super) dictionary_consts: Vec<(LocalModId, DefId)>,
    pub(super) dictionary_names: IndexSet<rust_js_program::js::Name>,
    /// What fills each item's hole, by its index.
    pub(super) holes: super::holes::Holes,
    /// Which items each module uses from another, and which JS imports.
    pub(super) references: IndexSet<(LocalModId, DefId)>,
    pub(super) package_uses: IndexSet<(LocalModId, rust_js_lower::bindings::Export)>,
    /// The bindings each module reads through another that holds them
    /// (ADR 0633).
    pub(super) held: IndexSet<(LocalModId, LocalModId, String, bool, DefId)>,
    /// Each module read through another that re-exports a function (rd137).
    pub(super) reexporters: IndexSet<(LocalModId, LocalModId)>,
    /// Each copy of a function another module calls, by its index (ADR
    /// 0051, amended).
    pub(super) copy_references: IndexSet<(LocalModId, DefId, usize)>,
    /// Each method a class takes of a trait of classes' default, its
    /// prototype's, by the class (rd283).
    pub(super) class_defaults: Vec<(DefId, js::Function)>,
    /// Each class's prototype a `js::on_load!` replaces (#448), each ES class's read (#568).
    pub(super) prototypes: rust_js_lower::classes::Prototypes,
}

impl Pass {
    /// `module`'s default, its function, class or constant named `name`,
    /// of `constants` its own, is its default alone, whatever its
    /// visibility: `const v = ..; export default v;`, never `export const
    /// v` too, as every importer imports it as the default (ADRs 0611,
    /// 0773, amended). A class is its constructor and prototype (ADR 0761).
    pub(super) fn default_alone(&mut self, module: LocalModId, name: &str, constants: &mut [js::Const]) {
        for constant in constants.iter_mut().filter(|c| c.name.is(name)) {
            constant.export = false;
        }
        for function in (self.functions.entry(module).or_default().iter_mut()).filter(|f| f.name.is(name)) {
            function.export = false;
        }
        for class in (self.namespaces.entry(module).or_default().iter_mut()).filter(|n| n.name.is(name)) {
            class.export = false;
        }
    }
}
