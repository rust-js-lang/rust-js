//! How each Rust value is represented in JS: its shape, a number's kind and
//! literal, an `Option`'s payload, a cell, what a `&mut` to it is (an
//! object, a box, a handle), a constant's value. When a value is copied is
//! `copies.rs`'s, and what can be represented `support.rs`'s.

use super::bindings::{self, field_key, unit_name};
use super::recognition::{StdItem, cell_value_field, is_std_def};
use super::{FnCx, R, Shape};
use crate::js;
use crate::js::{Expr, Op, Prop, Stmt, StmtKind};
use rustc_abi::VariantIdx;
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{CtorKind, DefKind};
use rustc_middle::mir::ConstValue;
use rustc_middle::mir::interpret::{AllocId, ConstAllocation, GlobalAlloc, GlobalId, Pointer, Scalar, alloc_range};
use rustc_middle::ty;
use rustc_middle::ty::Ty;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalDefId};
use rustc_span::{Span, Symbol};

mod cells;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// That `subject` is one of `variants` of its enum, an or-pattern's,
    /// which MIR sends to one block: each one's test, `||`; of an untagged
    /// enum's, what they are together, `Object | Array`'s `typeof subject
    /// === "object"` (ADR 0668); `present` as `variant_test`'s. Each test
    /// of one is where the first of `variants` it tests is `written`, where
    /// that's given, an or-pattern's alternative: what a comment before it
    /// is before (ADR 0444, amended).
    pub(in crate::lower) fn variants_test(
        &self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variants: &[VariantIdx],
        written: &[js::Span],
        present: bool,
    ) -> Expr {
        if bindings::is_untagged(self.tcx, adt.did()) {
            let variants: Vec<&ty::VariantDef> = variants.iter().map(|&v| adt.variant(v)).collect();
            return self.untagged_variant_test(ty, &variants, written, &subject, present);
        }
        (variants.iter().enumerate())
            .map(|(i, &variant)| {
                let test = self.variant_test(subject.clone(), ty, adt, variant, present);
                match written.get(i) {
                    Some(&span) if test.span.is_none() => Expr { span, ..test },
                    _ => test,
                }
            })
            .reduce(|either, test| Expr::bin(Op::Or, either, test))
            .expect("a variant to test")
    }

    /// That `subject` is `variant` of its enum: the one test of it, what a
    /// pattern, a clone, `==`, a drop and a codec ask. An `Option`'s
    /// presence, an untagged one's kind (ADR 0214), a discriminated union's
    /// `otherwise` (ADR 0284), an `Ordering`'s number, or its tag. An
    /// untagged one `present`, `classify(Some(value))`'s, is never
    /// `undefined` nor `null`, which no test leaves out then (ADR 0676).
    pub(in crate::lower) fn variant_test(
        &self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variant: VariantIdx,
        present: bool,
    ) -> Expr {
        let def = adt.variant(variant);
        if self.tcx.is_lang_item(adt.did(), LangItem::Option) {
            let some = self.tcx.is_lang_item(def.def_id, LangItem::OptionSome);
            return Expr::presence(subject, some);
        }
        // A write's result's `Ok`, `undefined`, and its `Err`, the error it
        // caught (ADRs 0187, 0462).
        if self.is_write_result(ty) {
            let op = if variant.as_u32() == 0 { Op::Eq } else { Op::Ne };
            return Expr::bin(op, subject, Expr::undefined());
        }
        // An untagged enum's variant, told by its value's kind (ADR 0214).
        if bindings::is_untagged(self.tcx, adt.did()) {
            return self.untagged_variant_test(ty, &[def], &[], &subject, present);
        }
        // A discriminated union's `otherwise`: an object whose tag is none
        // of the others' (ADR 0284).
        if bindings::is_tagged_otherwise(self.tcx, adt.did(), def) {
            let key = bindings::tag_key(self.tcx, adt.did());
            let tests = (adt.variants().iter())
                .filter(|other| other.def_id != def.def_id)
                .map(|other| {
                    let name = bindings::variant_tag(self.tcx, other);
                    Expr::bin(Op::Ne, Expr::member(subject.clone(), &key), name)
                })
                .reduce(|a, b| Expr::bin(Op::And, a, b));
            return tests.unwrap_or_else(|| Expr::bool(true));
        }
        if let Some(n) = super::recognition::ordering_value(self.tcx, adt.did(), def.name) {
            return Expr::bin(Op::Eq, subject, Expr::int(n));
        }
        let unit = def.fields.is_empty() && bindings::declared_tag(self.tcx, adt.did()).is_none();
        match unit {
            true => Expr::bin(Op::Eq, subject, self.unit_variant(adt.did(), def)),
            // A unit variant written in place, its name, is no other: a
            // string has no tag.
            false if matches!(subject.kind, js::ExprKind::Str(_)) => Expr::bool(false),
            false => Expr::bin(
                Op::Eq,
                Expr::member(subject, bindings::tag_key(self.tcx, adt.did())),
                bindings::variant_tag(self.tcx, def),
            ),
        }
    }

    /// A variant without fields, as a body writes it: of a numbered enum,
    /// its module's `const`, by its name, `Pending`, imported where it's
    /// another module's or crate's (ADR 0722, amended), where this body
    /// reads it so (`reads_by_name`); else its value,
    /// `bindings::unit_variant`'s. The one answer of each test, value and
    /// constant of one a body writes.
    pub(in crate::lower) fn unit_variant(&self, adt: DefId, variant: &ty::VariantDef) -> Expr {
        match self.is_rust_fn(variant.def_id) && self.reads_by_name(variant.def_id) {
            true => self.fn_ref(variant.def_id),
            false => bindings::unit_variant(self.tcx, adt, variant),
        }
    }

    /// A value rustc computed, as a body writes it: `const_js`'s, each
    /// variant without fields `unit_variant`'s.
    pub(in crate::lower) fn const_value(&self, value: ty::Value<'tcx>) -> Option<Expr> {
        const_js_by(self.tcx, value, &|adt, variant| self.unit_variant(adt, variant))
    }

    /// Which variant `subject` is, of its enum `ty`, as rustc numbers it, its
    /// discriminant (ADR 0393): by its variant's name where its value says
    /// it once (`named_discriminant`), else each variant's test in turn, the
    /// last one's the rest. Of a type that's no enum, `0`, as Rust's is. Each
    /// a BigInt where `big`, a 64-bit `repr`'s as a derived `Hash` reads it.
    pub(in crate::lower) fn discriminant_value(
        &mut self,
        subject: Expr,
        ty: Ty<'tcx>,
        big: bool,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let literal = |n: i128| match big {
            true => Expr::bigint(n),
            false => Expr::int(n),
        };
        let ty::Adt(adt, _) = ty.kind() else {
            if subject.has_effects() {
                out.push(StmtKind::Expr(subject).at(js::Span::NONE));
            }
            return literal(0);
        };
        let adt = *adt;
        if !adt.is_enum() {
            return literal(0);
        }
        let subject = match self.named_discriminant(subject, adt, big) {
            Ok(named) => return named,
            Err(subject) => subject,
        };
        let mut variants: Vec<(VariantIdx, i128)> = (adt.variants().indices())
            .zip(super::discriminants(self.tcx, adt))
            .map(|(variant, (_, n))| (variant, n))
            .collect();
        let Some((_, last)) = variants.pop() else {
            return literal(0);
        };
        let subject = match subject.reads_same() || variants.is_empty() {
            true => subject,
            false => self.spill("value", subject, out),
        };
        variants.into_iter().rev().fold(literal(last), |rest, (variant, n)| {
            Expr::cond(
                self.variant_test(subject.clone(), ty, adt, variant, false),
                literal(n),
                rest,
            )
        })
    }

    /// An enum's discriminant, of `subject`, where its value says its
    /// variant's name once (ADR 0013): a string enum's is the name, a tagged
    /// one's its tag (ADR 0284). `["Red", "Green"].indexOf(color)` when they
    /// count up from 0, else looked up by name; each a BigInt where `big`.
    /// `Err` of `subject` where it isn't so.
    pub(in crate::lower) fn named_discriminant(
        &self,
        subject: Expr,
        adt: ty::AdtDef<'tcx>,
        big: bool,
    ) -> Result<Expr, Expr> {
        let tcx = self.tcx;
        // A numbered enum's value is its discriminant (ADR 0722).
        if bindings::numbered(tcx, adt) {
            return Ok(match big {
                true => super::std_types::number::to_bigint(subject),
                false => subject,
            });
        }
        let name = if is_string_enum(tcx, adt) {
            subject
        } else if is_fieldless_enum(adt)
            && !bindings::is_untagged(tcx, adt.did())
            && let Some(key) = bindings::declared_tag(tcx, adt.did())
            && !adt
                .variants()
                .iter()
                .any(|v| bindings::is_tagged_otherwise(tcx, adt.did(), v))
        {
            Expr::member(subject, &key)
        } else {
            return Err(subject);
        };
        let discriminants = super::discriminants(tcx, adt);
        let counting = discriminants.iter().enumerate().all(|(i, &(_, d))| d == i as i128);
        if counting {
            let names = Expr::array(discriminants.into_iter().map(|(n, _)| Expr::str(n)).collect());
            let index = Expr::call(Expr::member(names, "indexOf"), vec![name]);
            return Ok(if big {
                super::std_types::number::to_bigint(index)
            } else {
                index
            });
        }
        let table = Expr::object(
            discriminants
                .into_iter()
                .map(|(n, d)| Prop::Field(n, if big { Expr::bigint(d) } else { Expr::int(d) }))
                .collect(),
        );
        Ok(Expr::index(table, name))
    }

    /// What an `Option<T>`'s `T` is in JS: through references, `Box` and `Rc`,
    /// which are the value itself (ADR 0023), but for a counted `Rc`, an
    /// object of its own (ADR 0320), and a `&mut` that's a cell, a handle
    /// on its place (ADR 0099), even of a generic `T`.
    fn payload(&self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        loop {
            ty = match ty.kind() {
                _ if self.is_cell(ty) => return ty,
                ty::Ref(_, inner, _) => *inner,
                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) && self.counted_rc(ty).is_none() => args.type_at(0),
                ty::Adt(_, args) if self.recognition().pinned(ty).is_some() => args.type_at(0),
                _ => return ty,
            };
        }
    }

    /// An `Option<T>` whose `T` might look like `None`: a type parameter, or
    /// a `()` or an `Option` itself. `Some` of it is boxed when it does
    /// (ADR 0051).
    pub(super) fn boxed_payload(&self, ty: Ty<'tcx>) -> bool {
        let payload = self.payload(ty);
        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload)) || self.can_be_nullish(ty)
    }

    /// Is `ty` a type parameter its bounds say is never `undefined` or
    /// `null`, so `Some` of one is the value itself? One a binding of its
    /// bound reads, `node.sortIndex` of `T: Node`, as the binding takes on
    /// trust (ADR 0307); or one bound by `FnOnce`, `FnMut` or `Fn`, a JS
    /// function whatever it is (ADR 0051, amended).
    pub(super) fn defined_by_its_bounds(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(_)) && (self.read_by_its_bounds(ty) || self.is_callable(ty))
    }

    fn read_by_its_bounds(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(_))
            && self.typing_env.param_env.caller_bounds().iter().any(|clause| {
                clause.as_trait_clause().is_some_and(|bound| {
                    bound.self_ty().skip_binder() == ty
                        && (self.tcx.associated_items(bound.def_id()).in_definition_order())
                            .any(|item| item.is_method() && bindings::is_binding(self.tcx, item.def_id))
                })
            })
    }

    /// Can a `T` be `undefined` or `null` in JS? Then `Option<T>` can't be
    /// `T` itself: `Some(())` and `None` would be the same value.
    pub(super) fn can_be_nullish(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.payload(ty);
        ty.is_unit()
            || (super::recognition::known_by_bounds(ty) && !self.defined_by_its_bounds(ty))
            || self.option_of(ty).is_some()
            // `undefined` before it's written (ADR 0332).
            || self.recognition().uninit_of(ty).is_some()
            // An uncounted `Weak::new()`, `null` (ADR 0843).
            || self.uncounted_weak(ty).is_some()
            || matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct() && adt.non_enum_variant().fields.is_empty())
            // An untagged enum of `undefined` or `null`, `js::Kind` (ADR 0676).
            || matches!(ty.kind(), ty::Adt(adt, _) if super::untagged::holds_nullish(self.tcx, *adt))
    }

    /// Does a `ty` have one value, as a zero-sized type does: `()`, a unit
    /// struct, `[S; 1]` of one? Then no write to one changes what a program
    /// sees.
    pub(super) fn has_one_value(&self, ty: Ty<'tcx>) -> bool {
        self.layout(ty).is_some_and(|layout| layout.is_zst())
    }

    /// Is a `ty` value a JS object? Then a reference to it, even `&mut`, can
    /// be the object itself: changes through it change the one object (ADR 0025).
    /// A value that a `&mut` to must be a box to change (ADR 0072): one that
    /// isn't a JS object, as a `String`, a number or a fieldless enum is.
    /// Is a `&mut` of a `ty` a handle, `{ value }`, where std gives one to a
    /// closure, `retain_mut`'s, or a guard derefs to one: of a value JS can't
    /// change in place, `is_boxable`, or of a type parameter, which generic
    /// code reads through its `value`, a box's (ADR 0454).
    pub(super) fn handles_mut(&self, ty: Ty<'tcx>) -> bool {
        self.is_boxable(ty) || self.is_unknown(ty)
    }

    pub(super) fn is_boxable(&self, ty: Ty<'tcx>) -> bool {
        // What an `impl Trait` is, a returned closure's the closure.
        let ty = self.reveal(ty);
        // A `Pin` is its pointer (ADR 0329), and a `&mut` to a pinned
        // reference the pin, which only what it points at is changed through.
        if let Some(pointer) = self.recognition().pinned(ty) {
            return !self.mut_is_the_pin(ty) && self.is_boxable(pointer);
        }
        // A `&mut` to a value JS can't change in place is a cell, which can't
        // be changed in place either: a `&mut` to it, a cell of the cell. A
        // `&mut` to a plain object is the object, which a write of the whole
        // through a `&mut` to it, `*r = &mut other`, makes another: a cell
        // too, rust_decimal's `impl AddAssign for &mut Decimal`'s `&mut self`.
        // Not of an iterator or a function, each the JS one it steps or calls
        // (ADRs 0071, 0099), nor of a slice, which may be a view of part of
        // an array, split through a held `&mut` as it is.
        if let ty::Ref(_, inner, Mutability::Mut) = *ty.kind() {
            let plain_object = self.is_object(inner)
                && !inner.is_slice()
                && !self.is_callable(inner)
                && !self.implements_iterator(inner)
                && !self.is_generic_iter(inner);
            return self.is_cell_pointee(inner) || plain_object;
        }
        // A counted `Rc` or its `Weak` is an object its owners share: a `&mut`
        // to one is a cell, so `*r = v` gives the place another (ADR 0523).
        let shared = self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some();
        !self.is_unknown(ty)
            && (!self.is_object(ty) || shared)
            // A `&mut dyn FnMut()` is the function (ADR 0099).
            && !self.is_callable(ty)
            && self.unsupported_part(ty).is_none()
    }

    /// A type parameter a `&mut` is to, `&mut T` or `&mut Self`, where `param_env`
    /// says what it's bound by: a box, whatever it is (ADR 0099), since generic
    /// code is compiled once and `T` may be a number. Not one that's a JS
    /// function or a JS iterator already: a closure, whose `&mut` is the
    /// closure, or an iterator (ADR 0061).
    pub(super) fn is_generic_boxed(&self, ty: Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> bool {
        let typing_env = ty::TypingEnv::new(param_env, ty::TypingMode::non_body_analysis());
        self.recognition().is_unknown_in(ty, typing_env)
            && !self.declared_bounds(ty, param_env).into_iter().any(|id| {
                self.tcx.fn_trait_kind_from_def_id(id).is_some()
                    || is_std_def(self.tcx, id, StdItem::Iterator)
                    || is_std_def(self.tcx, id, StdItem::IntoIterator)
            })
    }

    /// A JS function: a closure, a function, a `dyn Fn` and the like, or a
    /// type parameter bound by one of them, an `impl FnMut` too. A `&mut` to
    /// one is the function (ADR 0099): calling it changes what it captured,
    /// as calling it through the `&mut` does.
    pub(super) fn is_callable(&self, ty: Ty<'tcx>) -> bool {
        let fn_trait = |id: DefId| self.tcx.fn_trait_kind_from_def_id(id).is_some();
        match ty.kind() {
            ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..) => true,
            ty::Dynamic(predicates, ..) => super::recognition::is_fn_dyn(self.tcx, predicates),
            _ if self.is_unknown(ty) => (self.declared_bounds(ty, self.typing_env.param_env).into_iter()).any(fn_trait),
            _ => false,
        }
    }

    /// What a `&mut` to is a cell: a value JS can't change in place, or a
    /// type parameter, which may be one (ADR 0099).
    pub(super) fn is_cell_pointee(&self, ty: Ty<'tcx>) -> bool {
        self.is_boxable(ty) || self.is_generic_boxed(ty, self.typing_env.param_env)
    }

    /// A type that's the value it holds, which it always does: a
    /// `ManuallyDrop`, what `pin!` pins. Not a `MaybeUninit`, which may hold
    /// none yet, a cell to be written (ADR 0332).
    fn always_holds(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        self.transparent(ty)
            .filter(|_| self.recognition().uninit_of(ty).is_none())
    }

    /// Whether what a `&mut` to a `ty` is, isn't a cell, is changed in place
    /// to write it: an object, or an iterator given as it is (ADRs 0147,
    /// 0411). The one answer, of `*r = v`, `$assign`, and of `mem::replace`
    /// and the like, `$take` (ADR 0553).
    /// Not a guard's, whose object is the cell it guards: writing it in
    /// place would write what the cell holds.
    pub(super) fn written_in_place(&self, ty: Ty<'tcx>) -> bool {
        !self.is_cell_pointee(ty) && !self.is_guard(ty) && (self.is_object(ty) || self.iterator_given_as_it_is(ty))
    }

    /// A `&mut` to a value JS can't change in place, as a value: a cell, a box
    /// or a handle, whose `value` is the place it points at (ADR 0099).
    pub(super) fn is_cell(&self, ty: Ty<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Ref(_, inner, Mutability::Mut) if self.is_cell_pointee(inner))
    }

    /// A value of type `ty` seen through its references, as the code that
    /// shows, compares or orders one reads it: a `&` is the value (ADR 0023),
    /// a `&mut` to a value JS can't change in place a cell, whose `value` it
    /// is (ADR 0099), or the place of a `Handle` on one.
    pub(super) fn through_refs(&self, mut value: Expr, mut ty: Ty<'tcx>) -> (Expr, Ty<'tcx>) {
        while let ty::Ref(_, inner, mutability) = *ty.kind() {
            if mutability == Mutability::Mut && self.is_cell_pointee(inner) {
                value = match value.kind {
                    js::ExprKind::Handle(place, _) => *place,
                    _ => Expr::held(value),
                };
            }
            ty = inner;
        }
        (value, ty)
    }

    /// Is one of the references `ty` is behind a `&mut` cell, so that its JS
    /// value is an object, not what it points at (ADR 0099)?
    pub(super) fn has_cell_layer(&self, mut ty: Ty<'tcx>) -> bool {
        while let ty::Ref(_, inner, mutability) = *ty.kind() {
            if mutability == Mutability::Mut && self.is_cell_pointee(inner) {
                return true;
            }
            ty = inner;
        }
        false
    }

    /// Is `fn_id`'s parameter `i` a box: a `&mut` to a value JS can't change in
    /// place (ADR 0074), or to a type parameter (ADR 0099)?
    pub(super) fn param_is_box(&self, fn_id: DefId, i: usize) -> bool {
        let inputs = self
            .tcx
            .fn_sig(fn_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .inputs();
        match inputs.get(i).map(|input| *input.kind()) {
            Some(ty::Ref(_, inner, Mutability::Mut)) => {
                self.is_boxable(inner) || self.is_generic_boxed(inner, self.tcx.param_env(fn_id))
            }
            _ => false,
        }
    }

    pub(super) fn is_object(&self, ty: Ty<'tcx>) -> bool {
        // An `impl Trait` a function returns is the value it stands for: a
        // `&mut` to an `impl Iterator`'s array is the array.
        let ty = self.reveal(ty);
        // What's the value it holds is an object as that value is: a
        // `&mut` to a transparent struct of an array is the array (ADR 0584).
        if let Some(held) = self.always_holds(ty) {
            return self.is_object(held);
        }
        matches!(self.shape(ty), Shape::Object(_) | Shape::Array(_))
            || self.is_js_object(ty)
            // A slice or an array is a JS array: `&mut` to one, as `sort` takes, is it.
            || ty.is_slice()
            // A `dyn Iterator` is a JS iterator, which steps itself.
            || self.recognition().is_dyn_iter(ty)
            || ty.is_array()
            || [StdItem::Cell, StdItem::RefCell, StdItem::Atomic, StdItem::OnceCell, StdItem::LazyCell]
                .into_iter()
                .any(|item| self.is_std_type(ty, item))
            // A counted `Rc`, and its `Weak`, are `{ value, strong, weak }` (ADR 0320).
            || self.counted_rc(ty).is_some()
            || self.counted_weak(ty).is_some()
            // A lock, and a guard, which is its cell (ADR 0328).
            || self.recognition().is_borrowed_cell(ty)
            || self.is_guard(ty)
            || self.is_vec_like(ty)
            || self.is_map(ty)
            // A union, the object of the field it holds (ADR 0406).
            || self.is_union(ty)
            // A `dyn` that's a pair is an object (ADR 0049): a `&mut` to one
            // is the pair, whose `value` its `&mut self` methods write.
            || matches!(ty.kind(), ty::Dynamic(..)) && self.is_pair_dyn(ty)
            // An enum each of whose variants has fields: each is an object
            // (ADR 0033). One with a fieldless variant isn't, the crate's,
            // std's, `Bound`'s `Unbounded`, or serde_json's `Value`'s `Null`
            // (ADR 0083): that variant is a string, which can't become
            // another in place, so `*r = ..` is its place's, as a number's
            // is (ADR 0147). `Option` is its value itself (ADR 0030), not an
            // object.
            || matches!(ty.kind(), ty::Adt(adt, _) if adt.is_enum()
                && !self.tcx.is_lang_item(adt.did(), LangItem::Option)
                // An untagged enum is its payload (ADR 0214), which `*r = v`
                // can't make another's in place: a number isn't an object,
                // an array no plain object, and a class's instance keeps its
                // prototype (#347). A `&mut` to one is a place.
                && !bindings::is_untagged(self.tcx, adt.did())
                && adt.variants().iter().all(|v| !v.fields.is_empty()))
    }

    /// Each variant's payload of `ty`, an untagged enum, `None` where one has
    /// none, its name's string or `undefined` (ADR 0676), or `ty` is no such
    /// enum.
    fn untagged_payloads(&self, ty: Ty<'tcx>) -> Option<Vec<Ty<'tcx>>> {
        let (adt, args) = super::untagged::untagged_adt(self.tcx, self.reveal(ty))?;
        (adt.variants().iter())
            .map(|v| match v.fields.iter().collect::<Vec<_>>()[..] {
                [field] => Some(field.ty(self.tcx, args).skip_normalization()),
                _ => None,
            })
            .collect()
    }

    /// Might a `ty` value be a JS object something changes in place? An
    /// object, or an enum with a fieldless variant: not an object for a `&mut`
    /// (ADR 0147), its variants with fields are objects all the same.
    pub(super) fn may_be_object(&self, ty: Ty<'tcx>) -> bool {
        self.is_object(ty)
            || matches!(self.reveal(ty).kind(), ty::Adt(adt, _) if adt.is_enum()
                && !self.tcx.is_lang_item(adt.did(), LangItem::Option)
                && !bindings::is_untagged(self.tcx, adt.did())
                && adt.variants().iter().any(|v| !v.fields.is_empty()))
            // An untagged enum's, its payload, where one may be an object.
            || self.untagged_payloads(ty).is_some_and(|payloads| payloads.iter().any(|&p| self.may_be_object(p)))
    }

    /// An enum variant's fields as JS properties (ADR 0033): `_0`, `_1` for a
    /// tuple variant, as in ReScript, and their names for a struct variant.
    pub(super) fn variant_fields(
        &self,
        variant: &ty::VariantDef,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Vec<(String, Ty<'tcx>)> {
        variant
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| (variant_field(self.tcx, variant, i), self.field_ty(f, args)))
            .collect()
    }

    /// How a struct or tuple type looks in JS.
    pub(super) fn shape(&self, ty: Ty<'tcx>) -> Shape<'tcx> {
        if let Some(held) = self.always_holds(ty) {
            return self.shape(held);
        }
        // A path is its text, a string (ADR 0173), whatever its fields; and a
        // struct of std's that's a number, a `Duration`'s nanoseconds (ADRs
        // 0177, 0188, 0447), has none in JS.
        if self.is_std_wrapper(ty)
            || self.is_js_object(ty)
            || self.recognition().is_symbol_for(ty)
            || self.recognition().is_path_like(ty)
            || Num::of(ty).is_some()
        {
            return Shape::Other;
        }
        match ty.kind() {
            ty::Tuple(tys) if !tys.is_empty() => Shape::Array(tys.to_vec()),
            ty::Adt(adt, args) if adt.is_struct() => {
                let variant = adt.non_enum_variant();
                let fields = variant
                    .fields
                    .iter()
                    .map(|f| (field_key(self.tcx, f), self.field_ty(f, args)));
                match variant.ctor_kind() {
                    None => Shape::Object(fields.collect()),
                    Some(CtorKind::Fn) => Shape::Array(fields.map(|(_, ty)| ty).collect()),
                    Some(CtorKind::Const) => Shape::Other,
                }
            }
            _ => Shape::Other,
        }
    }

    /// Field `i` of `ty`, a struct, as rustc declares it, which `shape`
    /// lists in its order: what `held` and `holding` ask of a struct's
    /// field. `None` of a tuple's, or an enum's variant's.
    pub(super) fn struct_field_def(&self, ty: Ty<'tcx>, i: usize) -> Option<&'tcx ty::FieldDef> {
        match ty.kind() {
            ty::Adt(adt, _) if adt.is_struct() => adt.non_enum_variant().fields.get(rustc_abi::FieldIdx::from_usize(i)),
            _ => None,
        }
    }

    /// Field `i` of a `ty` value: `base.x`, or `base[0]` for tuples.
    pub(super) fn project(&self, base: Expr, ty: Ty<'tcx>, i: usize) -> Expr {
        // What's the value it holds: that field is it, a marker beside it
        // nothing (ADR 0584).
        if self.always_holds(ty).is_some() {
            let held = match ty.kind() {
                ty::Adt(adt, _) => super::recognition::transparent_field(self.tcx, *adt).unwrap_or(0),
                _ => 0,
            };
            return if i == held { base } else { Expr::undefined() };
        }
        // The registry's symbol's key (ADR 0634).
        if self.recognition().is_symbol_for(ty) {
            return symbol_key(base);
        }
        match (self.shape(ty), &base.kind) {
            // A part of `[a, b]` (a `match (a, b)` subject) is just `a`.
            (Shape::Array(_), js::ExprKind::Array(items)) if !base.has_effects() => items[i].clone(),
            (Shape::Array(_), _) => Expr::index(base, Expr::int(i as i128)),
            // A flattened struct's fields are its parent's: read through it,
            // `props.html.title` is `props.title` (ADR 0205).
            (Shape::Object(_), _) if super::bindings::is_flatten_field(self.tcx, ty, i) => base,
            (Shape::Object(fields), _) => {
                let field = Expr::member(base, fields[i].0.clone());
                let def = self.struct_field_def(ty, i);
                match self.nullable_cell_field(ty, (i, fields[i].1)) {
                    // One there where it's made, which stores `null` for
                    // `None`, so what's read of it is never `undefined`; not
                    // an optional one, absent until it's set (ADRs 0620,
                    // amended, 0810).
                    true if self.null_where_made(ty, i) => self.nullable_cell_handle(field.defined(), def, fields[i].1),
                    true => self.nullable_cell_handle(field, def, fields[i].1),
                    false => self.held(field, def, fields[i].1),
                }
            }
            (Shape::Other, _) => unreachable!("fields of a type without fields"),
        }
    }

    pub(super) fn num(&self, ty: Ty<'tcx>, span: Span) -> R<Num> {
        Num::of(ty).ok_or_else(|| super::refusal::refuse(self.tcx, span, super::refusal::Refused::Type("values", ty)))
    }
}

/// Number representations. Up to 32 bits, `f32` and `f64`, a plain JS number,
/// and `i64` and `u64`, a BigInt (ADR 0086); the difference is how results
/// are wrapped back into range, or for an `f32`, rounded to one (ADR 0122).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Num {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
}

/// An `f32` as JS writes it: its shortest digits, `1.5`, where they're the
/// number exactly, and else rounded to it, `Math.fround(0.1)` (ADR 0122).
pub(super) fn f32_literal(v: f32) -> Expr {
    let shortest: f64 = format!("{v:e}").parse().expect("Rust writes a float it reads");
    if !v.is_finite() || shortest == f64::from(v) {
        return Expr::num(f64::from(v));
    }
    Expr::call(Expr::member(Expr::var("Math"), "fround"), vec![Expr::num(shortest)])
}

impl Num {
    pub(super) fn of(ty: Ty<'_>) -> Option<Num> {
        Some(match ty.kind() {
            ty::Int(ty::IntTy::I8) => Num::I8,
            ty::Int(ty::IntTy::I16) => Num::I16,
            // `isize` and `usize` are 32 bits, as on wasm32 (ADR 0025).
            ty::Int(ty::IntTy::I32 | ty::IntTy::Isize) => Num::I32,
            ty::Int(ty::IntTy::I64) => Num::I64,
            ty::Int(ty::IntTy::I128) => Num::I128,
            ty::Uint(ty::UintTy::U8) => Num::U8,
            ty::Uint(ty::UintTy::U16) => Num::U16,
            ty::Uint(ty::UintTy::U32 | ty::UintTy::Usize) => Num::U32,
            ty::Uint(ty::UintTy::U64) => Num::U64,
            ty::Uint(ty::UintTy::U128) => Num::U128,
            ty::Float(ty::FloatTy::F32) => Num::F32,
            ty::Float(ty::FloatTy::F64) => Num::F64,
            // A `NonZero<T>` is its number, and so is the pattern type it keeps
            // it in, `(u32) is 1..` (ADR 0177).
            ty::Pat(base, _) => return Num::of(*base),
            ty::Adt(adt, args) if super::recognition::is_non_zero(adt.did()) => return Num::of(args.type_at(0)),
            // A `Duration` is its nanoseconds (ADR 0188).
            ty::Adt(adt, _) if super::recognition::is_duration(adt.did()) => Num::U128,
            // So is a clock's time, since a time before it, and a
            // `SystemTimeError` the `Duration` it holds (ADR 0447).
            ty::Adt(adt, _) => match super::recognition::clock_type(adt.did())? {
                super::recognition::ClockType::Error => Num::U128,
                _ => Num::I128,
            },
            _ => return None,
        })
    }

    pub(super) fn bits(self) -> u32 {
        match self {
            Num::I8 | Num::U8 => 8,
            Num::I16 | Num::U16 => 16,
            Num::I32 | Num::U32 | Num::F32 => 32,
            Num::I64 | Num::U64 | Num::F64 => 64,
            Num::I128 | Num::U128 => 128,
        }
    }

    /// An `f32` or an `f64`: a JS number, not an integer (ADR 0122).
    pub(super) fn float(self) -> bool {
        matches!(self, Num::F32 | Num::F64)
    }

    pub(super) fn signed(self) -> bool {
        matches!(self, Num::I8 | Num::I16 | Num::I32 | Num::I64 | Num::I128)
    }

    /// A 64-bit or a 128-bit integer: a JS BigInt, which mixes only with
    /// another (ADRs 0086, 0171).
    pub(super) fn big(self) -> bool {
        matches!(self, Num::I64 | Num::U64 | Num::I128 | Num::U128)
    }

    /// `n` as a literal of this type: `5`, or `5n`. A `u128`'s `n` is its
    /// bits, as an `i128` holds them: `-1` is `u128::MAX`.
    pub(super) fn literal(self, n: i128) -> Expr {
        match self {
            Num::U128 => Expr::biguint(n as u128),
            _ if self.big() => Expr::bigint(n),
            _ => Expr::int(n),
        }
    }

    /// The inclusive value range, for integers: its top a `u128`, which
    /// `u128::MAX` needs.
    pub(super) fn range(self) -> (i128, u128) {
        let bits = self.bits();
        if self.signed() {
            (i128::MIN >> (128 - bits), (i128::MAX >> (128 - bits)) as u128)
        } else {
            (0, u128::MAX >> (128 - bits))
        }
    }

    /// Wrap an exact JS result back into this type's range, like Rust's
    /// wrapping arithmetic: `x | 0` for i32, `x >>> 0` for u32, and so on.
    pub(super) fn wrap(self, e: Expr) -> Expr {
        // A constant is wrapped here, not in the JS: `Code::NotFound as u32`
        // is `404`, not `(404 + 0 | 0) >>> 0`.
        // An `f32` constant is rounded here too.
        if self == Num::F32
            && let js::ExprKind::Num(n) = e.kind
        {
            return f32_literal(n as f32);
        }
        // A 128-bit one is in range already: `const_int` is of `i128`s.
        if self.bits() == 128
            && let Some(n) = const_int(&e)
        {
            return self.literal(n);
        }
        if !self.float()
            && let Some(n) = const_int(&e)
        {
            let size = 1i128 << self.bits();
            let wrapped = n.rem_euclid(size);
            return self.literal(if self.signed() && wrapped >= size / 2 {
                wrapped - size
            } else {
                wrapped
            });
        }
        let bits = Expr::int(i128::from(self.bits()));
        let as_n = |name: &str| Expr::call(Expr::member(Expr::var("BigInt"), name), vec![bits.clone(), e.clone()]);
        match self {
            Num::I64 | Num::I128 => as_n("asIntN"),
            Num::U64 | Num::U128 => as_n("asUintN"),
            Num::I32 => Expr::bin(Op::BitOr, e, Expr::num(0)),
            Num::U32 => Expr::bin(Op::UShr, e, Expr::num(0)),
            Num::I8 | Num::I16 => {
                let shift = 32 - self.bits();
                Expr::bin(Op::Shr, Expr::bin(Op::Shl, e, Expr::num(shift)), Expr::num(shift))
            }
            Num::U8 | Num::U16 => Expr::bin(Op::BitAnd, e, Expr::int(self.range().1 as i128)),
            // The nearest `f32`: of an exact result, Rust's (ADR 0122).
            Num::F32 => Expr::call(Expr::member(Expr::var("Math"), "fround"), vec![e]),
            Num::F64 => e,
        }
    }
}

/// An integer the JS computes from constants alone: `404 + 0`.
fn const_int(e: &Expr) -> Option<i128> {
    Some(match &e.kind {
        js::ExprKind::Num(n) if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 => *n as i128,
        js::ExprKind::BigInt(n) => *n,
        js::ExprKind::Unary(js::UnaryOp::Neg, x) => -const_int(x)?,
        js::ExprKind::Binary(op, a, b) => {
            let (a, b) = (const_int(a)?, const_int(b)?);
            match op {
                Op::Add => a.checked_add(b)?,
                Op::Sub => a.checked_sub(b)?,
                Op::Mul => a.checked_mul(b)?,
                _ => return None,
            }
        }
        _ => return None,
    })
}

pub(super) fn is_fieldless_enum(adt: ty::AdtDef<'_>) -> bool {
    adt.is_enum() && adt.variants().iter().all(|v| v.fields.is_empty())
}

/// Whether an enum's JS value is a string, its variant's name (ADR 0013):
/// a fieldless one's, but not a tagged one's (ADR 0284), whose variants are
/// objects, `{ kind: "on" }`. What `===`, `includes` and a `Map`'s keys
/// compare by: the one answer each asks.
pub(super) fn is_string_enum(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> bool {
    is_fieldless_enum(adt) && super::bindings::declared_tag(tcx, adt.did()).is_none() && !bindings::numbered(tcx, adt)
}

/// What rustc computes for constant `def_id` of `args`, or its `promoted`
/// part, as a value tree (ADR 0031): the one evaluation, of an item's
/// constant and a MIR body's alike. The query itself, not
/// `const_eval_global_id_for_typeck`, which reports a constant too large
/// for a value tree as rustc's own error where rust-js reports it as
/// unsupported.
pub(super) fn valtree<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    promoted: Option<rustc_middle::mir::Promoted>,
    span: Span,
) -> Option<ty::ValTree<'tcx>> {
    let instance = super::items::resolve_instance_in(tcx, typing_env, def_id, args).ok()??;
    let cid = GlobalId { instance, promoted };
    let inputs = tcx.erase_and_anonymize_regions(typing_env.with_post_analysis_normalized(tcx).as_query_input(cid));
    tcx.at(span).eval_to_valtree(inputs).ok()
}

/// Whether rustc's value of a `const` has no byte written, a
/// `MaybeUninit::uninit()`'s: JS's `undefined` (ADR 0332), which a use
/// needn't copy. Value trees hold no such value, so it's read from the
/// memory rustc made of it.
fn const_uninit<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    span: Span,
) -> bool {
    let Ok(Some(instance)) = super::items::resolve_instance_in(tcx, typing_env, def_id, args) else {
        return false;
    };
    let cid = GlobalId {
        instance,
        promoted: None,
    };
    let inputs = tcx.erase_and_anonymize_regions(typing_env.with_post_analysis_normalized(tcx).as_query_input(cid));
    let Ok(alloc) = tcx.at(span).eval_to_allocation_raw(inputs) else {
        return false;
    };
    let memory = tcx.global_alloc(alloc.alloc_id).unwrap_memory();
    let whole = alloc_range(rustc_abi::Size::ZERO, memory.inner().size());
    whole.size.bytes() > 0
        && memory
            .inner()
            .init_mask()
            .is_range_initialized(whole)
            .is_err_and(|uninit| uninit.start == whole.start && uninit.size == whole.size)
}

/// A `const`'s value as JS writes it, where it can: what rustc computed
/// (ADRs 0031, 0127), or `undefined` of one with no byte written.
pub(super) fn const_value_js<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    span: Span,
) -> Option<Expr> {
    eval_const(tcx, typing_env, def_id, args, span)
        .and_then(|value| const_js(tcx, value))
        .or_else(|| const_uninit(tcx, typing_env, def_id, args, span).then(Expr::undefined))
}

/// What rustc computed for a `const`, as a value tree (ADR 0031).
pub(super) fn eval_const<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    span: Span,
) -> Option<ty::Value<'tcx>> {
    let args = tcx
        .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(args))
        .ok()?;
    let valtree = valtree(tcx, typing_env, def_id, args, None, span)?;
    let ty = tcx.type_of(def_id).instantiate(tcx, args).skip_normalization();
    Some(ty::Value {
        ty: tcx.normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty)),
        valtree,
    })
}

/// What rustc computed for a `static` (ADR 0096), as a `const`'s value tree.
/// rustc makes value trees only of constants, since a static is a place, so
/// this reads one out of the static's memory. None for a value no value tree
/// holds: a reference to another static, a pointer, a `dyn`.
pub(super) fn static_value<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> Option<ty::Value<'tcx>> {
    let memory = tcx.eval_static_initializer(def_id).ok()?;
    let ty = tcx.type_of(def_id).instantiate_identity().skip_normalization();
    let valtree = valtree_at(tcx, Pointer::from(unchanging(tcx, memory)), ty)?;
    Some(ty::Value { ty, valtree })
}

/// Memory rustc reads for a value tree only if it can't change, as a `static
/// mut`'s or an atomic's can: a copy of it, marked so.
fn unchanging<'tcx>(tcx: TyCtxt<'tcx>, memory: ConstAllocation<'tcx>) -> AllocId {
    let mut copy = memory.inner().clone();
    copy.mutability = Mutability::Not;
    tcx.reserve_and_set_memory_alloc(tcx.mk_const_alloc(copy))
}

/// Where a pointer in a static's memory points, readable. A `&` in one is
/// to memory of its own, a nested static; one to another static, of a
/// `primitive` it reads, a number, `&A.count`, where that static can't
/// change, is that static's: its value is the reference's, as a reference
/// is its value (ADR 0023), and no program tells a number's copy from it.
/// Any other is `None`.
fn pointee(tcx: TyCtxt<'_>, pointer: Scalar, primitive: bool) -> Option<Pointer> {
    let pointer = pointer.to_pointer(&tcx).discard_err()?.into_pointer_or_addr().ok()?;
    let (prov, offset) = pointer.prov_and_relative_offset();
    let memory = match tcx.try_get_global_alloc(prov.alloc_id())? {
        GlobalAlloc::Memory(memory) if memory.inner().mutability == Mutability::Not => return Some(pointer),
        GlobalAlloc::Memory(memory) => memory,
        GlobalAlloc::Static(d) if matches!(tcx.def_kind(d), DefKind::Static { nested: true, .. }) => {
            tcx.eval_static_initializer(d).ok()?
        }
        GlobalAlloc::Static(d)
            if primitive
                && !tcx.is_mutable_static(d)
                && tcx
                    .type_of(d)
                    .instantiate_identity()
                    .skip_normalization()
                    .is_freeze(tcx, ty::TypingEnv::fully_monomorphized()) =>
        {
            tcx.eval_static_initializer(d).ok()?
        }
        _ => return None,
    };
    Some(Pointer::new(unchanging(tcx, memory).into(), offset))
}

/// The value tree of the `ty` at `at`, as rustc makes one for a constant.
fn valtree_at<'tcx>(tcx: TyCtxt<'tcx>, at: Pointer, ty: Ty<'tcx>) -> Option<ty::ValTree<'tcx>> {
    let (prov, offset) = at.prov_and_relative_offset();
    valtree_of(
        tcx,
        ConstValue::Indirect {
            alloc_id: prov.alloc_id(),
            offset,
        },
        ty,
    )
}

fn valtree_of<'tcx>(tcx: TyCtxt<'tcx>, value: ConstValue, ty: Ty<'tcx>) -> Option<ty::ValTree<'tcx>> {
    let typing_env = ty::TypingEnv::fully_monomorphized();
    let word = tcx.data_layout.pointer_size();
    let read = |words: u64, size, provenance| {
        let ConstValue::Indirect { alloc_id, offset } = value else {
            return None;
        };
        let memory = tcx.global_alloc(alloc_id).unwrap_memory();
        memory
            .inner()
            .read_scalar(&tcx, alloc_range(offset + word * words, size), provenance)
            .ok()
    };
    match ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => {
            let scalar = match value {
                ConstValue::Scalar(scalar) => scalar,
                _ => read(0, tcx.layout_of(typing_env.as_query_input(ty)).ok()?.size, false)?,
            };
            Some(ty::ValTree::from_scalar_int(tcx, scalar.try_to_scalar_int().ok()?))
        }
        // A reference's value tree is its referent's. A `&str` or a `&[T]`
        // is two words: where its items are, and how many.
        ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Str | ty::Slice(_)) => {
            let (data, len) = match value {
                ConstValue::Slice { alloc_id, meta } => (Scalar::from_pointer(alloc_id.into(), &tcx), meta),
                _ => (
                    read(0, word, true)?,
                    read(1, word, false)?.to_target_usize(&tcx).discard_err()?,
                ),
            };
            let len_usize = usize::try_from(len).ok()?;
            match *inner.kind() {
                ty::Str if len == 0 => Some(ty::ValTree::from_raw_bytes(tcx, &[])),
                ty::Str => {
                    let (prov, offset) = pointee(tcx, data, false)?.prov_and_relative_offset();
                    let memory = tcx.global_alloc(prov.alloc_id()).unwrap_memory();
                    let start = usize::try_from(offset.bytes()).ok()?;
                    let bytes = memory
                        .inner()
                        .inspect_with_uninit_and_ptr_outside_interpreter(start..start + len_usize);
                    Some(ty::ValTree::from_raw_bytes(tcx, bytes))
                }
                ty::Slice(_) if len == 0 => Some(ty::ValTree::zst(tcx)),
                ty::Slice(item) => valtree_at(tcx, pointee(tcx, data, false)?, Ty::new_array(tcx, item, len)),
                _ => None,
            }
        }
        ty::Ref(_, inner, _) if inner.is_sized(tcx, typing_env) => {
            let pointer = match value {
                ConstValue::Scalar(scalar) => scalar,
                _ => read(0, word, true)?,
            };
            let primitive = matches!(
                inner.kind(),
                ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_)
            );
            valtree_at(tcx, pointee(tcx, pointer, primitive)?, *inner)
        }
        ty::Adt(adt, _) if adt.is_union() => None,
        // A JS value, react's `ElementType` say, is nothing in Rust's memory:
        // its initializer says what it is (ADR 0234).
        ty::Adt(adt, args) if marks_js_object(tcx, *adt, args) => None,
        ty::Array(..) | ty::Tuple(_) | ty::Adt(..) => {
            let parts = tcx.try_destructure_mir_constant_for_user_output(value, ty)?;
            // An enum's starts with its variant's index, as rustc's own does.
            // (The destructured value has one for a struct too.)
            let variant = parts.variant.filter(|_| ty.is_enum()).map(|v| {
                ty::Const::new_value(tcx, ty::ValTree::from_scalar_int(tcx, v.as_u32().into()), tcx.types.u32)
            });
            let fields = parts
                .fields
                .iter()
                .map(|&(field, ty)| Some(ty::Const::new_value(tcx, valtree_of(tcx, field, ty)?, ty)))
                .collect::<Option<Vec<_>>>()?;
            Some(ty::ValTree::from_branches(tcx, variant.into_iter().chain(fields)))
        }
        _ => None,
    }
}

/// A struct that's a JS value, as `Recognition::is_js_object` tells one:
/// `PhantomData` of an extern type or of `#[rust_js::js_object]` first.
pub(super) fn marks_js_object<'tcx>(tcx: TyCtxt<'tcx>, adt: ty::AdtDef<'tcx>, args: ty::GenericArgsRef<'tcx>) -> bool {
    adt.is_struct() && adt.non_enum_variant().fields.iter().next().is_some_and(|field| {
        matches!(field.ty(tcx, args).skip_normalization().kind(), ty::Adt(marker, marked) if marker.is_phantom_data()
        && marked.types().next().is_some_and(|t| match t.kind() {
            ty::Foreign(_) => true,
            ty::Adt(object, _) => tcx
                .get_attrs_by_path(object.did(), &[Symbol::intern("rust_js"), Symbol::intern("js_object")])
                .next()
                .is_some(),
            _ => false,
        }))
    })
}

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
pub(super) fn zero_sized_js<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Expr> {
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
pub(super) fn const_js<'tcx>(tcx: TyCtxt<'tcx>, value: ty::Value<'tcx>) -> Option<Expr> {
    const_js_by(tcx, value, &|adt, variant| bindings::unit_variant(tcx, adt, variant))
}

/// `const_js`, each variant without fields as `unit` writes it.
fn const_js_by<'tcx>(
    tcx: TyCtxt<'tcx>,
    value: ty::Value<'tcx>,
    unit: &dyn Fn(DefId, &ty::VariantDef) -> Expr,
) -> Option<Expr> {
    let ty = value.ty;
    if ty.is_bool() {
        return value.try_to_bool().map(Expr::bool);
    }
    // A `Duration`'s is its nanoseconds, of std's `secs` and `nanos` (ADR 0188).
    if super::recognition::is_duration_ty(ty) {
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
    if super::recognition::clock_type_of(ty).is_some() {
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
        && super::recognition::is_symbol_for_def(tcx, adt.did())
    {
        let ty::ValTreeKind::Branch(items) = &**value.valtree else {
            return None;
        };
        let [key] = &items[..] else { return None };
        return Some(symbol_for(const_js_by(tcx, key.try_to_value()?, unit)?));
    }
    // A `Discriminant`'s is its number, inside std's struct (ADR 0393).
    if super::recognition::is_discriminant_type(tcx, ty) {
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
                    Some(&inner) => const_js_by(tcx, inner, unit).map(super::std_types::option::some_literal),
                    None => Some(Expr::undefined()),
                };
            }
            if let Some(n) = ordering_value(tcx, adt.did(), variant.name) {
                return Some(Expr::int(n));
            }
            // An untagged enum's variant is its payload (ADR 0214).
            if super::bindings::is_untagged(tcx, adt.did()) {
                return fields.first().and_then(|&payload| const_js_by(tcx, payload, unit));
            }
            if fields.is_empty() {
                return Some(unit(adt.did(), variant));
            }
            let values = fields_of(fields)?;
            let props = values
                .into_iter()
                .enumerate()
                .map(|(i, v)| Prop::Field(variant_field(tcx, variant, i), v));
            Some(Expr::object(
                std::iter::once(Prop::Field(
                    super::bindings::tag_key(tcx, adt.did()),
                    super::bindings::variant_tag(tcx, variant),
                ))
                .chain(props)
                .collect(),
            ))
        }
        // A `#[repr(transparent)]` struct's is its field's (ADR 0584).
        ty::Adt(adt, _) if let Some(at) = super::recognition::transparent_field(tcx, *adt) => {
            const_js_by(tcx, *children()?.get(at)?, unit)
        }
        // A std struct's fields are its own, not the JS value rust-js makes
        // of it: `iter::empty()` is no `[undefined]`. Only a `PhantomData`, and
        // `Reverse(x)`, `[x]`, are their fields.
        // A `RefCell` of an object is the object, where generic code has
        // none of any value (ADR 0863), which only a body lowering it knows:
        // its initializer's, as one that can't be said here is.
        ty::Adt(adt, args)
            if is_std_def(tcx, adt.did(), StdItem::RefCell) && super::drops::holds_itself(tcx, args.type_at(0)) =>
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
        ty::Adt(adt, _) if adt.is_struct() && !super::recognition::struct_is_its_fields(tcx, adt.did()) => None,
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
            if variant.ctor_kind().is_none() && variant.fields.iter().any(ref_cell) {
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
                            Prop::Field(field_key(tcx, f), v)
                        })
                        .collect(),
                )),
            }
        }
        _ => None,
    }
}

/// The JS property for field `i` of an enum variant (ADR 0033): `_0` in a
/// tuple variant, as in ReScript, and its name in a struct variant.
pub(super) fn variant_field(tcx: TyCtxt<'_>, variant: &ty::VariantDef, i: usize) -> String {
    match variant.ctor_kind() {
        Some(CtorKind::Fn) => js::positional(i),
        _ => field_key(tcx, variant.fields.iter().nth(i).expect("a field of this variant")),
    }
}

/// An `Ordering` is -1, 0 or 1 (ADR 0036), its discriminant, which a JS
/// comparator returns as it is.
pub(super) use super::recognition::ordering_value;

/// A `js::SymbolFor` of `key`, the registry's symbol, `Symbol.for(key)`
/// (ADR 0634): the one way one is made, a constant's, a value's, a
/// constructor's.
pub(super) fn symbol_for(key: Expr) -> Expr {
    Expr::call(symbol_for_function(), vec![key])
}

/// `js::SymbolFor` itself, its constructor as a value: `Symbol.for`, the
/// registry's function, which reads no `this` and only its first argument
/// (ADR 0964).
pub(super) fn symbol_for_function() -> Expr {
    Expr::member(Expr::var("Symbol"), "for")
}

/// A `js::SymbolFor`'s key, its `.0`: `Symbol.keyFor(symbol)` (ADR 0634).
fn symbol_key(symbol: Expr) -> Expr {
    Expr::call(Expr::member(Expr::var("Symbol"), "keyFor"), vec![symbol])
}

/// The symbol whose key `read` is, where it's a `js::SymbolFor`'s,
/// `symbol_key`'s: what a test of its key against text compares instead,
/// `symbol === Symbol.for(text)` (ADR 0634).
pub(super) fn symbol_of_key(read: &Expr) -> Option<&Expr> {
    match &read.kind {
        js::ExprKind::Call(callee, args)
            if matches!(&callee.kind, js::ExprKind::Member(object, name)
                if name == "keyFor" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Symbol")) =>
        {
            match &args[..] {
                [symbol] => Some(symbol),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A `char` constant (ADR 0034).
pub(super) fn char_value(value: ty::Value<'_>) -> Option<char> {
    if !value.ty.is_char() {
        return None;
    }
    char::from_u32(value.try_to_leaf()?.to_u32())
}

pub(super) fn num_literal(bits: u128, num: Num) -> Expr {
    if num == Num::F64 {
        return Expr::num(f64::from_bits(bits as u64));
    }
    if num == Num::F32 {
        return f32_literal(f32::from_bits(bits as u32));
    }
    let unused = 128 - num.bits();
    let n = if num.signed() {
        ((bits << unused) as i128) >> unused
    } else {
        bits as i128
    };
    num.literal(n)
}

/// What takes a `()` given to JS: a binding's parameter, its `DefId` and
/// index, or a `dyn` of a trait, the `()` a reference to it.
pub(super) enum UnitTo<'tcx> {
    Param(DefId, usize),
    Dyn(Ty<'tcx>),
}

/// What JS is given of a `()` given to `to`, where it isn't `undefined`,
/// what `()` is everywhere (ADR 0020), as a value of any shape, `x ===
/// undefined` of `js::strict_equals(x, ())` (ADR 0775): `[]`, a list of
/// none, where it's a list of items, a tuple of them its array. That's
/// rest arguments, a variadic binding's last parameter or a `dyn` of
/// `js::Rest`, spread, and a hook's dependencies, `react::DependencyList`,
/// `use_memo(f, ())`'s `useMemo(f, [])`. The one answer.
pub(super) fn unit_given<'tcx>(tcx: TyCtxt<'tcx>, to: UnitTo<'tcx>) -> Option<Expr> {
    let rest = [Symbol::intern("rust_js"), Symbol::intern("rest")];
    let lists = |id: DefId| {
        super::recognition::is_dependency_list(tcx, id) || tcx.get_attrs_by_path(id, &rest).next().is_some()
    };
    let list = match to {
        UnitTo::Param(def_id, i) => {
            let declared = tcx.fn_sig(def_id).instantiate_identity().skip_normalization();
            let declared = declared.skip_binder().inputs();
            let bounded = |ty: Ty<'tcx>| {
                tcx.param_env(def_id).caller_bounds().iter().any(|clause| {
                    clause
                        .as_trait_clause()
                        .is_some_and(|bound| bound.self_ty().skip_binder() == ty && lists(bound.def_id()))
                })
            };
            (bindings::is_variadic(tcx, def_id) && i + 1 == declared.len())
                || declared
                    .get(i)
                    .is_some_and(|&ty| matches!(ty.kind(), ty::Param(_)) && bounded(ty))
        }
        UnitTo::Dyn(ty) => matches!(ty.kind(), ty::Dynamic(traits, ..) if traits.principal_def_id().is_some_and(lists)),
    };
    list.then(|| Expr::array(Vec::new()))
}
