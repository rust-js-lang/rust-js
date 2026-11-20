//! What a `#[rust_js::flatten]` field holds as its parent's own keys,
//! beside a struct's fields (ADR 0204): a `RefCell` of a discriminated
//! union, a class's state (rd380, ADR 1044), an enum each of whose
//! variants a key of its own tells, a node's kind (#504, ADR 1071), and an
//! untagged enum each of whose variants holds a record (#1818). The
//! crate's of each, `flattened`, is the one answer representation,
//! validation and declarations read.

use super::{declared_tag, field_key, is_flatten, is_otherwise, is_test_field, is_untagged};
use crate::collections::{IndexMap, IndexSet};
use crate::lower::Body;
use rustc_hir::def::{CtorKind, DefKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// Whether `field` is a class's flattened state, `#[rust_js::flatten]` of a
/// `RefCell` of a discriminated union (`flattened_state`, ADR 1044): its
/// object's keys, which tell it from another by the object they're on, so
/// its `swap` counts no borrow (#687).
pub(in crate::lower) fn is_flattened_state_field(tcx: TyCtxt<'_>, field: &ty::FieldDef) -> bool {
    is_flatten(tcx, field)
        && flattened_state(tcx, tcx.type_of(field.did).instantiate_identity().skip_normalization()).is_some()
}

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
fn keyed_shape(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> Option<Keyed> {
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

/// The keys of `adt`'s records, where it's a `#[rust_js::untagged]` enum
/// each of whose variants holds one, a struct of named fields of a crate's,
/// or a reference to one (#1818): its value is that record's object, as
/// any untagged enum's is its payload, so flattened, the record's keys are
/// the parent's, `{ key, ...composition }`, and the untagged test of the
/// parent, `"nativeEvent" in event`'s kind, is the record's where the
/// parent has none of those keys of its own (`validation`). Each key once,
/// in the order they're first declared. `None` of any other enum: one of
/// a variant without fields, whose value is no object; of a JS object,
/// which an `instanceof` or a binding's test tells, as the parent isn't;
/// or of a record with a flattened field, or a key a symbol names.
pub(in crate::lower) fn untagged_records(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> Option<Vec<String>> {
    if !adt.is_enum() || !is_untagged(tcx, adt.did()) || crate::lower::recognition::is_std_crate(tcx, adt.did()) {
        return None;
    }
    let mut keys: Vec<String> = Vec::new();
    for variant in adt.variants() {
        let ([field], Some(CtorKind::Fn)) = (&variant.fields.raw[..], variant.ctor_kind()) else {
            return None;
        };
        let held = tcx.type_of(field.did).instantiate_identity().skip_normalization();
        let ty::Adt(record, args) = held.peel_refs().kind() else {
            return None;
        };
        if !record.is_struct()
            || record.non_enum_variant().ctor.is_some()
            || crate::lower::recognition::is_std_crate(tcx, record.did())
            || crate::lower::representation::marks_js_object(tcx, *record, args)
        {
            return None;
        }
        for field in &record.non_enum_variant().fields {
            if is_flatten(tcx, field) {
                return None;
            }
            let key = field_key(tcx, field).name()?.to_owned();
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    Some(keys)
}

/// What the crate's flattened fields hold that's their parents' own keys,
/// beside a struct's fields.
#[derive(Default)]
pub(in crate::lower) struct Flattened {
    /// The enums its classes hold as a flattened state (`flattened_state`),
    /// each its object's own keys (rd380, ADR 1044): a `&mut` to one is a
    /// handle, whose setter writes each key there, wherever it's given,
    /// never the object, which holds the class's other keys too (#478).
    pub states: IndexSet<DefId>,
    /// Its enums told by their keys (`keyed_shape`, #504): each one a
    /// field flattens, `kind: NodeKind`, or a struct's flattened type
    /// parameter is given, `ReactiveNode<NodeKind>` of `kind: K`, where
    /// the crate names it so. Every value of one is its keys, a variant's
    /// told by its own, `'getter' in node`, where it's flattened or not.
    pub keyed: IndexSet<DefId>,
    /// Each struct's flattened type parameters, by their indices: what
    /// each type naming it gives one is flattened.
    pub params: IndexMap<DefId, Vec<u32>>,
    /// Its untagged enums of records a field flattens (`untagged_records`,
    /// #1818): each value its record's object, as any untagged enum's is,
    /// whose keys are its parent's, the parent itself where it's lent.
    pub untagged: IndexSet<DefId>,
    /// The structs a flattened field holds that are read whole somewhere,
    /// lent, `&range.range`, or by value, `range.range`, where the value is
    /// its parent's object, which has the parent's keys too (#1820): each
    /// value of one is its fields alone where it's copied, compared or
    /// hashed, `{ min: r.min, max: r.max, step: r.step }`, the copy owner's
    /// (`copies::projection`). Of a library, each a consumer can reach,
    /// which a consumer may lend.
    pub projected: IndexSet<DefId>,
}

impl Flattened {
    /// Whether a value of `ty`, a flattened field's, is its parent's keys
    /// as an enum's: a keyed one's, or an untagged one's of records,
    /// matched where it's flattened as the parent object (#504, #1818).
    pub(in crate::lower) fn enum_of_keys(&self, ty: Ty<'_>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.keyed.contains(&adt.did()) || self.untagged.contains(&adt.did()))
    }

    /// Whether a value of `ty` may be its parent's object, a flattened
    /// struct read whole somewhere (`projected`, #1820).
    pub(in crate::lower) fn is_projected(&self, ty: Ty<'_>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.projected.contains(&adt.did()))
    }

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
    library: bool,
) -> Flattened {
    let mut found = Flattened::default();
    // A flattened struct read whole, lent or by value, `&r.range` or
    // `r.range`, its parent's object (#1820); of a library, each one in a
    // struct a consumer can reach, which it may lend.
    for body in bodies {
        let thir = &body.thir;
        let through = read_through(thir);
        for (e, expr) in thir.exprs.iter_enumerated() {
            if let rustc_middle::thir::ExprKind::Field { lhs, name, .. } = expr.kind
                && super::is_flatten_field(tcx, thir[lhs].ty.peel_refs(), name.as_usize())
                && !through.contains(&e)
                && let Some(adt) = plain_struct(tcx, expr.ty)
            {
                found.projected.insert(adt);
            }
        }
    }
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
            if library
                && crate::lower::library::reachable(tcx, id.to_def_id())
                && let Some(adt) = plain_struct(tcx, ty)
            {
                found.projected.insert(adt);
            }
            match ty.kind() {
                ty::Adt(adt, _) if adt.did().is_local() && keyed_shape(tcx, *adt).is_some() => {
                    found.keyed.insert(adt.did());
                }
                ty::Adt(adt, _) if untagged_records(tcx, *adt).is_some() => {
                    found.untagged.insert(adt.did());
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
            DefKind::TyAlias | DefKind::Const | DefKind::Static { .. } | DefKind::AssocConst | DefKind::Field => {
                types.push(tcx.type_of(id).instantiate_identity().skip_normalization());
            }
            _ => {}
        }
    }
    let mut keyed = IndexSet::default();
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
                // And a library's class's flattened state, its object's keys
                // as the crate's own is, which a `replace` of copies (#1030).
                for field in (adt.non_enum_variant().fields.iter()).filter(|field| is_flatten(tcx, field)) {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    if let Some((state, _)) = flattened_state(tcx, ty) {
                        found.states.insert(state.did());
                    }
                }
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

/// What a flattened field's value holds that its parent's object copies,
/// `{ ...item, severity }` (ADR 0204, amended), while something else may
/// hold it too: a field's own `&Item`, an untagged enum's variant's,
/// `Direct(&'a CompositionFields)` (#1818), and a JS object, whose handle
/// any copy of shares it. Each is shared with its source in Rust, and a copy
/// in JS, so each is asked `copy_seen`, and a reference's referent
/// `copy_seen_in_crate`, which must find nothing that tells them apart
/// (#1819, #1822).
pub(in crate::lower) fn copied_references<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    match ty.kind() {
        ty::Ref(..) => vec![ty],
        ty::Adt(adt, args) if crate::lower::representation::marks_js_object(tcx, *adt, args) => vec![ty],
        ty::Adt(adt, args) if untagged_records(tcx, *adt).is_some() => (adt.variants().iter())
            .flat_map(|variant| variant.fields.iter())
            .map(|field| field.ty(tcx, args).skip_normalization())
            .filter(|held| held.is_ref())
            .collect(),
        _ => Vec::new(),
    }
}

/// Why a flattened JS object is refused (#1822): JS may write it through
/// any handle on it, `dict::set` or a function it's given, so its parent's
/// object, a copy of its keys, would miss a key set after it's made. The
/// copy upstream's `{ ...files }` makes, `js::object::spread`, says so.
const JS_OBJECT_COPIED: &str = "a JS object, which anything holding it may write, `js::dict::set` or a JS function it's given, so its parent's object, a copy of its keys made once, would miss a key set later: copy it where it's made instead, `js::object::spread(..)`, as JS's `{ ...files }` does";

/// Why a type parameter given a JS object is refused flattened, as a
/// flattened field of one is (#1822).
pub(in crate::lower) fn js_object_copied() -> &'static str {
    JS_OBJECT_COPIED
}

/// Of `reference`, one `copied_references` gives, what of its type alone
/// would tell its parent's copy from its referent (#1819): a JS object,
/// which anything holding it may write (#1822); a `&mut`, whose writes
/// through the parent its referent wouldn't see; or a referent with a cell
/// among its own keys, `Cell` or `RefCell`, whose writes through either
/// the other wouldn't see, as rustc's `Freeze` says. `typing_env` is where
/// the field is declared.
pub(in crate::lower) fn copy_seen<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    reference: Ty<'tcx>,
) -> Option<&'static str> {
    let (referent, mutability) = match *reference.kind() {
        ty::Ref(_, referent, mutability) => (referent, mutability),
        _ => (reference, ty::Mutability::Not),
    };
    if matches!(referent.kind(), ty::Adt(adt, args) if crate::lower::representation::marks_js_object(tcx, *adt, args)) {
        return Some(JS_OBJECT_COPIED);
    }
    if mutability.is_mut() {
        return Some(
            "a `&mut`, whose writes through its parent's object, a copy of what it refers to, wouldn't be its referent's",
        );
    }
    if !referent.is_freeze(tcx, typing_env) {
        return Some(
            "of a type with a `Cell` or `RefCell` of its own, which its parent's object would hold a copy of, so a write through either wouldn't be seen through the other",
        );
    }
    None
}

/// Of `referent`, what a flattened reference refers to, whether the crate
/// would tell its parent's copy from it (#1819): it changes one in place,
/// or compares its address, a raw pointer to it, as the mutation owner's
/// `changed` says, whose copies are another's.
pub(in crate::lower) fn copy_seen_in_crate<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    changed: &IndexSet<Ty<'tcx>>,
    referent: Ty<'tcx>,
) -> bool {
    (changed.iter()).any(|&changed| crate::lower::copies::instance_of(recognition, referent, changed))
}

/// The flattened fields read through in `thir`, each `lhs` of a field
/// read past its derefs, `props.html.title`'s `props.html`,
/// `(*report.item).line`'s `report.item`: what reads its parent's keys,
/// as any field read does, never the flattened value whole (ADR 0205).
pub(in crate::lower) fn read_through(thir: &rustc_middle::thir::Thir<'_>) -> IndexSet<rustc_middle::thir::ExprId> {
    use rustc_middle::thir::ExprKind;
    (thir.exprs.iter())
        .filter_map(|expr| match expr.kind {
            ExprKind::Field { lhs, .. } => {
                let mut lhs = crate::lower::strip(thir, lhs);
                while let ExprKind::Deref { arg } = thir[lhs].kind {
                    lhs = crate::lower::strip(thir, arg);
                }
                Some(lhs)
            }
            _ => None,
        })
        .collect()
}

/// Of `ty`, a flattened field's, the struct of named fields of a crate's
/// it is by value, whose object its fields are, and its own flattened
/// fields' in turn: not a reference, which #1819's rules hold apart, a
/// cell's state, a keyed or an untagged enum, or a JS object (#1820).
pub(in crate::lower) fn plain_struct<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<DefId> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    (adt.is_struct()
        && adt.non_enum_variant().ctor.is_none()
        && !crate::lower::recognition::is_std_crate(tcx, adt.did())
        && !crate::lower::representation::marks_js_object(tcx, *adt, args)
        && (adt.non_enum_variant().fields.iter())
            .filter(|field| is_flatten(tcx, field))
            .all(|field| plain_struct(tcx, field.ty(tcx, args).skip_normalization()).is_some()))
    .then_some(adt.did())
}

/// That `subject` is a keyed enum's variant `key` tells, `'getter' in
/// node`: its key is there whatever it holds (#504).
pub(in crate::lower) fn keyed_test(subject: crate::js::Expr, key: &str) -> crate::js::Expr {
    crate::js::Expr::bin(crate::js::Op::In, crate::js::Expr::str(key), subject)
}
