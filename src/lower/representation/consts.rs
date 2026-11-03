//! A value rustc computed as JS (ADR 0031): a constant's, each variant
//! without fields as its writer says, and a keyed enum's variant its keys
//! (#504).

use super::super::bindings::{self, field_key, unit_name};
use super::{Num, char_value, marks_js_object, num_literal, ordering_value, symbol_for, variant_field};
use crate::js;
use crate::js::{Expr, Prop};
use crate::lower::recognition::{StdItem, cell_value_field, is_std_def};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::CtorKind;
use rustc_middle::ty;
use rustc_middle::ty::Ty;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// A constant value as a JS literal, in the shapes of ADRs 0011, 0013, 0020
/// and 0030: numbers, strings, `{ x: 0, y: 0 }`, `[a, b]`, `"High"`,
/// `undefined` for `None`.
/// What a value tree holds inside std's structs of one field each: a
/// `NonZero`'s number, a `Discriminant`'s, a `Duration`'s nanoseconds.
fn innermost(mut value: ty::Value<'_>) -> Option<ty::Value<'_>> {
    while let ty::ValTreeKind::Branch(items) = &**value.valtree
        && let [item] = &items[..]
    {
        value = item.try_to_value()?;
    }
    Some(value)
}

/// The JS value of a type of no size, which has one value: `undefined` of
/// `()` or a unit struct, `[]` of `[T; 0]`, as `const_js` writes a constant
/// of it (ADR 0562). `None` of a type with a size, or one `const_js` can't
/// write.
pub(in crate::lower) fn zero_sized_js<'tcx>(
    tcx: TyCtxt<'tcx>,
    keyed: &std::collections::HashSet<DefId>,
    ty: Ty<'tcx>,
) -> Option<Expr> {
    fn tree<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<ty::ValTree<'tcx>> {
        let part = |t: Ty<'tcx>| Some(ty::Const::new_value(tcx, tree(tcx, t)?, t));
        let parts: Vec<ty::Const<'tcx>> = match ty.kind() {
            ty::Array(item, len) => {
                let len = len.try_to_target_usize(tcx)?;
                (0..len).map(|_| part(*item)).collect::<Option<_>>()?
            }
            ty::Tuple(items) => items.iter().map(part).collect::<Option<_>>()?,
            ty::Adt(adt, args) if adt.is_struct() => (adt.non_enum_variant().fields.iter())
                .map(|field| part(field.ty(tcx, args).skip_normalization()))
                .collect::<Option<_>>()?,
            _ => return None,
        };
        Some(ty::ValTree::from_branches(tcx, parts))
    }
    let layout = tcx
        .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(ty))
        .ok()?;
    if !layout.is_zst() {
        return None;
    }
    const_js(
        tcx,
        keyed,
        ty::Value {
            ty,
            valtree: tree(tcx, ty)?,
        },
    )
}

/// A value rustc computed, in JS (ADR 0031), each variant without fields
/// its value (`bindings::unit_variant`): a module's `const`'s, made when
/// it loads, which reads no other, nor a numbered enum's `const` of a
/// variant another module may not have made yet (ADR 0722, amended). A
/// body's is `FnCx::const_value`'s, each variant as it writes one.
pub(in crate::lower) fn const_js<'tcx>(
    tcx: TyCtxt<'tcx>,
    keyed: &std::collections::HashSet<DefId>,
    value: ty::Value<'tcx>,
) -> Option<Expr> {
    let unit = |adt, variant: &ty::VariantDef| match keyed.contains(&adt) {
        true => Expr::object(Vec::new()),
        false => bindings::unit_variant(tcx, adt, variant),
    };
    const_js_by(tcx, value, &Units { unit: &unit, keyed })
}

/// How a value rustc computed writes an enum's variant: one without fields
/// as `unit` does, and one of `keyed`, an enum told by its keys, an object
/// of each of them, no tag (#504).
pub(super) struct Units<'a> {
    pub(super) unit: &'a dyn Fn(DefId, &ty::VariantDef) -> Expr,
    pub(super) keyed: &'a std::collections::HashSet<DefId>,
}

/// `const_js`, each variant without fields as `unit` writes it.
pub(super) fn const_js_by<'tcx>(tcx: TyCtxt<'tcx>, value: ty::Value<'tcx>, unit: &Units<'_>) -> Option<Expr> {
    let ty = value.ty;
    if ty.is_bool() {
        return value.try_to_bool().map(Expr::bool);
    }
    // A `Duration`'s is its nanoseconds, of std's `secs` and `nanos` (ADR 0188).
    if crate::lower::recognition::is_duration_ty(ty) {
        let ty::ValTreeKind::Branch(items) = &**value.valtree else {
            return None;
        };
        let [secs, nanos] = &items[..] else { return None };
        let secs = secs.try_to_value()?.try_to_leaf()?.to_bits_unchecked();
        let nanos = innermost(nanos.try_to_value()?)?.try_to_leaf()?.to_bits_unchecked();
        return Some(num_literal(secs * 1_000_000_000 + nanos, Num::U128));
    }
    // A clock's is its nanoseconds, of its seconds and nanoseconds however
    // its platform keeps them: `UNIX_EPOCH`'s, `0n` (ADR 0447).
    if crate::lower::recognition::clock_type_of(ty).is_some() {
        let mut leaves = Vec::new();
        let mut pending = vec![value];
        while let Some(part) = pending.pop() {
            match &**part.valtree {
                ty::ValTreeKind::Branch(items) => {
                    pending.extend(items.iter().rev().filter_map(|item| item.try_to_value()));
                }
                ty::ValTreeKind::Leaf(leaf) => leaves.push(leaf.to_bits_unchecked()),
            }
        }
        let [secs, nanos] = leaves[..] else { return None };
        let nanos = i128::from(secs as u64 as i64) * 1_000_000_000 + nanos as i128;
        return Some(Expr::bigint(nanos));
    }
    if let Some(num) = Num::of(ty) {
        // A `NonZero`'s is its number, inside std's `NonZeroU8Inner` (ADR 0177).
        return Some(num_literal(innermost(value)?.try_to_leaf()?.to_bits_unchecked(), num));
    }
    if let Some(c) = char_value(value) {
        return Some(Expr::str(c.to_string()));
    }
    // The registry's symbol, `Symbol.for("react.memo")`, of its key (ADR 0634).
    if let ty::Adt(adt, _) = ty.kind()
        && crate::lower::recognition::is_symbol_for_def(tcx, adt.did())
    {
        let ty::ValTreeKind::Branch(items) = &**value.valtree else {
            return None;
        };
        let [key] = &items[..] else { return None };
        return Some(symbol_for(const_js_by(tcx, key.try_to_value()?, unit)?));
    }
    // A `Discriminant`'s is its number, inside std's struct (ADR 0393).
    if crate::lower::recognition::is_discriminant_type(tcx, ty) {
        let leaf = innermost(value)?.try_to_leaf()?;
        return Some(Expr::int(leaf.to_int(leaf.size())));
    }
    // An enum's value tree starts with its variant's index, then its fields.
    let children = || -> Option<Vec<ty::Value<'tcx>>> {
        match &**value.valtree {
            ty::ValTreeKind::Branch(items) => items.iter().map(|c| c.try_to_value()).collect(),
            ty::ValTreeKind::Leaf(_) => None,
        }
    };
    let all = |values: &[ty::Value<'tcx>]| {
        values
            .iter()
            .map(|&v| const_js_by(tcx, v, unit))
            .collect::<Option<Vec<_>>>()
    };
    // A field's: of a `Cell`, what it holds, as a `Cell` in an object's
    // field is the property set in place (ADR 0288).
    let fields_of = |values: &[ty::Value<'tcx>]| {
        (values.iter())
            .map(|&v| {
                let js = const_js_by(tcx, v, unit)?;
                Some(match v.ty.kind() {
                    ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Cell) => Expr::held(js),
                    _ => js,
                })
            })
            .collect::<Option<Vec<_>>>()
    };
    match ty.kind() {
        ty::Ref(_, inner, _) if inner.is_str() => {
            Some(Expr::str(std::str::from_utf8(value.try_to_raw_bytes(tcx)?).ok()?))
        }
        ty::Ref(_, inner, _) => const_js_by(
            tcx,
            ty::Value {
                ty: *inner,
                valtree: value.valtree,
            },
            unit,
        ),
        ty::Tuple(items) if items.is_empty() => Some(Expr::undefined()),
        ty::Tuple(_) | ty::Array(..) | ty::Slice(_) => Some(Expr::array(all(&children()?)?)),
        ty::Adt(adt, _) if adt.is_enum() => {
            let items = children()?;
            let (index, fields) = items.split_first()?;
            let variant = adt.variant(index.try_to_leaf()?.to_u32().into());
            if tcx.is_lang_item(adt.did(), LangItem::Option) {
                return match fields.first() {
                    Some(&inner) => const_js_by(tcx, inner, unit).map(crate::lower::std_types::option::some_literal),
                    None => Some(Expr::undefined()),
                };
            }
            if let Some(n) = ordering_value(tcx, adt.did(), variant.name) {
                return Some(Expr::int(n));
            }
            // An untagged enum's variant is its payload (ADR 0214).
            if crate::lower::bindings::is_untagged(tcx, adt.did()) {
                return fields.first().and_then(|&payload| const_js_by(tcx, payload, unit));
            }
            if fields.is_empty() {
                return Some((unit.unit)(adt.did(), variant));
            }
            let values = fields_of(fields)?;
            let props = values
                .into_iter()
                .enumerate()
                .map(|(i, v)| Prop::Field(variant_field(tcx, variant, i), v));
            if unit.keyed.contains(&adt.did()) {
                let mut made = Expr::object(props.collect());
                if let js::ExprKind::Object(props, layout) = &mut made.kind {
                    layout.kept = (props.iter())
                        .filter_map(|p| match p {
                            Prop::Field(key, _) => Some(key.clone()),
                            _ => None,
                        })
                        .collect();
                }
                return Some(made);
            }
            Some(Expr::object(
                std::iter::once(Prop::Field(
                    crate::lower::bindings::tag_key(tcx, adt.did()),
                    crate::lower::bindings::variant_tag(tcx, variant),
                ))
                .chain(props)
                .collect(),
            ))
        }
        // A `#[repr(transparent)]` struct's is its field's (ADR 0584).
        ty::Adt(adt, _) if let Some(at) = crate::lower::recognition::transparent_field(tcx, *adt) => {
            const_js_by(tcx, *children()?.get(at)?, unit)
        }
        // A std struct's fields are its own, not the JS value rust-js makes
        // of it: `iter::empty()` is no `[undefined]`. Only a `PhantomData`, and
        // `Reverse(x)`, `[x]`, are their fields.
        // A `RefCell` of an object is the object, where generic code has
        // none of any value (ADR 0863), which only a body lowering it knows:
        // its initializer's, as one that can't be said here is.
        ty::Adt(adt, args)
            if is_std_def(tcx, adt.did(), StdItem::RefCell)
                && crate::lower::drops::holds_itself(tcx, args.type_at(0)) =>
        {
            None
        }
        // A `Cell` or a `RefCell` is `{ value }`, what its `UnsafeCell` holds,
        // as one made at run time is.
        ty::Adt(adt, _)
            if [StdItem::Cell, StdItem::RefCell]
                .into_iter()
                .any(|item| is_std_def(tcx, adt.did(), item)) =>
        {
            let at = cell_value_field(*adt)?;
            let unsafe_cell = *children()?.get(at)?;
            let inner = match &*unsafe_cell.valtree {
                ty::ValTreeKind::Branch(items) => items.first()?.try_to_value()?,
                ty::ValTreeKind::Leaf(_) => return None,
            };
            Some(Expr::object(vec![Prop::Field(
                js::HELD.into(),
                const_js_by(tcx, inner, unit)?,
            )]))
        }
        // An atomic is `{ value }` too (ADR 0096). What it holds is stored
        // as an integer of its size, in a struct that aligns it.
        ty::Adt(adt, args) if is_std_def(tcx, adt.did(), StdItem::Atomic) => {
            let mut stored = value.valtree;
            while let ty::ValTreeKind::Branch(items) = &**stored {
                let [only] = &items[..] else {
                    return None;
                };
                stored = only.try_to_value()?.valtree;
            }
            let inner = const_js_by(
                tcx,
                ty::Value {
                    ty: args.type_at(0),
                    valtree: stored,
                },
                unit,
            )?;
            Some(Expr::object(vec![Prop::Field(js::HELD.into(), inner)]))
        }
        // A range, `{ start, end }`, as one made at run time is (ADR 0129):
        // an inclusive one's `exhausted`, false of one not yet iterated.
        ty::Adt(adt, _)
            if tcx.is_lang_item(adt.did(), LangItem::Range)
                || tcx.is_lang_item(adt.did(), LangItem::RangeInclusiveStruct) =>
        {
            let items = children()?;
            if let Some(exhausted) = items.get(2)
                && exhausted.try_to_bool() != Some(false)
            {
                return None;
            }
            let [start, end] = [items.first()?, items.get(1)?].map(|&v| const_js_by(tcx, v, unit));
            Some(Expr::object(vec![
                Prop::Field("start".into(), start?),
                Prop::Field("end".into(), end?),
            ]))
        }
        // `a..`, `{ start }`, and `..b` or `..=b`, `{ end }`, as one made at
        // run time is.
        ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::RangeFrom) => {
            let start = const_js_by(tcx, *children()?.first()?, unit)?;
            Some(Expr::object(vec![Prop::Field("start".into(), start)]))
        }
        ty::Adt(adt, _)
            if tcx.is_lang_item(adt.did(), LangItem::RangeTo)
                || tcx.is_lang_item(adt.did(), LangItem::RangeToInclusive) =>
        {
            let end = const_js_by(tcx, *children()?.first()?, unit)?;
            Some(Expr::object(vec![Prop::Field("end".into(), end)]))
        }
        ty::Adt(adt, _) if adt.is_struct() && !crate::lower::recognition::struct_is_its_fields(tcx, adt.did()) => None,
        // A JS value, a binding's, is nothing of rustc's value, its markers,
        // a static's or a constant's: its initializer says what it is,
        // react's `ElementType`'s (ADR 0234), `HostType::new("Text")`'s
        // `"Text"` (ADR 0220, amended).
        ty::Adt(adt, args) if marks_js_object(tcx, *adt, args) => None,
        ty::Adt(adt, args) if adt.is_struct() => {
            let variant = adt.non_enum_variant();
            // A `RefCell` field may be the value it holds, by its declaration
            // and what the crate does with it, which only a body lowering it
            // knows: its initializer's, as a `RefCell` of an object is (ADR
            // 0863, amended).
            let ref_cell = |field: &ty::FieldDef| {
                let declared = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                matches!(declared.kind(), ty::Adt(cell, _) if is_std_def(tcx, cell.did(), StdItem::RefCell))
            };
            // So may a field a symbol names, read where its initializer runs,
            // as that symbol is made (ADR 1051).
            if variant.ctor_kind().is_none()
                && variant
                    .fields
                    .iter()
                    .any(|f| ref_cell(f) || bindings::named_by_symbol(tcx, f.did))
            {
                return None;
            }
            let values = match variant.ctor_kind() {
                None => fields_of(&children()?)?,
                _ => all(&children()?)?,
            };
            match variant.ctor_kind() {
                Some(CtorKind::Const) => Some(unit_name(tcx, adt.did()).map_or_else(Expr::undefined, Expr::str)),
                Some(CtorKind::Fn) => Some(Expr::array(values)),
                // A `#[rust_js::nullable]` field's `None` is `null`, its
                // `Cell`'s too, a generic one's where it's of an `Option`,
                // as where its struct is made at run time; an optional
                // one's left out (ADRs 0275, 0586, 0810, amended).
                None => Some(Expr::object(
                    (variant.fields.iter().zip(values))
                        .map(|(f, v)| {
                            let nullable = bindings::null_where_made(tcx, f)
                                && bindings::nullable_option(tcx, f.ty(tcx, args).skip_normalization()).is_some();
                            let v = match nullable {
                                true => js::Storage::NullForNone.written(v, false),
                                false => v,
                            };
                            Prop::Field(field_key(tcx, f).text().to_owned(), v)
                        })
                        .collect(),
                )),
            }
        }
        _ => None,
    }
}
