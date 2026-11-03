//! What a `#[rust_js::flatten]` field holds as its parent's own keys,
//! beside a struct's fields (ADR 0204): a `RefCell` of a discriminated
//! union, a class's state (rd380, ADR 1044), and an enum each of whose
//! variants a key of its own tells, a node's kind (#504, ADR 1071). The
//! crate's of each, `flattened`, is the one answer representation,
//! validation and declarations read.

use super::{declared_tag, field_key, is_flatten, is_otherwise, is_test_field, is_untagged};
use crate::lower::Body;
use rustc_hir::def::{CtorKind, DefKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Of `ty`, a flattened field's type, the state it holds where it's a
/// `RefCell` of a discriminated union, `#[rust_js::tag = "status"]`, each
/// variant of named fields or none (rd380, ADR 1044): its tag and each
/// key a variant has, in the order they're first declared, the parent's
/// own properties. `None` of any other type.
pub(in crate::lower) fn flattened_state<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
) -> Option<(ty::AdtDef<'tcx>, Vec<String>)> {
    let ty::Adt(cell, args) = ty.kind() else { return None };
    if !crate::lower::recognition::is_std_def(tcx, cell.did(), crate::lower::recognition::StdItem::RefCell) {
        return None;
    }
    let ty::Adt(state, _) = args.type_at(0).kind() else {
        return None;
    };
    let tag = declared_tag(tcx, state.did())?;
    if !state.is_enum()
        || (state.variants().iter()).any(|v| v.ctor.is_some() && !v.fields.is_empty() || is_otherwise(tcx, v.def_id))
    {
        return None;
    }
    let mut keys = vec![tag];
    for field in state.variants().iter().flat_map(|v| v.fields.iter()) {
        let key = field_key(tcx, field).text().to_string();
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    Some((*state, keys))
}

/// An enum told apart by its keys (#504): each variant of named fields
/// has one no other variant has, which is in its object whatever it holds,
/// `undefined` too, and tells it, `'getter' in node`; one variant at most
/// has none, `{}`, what the others aren't.
#[derive(Clone, Debug)]
pub(in crate::lower) struct Keyed {
    /// The key that tells each variant, by its index: the field
    /// `#[rust_js::test]` marks, else its first no other variant has;
    /// `None` of the one without fields.
    pub tells: Vec<Option<String>>,
    /// Each key a variant has, in the order they're first declared.
    pub keys: Vec<String>,
}

impl Keyed {
    /// The keys of variant `index`'s that tells it, or `None` of the one
    /// without fields.
    pub(in crate::lower) fn tell(&self, index: rustc_abi::VariantIdx) -> Option<&str> {
        self.tells[index.as_usize()].as_deref()
    }
}

/// `adt`'s keys where it may be told apart by them, an enum of no tag
/// (`#[rust_js::tag]`) nor `#[rust_js::untagged]`, nor std's, each
/// variant of named fields or none, one at most of none: `None` where it
/// can't. Whether it is so is `Flattened::keyed`'s.
pub(in crate::lower) fn keyed_shape(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> Option<Keyed> {
    if !adt.is_enum()
        || declared_tag(tcx, adt.did()).is_some()
        || is_untagged(tcx, adt.did())
        || crate::lower::recognition::is_std_crate(tcx, adt.did())
    {
        return None;
    }
    let mut own: Vec<Vec<String>> = Vec::new();
    for variant in adt.variants() {
        if is_otherwise(tcx, variant.def_id) || variant.ctor_kind() == Some(CtorKind::Fn) {
            return None;
        }
        let keys = (variant.fields.iter())
            .map(|field| field_key(tcx, field).name().map(str::to_owned))
            .collect::<Option<Vec<String>>>()?;
        own.push(keys);
    }
    let units = own.iter().filter(|keys| keys.is_empty()).count();
    if units > 1 || units == own.len() {
        return None;
    }
    let mut tells = Vec::new();
    for (i, (variant, keys)) in adt.variants().iter().zip(&own).enumerate() {
        if keys.is_empty() {
            tells.push(None);
            continue;
        }
        let unique = |key: &String| (own.iter().enumerate()).all(|(j, other)| j == i || !other.contains(key));
        let marked: Vec<&String> = (variant.fields.iter().zip(keys))
            .filter(|(field, _)| is_test_field(tcx, field.did))
            .map(|(_, key)| key)
            .collect();
        let tell = match marked.as_slice() {
            [] => keys.iter().find(|key| unique(key))?,
            [one] if unique(one) => *one,
            _ => return None,
        };
        tells.push(Some(tell.clone()));
    }
    let mut keys: Vec<String> = Vec::new();
    for key in own.into_iter().flatten() {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    Some(Keyed { tells, keys })
}

/// What the crate's flattened fields hold that's their parents' own keys,
/// beside a struct's fields.
#[derive(Default)]
pub(in crate::lower) struct Flattened {
    /// The enums its classes hold as a flattened state (`flattened_state`),
    /// each its object's own keys (rd380, ADR 1044): a `&mut` to one is a
    /// handle, whose setter writes each key there, wherever it's given,
    /// never the object, which holds the class's other keys too (#478).
    pub states: HashSet<DefId>,
    /// Its enums told by their keys (`keyed_shape`, #504): each one a
    /// field flattens, `kind: NodeKind`, or a struct's flattened type
    /// parameter is given, `ReactiveNode<NodeKind>` of `kind: K`, where
    /// the crate names it so. Every value of one is its keys, a variant's
    /// told by its own, `'getter' in node`, where it's flattened or not.
    pub keyed: HashSet<DefId>,
    /// Each struct's flattened type parameters, by their indices: what
    /// each type naming it gives one is flattened.
    pub params: HashMap<DefId, Vec<u32>>,
}

impl Flattened {
    /// `adt`'s keys, where its values are told by them.
    pub(in crate::lower) fn keyed(&self, tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> Option<Keyed> {
        match self.keyed.contains(&adt.did()) {
            true => keyed_shape(tcx, adt),
            false => None,
        }
    }

    /// What each struct of a flattened type parameter is given for it
    /// in `ty`, wherever it names one.
    pub(in crate::lower) fn given_params<'tcx>(&self, ty: Ty<'tcx>) -> Vec<(DefId, Ty<'tcx>)> {
        let mut given = Vec::new();
        for arg in ty.walk() {
            if let Some(ty) = arg.as_type()
                && let ty::Adt(adt, args) = ty.kind()
                && let Some(at) = self.params.get(&adt.did())
            {
                given.extend(at.iter().map(|&i| (adt.did(), args.type_at(i as usize))));
            }
        }
        given
    }
}

/// The crate's flattened states and keyed enums, of its structs' fields
/// and what `bodies` and its items' types give their flattened type
/// parameters; and the libraries' keyed enums they name, as `foreign`'s
/// contracts say (library ABI 15).
pub(in crate::lower) fn flattened<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
) -> Flattened {
    let mut found = Flattened::default();
    let definitions: Vec<_> = tcx.hir_crate_items(()).definitions().collect();
    for &id in &definitions {
        if tcx.def_kind(id) != DefKind::Struct {
            continue;
        }
        for field in (tcx.adt_def(id).non_enum_variant().fields.iter()).filter(|f| is_flatten(tcx, f)) {
            let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
            if let Some((state, _)) = flattened_state(tcx, ty) {
                found.states.insert(state.did());
            }
            match ty.kind() {
                ty::Adt(adt, _) if adt.did().is_local() && keyed_shape(tcx, *adt).is_some() => {
                    found.keyed.insert(adt.did());
                }
                ty::Param(param) => found.params.entry(id.to_def_id()).or_default().push(param.index),
                _ => {}
            }
        }
    }
    // Each type the crate names: what its bodies' expressions are of, and
    // its items' signatures and fields.
    let mut types: Vec<Ty<'tcx>> = (bodies.iter())
        .flat_map(|body| body.thir.exprs.iter().map(|expr| expr.ty))
        .collect();
    for &id in &definitions {
        match tcx.def_kind(id) {
            DefKind::Fn | DefKind::AssocFn => {
                let sig = tcx.fn_sig(id).instantiate_identity().skip_normalization().skip_binder();
                types.extend(sig.inputs_and_output.iter());
            }
            DefKind::TyAlias
            | DefKind::Const { .. }
            | DefKind::Static { .. }
            | DefKind::AssocConst { .. }
            | DefKind::Field => {
                types.push(tcx.type_of(id).instantiate_identity().skip_normalization());
            }
            _ => {}
        }
    }
    let mut keyed = HashSet::new();
    for ty in types {
        // A library's enum its contract says is keyed, and a library's
        // struct's flattened type parameters, which its consumer gives too.
        for ty in ty.walk().filter_map(|arg| arg.as_type()) {
            let ty::Adt(adt, _) = ty.kind() else { continue };
            if adt.did().is_local() {
                continue;
            }
            if adt.is_enum() && foreign.keyed(adt.did()) {
                keyed.insert(adt.did());
            }
            if adt.is_struct() && !found.params.contains_key(&adt.did()) {
                let at: Vec<u32> = (adt.non_enum_variant().fields.iter())
                    .filter(|field| is_flatten(tcx, field))
                    .filter_map(|field| {
                        match tcx
                            .type_of(field.did)
                            .instantiate_identity()
                            .skip_normalization()
                            .kind()
                        {
                            ty::Param(param) => Some(param.index),
                            _ => None,
                        }
                    })
                    .collect();
                found.params.insert(adt.did(), at);
            }
        }
        for (_, given) in found.given_params(ty) {
            if let ty::Adt(adt, _) = given.kind()
                && adt.did().is_local()
                && keyed_shape(tcx, *adt).is_some()
            {
                keyed.insert(adt.did());
            }
        }
    }
    found.keyed.extend(keyed);
    found
}

/// That `subject` is a keyed enum's variant `key` tells, `'getter' in
/// node`: its key is there whatever it holds (#504).
pub(in crate::lower) fn keyed_test(subject: crate::js::Expr, key: &str) -> crate::js::Expr {
    crate::js::Expr::bin(crate::js::Op::In, crate::js::Expr::str(key), subject)
}
