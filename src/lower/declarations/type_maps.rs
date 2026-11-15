//! A trait whose impls for named unit structs are one TypeScript object
//! type (createDetails, ADR 1164): `#[rust_js::type_map(map =
//! "ReasonVocabulary", keys = "ChangeReason", lookup = "EventOf")]` on
//! `trait Reason { type Event; }` declares each impl's unit struct's name
//! its key and its one associated type its value, `type ReasonVocabulary =
//! { "trigger-press": MouseEvent | KeyboardEvent; .. }`, in the impls'
//! order, each key with its struct's doc comment; `keys`, `keyof` it; and
//! `lookup`, a key's value, or of any other string the value the trait's
//! impl for `&'static str` gives, `type EventOf<Reason extends string> =
//! Reason extends ChangeReason ? ReasonVocabulary[Reason] : Event`. The
//! impls are the one source of all three.

use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol};
use serde_json::{Value, json};

use super::Declarations;
use super::comments::documented;

/// The names `#[rust_js::type_map(..)]` gives: its map's, and its keys'
/// and lookup's where it names them.
struct Names {
    map: String,
    keys: Option<String>,
    lookup: Option<String>,
}

/// `trait_id`'s `#[rust_js::type_map(..)]`, or where it's written otherwise,
/// its span.
fn names(tcx: TyCtxt<'_>, trait_id: DefId) -> Option<Result<Names, Span>> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("type_map")];
    let attr = tcx.get_attrs_by_path(trait_id, &path).next()?;
    let Some(items) = attr.meta_item_list() else {
        return Some(Err(attr.span()));
    };
    let mut names = Names {
        map: String::new(),
        keys: None,
        lookup: None,
    };
    for item in &items {
        let (Some(ident), Some(value)) = (item.ident(), item.meta_item().and_then(|meta| meta.value_str())) else {
            return Some(Err(attr.span()));
        };
        let value = value.to_string();
        match ident.as_str() {
            "map" if names.map.is_empty() => names.map = value,
            "keys" if names.keys.is_none() => names.keys = Some(value),
            "lookup" if names.lookup.is_none() => names.lookup = Some(value),
            _ => return Some(Err(attr.span())),
        }
    }
    match names.map.is_empty() || names.lookup.is_some() && names.keys.is_none() {
        true => Some(Err(attr.span())),
        false => Some(Ok(names)),
    }
}

/// Whether `assoc` is the associated type of a `#[rust_js::type_map]`
/// trait, whose projection is its lookup's, or its map's indexed by `Self`.
pub(super) fn of_a_type_map(tcx: TyCtxt<'_>, assoc: DefId) -> bool {
    tcx.opt_parent(assoc).is_some_and(|parent| {
        tcx.def_kind(parent) == rustc_hir::def::DefKind::Trait && matches!(names(tcx, parent), Some(Ok(_)))
    })
}

impl<'tcx> Declarations<'_, 'tcx> {
    /// `<T as Reason>::Event` of a `#[rust_js::type_map]` trait: its lookup
    /// of `T`, `EventOf<T>`, or without one its map's, `ReasonVocabulary[T]`,
    /// so a generic struct's field of it says what a caller's reason gives.
    pub(super) fn type_map_projection(&mut self, assoc: DefId, args: ty::GenericArgsRef<'tcx>) -> Value {
        let names = match names(self.tcx, self.tcx.parent(assoc)) {
            Some(Ok(names)) => names,
            _ => unreachable!("`of_a_type_map` said it's one"),
        };
        let of = self.ts(args.type_at(0));
        match names.lookup {
            Some(lookup) => json!({ "kind": "reference", "name": lookup, "args": [of] }),
            None => {
                json!({ "kind": "indexed", "object": { "kind": "reference", "name": names.map, "args": [] }, "index": of })
            }
        }
    }

    /// What `trait_id`'s `#[rust_js::type_map(..)]` declares: its map, and
    /// its keys and lookup where it names them; none of a trait without one.
    pub(super) fn type_map(&mut self, trait_id: DefId) -> Vec<Value> {
        let tcx = self.tcx;
        let names = match names(tcx, trait_id) {
            None => return Vec::new(),
            Some(Ok(names)) => names,
            Some(Err(span)) => {
                tcx.dcx().span_err(
                    span,
                    "rust-js: `#[rust_js::type_map(map = \"ReasonVocabulary\", keys = \"ChangeReason\", lookup = \"EventOf\")]` names its map, and its keys and the lookup of them where it has them, a lookup only with keys",
                );
                return Vec::new();
            }
        };
        let types: Vec<DefId> = (tcx.associated_items(trait_id).in_definition_order())
            .filter(|item| matches!(item.kind, ty::AssocKind::Type { .. }))
            .map(|item| item.def_id)
            .collect();
        let [assoc] = types[..] else {
            tcx.dcx().span_err(
                tcx.def_span(trait_id),
                "rust-js: a `#[rust_js::type_map]` trait has one associated type, each impl's value in its map",
            );
            return Vec::new();
        };
        // The crate's impls, in the order its source writes them.
        let mut impls: Vec<DefId> = tcx.all_impls(trait_id).filter(|imp| imp.is_local()).collect();
        impls.sort_by_key(|&imp| tcx.def_span(imp).lo());
        let mut members = Vec::new();
        let mut otherwise = None;
        for imp in impls {
            let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
            let Some(value) = self.impl_type(imp, assoc) else {
                continue;
            };
            match self_ty.kind() {
                ty::Adt(adt, _) if adt.is_struct() => {
                    let Some(key) = crate::lower::bindings::unit_name(tcx, adt.did()) else {
                        tcx.dcx().span_err(
                            tcx.def_span(imp),
                            "rust-js: a `#[rust_js::type_map]` trait's impl is of a unit struct whose `#[rust_js::name = \"..\"]` is its key, or of `&'static str`, its lookup's other strings",
                        );
                        continue;
                    };
                    let member =
                        json!({ "kind": "property", "name": key, "optional": false, "readonly": false, "type": value });
                    members.push(documented(member, tcx, adt.did()));
                }
                ty::Ref(_, str, _) if str.is_str() => otherwise = Some(value),
                _ => {
                    tcx.dcx().span_err(
                        tcx.def_span(imp),
                        "rust-js: a `#[rust_js::type_map]` trait's impl is of a unit struct whose `#[rust_js::name = \"..\"]` is its key, or of `&'static str`, its lookup's other strings",
                    );
                }
            }
        }
        let reference = |name: &str| json!({ "kind": "reference", "name": name, "args": [] });
        let alias = |name: &str, parameters: Value, ty: Value| json!({ "kind": "type", "name": name, "exported": true, "declare": false, "typeParameters": parameters, "type": ty });
        let mut declared = vec![documented(
            alias(&names.map, json!([]), json!({ "kind": "object", "members": members })),
            tcx,
            trait_id,
        )];
        if let Some(keys) = &names.keys {
            declared.push(alias(
                keys,
                json!([]),
                json!({ "kind": "keyof", "type": reference(&names.map) }),
            ));
        }
        if let (Some(keys), Some(lookup)) = (&names.keys, &names.lookup) {
            // Its type parameter the trait's name, `EventOf<Reason extends string>`.
            let parameter = tcx.item_name(trait_id).to_string();
            let conditional = json!({
                "kind": "conditional",
                "check": reference(&parameter),
                "extends": reference(keys),
                "whenTrue": { "kind": "indexed", "object": reference(&names.map), "index": reference(&parameter) },
                "whenFalse": otherwise.unwrap_or_else(|| json!({ "kind": "keyword", "keyword": "never" })),
            });
            let parameters = json!([{ "name": parameter, "constraint": { "kind": "keyword", "keyword": "string" } }]);
            declared.push(alias(lookup, parameters, conditional));
        }
        declared
    }

    /// The TypeScript type `imp` gives the trait's associated type `assoc`.
    fn impl_type(&mut self, imp: DefId, assoc: DefId) -> Option<Value> {
        let item = (self.tcx.associated_items(imp).in_definition_order())
            .find(|item| item.trait_item_def_id() == Some(assoc))?;
        let ty: Ty<'tcx> = self
            .tcx
            .type_of(item.def_id)
            .instantiate_identity()
            .skip_normalization();
        Some(self.ts(ty))
    }
}
