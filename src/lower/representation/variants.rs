//! How a value's variant is told, the one test of it each reader asks: an
//! `Option`'s presence, an untagged enum's kind, a keyed one's keys, a
//! discriminated union's tag, an `Ordering`'s number, or its tag. Split from
//! representation.rs by concept (#2208).

use super::super::FnCx;
use super::super::bindings;
use crate::js;
use crate::js::{Expr, Op, UnaryOp};
use rustc_abi::VariantIdx;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty;
use rustc_middle::ty::Ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// That `subject` is one of `variants` of its enum, an or-pattern's,
    /// which MIR sends to one block: each one's test, `||`; of an untagged
    /// enum's, what they are together, `Object | Array`'s `typeof subject
    /// === "object"` (ADR 0668); `present` as `variant_test`'s. Each test
    /// of one is where the first of `variants` it tests is `written`, where
    /// that's given, an or-pattern's alternative: what a comment before it
    /// is before (ADR 0444, amended).
    #[track_caller]
    pub(in crate::lower) fn variants_test(
        &self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variants: &[VariantIdx],
        written: &[js::Span],
        (present, none): (bool, Option<js::OnlyNone>),
    ) -> Expr {
        self.variants_test_by((subject, None), ty, adt, variants, written, (present, none))
    }

    /// `variants_test`, of the tag `tag` where it's given, read once where
    /// JS may have made the value (`held_tag`, #2208).
    #[track_caller]
    pub(in crate::lower) fn variants_test_by(
        &self,
        (subject, tag): (Expr, Option<&Expr>),
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variants: &[VariantIdx],
        written: &[js::Span],
        (present, none): (bool, Option<js::OnlyNone>),
    ) -> Expr {
        if bindings::is_untagged(self.tcx, adt.did()) {
            let variants: Vec<&ty::VariantDef> = variants.iter().map(|&v| adt.variant(v)).collect();
            return self.untagged_variant_test(ty, &variants, written, &subject, present);
        }
        // A keyed enum's of its variant without fields too: what the others
        // left out aren't, `!('getter' in dep) && !('currentValue' in dep)`
        // (#504).
        if let Some(keyed) = self.krate.flattened.keyed(self.tcx, adt)
            && variants.iter().any(|&v| keyed.tell(v).is_none())
        {
            return (adt.variants().indices())
                .filter(|v| !variants.contains(v))
                .filter_map(|v| keyed.tell(v))
                .map(|key| Expr::unary(UnaryOp::Not, bindings::keyed_test(subject.clone(), key)))
                .reduce(|both, test| Expr::bin(Op::And, both, test))
                .unwrap_or_else(|| Expr::bool(true));
        }
        (variants.iter().enumerate())
            .map(|(i, &variant)| {
                let test = self.variant_test_by((subject.clone(), tag), ty, adt, variant, (present, none));
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
    #[track_caller]
    pub(in crate::lower) fn variant_test(
        &self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variant: VariantIdx,
        present: bool,
    ) -> Expr {
        self.variant_test_of(subject, ty, adt, variant, (present, None))
    }

    /// `variant_test`, an `Option`'s presence of the one `None` MIR says
    /// it can be where it's read, `none` (`NoneForms`, #2112 C S5).
    #[track_caller]
    pub(in crate::lower) fn variant_test_of(
        &self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variant: VariantIdx,
        (present, none): (bool, Option<js::OnlyNone>),
    ) -> Expr {
        self.variant_test_by((subject, None), ty, adt, variant, (present, none))
    }

    /// The tag a switch over a value of the enum `ty` reads once, where JS
    /// may have made or given the value, whose tag a getter may answer each
    /// read anew (`fields_read_once`, ADR 1022, #2208): an object's tag, of
    /// an enum told by one; none of one told otherwise, an `Option`'s
    /// presence, an untagged one's kind, a keyed one's keys, an
    /// `Ordering`'s number, a write's result.
    pub(in crate::lower) fn held_tag(&self, subject: &Expr, ty: Ty<'tcx>, adt: ty::AdtDef<'tcx>) -> Option<Expr> {
        let told_by_tag = !self.tcx.is_lang_item(adt.did(), LangItem::Option)
            && !self.is_write_result(ty)
            && !bindings::is_untagged(self.tcx, adt.did())
            && self.krate.flattened.keyed(self.tcx, adt).is_none()
            && (adt.variants().iter())
                .all(|v| crate::lower::recognition::ordering_value(self.tcx, adt.did(), v.name).is_none())
            && (adt.variants().iter())
                .any(|v| !v.fields.is_empty() || bindings::declared_tag(self.tcx, adt.did()).is_some());
        (told_by_tag && self.fields_read_once(ty))
            .then(|| Expr::member(subject.clone(), bindings::tag_key(self.tcx, adt.did())))
    }

    /// `variant_test_of`, of the tag `tag` where it's given (`held_tag`).
    #[track_caller]
    pub(in crate::lower) fn variant_test_by(
        &self,
        (subject, tag): (Expr, Option<&Expr>),
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variant: VariantIdx,
        (present, none): (bool, Option<js::OnlyNone>),
    ) -> Expr {
        let tag_of = |subject: Expr, key: &str| tag.cloned().unwrap_or_else(|| Expr::member(subject, key));
        let def = adt.variant(variant);
        if self.tcx.is_lang_item(adt.did(), LangItem::Option) {
            let some = self.tcx.is_lang_item(def.def_id, LangItem::OptionSome);
            return Expr::presence(subject, some, none);
        }
        // A write's result's `Ok`, `undefined`, and its `Err`, the error it
        // caught (ADRs 0187, 0462).
        if self.is_write_result(ty) {
            let op = if variant.as_u32() == 0 { Op::Eq } else { Op::Ne };
            return Expr::bin(op, subject, Expr::js_undefined());
        }
        // An untagged enum's variant, told by its value's kind (ADR 0214).
        if bindings::is_untagged(self.tcx, adt.did()) {
            return self.untagged_variant_test(ty, &[def], &[], &subject, present);
        }
        // A keyed enum's, by its own key, `'getter' in node`; its variant
        // without fields, by none of the others' (#504).
        if let Some(keyed) = self.krate.flattened.keyed(self.tcx, adt) {
            return match keyed.tell(variant) {
                Some(key) => bindings::keyed_test(subject, key),
                None => self.variants_test(subject, ty, adt, &[variant], &[], (present, none)),
            };
        }
        // A discriminated union's `otherwise`: an object whose tag is none
        // of the others' (ADR 0284).
        if bindings::is_tagged_otherwise(self.tcx, adt.did(), def) {
            let key = bindings::tag_key(self.tcx, adt.did());
            let tests = (adt.variants().iter())
                .filter(|other| other.def_id != def.def_id)
                .map(|other| {
                    let name = bindings::variant_tag(self.tcx, other);
                    Expr::bin(Op::Ne, tag_of(subject.clone(), &key), name)
                })
                .reduce(|a, b| Expr::bin(Op::And, a, b));
            return tests.unwrap_or_else(|| Expr::bool(true));
        }
        if let Some(n) = crate::lower::recognition::ordering_value(self.tcx, adt.did(), def.name) {
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
                tag_of(subject, &bindings::tag_key(self.tcx, adt.did())),
                bindings::variant_tag(self.tcx, def),
            ),
        }
    }
}
