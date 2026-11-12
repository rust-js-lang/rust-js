//! When a value is copied. Rust copies a `Copy` value where it's read, and
//! JS shares an object: a copy is made only where one of them may change in
//! place, which is found once for each type (ADRs 0020, 0052, 0100).

use super::recognition::StdItem;
use super::{FnCx, Shape};
use crate::collections::{IndexMap, IndexSet};
use crate::js;
use crate::js::{Expr, Op, Prop};
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A fresh `ty` value equal to the one at `place`: `{ ...p }`, `[t[0], t[1]]`.
    /// A field that also contains mutated types is copied in turn.
    pub(super) fn copy(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        // A copied default's `Self` is the impl's type, a number say, which
        // needs no copy (ADR 0049).
        if self.is_unknown(ty)
            && let Some(dictionary) =
                self.given_evidence(|tr| tr.self_ty() == ty && self.tcx.is_lang_item(tr.def_id, LangItem::Copy))
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
            return Expr::call(
                Expr::arrow(
                    vec!["value".into()],
                    vec![js::StmtKind::Return(Some(copied(Expr::var("value")))).at(js::Span::NONE)],
                ),
                vec![place],
            );
        }
        // A copy reads its source once per part: a value that isn't a place,
        // like `$unwrap(v[0])`, is taken once, `((value) => ..)(source)`.
        let many = match self.shape(ty) {
            Shape::Object(fields) => fields.iter().any(|&(_, t)| self.contains_mutated(t)),
            Shape::Array(tys) => tys.len() > 1,
            Shape::Other => false,
        };
        if many && !place.reads_same() {
            let body = self.copy(Expr::var("value"), ty);
            return Expr::call(
                Expr::arrow(
                    vec!["value".into()],
                    vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
                ),
                vec![place],
            );
        }
        match self.shape(ty) {
            Shape::Object(fields) => {
                let mut props = vec![Prop::Spread(place.clone())];
                // A `Copy` struct's, which a symbol names no field of
                // (`validate`, ADR 1051).
                for (name, t) in Self::named_fields(fields) {
                    if self.contains_mutated(t) {
                        let field = self.copy(Expr::member(place.clone(), name.clone()), t);
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
                        if self.contains_mutated(t) {
                            self.copy(item, t)
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
                        let body = copy(Expr::var("value"));
                        Expr::call(
                            Expr::arrow(
                                vec!["value".into()],
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
                if let Some(inner) = self.option_of(ty) {
                    if !self.contains_mutated(inner) {
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
                    let probe = Expr::var("value");
                    if !boxed && self.copy(probe.clone(), item).kind == probe.kind {
                        return place;
                    }
                    return once(place, &|o| {
                        let mut none = Expr::bin(Op::LooseEq, o.clone(), Expr::null());
                        if boxed {
                            none = Expr::bin(Op::Or, none, super::std_types::option::is_some_box(o.clone()));
                        }
                        Expr::cond(none, o.clone(), self.copy(o, item))
                    });
                }
                // An array that's changed in place: `a.slice()`, or a copy of
                // each item that is too.
                if let ty::Array(item, _) = ty.kind() {
                    if !self.contains_mutated(*item) {
                        return Expr::call(Expr::member(place, "slice"), Vec::new());
                    }
                    let body = self.copy(Expr::var("item"), *item);
                    let copy = Expr::arrow(
                        vec!["item".into()],
                        vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
                    );
                    return Expr::call(Expr::member(place, "map"), vec![copy]);
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
                        .any(|t| self.contains_mutated(t));
                    if (itself || changed) && self.sliced_untagged(ty, &|t| self.contains_mutated(t)) {
                        return Expr::call(Expr::member(place, "slice"), Vec::new());
                    }
                    return once(place, &|e| {
                        let mut value = e.clone();
                        for variant in adt.variants().iter().rev() {
                            let Some(&(_, t)) = self.variant_fields(variant, args).first() else {
                                continue;
                            };
                            if !(itself || self.contains_mutated(t)) {
                                continue;
                            }
                            let test = self.untagged_variant_test(ty, &[variant], &[], &e, false);
                            value = Expr::cond(test, self.copy(e.clone(), t), value);
                        }
                        value
                    });
                }
                // `{ TAG: "Line", _0: .. }`: a variant with a part that changes
                // gets a copy, and every other value is itself.
                once(place, &|e| {
                    let mut value = e.clone();
                    // Changed in place itself, through a `&mut`: every variant with fields.
                    let itself = self.mutated_itself(ty);
                    for variant in adt.variants().iter().rev() {
                        let fields = self.variant_fields(variant, args);
                        if fields.is_empty() || !(itself || fields.iter().any(|&(_, t)| self.contains_mutated(t))) {
                            continue;
                        }
                        let mut props = vec![Prop::Spread(e.clone())];
                        for (name, t) in fields {
                            if self.contains_mutated(t) {
                                props.push(Prop::Field(name.clone(), self.copy(Expr::member(e.clone(), name), t)));
                            }
                        }
                        let tag =
                            self.variant_test(e.clone(), ty, *adt, adt.variant_index_with_id(variant.def_id), false);
                        value = Expr::cond(tag, Expr::object(props), value);
                    }
                    value
                })
            }
        }
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

    /// Rust copies a `Copy` value when it's read, and JS objects are shared
    /// references. The two only disagree if one of the copies is later
    /// changed in place, which needs a type in `mutated`. So only those
    /// types are copied, and everything else stays shared.
    pub(super) fn copy_if_needed(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        // An array made here, which nothing else holds, `[1, 2]` or a
        // `.slice(..)`, of items that need no copy of their own: itself.
        let fresh = super::iterators::made_array(&place)
            || matches!(&place.kind, js::ExprKind::Call(callee, _)
                if matches!(&callee.kind, js::ExprKind::Member(_, name) if name == "slice"));
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
        let Some(dictionary) =
            self.given_evidence(|tr| tr.self_ty() == ty && self.tcx.is_lang_item(tr.def_id, LangItem::Copy))
        else {
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
        Expr::call(
            Expr::arrow(
                vec!["value".into()],
                vec![js::StmtKind::Return(Some(cloned(Expr::var("value")))).at(js::Span::NONE)],
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
        let copied = self.copy_if_needed(Expr::var("item"), item);
        match &copied.kind {
            js::ExprKind::Var(name) if name == "item" => None,
            _ => Some(Expr::arrow(
                vec!["item".into()],
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
        self.tcx.type_is_copy_modulo_regions(self.typing_env, ty)
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
        if let Some(&mutated) = self.walks().mutated.borrow().get(&ty) {
            return mutated;
        }
        let mutated = self.contains_mutated_uncached(ty);
        self.walks().mutated.borrow_mut().insert(ty, mutated);
        mutated
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
        self.crosses_crates(ty) || self.krate.mutated.iter().any(|&mutated| self.instance_of(ty, mutated))
    }

    /// Might another crate change `ty` in place (ADR 0100)? Any crate using a
    /// library's type might, as its library's code may: this crate can't see
    /// it, so it assumes it does, of a type whose values may be JS objects: a
    /// fieldless enum's string can't change, an enum's variant with fields
    /// can. A tuple or an array is any crate's, so in a crate using a
    /// library, any may cross. A library's consumers' changes are their own,
    /// who compile what of its copies them (ADR 0914, amended); but
    /// `@rust-js/std`'s, compiled once for all (ADR 0561), are any crate's.
    pub(super) fn crosses_crates(&self, ty: Ty<'tcx>) -> bool {
        let shared = match ty.kind() {
            ty::Adt(adt, _) => {
                self.krate.foreign.in_library(adt.did())
                    || (self.krate.compiled_once && super::library::reachable(self.tcx, adt.did()))
            }
            ty::Tuple(_) | ty::Array(..) => self.krate.compiled_once || self.krate.foreign.any(),
            _ => false,
        };
        shared && self.may_be_object(ty)
    }

    /// Is `ty` a `Vec` type something may change (ADR 0052)? One another crate
    /// can hold might be changed there, `vecs_changed_elsewhere`.
    pub(super) fn vec_changed(&self, ty: Ty<'tcx>) -> bool {
        self.vecs_changed_elsewhere()
            || self
                .krate
                .changed_vecs
                .iter()
                .any(|&changed| self.instance_of(ty, changed))
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
        self.vecs_changed_elsewhere()
            || self.krate.changed_vecs.iter().any(|&changed| vec_of(changed))
            || self.contains_mutated(Ty::new_slice(self.tcx, item))
    }

    /// Might any `Vec` be changed where this crate can't see? One another
    /// crate can hold might be changed there (ADR 0100): in a crate using a
    /// library, or `@rust-js/std`, any may be (`crosses_crates`).
    fn vecs_changed_elsewhere(&self) -> bool {
        self.krate.compiled_once || self.krate.foreign.any()
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
        self.structural_clone_in(ty, &mut Vec::new())
    }

    pub(super) fn structural_clone_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if seen.contains(&ty) {
            return true;
        }
        seen.push(ty);
        let structural = match ty.kind() {
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
        };
        seen.pop();
        structural
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
/// types, or a bound of one not given, is one it may meet.
pub(super) fn instance_within_bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
    general: Ty<'tcx>,
) -> bool {
    use rustc_middle::ty::{TypeFoldable, TypeVisitableExt};
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
    typing_env.param_env.caller_bounds().iter().all(|bound| {
        let bound = bound.fold_with(&mut giving);
        bound.has_param() || {
            let obligation = rustc_trait_selection::traits::Obligation::new(
                tcx,
                rustc_trait_selection::traits::ObligationCause::dummy(),
                ty::ParamEnv::empty(),
                bound,
            );
            infcx.predicate_may_hold(&obligation)
        }
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

/// What the crate changes in place (ADR 0052): the types whose objects it
/// changes, and the `Vec`s.
#[derive(Clone, Copy)]
pub(super) struct Changed<'a, 'tcx> {
    pub(super) mutated: &'a IndexSet<Ty<'tcx>>,
    pub(super) vecs: &'a IndexSet<Ty<'tcx>>,
}

/// May a value of `ty` hold, by value, not behind a reference, what the
/// crate changes in place, `changed`: the coarse question whether a
/// library's function that copies or clones one is compiled here, where
/// its library's JS, which copies what its own crate changes, may share
/// one the crate then changes (ADR 0914, amended). `contains_mutated` is
/// the exact one, of the crate's own copies.
pub(super) fn may_hold_changed<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    changed: Changed<'_, 'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    fn walk<'tcx>(
        recognition: &super::recognition::Recognition<'_, 'tcx>,
        changed: Changed<'_, 'tcx>,
        ty: Ty<'tcx>,
        seen: &mut Vec<Ty<'tcx>>,
    ) -> bool {
        if seen.contains(&ty) {
            return false;
        }
        let tcx = recognition.tcx;
        let is = |general: &Ty<'tcx>| !recognition.is_unknown(*general) && instance_of(recognition, ty, *general);
        let pointer_like = |ty: Ty<'tcx>| {
            super::pointers::raw_pointee(tcx, ty).is_some()
                || matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::PhantomData))
        };
        seen.push(ty);
        let holds = match ty.kind() {
            ty::Tuple(_) | ty::Array(..) if changed.mutated.iter().any(is) => true,
            ty::Tuple(items) => items.iter().any(|item| walk(recognition, changed, item, seen)),
            ty::Array(item, _) | ty::Slice(item) => walk(recognition, changed, *item, seen),
            ty::Adt(..) if changed.mutated.iter().chain(changed.vecs).any(is) => true,
            // A pointer to one, a shared `Rc`'s, holds none by value; nor a
            // `PhantomData`; nor std's of a lifetime, which borrows what it
            // reads, a `Ref` or an `Iter`; nor std's of pointers alone, the
            // `Unique` optimized MIR reads a `Box` through.
            ty::Adt(adt, args)
                if pointer_like(ty)
                    || super::recognition::rc_pointee(tcx, ty).is_some()
                    || super::recognition::weak_pointee(tcx, ty).is_some()
                    || (super::recognition::is_std_crate(tcx, adt.did())
                        && (args.regions().next().is_some()
                            || (adt.is_struct()
                                && adt
                                    .all_fields()
                                    .all(|field| pointer_like(field.ty(tcx, args).skip_normalization()))))) =>
            {
                false
            }
            // A field's type as the bounds where it's asked say it is, a
            // `T::Item` of `T: Iterator<Item = Vec<u32>>` a `Vec` (#1023).
            ty::Adt(adt, args) => {
                let field_ty = |field: &ty::FieldDef| {
                    let ty = field.ty(tcx, args).skip_normalization();
                    (tcx.try_normalize_erasing_regions(recognition.typing_env, ty::Unnormalized::new_wip(ty)))
                        .unwrap_or(ty)
                };
                args.types().any(|arg| walk(recognition, changed, arg, seen))
                    || (!super::recognition::is_std_crate(tcx, adt.did())
                        && adt
                            .all_fields()
                            .any(|field| walk(recognition, changed, field_ty(field), seen)))
            }
            _ => false,
        };
        seen.pop();
        holds
    }
    walk(recognition, changed, ty, &mut Vec::new())
}

/// Can one JS value be every use of a constant of `ty`, which Rust makes
/// afresh at each? A function, a closure, a shared reference, a `dyn`
/// behind one, a number and a string can't change in place; an array, a
/// tuple, an `Option` and the crate's own struct or enum of those, or a
/// library's, can, and is copied where it's read if something changes one,
/// as any value is.
pub(super) fn shareable<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    fn walk<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if seen.contains(&ty) {
            return true;
        }
        seen.push(ty);
        let frozen = match ty.kind() {
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
                tcx.type_is_copy_modulo_regions(ty::TypingEnv::fully_monomorphized(), ty)
            }
            // The crate's own struct or enum, or a library's, React's
            // `Dispatcher`, compiled as its fields say too (ADR 0100); not
            // std's, whose fields are no JS value's.
            ty::Adt(adt, args) if !super::recognition::is_std_crate(tcx, adt.did()) && !adt.is_union() => adt
                .all_fields()
                .all(|field| walk(tcx, field.ty(tcx, args).skip_normalization(), seen)),
            _ => false,
        };
        seen.pop();
        frozen
    }
    walk(tcx, ty, &mut Vec::new())
}
