//! A type alias in a module's declarations (ADR 0196): one the Rust writes
//! by its name, as TypeScript has it, where rustc's type has it expanded;
//! one whose `#[rust_js::types]` names a type, that type; and one whose
//! `#[rust_js::types]` writes a type of its own, declared as written (rd538,
//! ADR 0811, amended).

use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, Given, Written, reference, written_types};

/// Whether `did` is an alias whose `#[rust_js::types]` writes a type of its
/// own, not a name: ReactFlightReplyClient's `ReactServerValue`, a
/// `js::Unknown` whose declaration is upstream's union, `export type
/// ReactServerValue = string | number | ..;`, named where it's used (rd538,
/// ADR 0811, amended). One that names a type, `react#MouseEventHandler<T>`,
/// is that type wherever it's used, and declares nothing of its own.
pub(in crate::lower) fn declares_written_alias(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.def_kind(did) == DefKind::TyAlias
        && written_types(tcx, did).is_some_and(|written| !matches!(written, Written::Reference { .. }))
}

impl<'tcx> Declarations<'_, 'tcx> {
    /// A type as the Rust writes it, `of` making it of rustc's: each alias
    /// written in it by its name, as TypeScript has it, where rustc's type
    /// has it expanded, the crate's own, `Context<Toc>`, or one that says
    /// what it is to TypeScript, `MouseEventHandler<HTMLButtonElement>`.
    pub(super) fn as_written(&mut self, written: Option<&hir::Ty<'tcx>>, of: impl FnOnce(&mut Self) -> Value) -> Value {
        let mut named = Vec::new();
        let mut function_names = Vec::new();
        if let Some(written) = written {
            self.aliases_in(written, &mut named);
            self.written_function_names(written, &mut function_names);
        }
        let outer = (self.named.len(), self.function_names.len());
        self.named.extend(named);
        self.function_names.extend(function_names);
        let value = of(self);
        self.named.truncate(outer.0);
        self.function_names.truncate(outer.1);
        value
    }

    /// Each alias `written` names that TypeScript has by its name, and that.
    pub(super) fn aliases_in(&mut self, written: &hir::Ty<'tcx>, named: &mut Vec<(Ty<'tcx>, Value)>) {
        match &written.kind {
            hir::TyKind::Path(hir::QPath::Resolved(None, path)) => {
                if let Res::Def(DefKind::TyAlias, did) = path.res {
                    if let Some(value) = self.alias_named(written, path, did) {
                        let ty = rustc_hir_analysis::lower_ty(self.tcx, written);
                        named.push((self.tcx.erase_and_anonymize_regions(ty), value));
                        return;
                    }
                    // One not named, a private alias of the crate's, is what
                    // it's written as, its own aliases by their names, a
                    // `type ScheduleCallback = dyn Fn(PriorityLevel, ..)`'s
                    // `PriorityLevel` (#630). Not one of type parameters of
                    // its own, whose body is written of them.
                    if let Some(local) = did.as_local()
                        && !(self.tcx.generics_of(did).own_params.iter())
                            .any(|param| !matches!(param.kind, ty::GenericParamDefKind::Lifetime))
                        && let Some(body) = self.tcx.hir_node_by_def_id(local).ty()
                    {
                        self.aliases_in(body, named);
                    }
                }
                self.aliases_in_args(path, named);
            }
            // A `dyn Fn(PriorityLevel) -> Task`'s parameters and result as
            // written (#630); any other trait's arguments.
            hir::TyKind::TraitObject(bounds, _) => {
                for bound in *bounds {
                    self.aliases_in_args(bound.trait_ref.path, named);
                }
            }
            hir::TyKind::Ref(_, inner) | hir::TyKind::Ptr(inner) => self.aliases_in(inner.ty, named),
            hir::TyKind::Slice(inner) | hir::TyKind::Array(inner, _) => self.aliases_in(inner, named),
            hir::TyKind::Tup(items) => {
                for item in *items {
                    self.aliases_in(item, named);
                }
            }
            _ => {}
        }
    }

    /// Each alias written in `path`'s arguments, and in its constraints,
    /// `Fn(A) -> R`'s `R`.
    fn aliases_in_args(&mut self, path: &hir::Path<'tcx>, named: &mut Vec<(Ty<'tcx>, Value)>) {
        for args in path.segments.iter().filter_map(|s| s.args) {
            for arg in args.args {
                if let hir::GenericArg::Type(ty) = arg {
                    self.aliases_in(ty.as_unambig_ty(), named);
                }
            }
            for ty in args.constraints.iter().filter_map(|constraint| constraint.ty()) {
                self.aliases_in(ty, named);
            }
        }
    }

    /// An alias written, `did`, by its name: one that says what it is to
    /// TypeScript, or one of the crate's it declares, this module's or
    /// another's, imported from it (ADR 0210), of its arguments as written.
    pub(super) fn alias_named(&mut self, written: &hir::Ty<'tcx>, path: &hir::Path<'tcx>, did: DefId) -> Option<Value> {
        // One that names a type is that type; one that writes a type of its
        // own is declared, and named where it's exported (rd538).
        let own = declares_written_alias(self.tcx, did);
        if written_types(self.tcx, did).is_some() && !(own && self.tcx.visibility(did).is_public()) {
            return self.written_alias(written);
        }
        let name = match did.as_local() {
            Some(local) => {
                if !self.tcx.visibility(did).is_public() {
                    return None;
                }
                let home = self.tcx.parent_module_from_def_id(local);
                let name = self.tcx.item_name(did).to_string();
                if home != self.module {
                    let file = self.files.get(&home)?.clone();
                    self.imported(file, did, name.clone());
                }
                name
            }
            // A library's, imported from the declarations that export it
            // (ADR 0210, amended, rd642); where none do, one that writes a
            // type of its own as written, another expanded.
            None => match self.foreign.declared_type(did) {
                Some((from, name, default)) => {
                    self.library_imports.insert((from.clone(), name.clone(), *default));
                    name.clone()
                }
                None if own => return self.written_alias(written),
                None => return None,
            },
        };
        let given: Vec<&hir::Ty<'tcx>> = (path.segments.last()?.args.map(|args| args.args).unwrap_or_default())
            .iter()
            .filter_map(|arg| match arg {
                hir::GenericArg::Type(ty) => Some(ty.as_unambig_ty()),
                _ => None,
            })
            .collect();
        let args = (given.into_iter())
            .map(|given| {
                let ty = rustc_hir_analysis::lower_ty(self.tcx, given);
                self.as_written(Some(given), |out| out.ts(ty))
            })
            .collect();
        Some(reference(&name, args))
    }

    /// A type written as an alias that says what it is to TypeScript,
    /// `#[rust_js::types = "react#MouseEventHandler<T>"]`: that, as a person
    /// writes it, `MouseEventHandler<HTMLButtonElement>`, where rustc's
    /// type is the alias expanded. Its arguments are those written, one
    /// left to its default left out, as TypeScript's has the same.
    pub(super) fn written_alias(&mut self, written: &hir::Ty<'tcx>) -> Option<Value> {
        let hir::TyKind::Path(hir::QPath::Resolved(None, path)) = &written.kind else {
            return None;
        };
        let Res::Def(DefKind::TyAlias, did) = path.res else {
            return None;
        };
        let declared = written_types(self.tcx, did)?;
        let given: Vec<&hir::Ty<'tcx>> = (path.segments.last()?.args)
            .map(|args| args.args.iter())
            .into_iter()
            .flatten()
            .filter_map(|arg| match arg {
                hir::GenericArg::Type(ty) => Some(ty.as_unambig_ty()),
                _ => None,
            })
            .collect();
        let params: Vec<Symbol> = (self.tcx.generics_of(did).own_params.iter())
            .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Type { .. }))
            .map(|param| param.name)
            .collect();
        let at = |name: &str| params.iter().position(|param| param.as_str() == name);
        // Each of its parameters is what's written of it; one left to its
        // default, none written.
        let mut param = |out: &mut Self, name: &str| match at(name) {
            Some(at) => match given.get(at) {
                Some(given) => {
                    let ty = rustc_hir_analysis::lower_ty(out.tcx, given);
                    Given::Is(out.as_written(Some(given), |out| out.ts(ty)))
                }
                None => Given::Unknown,
            },
            None => Given::NotParam,
        };
        // Its arguments end where one's left to its default, as TypeScript's
        // has the same: `MouseEventHandler` of `T = Element`. Where one's
        // left so in another place, it's as rustc has it, expanded.
        if let Written::Reference {
            from,
            name,
            args: Some(wanted),
        } = &declared
        {
            let mut args = Vec::new();
            for wanted in wanted {
                if let Written::Reference {
                    from: None,
                    name,
                    args: None,
                } = wanted
                    && at(name).is_some_and(|at| at >= given.len())
                {
                    break;
                }
                args.push(self.written_value(wanted, &mut param, &mut Vec::new())?);
            }
            let name = self.imported_from(from.as_deref(), name, Some(did));
            return Some(reference(&name, args));
        }
        self.written_value(&declared, &mut param, &mut Vec::new())
    }

    /// `export type RouteTag = "foundation" | "intermediate";`, of an enum
    /// whose variants hold nothing: each is its name (ADR 0013).
    /// `export type Toc = TocItem[];` of `pub type Toc = Vec<TocItem>;`.
    pub(super) fn alias(&mut self, def_id: DefId) -> Value {
        let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
        // One whose `#[rust_js::types]` writes a type of its own, as it
        // writes it (rd538).
        let ty = match written_types(self.tcx, def_id).filter(|_| declares_written_alias(self.tcx, def_id)) {
            Some(declared) => self.written(def_id, &declared, ty::GenericArgs::identity_for_item(self.tcx, def_id)),
            None => self.as_written(self.written_type(def_id), |out| out.ts(ty)),
        };
        json!({
            "kind": "type",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "type": ty,
        })
    }
}
