//! A binding trait's associated type, declared through the TypeScript type
//! its `#[rust_js::types]` writes in the trait's `Self` and parameters
//! (#1817): webapi's `Listen<E>`'s `Event`, `EventMapOf<Self>[E]`, so
//! `<Target as Listen<Name>>::Event` is `EventMapOf<Target>[Name]`. A bound
//! of the trait, `Target: Listen<Name>`, says that projection is a type, so
//! it constrains each of the item's type parameters it indexes by as
//! TypeScript's own indexing does, `Name extends keyof EventMapOf<Target>`,
//! whether or not the signature names the projection. On a trait itself,
//! `#[rust_js::types]` is what a type it bounds is (`erasures`); on an
//! associated type, what its projection is: one attribute, of what it's
//! on. An associated type without one is `any`, as before.

use crate::collections::IndexMap;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::Declarations;
use super::written::{Written, written_types};

impl<'tcx> Declarations<'_, 'tcx> {
    /// `<T as Trait<..>>::Assoc` where `Assoc`'s `#[rust_js::types]` says
    /// what it is: that type, of the projection's `Self` and arguments.
    pub(super) fn projection(&mut self, assoc: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Value> {
        let declared = written_types(self.tcx, assoc)?;
        Some(self.written(self.tcx.parent(assoc), &declared, args))
    }

    /// Each of `def_id`'s type parameters a bound of a binding trait
    /// indexes an associated type's TypeScript type by, by its index, with
    /// the keys it's constrained to: `Name`'s `keyof EventMapOf<Target>` of
    /// `Target: Listen<Name>`.
    pub(super) fn indexed_params(&mut self, def_id: DefId) -> IndexMap<u32, Vec<Value>> {
        let tcx = self.tcx;
        let mut keys: IndexMap<u32, Vec<Value>> = IndexMap::default();
        let bounds: Vec<_> = (tcx.clauses_of(def_id).clauses.iter())
            .filter_map(|(clause, _)| clause.as_trait_clause())
            .map(|bound| bound.skip_binder().trait_ref)
            .collect();
        for bound in bounds {
            let trait_id = bound.def_id;
            let params = &tcx.generics_of(trait_id).own_params;
            for item in tcx.associated_item_def_ids(trait_id) {
                if tcx.def_kind(*item) != DefKind::AssocTy {
                    continue;
                }
                let Some(declared) = written_types(tcx, *item) else {
                    continue;
                };
                for (object, index) in indexings(&declared) {
                    // The trait's parameter it's indexed by, as the bound
                    // gives it: one of the item's own.
                    let Some(at) = params.iter().find(|p| p.name.as_str() == index) else {
                        continue;
                    };
                    let given: Ty<'tcx> = bound.args.type_at(at.index as usize);
                    let ty::Param(param) = given.kind() else {
                        continue;
                    };
                    let object = self.written(trait_id, object, bound.args);
                    let constraint = json!({ "kind": "keyof", "type": object });
                    let of = keys.entry(param.index).or_default();
                    if !of.contains(&constraint) {
                        of.push(constraint);
                    }
                }
            }
        }
        keys
    }
}

/// Each `object[index]` in `written` whose index is a name alone, with
/// that name.
fn indexings(written: &Written) -> Vec<(&Written, &str)> {
    let mut found = Vec::new();
    let mut walk = vec![written];
    while let Some(written) = walk.pop() {
        match written {
            Written::Indexed { object, index } => {
                if let Written::Reference {
                    from: None,
                    name,
                    args: None,
                } = &**index
                {
                    found.push((&**object, name.as_str()));
                }
                walk.push(object);
                walk.push(index);
            }
            Written::Union(types) | Written::Intersection(types) => walk.extend(types),
            Written::Conditional {
                check,
                extends,
                when_true,
                when_false,
            } => walk.extend([&**check, &**extends, &**when_true, &**when_false]),
            Written::Array { element, .. } => walk.push(element),
            Written::Reference { args: Some(args), .. } => walk.extend(args),
            _ => {}
        }
    }
    found
}
