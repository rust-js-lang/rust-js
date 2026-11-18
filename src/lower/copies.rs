//! When a value is copied. Rust copies a `Copy` value where it's read, and
//! JS shares an object: a copy is made only where one of them may change in
//! place, which is found once for each type (ADRs 0020, 0052, 0100).

mod in_place;
mod kept;
mod stored;
pub(in crate::lower) use in_place::InPlaceWriter;

use super::recognition::StdItem;
use super::{FnCx, Shape};
use crate::collections::{IndexMap, IndexSet};
use crate::js;
use crate::js::{Expr, Op, Prop};
use rustc_abi::VariantIdx;
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt};

/// A tagged enum's variants copied, by `variant_groups`: each group of
/// them of the same fields, and the others, each value itself.
#[derive(Default)]
pub(in crate::lower) struct VariantGroups<'v, 'tcx> {
    pub(in crate::lower) copied: Vec<VariantGroup<'v, 'tcx>>,
    pub(in crate::lower) others: Vec<VariantIdx>,
}

/// Variants of the same fields: their indices, the first of them, whose
/// fields' definitions a clone reads, and their fields.
pub(in crate::lower) struct VariantGroup<'v, 'tcx> {
    pub(in crate::lower) indices: Vec<VariantIdx>,
    pub(in crate::lower) first: &'v ty::VariantDef,
    pub(in crate::lower) fields: Vec<(String, Ty<'tcx>)>,
}

/// What `variant_groups` asks of a variant and its fields.
type OfVariant<'a, 'tcx> = &'a dyn Fn(&ty::VariantDef, &[(String, Ty<'tcx>)]) -> bool;

/// Whether `array` is an array made here, which nothing else holds: `[1,
/// 2]`, one `made_array` says is made, or a `.slice(..)` (`copy_if_needed`,
/// `mapped_as_itself`).
fn fresh_array(array: &Expr) -> bool {
    super::iterators::made_array(array)
        || matches!(&array.kind, js::ExprKind::Call(callee, _)
            if matches!(&callee.kind, js::ExprKind::Member(_, name) if name == "slice"))
}

/// What `copy`, a copy `copy_parts` makes, copies: `x` of `{ ...x }`, of
/// `x.slice()`, or of `x == null ? x : <either>`; else nothing. What a
/// temporary holding it is named by, as it would be holding `x` (ADR 0569,
/// #1923): `at` of `marks.at`'s copy, not `value`.
pub(in crate::lower) fn copied_source(copy: &Expr) -> Option<&Expr> {
    match &copy.kind {
        js::ExprKind::Object(props, _) => match &props[..] {
            [Prop::Spread(source)] => Some(source),
            _ => None,
        },
        js::ExprKind::Call(callee, args) if args.is_empty() => match &callee.kind {
            js::ExprKind::Member(source, name) if name == "slice" => Some(source),
            _ => None,
        },
        js::ExprKind::Cond(test, source, copied) => match &test.kind {
            js::ExprKind::Binary(Op::LooseEq, tested, null)
                if matches!(null.kind, js::ExprKind::Null)
                    && tested.kind == source.kind
                    && copied_source(copied).is_some_and(|inner| inner.kind == source.kind) =>
            {
                Some(source)
            }
            _ => None,
        },
        _ => None,
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The dictionary of `ty: Copy` this function was given: of a `T:
    /// Copy`, `TCopy`; of a trait that extends it, `trait Copied: Copy`,
    /// or one that extends that, its supertrait's, `TCopied.Copy()`; a
    /// where-clause's alike (#1925).
    fn copy_evidence(&self, ty: Ty<'tcx>) -> Option<Expr> {
        let copy = self.tcx.require_lang_item(LangItem::Copy, rustc_span::DUMMY_SP);
        self.evidence_for(ty::TraitRef::new(self.tcx, copy, [ty]))
    }

    /// A fresh `ty` value equal to the one at `place`: `{ ...p }`, `[t[0], t[1]]`.
    /// A field that also contains mutated types is copied in turn.
    pub(super) fn copy(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        self.copy_parts(place, ty, false)
    }

    /// `copy`, each part copied where `copied_part` says: of what a library
    /// stores (`stored`), each that may be an object too (#1907).
    fn copy_parts(&self, place: Expr, ty: Ty<'tcx>, stored: bool) -> Expr {
        // A copied default's `Self` is the impl's type, a number say, which
        // needs no copy (ADR 0049).
        if self.is_unknown(ty)
            && let Some(dictionary) = self.copy_evidence(ty)
        {
            // A caller whose value is its own copy gives none, as JS gives
            // none, which its declarations ask for: the value (ADR 0289,
            // amended). Read once, `((value) => ..)(source)`.
            let copied = |place: Expr| {
                let copy = Expr::call(Expr::member(dictionary.clone(), "copy"), vec![place.clone()]);
                Expr::cond(dictionary.clone(), copy, place)
            };
            if place.reads_same() {
                return copied(place);
            }
            let param = self.unspelled("value");
            return Expr::call(
                Expr::arrow(
                    vec![param.clone().into()],
                    vec![js::StmtKind::Return(Some(copied(Expr::var(&param)))).at(js::Span::NONE)],
                ),
                vec![place],
            );
        }
        // A copy reads its source once per part: a value that isn't a place,
        // like `$unwrap(v[0])`, is taken once, `((value) => ..)(source)`.
        let many = match self.shape(ty) {
            Shape::Object(fields) => fields.iter().any(|&(_, t)| self.copied_part(t, stored)),
            Shape::Array(tys) => tys.len() > 1,
            Shape::Other => false,
        };
        if many && !place.reads_same() {
            let param = self.unspelled("value");
            let body = self.copy_parts(Expr::var(&param), ty, stored);
            return Expr::call(
                Expr::arrow(
                    vec![param.clone().into()],
                    vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
                ),
                vec![place],
            );
        }
        match self.shape(ty) {
            Shape::Object(fields) => {
                let mut props = vec![Prop::Spread(place.clone())];
                // A `Copy` struct's, which a symbol names no field of
                // (`validate`, ADR 1051); a flattened one's own keys (#1847).
                let fields = match self.own_object_fields(ty) {
                    Some(own) => own.into_iter().map(|(name, t, _)| (name, t)).collect(),
                    None => Self::named_fields(fields),
                };
                for (name, t) in fields {
                    if self.copied_part(t, stored) {
                        let field = self.copy_parts(Expr::member(place.clone(), name.clone()), t, stored);
                        props.push(Prop::Field(name, field));
                    }
                }
                Expr::object(props)
            }
            Shape::Array(tys) => Expr::array(
                tys.into_iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let item = Expr::index(place.clone(), Expr::int(i as i128));
                        if self.copied_part(t, stored) {
                            self.copy_parts(item, t, stored)
                        } else {
                            item
                        }
                    })
                    .collect(),
            ),
            Shape::Other => {
                // Read more than once: a value that isn't a place is taken once.
                let once = |place: Expr, copy: &dyn Fn(Expr) -> Expr| {
                    if place.reads_same() {
                        copy(place)
                    } else {
                        let param = self.unspelled("value");
                        let body = copy(Expr::var(&param));
                        Expr::call(
                            Expr::arrow(
                                vec![param.clone().into()],
                                vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
                            ),
                            vec![place],
                        )
                    }
                };
                // A closure that captures something, where the crate reads a
                // `dyn Fn`'s address: a function of its own, `f.bind()`, as
                // the copy is a place of its own to Rust, which a pointer to
                // it tells apart (`copied_apart`, #1090, the user's decision).
                if self.copied_apart(ty) {
                    return Expr::call(Expr::member(place, "bind"), Vec::new());
                }
                // A closure that changes what it captured, which the crate
                // clones apart: its copy is its clone, of its own captures,
                // as Rust's copy is (ADR 0522, #1211).
                if self.own_captures_copied(ty) == Some(true) {
                    return once(place, &|f| Expr::call_helper("$clone", vec![f]));
                }
                // A write's result, `Ok(())`'s `undefined` or an `Err`'s
                // `$FmtError` (ADR 0187), which nothing changes in place:
                // itself, never a tagged object's copy (#1757).
                if self.is_write_result(ty) {
                    return place;
                }
                if let Some(inner) = self.option_of(ty) {
                    if !self.copied_part(inner, stored) {
                        return place;
                    }
                    // A box, `Some(None)`, holds nothing to copy (ADR 0051):
                    // only the value inside every `Some` does.
                    let mut item = inner;
                    while let Some(next) = self.option_of(item) {
                        item = next;
                    }
                    let boxed = item != inner;
                    // Of a value that's its own copy, a type parameter's no
                    // caller gives a copy function, the `Option` itself, not
                    // `value == null ? value : value` (ADR 0289, amended).
                    let probe = Expr::var(js::Name::declared("value".to_string()));
                    if !boxed && self.copy_parts(probe.clone(), item, stored).kind == probe.kind {
                        return place;
                    }
                    return once(place, &|o| {
                        let mut none = Expr::bin(Op::LooseEq, o.clone(), Expr::null());
                        if boxed {
                            none = Expr::bin(Op::Or, none, super::std_types::option::is_some_box(o.clone()));
                        }
                        Expr::cond(none, o.clone(), self.copy_parts(o, item, stored))
                    });
                }
                // An array that's changed in place: `a.slice()`, or a copy of
                // each item that is too.
                if let ty::Array(item, _) = ty.kind() {
                    if !self.copied_part(*item, stored) {
                        return Expr::builtin(place, "slice", Vec::new());
                    }
                    let name = self.fresh(crate::lower::Base::Named("item"));
                    let body = self.copy_parts(Expr::var(&name), *item, stored);
                    let copy = Expr::arrow(
                        vec![name.clone().into()],
                        vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
                    );
                    return Expr::builtin(place, "map", vec![copy]);
                }
                // A union, a union of its own (ADR 0406).
                if self.is_union(ty) {
                    return once(place, &|u| self.union_copy(u, ty));
                }
                let ty::Adt(adt, args) = ty.kind() else { return place };
                if !self.is_copy_enum(ty) {
                    return place;
                }
                // An untagged enum's variant is its payload (ADR 0214): one that
                // changes gets the payload's copy, told by its kind; each, of
                // one whose array is changed in place itself, through a
                // `&mut` (rd490).
                if self.untagged(ty).is_some() {
                    let itself = self.untagged_changed_itself(ty);
                    // Of arrays of what needs no copy of its own, the array's
                    // `slice()`, whatever its variant.
                    let changed = (adt.variants().iter())
                        .filter_map(|v| self.variant_fields(v, args).first().map(|&(_, t)| t))
                        .any(|t| self.copied_part(t, stored));
                    if (itself || changed) && self.sliced_untagged(ty, &|t| self.copied_part(t, stored)) {
                        return Expr::builtin(place, "slice", Vec::new());
                    }
                    return once(place, &|e| {
                        let mut value = e.clone();
                        for variant in adt.variants().iter().rev() {
                            let Some(&(_, t)) = self.variant_fields(variant, args).first() else {
                                continue;
                            };
                            if !(itself || self.copied_part(t, stored)) {
                                continue;
                            }
                            let test = self.untagged_variant_test(ty, &[variant], &[], &e, false);
                            value = Expr::cond(test, self.copy_parts(e.clone(), t, stored), value);
                        }
                        value
                    });
                }
                // `{ TAG: "Line", _0: .. }`: a variant with a part that changes
                // gets a copy, and every other value is itself. Changed in
                // place itself, through a `&mut`: every variant with fields.
                let itself = self.mutated_itself(ty);
                once(place, &|e| {
                    let copied = |_: &ty::VariantDef, fields: &[(String, Ty<'tcx>)]| {
                        !fields.is_empty() && (itself || fields.iter().any(|&(_, t)| self.copied_part(t, stored)))
                    };
                    let groups = self.variant_groups(adt, args, &copied, &|_, _| false);
                    let copies = (groups.copied.into_iter())
                        .map(|VariantGroup { indices, fields, .. }| {
                            let mut props = vec![Prop::Spread(e.clone())];
                            for (name, t) in fields {
                                if self.copied_part(t, stored) {
                                    props.push(Prop::Field(
                                        name.clone(),
                                        self.copy_parts(Expr::member(e.clone(), name), t, stored),
                                    ));
                                }
                            }
                            (indices, Expr::object(props))
                        })
                        .collect();
                    self.variants_tested((e.clone(), e.clone()), (ty, *adt), copies, &groups.others)
                })
            }
        }
    }

    /// A tagged enum's variants that are copied, of `copied`, grouped by
    /// their fields, each group's indices and its first variant, and the
    /// others, each value itself. Variants of the same fields are copied
    /// alike, by one copy and one test of them all, so a nested enum's
    /// copy is made once, not once a variant, whose JS and work grew as its
    /// variants to the power of its depth (#1821); not one `alone` says is
    /// its own, a variant whose fields' own definitions matter.
    pub(in crate::lower) fn variant_groups<'v>(
        &self,
        adt: &'v ty::AdtDef<'tcx>,
        args: ty::GenericArgsRef<'tcx>,
        copied: OfVariant<'_, 'tcx>,
        alone: OfVariant<'_, 'tcx>,
    ) -> VariantGroups<'v, 'tcx> {
        let mut groups: VariantGroups<'v, 'tcx> = VariantGroups::default();
        for (index, variant) in adt.variants().iter_enumerated() {
            let fields = self.variant_fields(variant, args);
            if !copied(variant, &fields) {
                groups.others.push(index);
                continue;
            }
            let own = alone(variant, &fields);
            match (groups.copied.iter_mut()).find(|g| g.fields == fields && !own && !alone(g.first, &g.fields)) {
                Some(group) => group.indices.push(index),
                None => groups.copied.push(VariantGroup {
                    indices: vec![index],
                    first: variant,
                    fields,
                }),
            }
        }
        groups
    }

    /// Of `e`, a tagged enum's value, `{ TAG: "Line", _0: .. }`, each group
    /// of its variants `variant_groups` found, its value, `copies`', and of
    /// every other variant `otherwise`, `e` itself of a copy, `0` of a
    /// comparison: one test of each group, `e.TAG === "First" ||
    /// e.TAG === "Second" ? { ...e, _0: .. } : e`; where one group's are
    /// all the variants, its test is of the others where they're fewer,
    /// `e === "Empty" ? e : { ...e, _0: .. }`, or, of none, its value.
    pub(in crate::lower) fn variants_tested(
        &self,
        (e, otherwise): (Expr, Expr),
        (ty, adt): (Ty<'tcx>, ty::AdtDef<'tcx>),
        copies: Vec<(Vec<VariantIdx>, Expr)>,
        others: &[VariantIdx],
    ) -> Expr {
        let tested = |indices: &[VariantIdx]| {
            (indices.iter())
                .map(|&index| self.variant_test(e.clone(), ty, adt, index, false))
                .reduce(|a, b| Expr::bin(Op::Or, a, b))
        };
        if let [(indices, _)] = copies.as_slice()
            && others.len() < indices.len()
        {
            let copy = copies.into_iter().next().map(|(_, copy)| copy).expect("one group");
            return match tested(others) {
                Some(test) => Expr::cond(test, otherwise, copy),
                None => copy,
            };
        }
        let mut value = otherwise;
        for (indices, copy) in copies.into_iter().rev() {
            let test = tested(&indices).expect("a group of one variant at least");
            value = Expr::cond(test, copy, value);
        }
        value
    }

    /// Whether `ty` is an untagged enum whose array is changed in place
    /// itself, through a `&mut`, each of its values one it holds itself
    /// (`untagged::arrays_in_place`, rd490): a copy or a clone of it is
    /// then its own array, whatever its payload's type. Of another, a
    /// `&mut` is a place, which `*r = v` gives another value (#347).
    pub(super) fn untagged_changed_itself(&self, ty: Ty<'tcx>) -> bool {
        self.untagged(ty)
            .is_some_and(|(adt, args)| super::untagged::arrays_in_place(self.tcx, adt, args))
            && self.mutated_itself(ty)
    }

    /// Whether a copy or a clone of `ty`, an untagged enum each of whose
    /// values is an array it holds itself (`untagged::arrays_in_place`,
    /// rd490), is the array's `slice()`: where none of its variants' items
    /// `needs` one of its own, a copy's `contains_mutated`, a clone's
    /// `needs_clone`.
    pub(super) fn sliced_untagged(&self, ty: Ty<'tcx>, needs: &dyn Fn(Ty<'tcx>) -> bool) -> bool {
        let Some((adt, args)) = self.untagged(ty) else {
            return false;
        };
        super::untagged::arrays_in_place(self.tcx, adt, args)
            && adt.variants().iter().all(|v| {
                let payload = self.field_ty(&v.fields.raw[0], args);
                match (self.shape(payload), payload.kind()) {
                    (Shape::Array(tys), _) => !tys.into_iter().any(needs),
                    (_, ty::Array(item, _)) => !needs(*item),
                    _ => false,
                }
            })
    }

    /// A place a raw pointer is taken to or into, moved, `let kept = m;`:
    /// a copy, as the value moved is its own allocation, at another address,
    /// as each live local is (#1535), and the place keeps its object, which
    /// a write after the move is over (#1500). Of what Rust's move copies,
    /// its bytes: each struct, tuple or array inline in it copied in turn,
    /// `{ ...o, inner: [o.inner[0], o.inner[1]] }`, and what it owns
    /// elsewhere, a `Vec`'s items, moved with it, shared, as is each of
    /// std's types, which JS holds in a form of its own, a `Map`. None of
    /// one moved whole, a `Vec` JS holds as one object with its items,
    /// whose copy would be another's (#1551).
    pub(super) fn moved_apart(&self, place: Expr, ty: Ty<'tcx>) -> Option<Expr> {
        // Of the crate's types, or a library's, a tuple or an array: held as
        // its fields, which `shape` lists, as a std type isn't.
        let own = |t: Ty<'tcx>| match t.kind() {
            ty::Adt(adt, _) => !self.recognition().in_sysroot(adt.did()),
            ty::Tuple(_) | ty::Array(..) => true,
            _ => false,
        };
        if !own(ty) {
            return None;
        }
        // A field held inline that's an object of its own, copied in turn.
        let inline = |t: Ty<'tcx>| {
            own(t)
                && self.is_object(t)
                && !self.mut_ref_form(t).is_cell()
                && (!matches!(self.shape(t), Shape::Other) || matches!(t.kind(), ty::Array(..)))
        };
        match self.shape(ty) {
            Shape::Object(fields) => {
                let mut props = vec![Prop::Spread(place.clone())];
                // A flattened one's own keys (#1847).
                let fields = match self.own_object_fields(ty) {
                    Some(own) => own.into_iter().map(|(name, t, _)| (name, t)).collect(),
                    None => Self::named_fields(fields),
                };
                for (name, t) in fields {
                    if inline(t) {
                        props.push(Prop::Field(
                            name.clone(),
                            self.moved_apart(Expr::member(place.clone(), name), t)?,
                        ));
                    }
                }
                Some(Expr::object(props))
            }
            Shape::Array(tys) => {
                let mut items = Vec::new();
                for (i, t) in tys.into_iter().enumerate() {
                    let item = Expr::index(place.clone(), Expr::int(i as i128));
                    items.push(if inline(t) { self.moved_apart(item, t)? } else { item });
                }
                Some(Expr::array(items))
            }
            // An array's items: each an object held inline copied in turn,
            // as a write over the place it was moved from keeps them, a
            // pointer's at each (#1553); else shared.
            Shape::Other => match *ty.kind() {
                ty::Array(item, _) if inline(item) => {
                    let name = self.fresh(crate::lower::Base::Named("item"));
                    let copied = self.moved_apart(Expr::var(&name), item)?;
                    Some(Expr::read(
                        place,
                        "map",
                        vec![Expr::arrow(
                            vec![js::Pattern::Name(name)],
                            vec![js::StmtKind::Return(Some(copied)).at(js::Span::NONE)],
                        )],
                    ))
                }
                ty::Array(..) => Some(Expr::read(place, "slice", Vec::new())),
                _ => None,
            },
        }
    }

    /// Whether a call's argument `at`, a `Copy` value read at `place` of the
    /// caller's `body`, is given uncopied, though its type is changed in
    /// place somewhere: its callee, `callee` of `args` as it resolves to a
    /// function of the crate's, only reads that parameter
    /// (`Mutated::params_only_read`), and nothing changes what `place` holds
    /// while the call runs, so Rust's copy is a snapshot nothing changes, as
    /// JS's shared object is. That is a local of the caller's or a part of
    /// one, not through a reference nor a static's, which the caller never
    /// borrows `&mut` or raw, by a closure it gives the call too,
    /// `incidence_at(c, || c.y += 1)` (#1801).
    pub(super) fn given_uncopied(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        (callee, args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        at: usize,
        place: rustc_middle::mir::Place<'tcx>,
    ) -> bool {
        use rustc_middle::mir::{Rvalue, StatementKind};
        let resolved = match self.resolve_instance(callee, args) {
            Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Item(_)) => instance.def_id(),
            _ => return false,
        };
        let only_read = (self.krate.mutated.params_only_read.get(&resolved)).is_some_and(|read| read.contains(&at));
        let own = !place.is_indirect();
        let lent = (body.basic_blocks.iter().flat_map(|data| data.statements.iter())).any(|statement| {
            matches!(&statement.kind, StatementKind::Assign(assign) if match &assign.1 {
                Rvalue::Ref(_, kind, lent) => kind.mutability().is_mut() && lent.local == place.local,
                Rvalue::RawPtr(_, lent) => lent.local == place.local,
                _ => false,
            })
        });
        only_read && own && !lent
    }

    /// Rust copies a `Copy` value when it's read, and JS objects are shared
    /// references. The two only disagree if one of the copies is later
    /// changed in place, which needs a type in `mutated`. So only those
    /// types are copied, and everything else stays shared.
    /// The fields of `ty`, a struct, whose object's own keys they are, as
    /// a copy, a clone or a move apart of it reads and writes each one it
    /// copies in turn (#1847): its own fields, and in place of a flattened
    /// field its struct's, in turn, the parent's keys too, never a key of
    /// the flattened field's name, which the object hasn't; a flattened
    /// field of another kind, a keyed enum's or a state's, its keys the
    /// parent's, left to the spread. `None` of a struct with no flattened
    /// field, whose shape's fields are these.
    pub(in crate::lower) fn own_object_fields(
        &self,
        ty: Ty<'tcx>,
    ) -> Option<Vec<(String, Ty<'tcx>, &'tcx ty::FieldDef)>> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if !adt.is_struct()
            || !adt
                .non_enum_variant()
                .fields
                .iter()
                .any(|f| super::bindings::is_flatten(self.tcx, f))
        {
            return None;
        }
        let mut fields = Vec::new();
        self.own_fields_into(ty, &mut fields);
        Some(fields)
    }

    fn own_fields_into(&self, ty: Ty<'tcx>, fields: &mut Vec<(String, Ty<'tcx>, &'tcx ty::FieldDef)>) {
        let ty::Adt(adt, args) = ty.kind() else { return };
        for field in &adt.non_enum_variant().fields {
            let field_ty = self.field_ty(field, args);
            if super::bindings::is_flatten(self.tcx, field) {
                if super::bindings::plain_struct(self.tcx, field_ty).is_some() {
                    self.own_fields_into(field_ty, fields);
                }
                continue;
            }
            fields.push((
                super::bindings::field_key(self.tcx, field).text().to_owned(),
                field_ty,
                field,
            ));
        }
    }

    /// A `ty` value of the fields of `ty`'s at `place` alone, a flattened
    /// struct's (`Flattened::projected`, #1820), where `place` may be its
    /// parent's object, which has the parent's keys too: `{ min: r.min,
    /// max: r.max, step: r.step }`, each field copied in turn where it's
    /// changed in place, and its own flattened fields' keys, which are
    /// `place`'s too. Read once where it's read more than once.
    pub(in crate::lower) fn projection(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        if !place.reads_same() {
            let param = self.unspelled("value");
            let body = self.projection(Expr::var(&param), ty);
            return Expr::call(
                Expr::arrow(
                    vec![param.clone().into()],
                    vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
                ),
                vec![place],
            );
        }
        let mut props = Vec::new();
        self.projected_props(&place, ty, &mut props);
        Expr::object(props)
    }

    fn projected_props(&self, place: &Expr, ty: Ty<'tcx>, props: &mut Vec<Prop>) {
        let ty::Adt(adt, args) = ty.kind() else {
            unreachable!("a flattened struct")
        };
        for field in &adt.non_enum_variant().fields {
            let field_ty = self.field_ty(field, args);
            if super::bindings::is_flatten(self.tcx, field) {
                self.projected_props(place, field_ty, props);
                continue;
            }
            let key = super::bindings::field_key(self.tcx, field).text().to_owned();
            let read = Expr::member(place.clone(), key.clone());
            let value = match self.contains_mutated(field_ty) {
                true => self.copy(read, field_ty),
                false => read,
            };
            props.push(Prop::Field(key, value));
        }
    }

    /// Is a part of `ty` copied by a copy of what holds it: one something
    /// changes in place, or, of a copy of what a library stores (`stored`,
    /// `copy_stored`), one that may be an object too (#1907).
    fn copied_part(&self, ty: Ty<'tcx>, stored: bool) -> bool {
        self.contains_mutated(ty) || (stored && self.holds_plain_object(ty, 0))
    }

    /// An array `array` of `from`, mapped by a function that's the identity
    /// in JS into one of `to`, `["a", "b"].map(String::from)`: the array
    /// itself, as JS's `map` would have made a new one, where no one can
    /// tell the two apart: one just made, one moved, as of a type that
    /// isn't `Copy`, or one neither of whose types anything changes in
    /// place. Else its copy, as a `Copy` array mapped stays usable and
    /// either may be written after, `b[0] = 5` (#1882).
    pub(in crate::lower) fn mapped_as_itself(&self, array: Expr, from: Ty<'tcx>, to: Ty<'tcx>) -> Expr {
        let told_apart = (self.contains_mutated(from) || self.contains_mutated(to)) && self.is_copy(from);
        match told_apart && !fresh_array(&array) {
            true => self.copy(array, from),
            false => array,
        }
    }

    pub(super) fn copy_if_needed(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        // An array made here, which nothing else holds, of items that need
        // no copy of their own: itself.
        let fresh = fresh_array(&place);
        let items_plain = match ty.kind() {
            ty::Array(item, _) => !self.contains_mutated(*item),
            _ => false,
        };
        if fresh && items_plain {
            crate::decisions::record_typed(
                "copy-if-needed",
                "fresh",
                crate::lower::recognition::kind_of(self.tcx, ty),
            );
            return place;
        }
        if self.contains_mutated(ty) && self.is_copy(ty) {
            crate::decisions::record_typed(
                "copy-if-needed",
                "copied",
                crate::lower::recognition::kind_of(self.tcx, ty),
            );
            self.copy(place, ty)
        } else {
            crate::decisions::record_typed(
                "copy-if-needed",
                "shared",
                crate::lower::recognition::kind_of(self.tcx, ty),
            );
            place
        }
    }

    /// A clone of a value of a type parameter bounded by `Copy` and not by
    /// `Clone`: its `Copy` dictionary's supertrait `Clone`'s, `TCopy.Clone()`,
    /// which a caller gives where the type's clone runs code, a hand-written
    /// `Clone` (`copy_clone_runs_code`), else its copy, `copy`'s: `TCopy?.Clone
    /// ? TCopy.Clone().clone(x) : TCopy ? TCopy.copy(x) : x` (ADR 0623,
    /// amended, #1451). Read once, `((value) => ..)(source)`.
    pub(super) fn cloned_by_copy_bound(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        let Some(dictionary) = self.copy_evidence(ty) else {
            return self.copy(place, ty);
        };
        let cloned = |place: Expr| {
            let clone = Expr::call(Expr::member(dictionary.clone(), "Clone"), Vec::new());
            Expr::cond(
                Expr::optional_member(dictionary.clone(), "Clone"),
                Expr::call(Expr::member(clone, "clone"), vec![place.clone()]),
                self.copy(place, ty),
            )
        };
        if place.reads_same() {
            return cloned(place);
        }
        let param = self.unspelled("value");
        Expr::call(
            Expr::arrow(
                vec![param.clone().into()],
                vec![js::StmtKind::Return(Some(cloned(Expr::var(&param)))).at(js::Span::NONE)],
            ),
            vec![place],
        )
    }

    /// A function of a `Copy` item's copy as std makes one where it copies
    /// its bits, `copied()`'s, `copy_from_slice`'s, `partition`'s push of a
    /// `&T` and an array made of a slice: `(item) => <copy>` where
    /// something changes one in place (`copy_if_needed`), never a call of a
    /// hand-written `Clone`, which std doesn't make there (ADR 0623,
    /// amended); `None` where the copy is the item itself.
    pub(super) fn bits_copy_fn(&self, item: Ty<'tcx>) -> Option<Expr> {
        let item_name = self.fresh(crate::lower::Base::Named("item"));
        let copied = self.copy_if_needed(Expr::var(&item_name), item);
        match &copied.kind {
            js::ExprKind::Var(name) if *name == item_name => None,
            _ => Some(Expr::arrow(
                vec![item_name.clone().into()],
                vec![js::StmtKind::Return(Some(copied)).at(js::Span::NONE)],
            )),
        }
    }

    /// A value read out of `place` as `ptr::read` reads it, its bits apart
    /// from the place, which a write after may change in place: a `Copy`
    /// value's copy, `copy_if_needed`'s; another's, of a type a `&mut`
    /// writes whole in place (ADR 0147), its top level apart, `$apart(v)`,
    /// as `mem::replace`'s `$take` gives what was there (#1351, the user's
    /// form). What it holds by a pointer, a `Vec`'s items, a `Box`'s
    /// value, is shared, as the read shares it.
    pub(super) fn read_out(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        if self.is_copy(ty) || !self.mut_ref_form(ty).written_in_place() {
            return self.copy_if_needed(place, ty);
        }
        Expr::call_helper("$apart", vec![place])
    }

    /// Is `ty` a `Copy` closure that captures something, one of size, a
    /// reference to which the crate makes a `&dyn Fn` where it reads a `dyn
    /// Fn`'s address (`Counted::copied_apart`, #1053's owner)? Its JS
    /// function is its place, so each copy of it is a
    /// function of its own (`copy`, #1090), and what holds one is copied
    /// as what holds what's changed in place is. One of no size is the
    /// function, its address no allocation's, as Rust's is.
    fn copied_apart(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Closure(..))
            && self.krate.counted.copied_apart(self.tcx, ty)
            && !self.has_one_value(ty)
            && self.is_copy(ty)
    }

    /// Of a `Copy` closure that changes what it captured, whether its copy
    /// is its clone: Rust gives each copy its own captures, where a copy of
    /// a JS function is the function, sharing them (ADR 0246), but one the
    /// crate clones apart (ADR 0522) is made again by `$clone`, of its own
    /// captures. `None` of what's no such closure.
    fn own_captures_copied(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Closure(id, args) = *ty.kind() else { return None };
        (super::analysis::writes_what_it_took(self.tcx, id, args) && self.is_copy(ty))
            .then(|| self.krate.closure_facts.clones_apart(id))
    }

    /// Would a copy of `ty` share what a closure it holds changes, as a copy
    /// of a JS function does: one it is or holds by value, in a tuple, an
    /// array, a struct's or an enum's field or another closure's capture,
    /// `Some(counter())` too, that changes what it captured, which the crate
    /// doesn't clone apart (#1211)? Such a copy is refused.
    pub(super) fn copies_own_captures(&self, ty: Ty<'tcx>) -> bool {
        self.holds_by_value(ty, &|t| self.own_captures_copied(t) == Some(false), 0)
    }

    /// Whether `ty` is, or holds by value, a type `found` says: not through
    /// a reference or a pointer, which a copy shares as Rust's does.
    fn holds_by_value(&self, ty: Ty<'tcx>, found: &dyn Fn(Ty<'tcx>) -> bool, depth: usize) -> bool {
        if found(ty) {
            return true;
        }
        if depth > 16 {
            return false;
        }
        match *ty.kind() {
            ty::Tuple(parts) => parts.iter().any(|t| self.holds_by_value(t, found, depth + 1)),
            ty::Array(item, _) => self.holds_by_value(item, found, depth + 1),
            ty::Closure(_, args) => {
                (args.as_closure().upvar_tys().iter()).any(|t| self.holds_by_value(t, found, depth + 1))
            }
            ty::Adt(adt, args) => adt
                .all_fields()
                .any(|f| self.holds_by_value(self.field_ty(f, args), found, depth + 1)),
            _ => false,
        }
    }

    pub(super) fn is_copy(&self, ty: Ty<'tcx>) -> bool {
        is_copy(self.tcx, self.typing_env, ty)
    }

    /// An enum rust-js writes as ADR 0033 says, that's `Copy`: one of the
    /// crate's own, or a library's (ADR 0100), or `Result`.
    pub(super) fn is_copy_enum(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if adt.is_enum()
            && (self.krate.foreign.compiled_by_rust_js(adt.did()) || self.is_std_type(ty, StdItem::Result)))
            && self.is_copy(ty)
    }

    /// Is each use of a constant of `ty` a copy of its one JS value: where
    /// something changes one in place, or, of one that isn't `Copy`, where
    /// it isn't `shareable`, a `Vec`'s, whose `push` changes it (ADR 0031)?
    pub(super) fn const_use_copied(&self, ty: Ty<'tcx>) -> bool {
        self.contains_mutated(ty) || (!self.is_copy(ty) && !shareable(self.tcx, ty))
    }

    /// A use of a constant's one JS value, `place`, copied (`const_use_copied`):
    /// a `Copy` value's copy, another's clone, `[...EMPTY]` of a `Vec`'s, as
    /// each use of a constant is a value of its own.
    pub(super) fn const_copy(&mut self, place: Expr, ty: Ty<'tcx>, span: rustc_span::Span) -> super::R<Expr> {
        if self.is_copy(ty) {
            return Ok(self.copy(place, ty));
        }
        let mut made = Vec::new();
        let clone = self.clone_value(place, ty, span, &mut made)?;
        match made.is_empty() {
            true => Ok(clone),
            false => Err(self.unsupported(span, &format!("a use of a constant of type `{ty}`"))),
        }
    }

    pub(super) fn contains_mutated(&self, ty: Ty<'tcx>) -> bool {
        // Each type once: a type met along two paths isn't walked twice. It
        // ends at a `Box`, which is `Other`, so no answer depends on another.
        let cached = self.walks().mutated.borrow().get(&ty).copied();
        let mutated = cached.unwrap_or_else(|| {
            let mutated = self.contains_mutated_uncached(ty);
            self.walks().mutated.borrow_mut().insert(ty, mutated);
            mutated
        });
        if !mutated {
            self.elided(ty);
        }
        mutated
    }

    /// `ty`, found changed in place by none of the crate's code, so what
    /// copies it is left out: of a library's function, said in its
    /// contract, so a consumer that changes it compiles the function itself
    /// (`Caches::elided`, #2112's change D). Every such answer of the
    /// function's, a copy's or not: more than it left out compiles more,
    /// never less.
    fn elided(&self, ty: Ty<'tcx>) {
        if self.krate.library && !self.krate.compiled_once && self.may_be_object(ty) {
            let shape = crate::lower::library::rc_shape(self.tcx, ty);
            let caches = crate::lower::session::caches(self.tcx);
            (caches.elided.borrow_mut())
                .entry(self.tcx.typeck_root_def_id(self.item))
                .or_default()
                .insert(shape);
        }
    }

    pub(super) fn contains_mutated_uncached(&self, ty: Ty<'tcx>) -> bool {
        // A type parameter may be anything changed in place, but a function's:
        // a copy of a JS function is the function (ADR 0246), unless one may
        // be cloned apart (ADR 0522).
        if self.is_unknown(ty) && self.is_callable(ty) {
            return self.krate.closure_facts.functions_clone_apart();
        }
        self.is_unknown(ty)
            || self.mutated_itself(ty)
            || self.copied_apart(ty)
            // A closure whose copy is its clone, of its own captures (#1211).
            || self.own_captures_copied(ty) == Some(true)
            || match self.shape(ty) {
                Shape::Object(fields) => fields.iter().any(|&(_, t)| self.contains_mutated(t)),
                Shape::Array(tys) => tys.iter().any(|&t| self.contains_mutated(t)),
                // `Some(x)` is `x` (ADR 0030), and a variant's fields are
                // its object's (ADR 0033).
                Shape::Other => match ty.kind() {
                    _ if let Some(inner) = self.option_of(ty) => self.contains_mutated(inner),
                    ty::Adt(adt, args) if self.is_copy_enum(ty) => adt.variants().iter().any(|v| {
                        self.variant_fields(v, args)
                            .iter()
                            .any(|&(_, t)| self.contains_mutated(t))
                    }),
                    ty::Adt(..) if self.is_union(ty) => self.union_holds_mutated(ty),
                    // An array of what's changed in place holds what's changed:
                    // a copy of it copies each item (ADR 0052).
                    ty::Array(item, _) => self.contains_mutated(*item),
                    _ => false,
                },
            }
    }

    /// Is `ty` itself changed in place somewhere, not just a part of it?
    pub(super) fn mutated_itself(&self, ty: Ty<'tcx>) -> bool {
        let mutated =
            self.crosses_crates(ty) || (self.krate.mutated.types.iter()).any(|&mutated| self.instance_of(ty, mutated));
        if !mutated {
            self.elided(ty);
        }
        mutated
    }

    /// Might another crate change `ty` in place (ADR 0100)? A library's type,
    /// or a tuple or an array, which is any crate's, where a library's
    /// contract says its code changes one (`Foreign::changed_by_a_library`,
    /// #1871), of a type whose values may be JS objects: a fieldless enum's
    /// string can't change, an enum's variant with fields can. A library's
    /// consumers' changes are their own, who compile what of its copies them
    /// (ADR 0914, amended); but `@rust-js/std`'s, compiled once for all (ADR
    /// 0561), are any crate's.
    pub(super) fn crosses_crates(&self, ty: Ty<'tcx>) -> bool {
        let foreign = &self.krate.foreign;
        let shared = match ty.kind() {
            ty::Adt(adt, _) => {
                (foreign.in_library(adt.did()) && foreign.changed_by_a_library(ty))
                    || (self.krate.compiled_once && super::library::reachable(self.tcx, adt.did()))
            }
            ty::Tuple(_) | ty::Array(..) => self.krate.compiled_once || foreign.changed_by_a_library(ty),
            _ => false,
        };
        shared && self.may_be_object(ty)
    }

    /// Is `ty` a `Vec` type something may change (ADR 0052)? One a library's
    /// code changes too, as its contract says (#1871), and in
    /// `@rust-js/std`, compiled once for all, any.
    pub(super) fn vec_changed(&self, ty: Ty<'tcx>) -> bool {
        let changed = self.krate.compiled_once
            || self.krate.foreign.changed_by_a_library(ty)
            || self
                .krate
                .changed_vecs
                .iter()
                .any(|&changed| self.instance_of(ty, changed));
        if !changed {
            self.elided(ty);
        }
        changed
    }

    /// Might the array of `item`s that a `Vec<item>` and a boxed `[item]`
    /// are be changed in place, as either? They are one JS array, which
    /// `into_vec()`, `into_boxed_slice()` and a C string's
    /// `into_bytes_with_nul()` hand from one to the other without a copy:
    /// a clone of one is told apart by a change of the other (ADR 0530,
    /// amended). A `Vec<item>` something changes, `vec_changed`'s, or a
    /// `[item]` changed in place, `contains_mutated`'s.
    pub(super) fn items_changed(&self, item: Ty<'tcx>) -> bool {
        let vec_of = |changed: Ty<'tcx>| {
            self.is_std_type(changed, StdItem::Vec)
                && matches!(changed.kind(), ty::Adt(_, args) if self.instance_of(item, args.type_at(0)))
        };
        self.krate.compiled_once
            || self.krate.foreign.changes_a_vec_of(item)
            || self.krate.changed_vecs.iter().any(|&changed| vec_of(changed))
            || self.contains_mutated(Ty::new_slice(self.tcx, item))
    }

    /// Is `ty` one of the types `general` stands for? A type mutated in a
    /// generic function has its parameters: `Holder<T>` stands for every
    /// `Holder<..>`, but `Pair<u32>` only for itself, so a `Pair<bool>`
    /// needn't be copied because a `Pair<u32>` is changed. And `ty` of a
    /// generic function's is any its parameters may be: its `Holder<T>` is
    /// the `Holder<Numbers>` changed in place by its caller. Lifetimes don't
    /// matter.
    pub(super) fn instance_of(&self, ty: Ty<'tcx>, general: Ty<'tcx>) -> bool {
        instance_of(&self.recognition(), ty, general)
    }

    /// Whether reconstructing a value can replace its Clone implementation.
    /// References and Rc share their referent; owned fields must all qualify.
    pub(super) fn structural_clone(&self, ty: Ty<'tcx>) -> bool {
        self.structural_clone_in(ty, &mut IndexSet::default())
    }

    pub(super) fn structural_clone_in(&self, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) -> bool {
        if !seen.insert(ty) {
            return true;
        }

        match ty.kind() {
            ty::Ref(..) => true,
            ty::Param(_) | ty::Alias(..) | ty::Dynamic(..) => false,
            ty::Tuple(parts) => parts.iter().all(|t| self.structural_clone_in(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.structural_clone_in(*item, seen),
            // A counted `Rc`'s clone counts it (ADR 0320).
            ty::Adt(..) if self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some() => false,
            ty::Adt(..)
                if self.is_rc(ty)
                    || self.uncounted_weak(ty).is_some()
                    || self.is_string_like(ty)
                    || self.is_js_object(ty) =>
            {
                true
            }
            ty::Adt(..) if self.has_user_impl(self.clone_trait(), ty) => false,
            ty::Adt(_, args) if self.is_std_wrapper(ty) || self.is_map(ty) => {
                args.types().all(|t| self.structural_clone_in(t, seen))
            }
            ty::Adt(adt, args) => adt
                .all_fields()
                .all(|f| self.structural_clone_in(self.field_ty(f, args), seen)),
            _ => true,
        }
    }
}

/// Is `ty` one of the types `general` stands for (`FnCx::instance_of`)?
pub(super) fn instance_of<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
    general: Ty<'tcx>,
) -> bool {
    instance_of_in(recognition.tcx, recognition.typing_env, ty, general)
}

/// `instance_of`, of `tcx` and the body's `typing_env` alone, asked before
/// the crate's libraries are read too (`cloned_apart`, #1243).
pub(super) fn instance_of_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
    general: Ty<'tcx>,
) -> bool {
    let unknown = |t| super::recognition::is_unknown_where(tcx, t, typing_env);
    match (ty.kind(), general.kind()) {
        _ if unknown(general) || unknown(ty) => true,
        (ty::Adt(adt, args), ty::Adt(general_adt, general_args)) => {
            adt.did() == general_adt.did()
                && args
                    .iter()
                    .zip(general_args.iter())
                    .all(|(arg, general)| match (arg.as_type(), general.as_type()) {
                        (Some(arg), Some(general)) => instance_of_in(tcx, typing_env, arg, general),
                        _ => true,
                    })
        }
        (ty::Ref(_, inner, _), ty::Ref(_, general, _))
        | (ty::Slice(inner), ty::Slice(general))
        | (ty::Array(inner, _), ty::Array(general, _)) => instance_of_in(tcx, typing_env, *inner, *general),
        (ty::Tuple(parts), ty::Tuple(general)) => {
            parts.len() == general.len()
                && parts
                    .iter()
                    .zip(general.iter())
                    .all(|(p, g)| instance_of_in(tcx, typing_env, p, g))
        }
        _ => tcx.erase_and_anonymize_regions(ty) == tcx.erase_and_anonymize_regions(general),
    }
}

/// Is `ty` one of the types `general` stands for, `general` of the body
/// whose typing environment `typing_env` is, and its type parameters what
/// make it `ty` only where they meet that body's bounds? `instance_of`
/// takes a type parameter to stand for any type; but the body's where
/// clauses say which it can be: `Map<I, F>` of a function bound by
/// `I: Iterator<Item = u32>` is never a `Map` of a `slice::Iter<'_, i32>`,
/// whatever another function makes and clones (#955). A parameter given two
/// types is one it may meet; a bound of what's left of parameters, one not
/// given or `ty`'s own, holds where some type makes it (#1895).
pub(super) fn instance_within_bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
    general: Ty<'tcx>,
) -> bool {
    use rustc_middle::ty::TypeFoldable;
    use rustc_trait_selection::infer::TyCtxtInferExt;
    use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt;
    if !instance_of_in(tcx, typing_env, ty, general) {
        return false;
    }
    let mut given = IndexMap::default();
    if !parameters_given(ty, general, &mut given) || given.is_empty() {
        return true;
    }
    let infcx = tcx.infer_ctxt().build(ty::TypingMode::PostAnalysis);
    let mut giving = ty::BottomUpFolder {
        tcx,
        ty_op: |t: Ty<'tcx>| match *t.kind() {
            ty::Param(p) => given.get(&p.index).copied().unwrap_or(t),
            _ => t,
        },
        lt_op: |r| r,
        ct_op: |c| c,
    };
    // What's left of type parameters, `ty`'s own or those not given, may
    // be any type: each an inference variable, so a bound holds where some
    // type makes it, not wherever one's left. Another function's closure
    // `|held: U| Held { held }`, of its own `U`, is never `FnMut(i32) ->
    // i32`, whatever `U` is (#1895).
    let mut any = ty::BottomUpFolder {
        tcx,
        ty_op: |t: Ty<'tcx>| match *t.kind() {
            ty::Param(_) => infcx.next_ty_var(rustc_span::DUMMY_SP),
            _ => t,
        },
        // A lifetime asks nothing of what holds.
        lt_op: |r: ty::Region<'tcx>| match r.kind() {
            ty::ReEarlyParam(_) => tcx.lifetimes.re_erased,
            _ => r,
        },
        ct_op: |c: ty::Const<'tcx>| match c.kind() {
            ty::ConstKind::Param(_) => infcx.next_const_var(rustc_span::DUMMY_SP),
            _ => c,
        },
    };
    typing_env.param_env.caller_bounds().all(|bound| {
        let bound = bound.fold_with(&mut giving).fold_with(&mut any);
        let obligation = rustc_trait_selection::traits::Obligation::new(
            tcx,
            rustc_trait_selection::traits::ObligationCause::dummy(),
            ty::ParamEnv::empty(),
            bound,
        );
        infcx.predicate_may_hold(&obligation)
    })
}

/// Each type parameter of `general` and what it is in `ty`, of the parts
/// `instance_of` matches, into `given`; `false` where one is given two.
fn parameters_given<'tcx>(ty: Ty<'tcx>, general: Ty<'tcx>, given: &mut IndexMap<u32, Ty<'tcx>>) -> bool {
    match (ty.kind(), general.kind()) {
        (_, ty::Param(p)) => *given.entry(p.index).or_insert(ty) == ty,
        (ty::Adt(adt, args), ty::Adt(general_adt, general_args)) if adt.did() == general_adt.did() => args
            .types()
            .zip(general_args.types())
            .all(|(arg, general)| parameters_given(arg, general, given)),
        (ty::Ref(_, inner, _), ty::Ref(_, general, _))
        | (ty::Slice(inner), ty::Slice(general))
        | (ty::Array(inner, _), ty::Array(general, _)) => parameters_given(*inner, *general, given),
        (ty::Tuple(parts), ty::Tuple(general)) => parts
            .iter()
            .zip(general.iter())
            .all(|(p, g)| parameters_given(p, g, given)),
        _ => true,
    }
}

/// Whether a value of `ty` is `Copy` where `typing_env` says, as rustc's
/// trait solver answers of it there: the one place rust-js asks (#2112's
/// change D). A type parameter's of its bounds; an instance's of its
/// arguments.
pub(in crate::lower) fn is_copy<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    tcx.type_is_copy_modulo_regions(typing_env, ty)
}

/// What the crate changes in place (ADR 0052): the types whose objects it
/// changes, and the `Vec`s.
#[derive(Clone, Copy)]
pub(super) struct Changed<'a, 'tcx> {
    pub(super) mutated: &'a IndexSet<Ty<'tcx>>,
    pub(super) vecs: &'a IndexSet<Ty<'tcx>>,
}

/// Can one JS value be every use of a constant of `ty`, which Rust makes
/// afresh at each? A function, a closure, a shared reference, a `dyn`
/// behind one, a number and a string can't change in place; an array, a
/// tuple, an `Option` and the crate's own struct or enum of those, or a
/// library's, can, and is copied where it's read if something changes one,
/// as any value is.
pub(super) fn shareable<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    fn walk<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) -> bool {
        if !seen.insert(ty) {
            return true;
        }

        match ty.kind() {
            ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str => true,
            ty::FnPtr(..) | ty::FnDef(..) | ty::Closure(..) | ty::Dynamic(..) => true,
            ty::Ref(_, inner, Mutability::Not) => walk(tcx, *inner, seen),
            // A thread-local, its module's one variable, which every use is (ADR 0037).
            ty::Adt(adt, _) if super::recognition::is_std_def(tcx, adt.did(), StdItem::LocalKey) => true,
            ty::Array(item, _) | ty::Slice(item) => walk(tcx, *item, seen),
            ty::Tuple(items) => items.iter().all(|item| walk(tcx, item, seen)),
            ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => walk(tcx, args.type_at(0), seen),
            // A `MaybeUninit` is what it holds or `undefined` (ADR 0332), and a
            // `String` its JS string. Not a `Vec`, changed in place and copied
            // by nothing that reads it.
            ty::Adt(adt, args) if super::recognition::is_std_def(tcx, adt.did(), StdItem::MaybeUninit) => {
                walk(tcx, args.type_at(0), seen)
            }
            ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String) => true,
            // A `TypeId`, its type's key, a string (ADR 0331).
            ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::TypeId) => true,
            // A raw pointer, a `NonNull` too, is a handle on its place, or an
            // address, which nothing changes in place (ADR 0562).
            _ if super::pointers::raw_pointee(tcx, ty).is_some() => true,
            // A JS value that's `Copy`, react's `ElementType`, nothing changes
            // through (ADR 0234).
            ty::Adt(adt, args) if super::representation::marks_js_object(tcx, *adt, args) => {
                is_copy(tcx, ty::TypingEnv::fully_monomorphized(), ty)
            }
            // The crate's own struct or enum, or a library's, React's
            // `Dispatcher`, compiled as its fields say too (ADR 0100); not
            // std's, whose fields are no JS value's.
            ty::Adt(adt, args) if !super::recognition::is_std_crate(tcx, adt.did()) && !adt.is_union() => adt
                .all_fields()
                .all(|field| walk(tcx, field.ty(tcx, args).skip_normalization(), seen)),
            _ => false,
        }
    }
    walk(tcx, ty, &mut IndexSet::default())
}
