//! What a module holds of a binding as a value of its own, its `const`,
//! and where a path reads one through the module that holds it (ADRs 0622,
//! 0633).

use super::*;

/// A binding a module holds as a value of its own, its default or a `pub
/// use`: its `const`, the JS its path reads, `const assign =
/// Object.assign;` (ADRs 0622, 0633).
#[derive(Clone)]
pub(in crate::lower) struct HeldBinding {
    pub(in crate::lower) binding: DefId,
    /// Its name in the module in Rust, which a caller's path names.
    pub(in crate::lower) ident: Symbol,
    /// Its name in the module's JS, the `use`'s.
    pub(in crate::lower) name: String,
    /// What it reads, `Object.assign`.
    pub(in crate::lower) path: String,
    /// Whether a `pub use` names it, so the module exports it by its name.
    pub(in crate::lower) export: bool,
    /// Whether it's the module's default.
    pub(in crate::lower) default: bool,
    /// The `use` that names it.
    pub(in crate::lower) span: Option<Span>,
    /// Where it names the binding through another of the crate's modules
    /// that holds it: that module's, re-exported (ADR 0633), not a
    /// `const` of its own.
    pub(in crate::lower) through: Option<HeldRead>,
}

impl HeldBinding {
    /// Whether the module exports it by its name: a `pub use` names it,
    /// and it isn't the module's default, which is how every caller reads
    /// it, in the crate and in another, as upstream's `shared/assign` has
    /// it alone (ADR 0773).
    pub(in crate::lower) fn exported_by_name(&self) -> bool {
        self.export && !self.default
    }

    /// What a caller that names it through its module imports: the
    /// module's default, as a function's (ADR 0240), else its name.
    pub(in crate::lower) fn import(&self) -> Option<&str> {
        match (self.default, self.export) {
            (true, _) => Some("default"),
            (false, true) => Some(&self.name),
            (false, false) => None,
        }
    }
}

/// The JS a binding is that a module can hold as its `const`: a global
/// function, no method and no JS module's import, `Object.assign` (ADR
/// 0622); or a static, a global or a path in an import,
/// `react#*.__CLIENT_INTERNALS..` (ADR 0633). A JS module's export itself,
/// `react#useState`, is that module's, re-exported (ADR 0240).
pub(in crate::lower) fn held_path(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    match tcx.def_kind(def_id) {
        DefKind::Static { .. } if is_binding(tcx, def_id) => {
            let path = js_name(tcx, def_id);
            match js_import(&path) {
                Some((_, "")) => None,
                _ => Some(path),
            }
        }
        DefKind::Fn => match js_form(tcx, def_id) {
            JsForm::Call(name) if is_binding(tcx, def_id) && !is_method(tcx, def_id) && js_import(&name).is_none() => {
                Some(name)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Where a caller's path reads a binding through the module that holds it
/// (ADR 0633): the module, what the caller imports of it, the default or
/// a name, and its `const`'s name there.
#[derive(Clone)]
pub(in crate::lower) struct HeldRead {
    pub(in crate::lower) holder: Holder,
    pub(in crate::lower) import: String,
    pub(in crate::lower) name: String,
    pub(in crate::lower) binding: DefId,
}

/// A module that holds a binding: the crate's, or a library's, by its JS
/// module (ADR 0100).
#[derive(Clone)]
pub(in crate::lower) enum Holder {
    Module(LocalModId),
    Library(String),
}

/// What each of the crate's modules holds of a binding as a value of its
/// own, and where each path of its bodies names one through a module that
/// holds it, by the path's span, MIR's and THIR's of what it names (ADR
/// 0633). Asked once, of every module and body; each read, a call, a `fn`
/// taken, a static's, asks `reads`.
#[derive(Default)]
pub(in crate::lower) struct Held {
    pub(in crate::lower) modules: HashMap<LocalModId, Vec<HeldBinding>>,
    pub(in crate::lower) reads: HashMap<Span, HeldRead>,
}

pub(in crate::lower) fn held<'tcx>(tcx: TyCtxt<'tcx>, foreign: &crate::lower::library::Foreign<'_, 'tcx>) -> Held {
    let mut held = Held::default();
    tcx.hir_for_each_module(|module| {
        module_held(tcx, foreign, module, &mut held.modules);
    });
    let mut paths = Paths {
        tcx,
        foreign,
        modules: &mut held.modules,
        reads: HashMap::new(),
    };
    for owner in tcx.hir_body_owners() {
        rustc_hir::intravisit::walk_body(&mut paths, tcx.hir_body_owned_by(owner));
    }
    held.reads = paths.reads;
    held
}

/// Each path of a body that names a binding through a module that holds it.
struct Paths<'a, 'b, 'tcx> {
    tcx: TyCtxt<'tcx>,
    foreign: &'a crate::lower::library::Foreign<'b, 'tcx>,
    modules: &'a mut HashMap<LocalModId, Vec<HeldBinding>>,
    reads: HashMap<Span, HeldRead>,
}

impl<'tcx> rustc_hir::intravisit::Visitor<'tcx> for Paths<'_, '_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if let ExprKind::Path(rustc_hir::QPath::Resolved(None, path)) = expr.kind
            && let Res::Def(DefKind::Fn | DefKind::Static { .. }, binding) = path.res
            && held_path(self.tcx, binding).is_some()
        {
            let scope = self.tcx.parent_module(expr.hir_id);
            if let Some(read) = named_through(self.tcx, self.foreign, scope, path.segments, binding, self.modules) {
                self.reads.insert(expr.span, read);
            }
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// The module a path names `binding` through, where that module holds it
/// (ADR 0633): the module its last but one segment is, `crate::assign` of
/// `crate::assign::assign`; or, of a name alone, the `use` in `scope` that
/// names it, `use crate::assign::assign;`, or a glob's module. One that
/// names the binding itself, `js::object::assign`, or a module that only
/// declares it, reads it in place.
fn named_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    scope: LocalModId,
    segments: &[rustc_hir::PathSegment<'_>],
    binding: DefId,
    modules: &mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> Option<HeldRead> {
    let [.., module, last] = segments else {
        let [only] = segments else { return None };
        let used = tcx.hir_module_items(scope).free_items().find_map(|id| {
            let item = tcx.hir_item(id);
            match item.kind {
                ItemKind::Use(path, rustc_hir::UseKind::Single(ident))
                    if ident.name == only.ident.name
                        && matches!(path.res.value_ns, Some(Res::Def(_, named)) if named == binding) =>
                {
                    // One the module renames, its own `const`, read by its
                    // name there, `slice.call(arguments)` (ADR 0622, amended).
                    let own = (ident.name != tcx.item_name(binding)
                        && ident.name != rustc_span::symbol::kw::Underscore)
                        .then(|| {
                            let held = module_held(tcx, foreign, scope, modules);
                            let found = held.iter().find(|h| h.binding == binding && h.ident == ident.name)?;
                            Some(HeldRead {
                                holder: Holder::Module(scope),
                                import: found.name.clone(),
                                name: found.name.clone(),
                                binding,
                            })
                        })
                        .flatten();
                    own.or_else(|| named_through(tcx, foreign, scope, path.segments, binding, modules))
                }
                ItemKind::Use(path, rustc_hir::UseKind::Glob) => match path.res.type_ns {
                    Some(Res::Def(DefKind::Mod, module)) => {
                        held_by(tcx, foreign, module, only.ident.name, binding, modules)
                    }
                    _ => None,
                },
                _ => None,
            }
        });
        // Else one the module holds itself, its `const`, read by its name
        // there as upstream's module reads its own: a static it declares
        // and makes its default, `if (!ReactSharedInternalsServer)`, not
        // the path again (rd659).
        return used.or_else(|| {
            let held = module_held(tcx, foreign, scope, modules);
            let found = (held.iter())
                .find(|h| h.binding == binding && h.ident == only.ident.name && h.through.is_none())?;
            Some(HeldRead {
                holder: Holder::Module(scope),
                import: found.name.clone(),
                name: found.name.clone(),
                binding,
            })
        });
    };
    match module.res {
        Res::Def(DefKind::Mod, module) => held_by(tcx, foreign, module, last.ident.name, binding, modules),
        _ => None,
    }
}

/// How `module` holds `binding` by its Rust name `ident`, if it does, and
/// exports it: the crate's module, as `module_held` says, or a library's,
/// as its manifest does.
fn held_by<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    module: DefId,
    ident: Symbol,
    binding: DefId,
    modules: &mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> Option<HeldRead> {
    let Some(local) = module.as_local() else {
        return library_held(foreign, module, ident, binding);
    };
    let module = LocalModId::new_unchecked(local);
    found_in(module, module_held(tcx, foreign, module, modules), ident, binding)
}

/// How a library's `module` holds `binding` by `ident`, as its manifest
/// says.
fn library_held(
    foreign: &crate::lower::library::Foreign<'_, '_>,
    module: DefId,
    ident: Symbol,
    binding: DefId,
) -> Option<HeldRead> {
    let held = foreign.held(module, ident, binding)?;
    Some(HeldRead {
        holder: Holder::Library(held.from.clone()),
        import: held.export.clone(),
        name: held.name.clone(),
        binding,
    })
}

/// The import each `use` of `module` makes of a binding it names through a
/// library's module that holds it, by the binding: `use
/// shared::internals::REACT_SHARED_INTERNALS as ..;`'s
/// `shared/ReactSharedInternals`'s default, which a `use .. as` names as it
/// renames it, as any import is (rd651, ADR 0633, amended).
pub(in crate::lower) fn library_uses(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    module: LocalModId,
) -> HashMap<DefId, (String, String)> {
    (tcx.hir_module_items(module).free_items())
        .filter_map(|id| {
            let ItemKind::Use(path, rustc_hir::UseKind::Single(_)) = tcx.hir_item(id).kind else {
                return None;
            };
            let Some(Res::Def(_, binding)) = path.res.value_ns else {
                return None;
            };
            let [.., holder, last] = path.segments else { return None };
            let Res::Def(DefKind::Mod, holder) = holder.res else {
                return None;
            };
            let read = library_held(foreign, holder, last.ident.name, binding)?;
            let Holder::Library(from) = read.holder else {
                return None;
            };
            Some((binding, (from, read.import)))
        })
        .collect()
}

/// How the crate's `module`, of what it holds, `held`, holds `binding` by
/// `ident`, and exports it.
fn found_in(module: LocalModId, held: &[HeldBinding], ident: Symbol, binding: DefId) -> Option<HeldRead> {
    let found = held.iter().find(|h| h.binding == binding && h.ident == ident)?;
    Some(HeldRead {
        holder: Holder::Module(module),
        import: found.import()?.to_string(),
        name: found.name.clone(),
        binding,
    })
}

impl Held {
    /// The module a `use` names `binding` through where that module holds
    /// it, its path's last but one, `shared::assign` of `use
    /// shared::assign::assign;`, the crate's or a library's: what its
    /// import is of, placed where the `use` is (rd609, ADR 1063, amended).
    pub(in crate::lower) fn use_holder(
        &self,
        foreign: &crate::lower::library::Foreign<'_, '_>,
        segments: &[rustc_hir::PathSegment<'_>],
        binding: DefId,
    ) -> Option<Holder> {
        let [.., module, last] = segments else { return None };
        let Res::Def(DefKind::Mod, module) = module.res else {
            return None;
        };
        let read = match module.as_local() {
            None => library_held(foreign, module, last.ident.name, binding),
            Some(local) => {
                let local = LocalModId::new_unchecked(local);
                found_in(local, self.modules.get(&local)?, last.ident.name, binding)
            }
        };
        read.map(|read| read.holder)
    }
}

/// Each binding `module` names as a value of its own, its default or a
/// `pub use`: its `const`, the JS it reads as it is, `const assign =
/// Object.assign;`, which a JS caller calls with JS's arguments, exported
/// where it's a `pub use`'s (ADR 0622). Not one its `use` names through
/// another module that holds it, which is that module's, re-exported (ADR
/// 0633). Its JS, its `.d.ts` and its library's manifest ask this.
fn module_held<'m, 'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    module: LocalModId,
    modules: &'m mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> &'m Vec<HeldBinding> {
    if !modules.contains_key(&module) {
        // Rust refuses a cycle of `use`s, so none is met here.
        modules.insert(module, Vec::new());
        let held = held_of(tcx, foreign, module, modules);
        modules.insert(module, held);
    }
    &modules[&module]
}

fn held_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    module: LocalModId,
    modules: &mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> Vec<HeldBinding> {
    let default = (crate::lower::default_exports::default_exports(tcx, module).into_iter())
        .find_map(|(named, _)| named.filter(|&named| held_path(tcx, named).is_some()));
    let mut held: Vec<HeldBinding> = Vec::new();
    let mut add = |made: HeldBinding| match held.iter_mut().find(|h| h.name == made.name) {
        Some(was) => {
            was.export |= made.export;
            was.default |= made.default;
        }
        None => held.push(made),
    };
    // Where its `use` is, that names it; the default by its own name.
    for id in tcx.hir_module_items(module).free_items() {
        let item = tcx.hir_item(id);
        let ItemKind::Use(path, rustc_hir::UseKind::Single(ident)) = item.kind else {
            continue;
        };
        let Some(Res::Def(_, binding)) = path.res.value_ns else {
            continue;
        };
        let Some(read) = held_path(tcx, binding) else {
            continue;
        };
        let export = crate::lower::analysis::is_reexport(tcx, item);
        let name = use_name(tcx, item.owner_id.to_def_id(), (ident.name, binding), || {
            fn_name(tcx, binding)
        });
        let named_default = default == Some(binding) && name == fn_name(tcx, binding);
        // A private `use` that renames a global, `use js::array::SLICE as
        // slice;`, is the module's `const slice = Array.prototype.slice;`,
        // as upstream's is (the user's decision, ADR 0622, amended).
        let renamed_global = ident.name != tcx.item_name(binding)
            && ident.name != rustc_span::symbol::kw::Underscore
            && js_import(&read).is_none();
        if !export && !named_default && !renamed_global {
            continue;
        }
        let through = named_through(tcx, foreign, module, path.segments, binding, modules);
        // A library's, which a private `use` reads through it; a `pub use`
        // re-exports it from the library's file (ADR 0240, amended). And
        // another module's of the crate a private `use` renames, `use
        // crate::held::assign as merge;`, imported from it by that name, no
        // global's `const` of its own (rd651).
        if !export
            && through.as_ref().is_some_and(|read| match read.holder {
                Holder::Library(_) => true,
                Holder::Module(_) => !named_default,
            })
        {
            continue;
        }
        add(HeldBinding {
            binding,
            ident: ident.name,
            name,
            path: read,
            export,
            default: named_default,
            span: Some(item.span),
            through,
        });
    }
    if let Some(binding) = default
        && let Some(path) = held_path(tcx, binding)
    {
        add(HeldBinding {
            binding,
            ident: tcx.item_name(binding),
            name: fn_name(tcx, binding),
            path,
            export: false,
            default: true,
            span: None,
            through: None,
        });
    }
    held
}
