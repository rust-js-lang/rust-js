//! A checked function split in two (#2403 step 2b(a), ADR 1193): its JS
//! entry, `f`, which checks what a JS caller gives it and calls its body,
//! and its body, `f$`, which the crate's Rust calls, and a consumer's
//! through its library's contract, so Rust's own calls never pay for the
//! checks JS's do.
//!
//! ```js
//! export function f(a, b) {
//!   $checkU32(a, "f(a)");
//!   return f$(a, b);
//! }
//!
//! export function f$(a, b) { .. }
//! ```
//!
//! Which are split: each function whose parameters are checked where Rust
//! calls it, or may: every one of a library, whose consumers may call any,
//! and of a binary those its own bodies call. A function at its module's
//! top, or a type's namespace's method, `Perms.fromBitsRetain` and its body
//! `Perms.fromBitsRetain$` beside it in the object, which a Rust call calls
//! (#2403 lever (3)); never a class's or an object's member, whose `self` is
//! JS's `this`. Every parameter of which Rust names plainly, and that reads
//! no `js::arguments()` or `js::this()`, which its entry couldn't pass on.

use rust_js_program::collections::{IndexMap, IndexSet};
use rust_js_program::js::{self, Expr};
use rustc_middle::mir::TerminatorKind;
use rustc_span::def_id::{DefId, LocalModId};

use crate::{FnCx, FnInfo};

/// The crate's split functions, each by the name of its body, `f$`.
#[derive(Default)]
pub struct Split {
    functions: IndexMap<DefId, String>,
}

impl Split {
    /// The name of `id`'s body, which a Rust call of it calls, where it's
    /// split.
    pub fn unchecked(&self, id: DefId) -> Option<&str> {
        self.functions.get(&id).map(String::as_str)
    }

    /// Each split function, and its body's name.
    pub fn functions(&self) -> impl Iterator<Item = (DefId, &str)> {
        self.functions.iter().map(|(&id, name)| (id, name.as_str()))
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The crate's split functions, found the first time any lowering asks
    /// (`Lazy`, ADR 1148).
    pub fn split(&self) -> &'a Split {
        self.krate.split.get_or_find(|| self.find_split())
    }

    fn find_split(&self) -> Split {
        let tcx = self.tcx;
        // What the crate's own bodies call: a binary's Rust callers.
        let called: IndexSet<DefId> = match self.krate.library {
            true => IndexSet::default(),
            false => (self.krate.bodies.values().chain(self.krate.closures.values()))
                .filter_map(|body| body.mir.as_ref())
                .flat_map(|mir| mir.body.basic_blocks.iter())
                .filter_map(|data| match &data.terminator().kind {
                    TerminatorKind::Call { func, .. } => Some(func.const_fn_def()?.0),
                    _ => None,
                })
                .collect(),
        };
        let functions = (self.krate.bodies.iter())
            .filter(|&(&id, body)| {
                if !self.krate.fns.contains_key(&id) {
                    return false;
                }
                // Not one its module re-exports another function in place
                // of (rd575, ADR 1064), whose JS is that function's.
                !self.krate.forwarded.contains_key(&id)
                    // Nor a class's or an object's member, whose `self` is JS's `this`
                    // (ADRs 0761, 0943), which its body would lose.
                    && crate::classes::member(tcx, id).is_none()
                    && self.krate.at_module_top(id)
                    && (self.krate.library || called.contains(&id))
                    && !self.krate.reads_arguments.functions.contains(&id)
                    && !self.krate.reads_arguments.this.contains(&id)
                    && !crate::bindings::takes_rest(tcx, id)
                    && (tcx.fn_arg_idents(id).iter()).all(|ident| super::checks::named_plainly(Some(ident)))
                    && self.checks_its_parameters(id, &body.thir)
                    && self.checks_a_parameter(id)
            })
            .map(|(&id, _)| (id, format!("{}$", self.krate.fns[&id].name)))
            .collect();
        Split { functions }
    }

    /// The body of the crate's split function `def_id` where the path at
    /// `named` reads its entry through its own module's namespace,
    /// `People.fullName`: read so, `People.fullName$` (rd114). Through
    /// another module that re-exports the entry, which has no body's, that
    /// entry by that path. None where it's read otherwise.
    pub(crate) fn unchecked_named(&self, def_id: DefId, named: rustc_span::Span) -> Option<Expr> {
        if !def_id.is_local() {
            return self.library_unchecked_named(def_id, named);
        }
        let body = self.split().unchecked(def_id)?.to_owned();
        // A namespace's method's, its object's other member (`namespaced`).
        if self.krate.fns.get(&def_id)?.owner.is_some() {
            return None;
        }
        let target = self.krate.fns.get(&def_id)?.module;
        let entry = self.named_fn_ref(def_id, None, named);
        let js::ExprKind::Member(namespace, _) = &entry.kind else {
            return None;
        };
        let js::ExprKind::Symbol(symbol) = &namespace.kind else {
            return None;
        };
        // Through another module that re-exports it, `plugins::dispatch()`:
        // its entry by that path, `Plugins.dispatch`, as that module exports
        // no body (rd137, ADR 1193).
        if symbol.export == js::NAMESPACE && *symbol != crate::module_symbol(target, js::NAMESPACE) {
            return Some(entry);
        }
        if *symbol != crate::module_symbol(target, js::NAMESPACE) {
            return None;
        }
        self.dependencies
            .borrow_mut()
            .unchecked_references
            .insert((self.module, def_id));
        Some(Expr::member((**namespace).clone(), body).item())
    }

    /// A library's split function's body where the path at `named` reads
    /// its entry through the namespace of the file that has both,
    /// `Scheduler.now`: read so, `Scheduler.now$`, as `named_fn_ref`
    /// reads the entry. None where it's read otherwise.
    fn library_unchecked_named(&self, def_id: DefId, named: rustc_span::Span) -> Option<Expr> {
        let (from, body) = self.library_body(def_id)?;
        let through = (self.krate.module_paths.get(&named)).is_some_and(|path| path.function == def_id);
        let item = self.krate.foreign.item(def_id)?;
        if !through || item.member.is_some() || item.export == "default" {
            return None;
        }
        let namespace = (from, js::NAMESPACE.to_string());
        let local = self.krate.imports[&self.module].get(&namespace)?.clone();
        self.read_import_of(def_id, &local);
        (self.dependencies.borrow_mut().package_uses).insert((self.module, namespace));
        Some(Expr::member(Expr::module(&local), body).item())
    }

    /// A library's split function's body, by the file that exports it and
    /// its name there, `f$`, where this module imports the function by
    /// that file: not one the crate compiles itself, nor one its module
    /// reads through `require`; and not where it names the function
    /// through the library's package root or a module that re-exports it,
    /// which re-export the entry alone, so its call is the entry's (ADR
    /// 1193); nor where the entry is its file's default, which a consumer
    /// imports as the module it is (ADR 0773).
    fn library_body(&self, def_id: DefId) -> Option<(String, String)> {
        if self.asked_of_library(def_id) || self.krate.required_uses.contains(&(self.module, def_id)) {
            return None;
        }
        // A namespace's method's, its object's other member (`namespaced`).
        if self.krate.foreign.item(def_id)?.member.is_some() {
            return None;
        }
        let (from, body) = self.krate.foreign.unchecked(def_id)?;
        let (imported_from, entry) = self.krate.foreign.import_key(def_id, self.module)?;
        (imported_from == from && entry != "default").then_some((from, body))
    }

    /// The body of the crate's split function `def_id`, which a direct call
    /// of it calls, `f$`, of its module or imported from it; none where it
    /// isn't split.
    pub(crate) fn unchecked_ref(&self, def_id: DefId) -> Option<Expr> {
        // A library's, by its file, as its contract says (`library_body`).
        if !def_id.is_local() {
            let export = self.library_body(def_id)?;
            let local = self.krate.imports[&self.module].get(&export)?.clone();
            self.dependencies
                .borrow_mut()
                .package_uses
                .insert((self.module, export));
            return Some(Expr::module(&local).item());
        }
        let name = self.split().unchecked(def_id)?;
        let target = &self.krate.fns[&def_id];
        // A namespace's method's, its object's other member (`namespaced`).
        if target.owner.is_some() {
            return None;
        }
        if target.module == self.module {
            return Some(Expr::var(js::Name::module(name)).item());
        }
        let mut dependencies = self.dependencies.borrow_mut();
        dependencies.references.insert((self.module, def_id));
        dependencies.unchecked_references.insert((self.module, def_id));
        Some(
            Expr {
                facts: js::Facts::NONE,
                kind: js::ExprKind::Symbol(crate::module_symbol(target.module, name)),
                span: js::Span::NONE,
                held_as: Default::default(),
            }
            .item(),
        )
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of `callee`, the read of a namespace's method `def_id`, the crate's
    /// or a library's, `Perms.fromBitsRetain`, where it's split: its body,
    /// the object's other member, `Perms.fromBitsRetain$`, which a Rust call
    /// calls, a library's by its manifest (`Item::unchecked`).
    pub(crate) fn namespaced(&self, def_id: DefId, callee: Expr) -> Expr {
        let unchecked = match def_id.as_local() {
            Some(_) => (self.krate.fns.get(&def_id))
                .filter(|info| info.owner.is_some())
                .and_then(|_| self.split().unchecked(def_id))
                .map(str::to_owned),
            None => (self.krate.foreign.item(def_id))
                .filter(|item| item.member.is_some())
                .and_then(|item| item.unchecked.clone()),
        };
        match (unchecked, callee.kind) {
            (Some(body), js::ExprKind::Member(object, _)) => Expr::member(*object, body).item(),
            (_, kind) => Expr { kind, ..callee },
        }
    }
}

/// Each module's functions and namespaces, which `written` writes the split
/// ones of in two.
pub type Written<'a> = (
    &'a mut IndexMap<LocalModId, Vec<js::Function>>,
    &'a mut IndexMap<LocalModId, Vec<js::Namespace>>,
);

/// Each split function of `functions`, each module's, written in two: its
/// entry, by its own name, its checks first, then its body's call given
/// its parameters; and its body, by `f$`, exported where a library's entry
/// is, for its consumers, or another module calls it.
pub fn written(
    (split, library): (&Split, bool),
    fns: &IndexMap<DefId, FnInfo>,
    called_elsewhere: &IndexSet<(LocalModId, DefId)>,
    (functions, namespaces): Written<'_>,
) {
    for (id, unchecked) in split.functions() {
        let info = &fns[&id];
        // A namespace's method: its entry and its body, members of its
        // object, the entry's call of the body through it.
        if let Some(owner) = &info.owner {
            let Some(namespace) = (namespaces.get_mut(&info.module).into_iter().flatten())
                .find(|n| n.class.is_none() && n.name.is(owner))
            else {
                continue;
            };
            let object = Expr::var(&namespace.name);
            let Some(at) = namespace.methods.iter().position(|f| f.name.is(&info.name)) else {
                continue;
            };
            let body = split_in_two(&mut namespace.methods[at], unchecked, |name| {
                Expr::member(object, name).item()
            });
            namespace.methods.insert(at + 1, body);
            continue;
        }
        let Some(module) = functions.get_mut(&info.module) else {
            continue;
        };
        let Some(at) = module.iter().position(|f| f.name.is(&info.name)) else {
            continue;
        };
        let entry = &mut module[at];
        let export = entry.export;
        let mut body = split_in_two(entry, unchecked, |name| Expr::var(js::Name::module(name)).item());
        // A library's consumers call it; a program's other modules may.
        body.export =
            (library && export) || (called_elsewhere.iter()).any(|&(from, of)| of == id && from != info.module);
        module.insert(at + 1, body);
    }
}

/// `entry`, a checked function, made its entry, its checks then its body's
/// call, `unchecked` read by `read`, given its parameters; and its body,
/// named `unchecked`, its checks gone, given back.
fn split_in_two(entry: &mut js::Function, unchecked: &str, read: impl FnOnce(&str) -> Expr) -> js::Function {
    let checks = leading_checks(&entry.body);
    let mut body = entry.clone();
    body.name = js::Name::module(unchecked);
    body.body.drain(..checks);
    // Each parameter passed on by its name: one the body takes apart,
    // `[a, b]`, which no check reads, the entry takes whole, by a name
    // of its own.
    let mut args = Vec::new();
    for (at, param) in entry.params.iter_mut().enumerate() {
        let name = match param {
            js::Pattern::Name(name) | js::Pattern::Defaulted(name, _) => name.clone(),
            _ => {
                let name = js::Name::declared(format!("_{at}"));
                *param = js::Pattern::Name(name.clone());
                name
            }
        };
        args.push(Expr::var(name));
    }
    let call = Expr::call(read(unchecked), args);
    entry.body.truncate(checks);
    entry.body.push(js::StmtKind::Return(Some(call)).at(js::Span::NONE));
    body
}

/// Of each module, what it imports of each split function's body another
/// module calls: from that function's module, by the body's name.
pub fn imported(
    tcx: rustc_middle::ty::TyCtxt<'_>,
    split: &Split,
    fns: &IndexMap<DefId, FnInfo>,
    references: &IndexSet<(LocalModId, DefId)>,
) -> Vec<(LocalModId, (LocalModId, String, String, String))> {
    (references.iter())
        .filter_map(|&(from, id)| {
            let name = split.unchecked(id)?.to_owned();
            // By what the module's `use` renames its entry to, `close as
            // close_flight`: `import { close$ as closeFlight }`.
            let local = crate::analysis::renamed_in(tcx, from)
                .swap_remove(&id)
                .unwrap_or_else(|| name.clone());
            Some((from, (fns[&id].module, name.clone(), name, local)))
        })
        .collect()
}

/// How many of `stmts`, from the first, are the checks a function starts
/// with (`checks.rs`).
pub fn leading_checks(stmts: &[js::Stmt]) -> usize {
    (stmts.iter())
        .take_while(|stmt| matches!(&stmt.kind, js::StmtKind::Expr(e) | js::StmtKind::Assign(_, e) if is_check(e)))
        .count()
}

/// Whether `e` is one of the checks a function starts with (`checks.rs`).
fn is_check(e: &Expr) -> bool {
    e.helper_call_of_any(super::checks::CHECKS).is_some()
}
