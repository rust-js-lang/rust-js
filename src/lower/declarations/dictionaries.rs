//! A crate's traits in its declarations, as the type of their dictionaries
//! (#550, ADR 1077, amended): `export interface Shape<Self> { area(self:
//! Self): number; }`, each dictionary of an impl its module exports, `export
//! const circleShape: Shape<Circle>;`, and a bound's dictionary a function
//! takes after its arguments, `SShape: Shape<S>`. What a dictionary holds
//! is `traits::method_dictionary`'s answer.

use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, documented, keyword, reference};

/// The methods of the dictionaries of `trait_id`, a `pub` trait of the
/// crate's, where each is all a dictionary holds that generic code reads,
/// as its signature says (`method_dictionary`): then it's declared, and so
/// are its dictionaries, `interface Middleware<Self>`, which a library's
/// contract lists among the types its declarations export by name (#1870).
pub(in crate::lower) fn declared_methods<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    trait_id: DefId,
) -> Option<Vec<DefId>> {
    if !tcx.visibility(trait_id).is_public() {
        return None;
    }
    crate::lower::traits::method_dictionary(tcx, foreign, trait_id)
}

/// The types the declarations of `trait_id`, the crate's public trait,
/// declare, which a `pub use` of it re-exports (#2095): the type of its
/// dictionaries, `interface Shape<Self>`, by the trait's name, where it has
/// one (#550); and its type map's aliases, each by the name its attribute
/// gives it, which a `use .. as` doesn't rename (ADR 1164).
pub(in crate::lower) fn trait_types<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    trait_id: DefId,
) -> (Option<String>, Vec<String>) {
    let interface = declared_methods(tcx, foreign, trait_id).map(|_| tcx.item_name(trait_id).to_string());
    (interface, super::type_maps::aliases(tcx, trait_id))
}

impl<'tcx> Declarations<'_, 'tcx> {
    /// `declared_methods` of `trait_id`, of this crate's dependencies.
    fn declared_methods(&self, trait_id: DefId) -> Option<Vec<DefId>> {
        declared_methods(self.tcx, self.foreign, trait_id)
    }

    /// `export interface Shape<Self> { area(self: Self): number; describe(self:
    /// Self): string; }`: the type of `trait_id`'s dictionaries, of its
    /// impl's type, `Self`, each method its entry, of its value first, as
    /// the dictionary's function takes it. `None` of one not declared.
    pub(super) fn trait_interface(&mut self, trait_id: DefId) -> Option<Value> {
        let tcx = self.tcx;
        let methods = self.declared_methods(trait_id)?;
        let mut members = Vec::new();
        for method in methods {
            let outer = std::mem::take(&mut self.erased);
            self.erased = self.erased(method);
            let function = self.function(method);
            self.erased = outer;
            let declared = json!({
                "kind": "method",
                "name": crate::lower::bindings::fn_name(tcx, method),
                "optional": false,
                "typeParameters": [],
                "params": function["params"],
                "returns": function["returns"],
            });
            members.push(documented(declared, tcx, method));
        }
        Some(json!({
            "kind": "interface",
            "name": tcx.item_name(trait_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": [{ "name": "Self" }],
            "extends": [],
            "members": members,
        }))
    }

    /// `export const circleShape: Shape<Circle>;`: the dictionary of `imp`,
    /// an impl of a declared trait that takes nothing, its module's `const`
    /// (ADR 0592), which its module exports, as the trait is `pub`. Not a generic impl's,
    /// `wrapperShape(TDisplay)`, made of its own bounds' dictionaries.
    pub(super) fn impl_dictionary(&mut self, imp: DefId) -> Option<Value> {
        let tcx = self.tcx;
        if tcx.def_kind(imp) != (DefKind::Impl { of_trait: true })
            || !(tcx.generics_of(imp).own_params.iter())
                .all(|param| matches!(param.kind, ty::GenericParamDefKind::Lifetime))
        {
            return None;
        }
        // Named as a function is, where its module writes it.
        let name = (self.fns.get(&imp))
            .filter(|info| info.module == self.module)?
            .name
            .clone();
        let tr = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
        if !self.declares_dictionary(tr.def_id) {
            return None;
        }
        let ty = self.dictionary_type(tr);
        Some(json!({ "kind": "const", "name": name, "exported": true, "declare": false, "type": ty }))
    }

    /// Whether the dictionaries of `trait_id` are declared, where what
    /// names one can import it: a `pub` trait's of the crate's whose
    /// dictionaries hold its methods alone (`declared_methods`), in a module
    /// with a file. Not a std trait's, whose dictionaries are the
    /// runtime's own, nor a library's.
    pub(super) fn declares_dictionary(&self, trait_id: DefId) -> bool {
        // A library's whose declarations declare its dictionaries, as its
        // contract says, imported from them (#1870).
        let Some(local) = trait_id.as_local() else {
            return self.foreign.declared_type(trait_id).is_some();
        };
        let home = self.tcx.parent_module_from_def_id(local);
        (home == self.module || self.files.contains_key(&home)) && self.declared_methods(trait_id).is_some()
    }

    /// The type of the dictionary of `tr`, a bound of a trait whose
    /// dictionaries are declared (`declares_dictionary`): `Shape<S>`,
    /// imported from the trait's module where that's another.
    pub(super) fn dictionary_type(&mut self, tr: ty::TraitRef<'tcx>) -> Value {
        let of = self.ts(tr.self_ty());
        self.dictionary_of(tr.def_id, of)
    }

    /// `Shape<of>`: the type of a dictionary of `trait_id`, declared
    /// (`declares_dictionary`), of a value of `of`, imported from the
    /// trait's module where that's another.
    fn dictionary_of(&mut self, trait_id: DefId, of: Value) -> Value {
        if let Some((from, name, default)) = self.foreign.declared_type(trait_id) {
            self.library_imports.insert((from.clone(), name.clone(), *default));
            return reference(name, vec![of]);
        }
        let home = self.tcx.parent_module_from_def_id(trait_id.expect_local());
        let name = self.tcx.item_name(trait_id).to_string();
        if home != self.module {
            self.imported(self.files[&home].clone(), trait_id, name.clone());
        }
        reference(&name, vec![of])
    }

    /// `{ value: unknown; impl: Shape<unknown> }`: a `dyn` of `trait_id`,
    /// a trait whose dictionaries are declared, as its JS has it, a pair of
    /// a value and its impl's dictionary (ADR 0049), in that order. Of a
    /// value of some type, which no type parameter names: TypeScript's
    /// `unknown`, as `Promise<unknown>` is a promise of some type (#620).
    pub(super) fn pair(&mut self, trait_id: DefId) -> Value {
        let dictionary = self.dictionary_of(trait_id, keyword("unknown"));
        let property = |name: &str, ty: Value| json!({ "kind": "property", "name": name, "optional": false, "readonly": false, "type": ty });
        let members = [
            property(crate::js::HELD, keyword("unknown")),
            property(crate::js::DICTIONARY, dictionary),
        ];
        json!({ "kind": "object", "members": members })
    }
}
