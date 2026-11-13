//! `Clone`, `Default` and `PartialEq` where rust-js writes the
//! implementation: derived ones and std types' (ADRs 0052, 0053). A
//! hand-written one is called instead. `Clone`'s is `clones`, `Default`'s
//! `defaults`.

mod clones;
mod defaults;
mod in_turn;
mod not_equal;

use super::recognition::StdItem;
use super::recursive_fns::Kind;
use super::representation::Num;
use super::{FnCx, R, Shape};
use crate::collections::IndexMap;
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::temporary_names::{Base, Role};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// How JS compares a value as Rust's `==` does (`FnCx::js_eq`): by `===`
/// only, a float's; or by SameValueZero too, `includes`' and a `Map`'s.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum JsEq {
    Strict,
    SameValue,
}

/// Whether `ty` is a type of the crate's whose `PartialEq` is its address's,
/// `fn eq(&self, other: &Self) -> bool { ptr::eq(self, other) }`, and whose
/// `Hash` is too, `ptr::hash(self, state)`, its `ne` std's: equal only to
/// itself, as JS's `===` of an object is (ADR 0864). Of a value JS holds as
/// an object: a struct of fields or a JS object's binding. The one answer
/// a set's and a map's keys ask, and a library's contract lists.
pub(super) fn equal_only_to_itself<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    bodies: &IndexMap<DefId, &super::Body<'tcx>>,
    ty: Ty<'tcx>,
) -> bool {
    identity(recognition, bodies, ty, true)
}

/// Whether `==` of `ty` is its address's, `equal_only_to_itself`'s
/// `PartialEq` whatever its `Hash`, which a set and a map alone ask: `==`
/// of two is `===`, and its `PartialEq`'s dictionary is written only where
/// it's reached (ADR 0864, amended). The one answer `==` and the pipeline
/// ask.
pub(super) fn eq_by_address<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    bodies: &IndexMap<DefId, &super::Body<'tcx>>,
    ty: Ty<'tcx>,
) -> bool {
    identity(recognition, bodies, ty, false)
}

fn identity<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    bodies: &IndexMap<DefId, &super::Body<'tcx>>,
    ty: Ty<'tcx>,
    hashed: bool,
) -> bool {
    // An `Rc` or `Arc` of one, whose `==` and `Hash` are what it points
    // at's: its JS value, what it points at or the `{ value, strong, weak }`
    // every clone of it shares (ADR 0320), is one allocation's.
    if let Some(pointee) = super::recognition::rc_pointee(recognition.tcx, ty) {
        return identity(recognition, bodies, pointee, hashed);
    }
    let ty::Adt(adt, _) = ty.kind() else { return false };
    // A library's, as its contract says; or a binding crate's class's, its
    // `eq` the binding of JS's `===`, which no contract lists (ADR 1016).
    if !adt.did().is_local() {
        return match hashed {
            true => recognition.foreign.equal_only_to_itself(adt.did()),
            false => recognition.foreign.eq_by_address(adt.did()) || recognition.eq_bound_to_strict_equality(adt.did()),
        };
    }
    if !adt.is_struct() {
        return false;
    }
    let calls = |id: DefId, known: super::recognition::Std| {
        (bodies.get(&id))
            .and_then(|body| super::body_queries::forwards_to(&body.thir, 0))
            .is_some_and(|(callee, args)| recognition.classify(callee, args) == Some(known))
    };
    if !hashed {
        return (recognition.own_eq(adt.did())).is_some_and(|eq| calls(eq, super::recognition::Std::PtrEq));
    }
    let Some((eq, hash)) = recognition.eq_and_hash(adt.did()) else {
        return false;
    };
    calls(eq, super::recognition::Std::PtrEq)
        && calls(
            hash,
            super::recognition::Std::Hash(super::std_types::hash::HashOp::Address),
        )
}
impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// std types whose fields are what JS has, whose derived `Clone` and
    /// `PartialEq` are their fields': the enums `Option`, `Result`,
    /// `Ordering`, and a C string's errors (ADR 1119).
    fn is_known_std(&self, ty: Ty<'tcx>) -> bool {
        self.is_lang_adt(ty, LangItem::Option)
            || self.is_std_type(ty, StdItem::Result)
            || self.is_lang_adt(ty, LangItem::OrderingEnum)
            // Its `PartialEq` and `Clone` are its field's; its order isn't
            // (`cmp_value`).
            || self.is_reverse(ty)
            // Its `PartialEq`, `Clone` and order are its number's (ADR 0175).
            || self.recognition().int_newtype(ty).is_some()
            // Each is made in its fields' shape, `NulError(position,
            // bytes)`'s array, `IntoStringError { inner, error }`'s object.
            || super::recognition::is_c_str_error_part(self.tcx, ty)
    }

    /// Another of std's enums whose `PartialEq` is derived, `Bound`'s and a
    /// path's `Component`'s: the same variant, its fields equal, as any
    /// enum's (ADR 0033), as its derived `{:?}` is shown.
    pub(in crate::lower) fn std_enum_derives_eq(&self, ty: Ty<'tcx>) -> bool {
        self.std_enum_derives(self.tcx.lang_items().eq_trait(), ty)
    }

    /// And whose `Clone` is derived, `Bound`'s, quickcheck's (#1420): the
    /// same variant, its fields cloned, as any enum's (ADR 0033).
    pub(in crate::lower) fn std_enum_derives_clone(&self, ty: Ty<'tcx>) -> bool {
        self.std_enum_derives(self.tcx.lang_items().clone_trait(), ty)
    }

    /// Whether `ty` is one of std's enums, made as any enum is (ADR 0033),
    /// whose impl of `derived` is std's derive.
    fn std_enum_derives(&self, derived: Option<DefId>, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if adt.is_enum()
            && !adt.did().is_local()
            && !self.krate.foreign.in_library(adt.did()))
            && derived.is_some_and(|derived| self.recognition().derives(derived, ty))
    }

    pub(super) fn partial_eq_trait(&self) -> DefId {
        self.tcx.require_lang_item(LangItem::PartialEq, rustc_span::DUMMY_SP)
    }

    /// How JS's own comparison of a `ty` agrees with Rust's `==`, if it
    /// does: of a JS primitive, a string, a number, a `bool`, `()`, a
    /// fieldless enum of no `eq` of the crate's, `===` does; and
    /// SameValueZero, `includes`' and a `Map`'s, does too but of a float,
    /// whose `NaN` it finds where `==` doesn't. The one answer, which `==`,
    /// `contains` and a map's keys ask.
    pub(super) fn js_eq(&self, ty: Ty<'tcx>) -> Option<JsEq> {
        // A `&mut` to a number is a cell, an object (ADR 0099).
        if self.has_cell_layer(ty) {
            return None;
        }
        let ty = ty.peel_refs();
        // A parse error is its message, which tells its kind, as its derived
        // `==` compares (ADR 0063); a `TypeId` its type's key (ADR 0331).
        // A value of any shape, `js::Unknown`, by JS's `===` (ADR 0612).
        let any = self.any_shape(ty);
        let primitive = any
            // The registry's symbols, the same where their keys are (ADR 0634).
            || self.recognition().is_symbol_for(ty)
            || self.is_string_like(ty)
            || self.recognition().is_os_text(ty)
            || self.is_parse_error(ty)
            || self.recognition().is_type_id(ty)
            || self.recognition().is_discriminant(ty)
            // A file's type, the name of its kind, as std's mode bits are its
            // kind's (#1397).
            || crate::lower::recognition::fs_type(self.tcx, ty) == Some("FileType")
            // A function pointer, by identity, as Rust's by address (ADR 0262).
            || matches!(ty.kind(), ty::FnPtr(..))
            // One whose `==` is its address's, `ptr::eq`, and whose `Hash`
            // its address's, `ptr::hash`: JS's `===` of the object, which
            // a `Set` keys by (ADR 0864).
            || self.equal_only_to_itself(ty)
            || Num::of(ty).is_some()
            || ty.is_bool()
            || ty.is_unit()
            || matches!(ty.kind(), ty::Adt(adt, _)
                if (super::representation::is_string_enum(self.tcx, *adt) || super::bindings::numbered(self.tcx, *adt))
                    && !self.has_user_impl(self.partial_eq_trait(), ty));
        if primitive {
            return Some(match Num::of(ty).is_some_and(Num::float) || any {
                true => JsEq::Strict,
                false => JsEq::SameValue,
            });
        }
        self.untagged_js_eq(ty)
    }

    /// Of an untagged enum whose `==` is derived, how JS's own comparison
    /// agrees with it, if it does (ADR 0214, amended): its value is its
    /// variant's payload, or a unit variant's literal, `null` or
    /// `undefined`, each of a kind no other variant's is, as its
    /// declaration's check holds of one a `==` tells apart; so two are one
    /// variant where JS's comparison says they're alike, and equal where
    /// their payloads are, where each payload's own comparison agrees with
    /// its `==`. `Strict` where one's does only so, a float's.
    fn untagged_js_eq(&self, ty: Ty<'tcx>) -> Option<JsEq> {
        let (adt, args) = self.untagged(ty)?;
        if self.has_user_impl(self.partial_eq_trait(), ty) {
            return None;
        }
        let mut eq = JsEq::SameValue;
        for variant in adt.variants() {
            match variant.fields.raw.as_slice() {
                [] => {}
                [field] => {
                    let payload = self.field_ty(field, args);
                    // One of its own type, through a reference: compared as
                    // its fields are.
                    if payload.peel_refs() == ty {
                        return None;
                    }
                    if self.js_eq(payload)? == JsEq::Strict {
                        eq = JsEq::Strict;
                    }
                }
                _ => return None,
            }
        }
        Some(eq)
    }

    /// Whether `ty` is equal only to itself (ADR 0864), as
    /// `equal_only_to_itself` answers.
    pub(super) fn equal_only_to_itself(&self, ty: Ty<'tcx>) -> bool {
        equal_only_to_itself(&self.recognition(), self.krate.bodies, ty)
    }

    /// Whether `==` of `ty`, through its references, is `===` (ADR 0864,
    /// amended), as `eq_by_address` answers.
    pub(super) fn eq_by_address(&self, ty: Ty<'tcx>) -> bool {
        !self.has_cell_layer(ty) && eq_by_address(&self.recognition(), self.krate.bodies, ty.peel_refs())
    }

    /// Whether `ty`, through its references, is a JS value of any shape,
    /// `js::Unknown` (`declarations::any_shape`), which `===` compares.
    fn any_shape(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if super::declarations::any_shape(self.tcx, adt.did()))
    }

    /// Of a `ty` that `===` compares as `==` does (`js_eq`), whether it
    /// may be an object, which `===` compares by itself and `$eq` by what's
    /// in it: a value of any shape (ADR 0612, amended), one equal only to
    /// itself, a library's too, and an `Rc` of one (ADR 0864), and an
    /// untagged enum with a payload of one, which is its value (ADR 0214).
    pub(in crate::lower) fn eq_of_objects(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        let of_one = |(adt, args): (ty::AdtDef<'tcx>, ty::GenericArgsRef<'tcx>)| {
            (adt.variants().iter().flat_map(|variant| variant.fields.iter()))
                .any(|field| self.eq_of_objects(self.field_ty(field, args)))
        };
        self.any_shape(ty) || self.equal_only_to_itself(ty) || self.untagged(ty).is_some_and(of_one)
    }

    /// Does `==` on `ty` run code of its own anywhere in it: a hand-written
    /// `eq`, or a `T`'s, which might be one? If not, it compares field by
    /// field, element by element, which `$eq` does.
    pub(super) fn custom_eq(&self, ty: Ty<'tcx>) -> bool {
        self.custom_eq_in(ty, &mut Vec::new())
    }

    fn custom_eq_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) {
            return false;
        }
        // What `===` compares as `==` does: custom where it's an object,
        // which `===` compares by itself and `$eq` by what's in it (ADRs
        // 0612, amended, and 0864, amended).
        if self.js_eq(ty).is_some() {
            return self.eq_of_objects(ty);
        }
        seen.push(ty);
        let custom = match ty.kind() {
            _ if self.is_unknown(ty) => true,
            ty::Tuple(tys) => tys.iter().any(|t| self.custom_eq_in(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.custom_eq_in(*item, seen),
            ty::Adt(..) if self.has_user_impl(self.partial_eq_trait(), ty) => true,
            // A counted `Rc` compares what it points at, not its counts (ADR 0320).
            ty::Adt(..) if self.counted_rc(ty).is_some() => true,
            // A `Cow` compares what it borrows, not its variant (ADR 0400).
            ty::Adt(..) if self.recognition().is_cow(ty) => true,
            // A path compares its components, `$pathEq`, not its text, which
            // `$eq` would (#1467).
            ty::Adt(..) if self.recognition().compares_by_components(ty) => true,
            // A raw pointer, by its handle or a `dyn`'s pair, not what it
            // points to (ADRs 0407, 0562).
            _ if self.compared_by_address(ty) => true,
            // `Vec`, `Box`, `Rc` and cells compare what they hold, and every
            // std type, its type's arguments, not its private fields: a
            // `BTreeMap`'s `NonNull` nodes are no pointer the program compares.
            ty::Adt(adt, args) if self.is_std_wrapper(ty) || self.recognition().is_std(adt.did()) => {
                args.types().any(|t| self.custom_eq_in(t, seen))
            }
            ty::Adt(adt, args) => adt
                .all_fields()
                .any(|f| self.custom_eq_in(self.field_ty(f, args), seen)),
            _ => false,
        };
        seen.pop();
        custom
    }

    /// `a == b` for values of type `ty`: `a === b` for JS primitives, a call
    /// of a hand-written `eq`, `TPartialEq.eq(a, b)` in generic code, and
    /// `$eq(a, b)` for what compares field by field. A derived `==` of a
    /// type with a custom part compares its parts one by one.
    /// `a == b` of an `a` of `ty` and a `b` of `rhs`, `PartialEq<Rhs>`'s: of
    /// items of two types, `[A::Item] == [B::Item]`, each by its own; of two
    /// types a bound compares, `A::Item: PartialEq<B::Item>`, its
    /// dictionary's; else `eq_value`'s, of `ty`.
    pub(super) fn eq_values(
        &mut self,
        a: Expr,
        b: Expr,
        (ty, rhs): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (x, lhs) = self.through_refs(a.clone(), ty);
        let (y, other) = self.through_refs(b.clone(), rhs);
        if lhs != other {
            let items = |ty: Ty<'tcx>| match ty.kind() {
                ty::Array(item, _) | ty::Slice(item) => Some(*item),
                ty::Adt(_, args) if self.is_vec_like(ty) => Some(args.type_at(0)),
                _ => None,
            };
            if let (Some(item), Some(other_item)) = (items(lhs), items(other))
                && item != other_item
            {
                return self.eq_items(x, y, (item, other_item), true, span, out);
            }
            let tr = ty::TraitRef::new(self.tcx, self.partial_eq_trait(), [lhs, other]);
            if (lhs.has_param() || other.has_param())
                && let Some(dictionary) = self.evidence_for(tr)
            {
                return Ok(Expr::call(Expr::member(dictionary, "eq"), vec![x, y]));
            }
            // The crate's own `PartialEq<Other>`, `Cm == Mm`'s: its `eq`.
            if self.is_user_impl(tr) {
                let eq = self.tcx.associated_item_def_ids(self.partial_eq_trait())[0];
                return self.impl_call(eq, tr.args, vec![x, y], span);
            }
        }
        self.eq_value(a, b, ty, span, out)
    }

    /// `items.contains(&x)` of `item`s: JS's `includes` for what `===`
    /// compares (ADR 0053); `indexOf(x) >= 0` for what `===` compares but
    /// `NaN`, a float or a value of any shape, which `indexOf` never finds,
    /// as Rust's `==` doesn't, where `includes` would (ADR 0966); and `==`
    /// item by item for the rest.
    pub(super) fn contained(
        &mut self,
        items: Expr,
        x: Expr,
        item: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if self.eq_is_identity(item) {
            return Ok(Expr::builtin(items, "includes", vec![x]));
        }
        let x = match x.reads_same() {
            true => x,
            false => self.spill(Role::Item, x, out),
        };
        let mut body = Vec::new();
        let same = self.eq_value(Expr::var("each"), x.clone(), item, span, &mut body)?;
        let strict = matches!(&same.kind, js::ExprKind::Binary(Op::Eq, each, of)
            if matches!(&each.kind, js::ExprKind::Var(v) if v == "each") && **of == x);
        // Of what's never `NaN`, an object equal only to itself (ADR
        // 1016), `includes`, which finds what `indexOf` does.
        if body.is_empty() && strict {
            return Ok(match self.js_eq(item) {
                None => Expr::builtin(items, "includes", vec![x]),
                Some(_) => Expr::bin(Op::Ge, Expr::builtin(items, "indexOf", vec![x]), Expr::int(0)),
            });
        }
        body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
        let f = Expr::arrow(vec!["each".into()], body);
        Ok(Expr::call(Expr::member(items, "some"), vec![f]))
    }

    pub(super) fn eq_value(&mut self, a: Expr, b: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let (mut a, _) = self.through_refs(a, ty);
        let (mut b, mut ty) = self.through_refs(b, ty);
        // An `Rc`'s or an `Arc`'s of an `Eq` value: the same allocation
        // first, `Rc::ptr_eq`'s `===` (ADR 0582), as std's `RcEqIdent`
        // specializes it, then its value's, where that runs the crate's
        // code (#910). Of a value whose `Eq` only the caller knows, where
        // the caller's type says, `TEq` (#931).
        if let Some(pointee) = super::recognition::rc_pointee(self.tcx, ty)
            && self.custom_eq(pointee)
            && !self.eq_is_identity(pointee)
            && let Some(first) = self.rc_eq_first(ty, pointee, span)?
        {
            let counted = self.counted_rc(ty).is_some();
            let a = if a.reads_same() {
                a
            } else {
                self.spill(Role::Left, a, out)
            };
            let b = if b.reads_same() {
                b
            } else {
                self.spill(Role::Right, b, out)
            };
            let (x, y) = match counted {
                true => (
                    self.counted_pointee(a.clone(), pointee),
                    self.counted_pointee(b.clone(), pointee),
                ),
                false => (a.clone(), b.clone()),
            };
            let eq = self.eq_value(x, y, pointee, span, out)?;
            let same = match first {
                Some(asked) => Expr::bin(Op::And, asked, Expr::bin(Op::Eq, a, b)),
                None => Expr::bin(Op::Eq, a, b),
            };
            return Ok(Expr::bin(Op::Or, same, eq));
        }
        // A `Box`, an `Rc` and a `Pin` compare what they point at, which
        // they are in JS (ADRs 0023, 0329), its own `==` too, and a `Cow`
        // what it borrows.
        loop {
            match ty.kind() {
                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) && self.counted_rc(ty).is_none() => {
                    ty = args.type_at(0);
                }
                // A `Pin`, its target (`pin_target`, #1446).
                ty::Adt(..) if self.recognition().pinned(ty).is_some() => {
                    let pin = ty;
                    (a, ty) = self.pin_target(a, pin, span, out)?;
                    (b, _) = self.pin_target(b, pin, span, out)?;
                }
                // A `Cow` what it borrows (ADR 0400).
                ty::Adt(..) if let Some((x, borrowed)) = self.cow_deref(&a, ty) => {
                    (a, b, ty) = (x, self.cow_deref(&b, ty).expect("a `Cow`").0, borrowed);
                }
                // A wrapper of std's that's what it holds, a `ManuallyDrop`, or
                // a transparent one that derives `==`: what it holds, which it
                // is in JS (ADR 0584).
                ty::Adt(..) if let Some(held) = self.recognition().std_wrapped(ty) => ty = held,
                ty::Adt(..)
                    if let Some(held) = self.transparent(ty)
                        && self.recognition().derives(self.partial_eq_trait(), ty) =>
                {
                    ty = held;
                }
                _ => break,
            }
        }
        // A raw pointer: the same place, or the same address (ADR 0562).
        if self.compared_by_address(ty)
            && let Some((pointee, _)) = super::pointers::raw_pointee(self.tcx, ty)
        {
            return self.pointers_equal(a, b, pointee, span);
        }
        // A path's components, `a/b` and `a//b` alike (ADR 0401).
        if self.recognition().compares_by_components(ty) {
            return Ok(Expr::call(Expr::helper("$pathEq"), vec![a, b]));
        }
        // A registered symbol's key and text: the symbol and the text's,
        // `type === REACT_MEMO_TYPE` of `Symbol.keyFor(type) === "react.memo"`,
        // as a pattern of one tests it, which holds of any value (ADR 0634).
        if ty.is_str() {
            for (key, text) in [(&a, &b), (&b, &a)] {
                if let Some(symbol) = super::representation::symbol_of_key(key) {
                    let named = self.named_symbol(text, span)?;
                    return Ok(Expr::bin(Op::Eq, symbol.clone(), named));
                }
            }
            // A text a pattern names by a `const`, `Some(INITIALIZED) =>`:
            // its literal, named once tidy's done (ADR 0590, amended).
            if let Some(named) = self.named_text(&b, span)? {
                return Ok(Expr::bin(Op::Eq, a, named));
            }
        }
        // One whose `==` is its address's, its `Hash` whatever it is: the
        // object's `===` (ADR 0864, amended).
        if self.js_eq(ty).is_some() || self.eq_by_address(ty) {
            return Ok(self.strict_eq(ty, a, b));
        }
        // A type of a type parameter whose `PartialEq` a bound gives, a
        // method's `where Self: PartialEq` of a `Wrapping<T>`: its dictionary's,
        // as its parts' needn't be.
        if ty.has_param() && !self.is_unknown(ty) {
            let tr = ty::TraitRef::new_from_args(
                self.tcx,
                self.partial_eq_trait(),
                self.args_of(self.partial_eq_trait(), ty),
            );
            if let Some(dictionary) = self.evidence_for(tr) {
                return Ok(Expr::call(Expr::member(dictionary, "eq"), vec![a, b]));
            }
        }
        if self.is_unknown(ty) {
            let tr = ty::TraitRef::new_from_args(
                self.tcx,
                self.partial_eq_trait(),
                self.args_of(self.partial_eq_trait(), ty),
            );
            // Of a `T: Ord` or `T: PartialOrd` alone, its `PartialEq`, a
            // supertrait's, `TOrd.PartialOrd().PartialEq()`: never `cmp`,
            // which Rust doesn't tie to it (#944).
            if let Some(dictionary) = self.evidence_for(tr) {
                return Ok(Expr::call(Expr::member(dictionary, "eq"), vec![a, b]));
            }
            return Err(self.no_evidence(span, tr));
        }
        if self.has_user_impl(self.partial_eq_trait(), ty) {
            let eq = self.tcx.associated_item_def_ids(self.partial_eq_trait())[0];
            let args = self.args_of(self.partial_eq_trait(), ty);
            return self.impl_call(eq, args, vec![a, b], span);
        }
        // A tagged enum's fieldless variant, `{ kind: "On" }` (ADR 0284):
        // its tag, as the derive compares its variants.
        if let ty::Adt(adt, _) = ty.kind()
            && super::representation::is_fieldless_enum(*adt)
            && let Some(key) = super::bindings::declared_tag(self.tcx, adt.did())
        {
            return Ok(Expr::bin(Op::Eq, Expr::member(a, &key), Expr::member(b, &key)));
        }
        // A constant, like a fieldless variant's `"Nothing"`, is a JS
        // primitive: only itself is equal to it. `None` is also `null`; an
        // untagged enum's `undefined` isn't its `null` (ADR 0676).
        if a.is_constant() || b.is_constant() {
            let nullish =
                matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if super::untagged::holds_nullish(self.tcx, *adt));
            let none = !nullish && [&a, &b].iter().any(|x| matches!(x.kind, js::ExprKind::Undefined));
            let op = if none { Op::LooseEq } else { Op::Eq };
            return Ok(Expr::bin(op, a, b));
        }
        if let Some(inner) = self.option_of(ty) {
            // `==`, so that `null` and `undefined` are both `None`; of a
            // `Some` that's its payload, not one boxed, an object of its own
            // (ADR 0051), which compares by what's in it.
            if (self.js_eq(inner).is_some() || self.eq_by_address(inner)) && !self.boxed_payload(inner) {
                // Of a value of any shape, `(a ?? null) === (b ?? null)`:
                // two `None`s are equal, whichever of `null` and `undefined`
                // each is, and two `Some`s by `===`, as `==` can't, which
                // makes `"1"` of `1` (ADR 0612, amended).
                // Of an untagged enum's, too, whose `==` would make `"7"` of
                // a number's `7` (ADR 0214, amended).
                // Of two whose `None` is one, a getter's `T | null` each, `===` (rd117).
                return Ok(match self.any_shape(inner) || self.untagged(inner).is_some() {
                    true => js::nullish_equal(a, b),
                    false if a.facts.same_none(b.facts) => Expr::bin(Op::Eq, a, b),
                    false => Expr::bin(Op::LooseEq, a, b),
                });
            }
        }
        if self.range_kind(ty).is_some() {
            return self.range_eq(a, b, ty, span, out);
        }
        // Two maps, or two sets (ADR 0059): as many of each, and each key of
        // one the other's, of an equal value: `$mapEq(a, b, (x, y) => ..)`,
        // `$setEq(a, b)`.
        if self.is_map(ty)
            && let ty::Adt(_, args) = ty.kind()
        {
            if self.is_set(ty) {
                return Ok(Expr::call_helper("$setEq", vec![a, b]));
            }
            let (x, y) = (self.fresh(Role::X), self.fresh(Role::Y));
            let mut made = Vec::new();
            let same = self.eq_value(Expr::var(&x), Expr::var(&y), args.type_at(1), span, &mut made)?;
            let is = |e: &Expr, name: &str| matches!(&e.kind, js::ExprKind::Var(v) if v == name);
            let mut list = vec![a, b];
            match &same.kind {
                // `===`, which it compares by where it's given none.
                js::ExprKind::Binary(Op::Eq, l, r) if made.is_empty() && is(l, &x) && is(r, &y) => {}
                // A helper of the two, `$eq`: the helper.
                js::ExprKind::Call(callee, given)
                    if made.is_empty()
                        && callee.helper_name().is_some()
                        && matches!(&given[..], [l, r] if is(l, &x) && is(r, &y)) =>
                {
                    list.push((**callee).clone());
                }
                _ => {
                    made.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
                    list.push(Expr::arrow(vec![x.into(), y.into()], made));
                }
            }
            return Ok(Expr::call_helper("$mapEq", list));
        }
        let structural = matches!(ty.kind(), ty::Tuple(_) | ty::Array(..) | ty::Slice(_))
            || self.is_std_wrapper(ty)
            || matches!(ty.kind(), ty::Adt(adt, _) if !self.is_std(adt.did()) || self.is_known_std(ty))
            || self.std_enum_derives_eq(ty);
        if !structural || self.is_js_object(ty) || self.is_lang_adt(ty, LangItem::String) {
            return Err(self.unsupported(span, &format!("`==` on `{ty}`")));
        }
        if !self.custom_eq(ty) {
            // Two cells are each borrowed to compare; a value isn't, though
            // it's the object a `RefCell` is, read through its guard (ADR
            // 0863).
            let mut given = vec![a, b];
            if self.recognition().is_borrowed_cell(ty) {
                given.push(Expr::bool(true));
            }
            return Ok(Expr::call(Expr::helper("$eq"), given));
        }
        // A type inside itself, `Hir` in `HirKind::Capture`'s `Box<Hir>`: its
        // `==` is a function, which calls itself for the ones inside, as its
        // clone and its order are. Expanded in place, it never ended (#511).
        // One for each type and module (`recursive_fns`, #517).
        if self.is_recursive(ty) {
            let ty::Adt(adt, _) = ty.kind() else {
                unreachable!("a recursive type is an ADT");
            };
            let base = format!("eq{}", self.tcx.item_name(adt.did()));
            let name = self.recursive_fn(
                (Kind::Eq, ty, &base),
                &[Base::Role(Role::A), Base::Role(Role::B)],
                |cx, params, body| cx.eq_derived(params[0].clone(), params[1].clone(), ty, span, body),
                out,
            )?;
            return Ok(Expr::call(Expr::var(&name), vec![a, b]));
        }
        self.eq_derived(a, b, ty, span, out)
    }

    /// `eq_value`'s of a `ty` with a custom part, the derive's, of what it's
    /// made of in turn.
    fn eq_derived(&mut self, a: Expr, b: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        // Each is read more than once below.
        let a = if a.reads_same() {
            a
        } else {
            self.spill(Role::Left, a, out)
        };
        let b = if b.reads_same() {
            b
        } else {
            self.spill(Role::Right, b, out)
        };
        let std = |item: StdItem| self.is_std_type(ty, item);
        match ty.kind() {
            ty::Array(item, _) | ty::Slice(item) => self.eq_items(a, b, (*item, *item), true, span, out),
            ty::Adt(_, args) if self.is_vec_like(ty) => {
                // A list's `==` is its iterators', `Iterator::eq`, which asks
                // `==`; the rest's are slices', which ask `!=` (#962).
                let by_ne = !std(StdItem::LinkedList);
                self.eq_items(a, b, (args.type_at(0), args.type_at(0)), by_ne, span, out)
            }
            ty::Adt(_, args) if let Some(pointee) = self.counted_rc(ty) => {
                let (a, b) = (self.counted_pointee(a, pointee), self.counted_pointee(b, pointee));
                self.eq_value(a, b, args.type_at(0), span, out)
            }
            // Two `RefCell`s', each while it's borrowed (ADR 0328), held where
            // their values' `==` may ask; of one that's the object it holds,
            // the object (ADR 0863).
            ty::Adt(_, args) if std(StdItem::RefCell) && !self.recognition().std_alone(args.type_at(0)) => {
                let itself = self.is_value_cell(ty);
                let flagged = |mut given: Vec<Expr>| {
                    if itself {
                        given.push(Expr::bool(true));
                    }
                    given
                };
                let (x, y) = (self.fresh(Role::X), self.fresh(Role::Y));
                let mut body = Vec::new();
                let eq = self.eq_value(Expr::var(&x), Expr::var(&y), args.type_at(0), span, &mut body)?;
                body.push(StmtKind::Return(Some(eq)).at(js::Span::NONE));
                let inner = Expr::call(
                    Expr::helper("$withBorrow"),
                    flagged(vec![b, Expr::arrow(vec![y.into()], body)]),
                );
                Ok(Expr::call(
                    Expr::helper("$withBorrow"),
                    flagged(vec![
                        a,
                        Expr::arrow(vec![x.into()], vec![StmtKind::Return(Some(inner)).at(js::Span::NONE)]),
                    ]),
                ))
            }
            ty::Adt(_, args) if std(StdItem::RefCell) && self.is_value_cell(ty) => {
                let (a, b) = match self.quiet_ref_cell(ty, false) {
                    true => (a, b),
                    false => (
                        Expr::call(Expr::helper("$borrow"), vec![a]),
                        Expr::call(Expr::helper("$borrow"), vec![b]),
                    ),
                };
                self.eq_value(a, b, args.type_at(0), span, out)
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) => {
                let (a, b) = match std(StdItem::RefCell) {
                    true => (
                        Expr::call(Expr::helper("$borrow"), vec![a]),
                        Expr::call(Expr::helper("$borrow"), vec![b]),
                    ),
                    false => (a, b),
                };
                self.eq_value(Expr::held(a), Expr::held(b), args.type_at(0), span, out)
            }
            ty::Adt(_, args) if std(StdItem::OnceCell) => self.eq_value(
                Expr::held(a),
                Expr::held(b),
                Ty::new_option(self.tcx, args.type_at(0)),
                span,
                out,
            ),
            ty::Adt(_, args) if self.option_of(ty).is_some() => {
                let inner = args.type_at(0);
                let (x, y) = if self.boxed_payload(inner) {
                    (self.some_value(a.clone()), self.some_value(b.clone()))
                } else {
                    (a.clone(), b.clone())
                };
                let some = self.eq_value(x, y, inner, span, out)?;
                let none = Expr::bin(
                    Op::Or,
                    Expr::bin(Op::LooseEq, a.clone(), Expr::null()),
                    Expr::bin(Op::LooseEq, b.clone(), Expr::null()),
                );
                Ok(Expr::cond(none, Expr::bin(Op::LooseEq, a, b), some))
            }
            // An untagged enum's variant is its payload (ADR 0214): the same
            // variant, told by its kind, and its payloads equal.
            ty::Adt(adt, args) if self.untagged(ty).is_some() => {
                let mut value = Expr::bool(false);
                for variant in adt.variants().iter().rev() {
                    // One without fields, its name's string, is `b`'s the same.
                    let Some(field) = variant.fields.iter().next() else {
                        let both = self.untagged_variant_test(ty, &[variant], &[], &b, false);
                        value = Expr::cond(self.untagged_variant_test(ty, &[variant], &[], &a, false), both, value);
                        continue;
                    };
                    let payload = self.field_ty(field, args);
                    let same = self.eq_value(a.clone(), b.clone(), payload, span, out)?;
                    let both = Expr::bin(
                        Op::And,
                        self.untagged_variant_test(ty, &[variant], &[], &b, false),
                        same,
                    );
                    value = Expr::cond(self.untagged_variant_test(ty, &[variant], &[], &a, false), both, value);
                }
                Ok(value)
            }
            ty::Adt(adt, args) if adt.is_enum() => {
                // `{ TAG: "Line", _0: .. }` (ADR 0033): a variant with a custom
                // part compares its fields, and `$eq` the rest.
                let mut value = Expr::call(Expr::helper("$eq"), vec![a.clone(), b.clone()]);
                for variant in adt.variants().iter().rev() {
                    let fields = self.variant_fields(variant, args);
                    if !fields.iter().any(|&(_, t)| self.custom_eq(t)) {
                        continue;
                    }
                    let index = adt.variant_index_with_id(variant.def_id);
                    let tag = |cx: &Self, x: &Expr| cx.variant_test(x.clone(), ty, *adt, index, false);
                    let mut same = tag(self, &b);
                    for ((field, t), def) in fields.into_iter().zip(&variant.fields) {
                        let field = self.eq_value(
                            self.held(Expr::member(a.clone(), field.clone()), Some(def), t),
                            self.held(Expr::member(b.clone(), field), Some(def), t),
                            t,
                            span,
                            out,
                        )?;
                        same = Expr::bin(Op::And, same, field);
                    }
                    value = Expr::cond(tag(self, &a), same, value);
                }
                Ok(value)
            }
            _ => {
                let parts: Vec<(Expr, Expr, Ty<'tcx>)> = match self.shape(ty) {
                    Shape::Object(fields) => Self::named_fields(fields)
                        .into_iter()
                        .enumerate()
                        .map(|(i, (name, t))| {
                            let def = self.struct_field_def(ty, i);
                            let x = self.held(Expr::member(a.clone(), name.clone()), def, t);
                            (x, self.held(Expr::member(b.clone(), name), def, t), t)
                        })
                        .collect(),
                    Shape::Array(tys) => tys
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| {
                            (
                                Expr::index(a.clone(), Expr::int(i as i128)),
                                Expr::index(b.clone(), Expr::int(i as i128)),
                                t,
                            )
                        })
                        .collect(),
                    Shape::Other => return Err(self.unsupported(span, &format!("`==` on `{ty}`"))),
                };
                let mut all: Option<Expr> = None;
                for (x, y, t) in parts {
                    let part = self.eq_value(x, y, t, span, out)?;
                    all = Some(match all {
                        Some(all) => Expr::bin(Op::And, all, part),
                        None => part,
                    });
                }
                Ok(all.unwrap_or_else(|| Expr::bool(true)))
            }
        }
    }

    /// `a.length === b.length && a.every((x, i) => <x == b[i]>)`, of `a`'s
    /// items and `b`'s, each held where it's more than a place: `x` and `i`
    /// named fresh, as `b`, or one inside `<x == b[i]>`, may be named so.
    /// `by_ne` of a slice's, which asks `!(x != b[i])`.
    fn eq_items(
        &mut self,
        a: Expr,
        b: Expr,
        (item, other_item): (Ty<'tcx>, Ty<'tcx>),
        by_ne: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let a = if a.reads_same() {
            a
        } else {
            self.spill(Role::Items, a, out)
        };
        let b = if b.reads_same() {
            b
        } else {
            self.spill(Role::Other, b, out)
        };
        let (x, i) = (self.fresh(Role::X), self.fresh(Role::I));
        let mut body = Vec::new();
        let other = Expr::index(b.clone(), Expr::var(&i));
        // std's slice `==` asks each two `x != y`, until one is: of items
        // whose `!=` is more than `!(x == y)`, theirs (#962), of two item
        // types too, `A: PartialEq<B>`'s (#970).
        let same = match by_ne && self.own_ne_between(item, other_item) {
            true => Expr::unary(
                js::UnaryOp::Not,
                self.ne_values(Expr::var(&x), other, (item, other_item), span, &mut body)?,
            ),
            false => self.eq_values(Expr::var(&x), other, (item, other_item), span, &mut body)?,
        };
        body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
        let every = Expr::call(
            Expr::member(a.clone(), "every"),
            vec![Expr::arrow(vec![x.into(), i.into()], body)],
        );
        let lengths = Expr::bin(Op::Eq, Expr::member(a, "length"), Expr::member(b, "length"));
        Ok(Expr::bin(Op::And, lengths, every))
    }

    /// `(a, b) => <a == b>` for a dictionary's `eq`, of two `ty`s, or `$eq`
    /// itself. A `PartialEq<Rhs>` of another type than itself is its impl's
    /// dictionary, never one made here (`of_other`), so its `Rhs` is `ty`,
    /// but for references, which JS doesn't have.
    pub(super) fn eq_fn(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let mut body = Vec::new();
        let same = self.eq_value(Expr::var("a"), Expr::var("b"), ty, span, &mut body)?;
        Ok(self.compare_fn(same, body))
    }
}
