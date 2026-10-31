//! The constants a `match`'s pattern names, which a `switch`'s cases are
//! (`Written::switches`, ADR 0629).

use super::*;

impl<'tcx> FnCx<'_, 'tcx> {
    /// Each constant `pattern` names, if it names constants alone: a literal,
    /// a `const` (ADR 0590), a variant of no field that JS tests as a value
    /// it's equal to, `k === "Function"` (ADR 0013), each of an or-pattern.
    /// Of a value `present`, `classify(Some(value))`'s, as `untagged_case`
    /// has it (ADR 0676).
    pub(super) fn constants_of(&self, pattern: &thir::Pat<'tcx>, present: bool) -> Option<Vec<Constant>> {
        let valued = self.valued_constants(pattern, present)?;
        Some(valued.into_iter().map(|(constant, _)| constant).collect())
    }

    /// `constants_of`'s, each with the value its pattern is, where it's told
    /// (`body_queries::pattern_value`).
    pub(super) fn valued_constants(&self, pattern: &thir::Pat<'tcx>, present: bool) -> Option<Vec<Valued<'tcx>>> {
        let value = |constant| (constant, body_queries::pattern_value(self.tcx, pattern));
        match &pattern.kind {
            thir::PatKind::Deref { subpattern, .. } => self.valued_constants(subpattern, present),
            // Of a number, a `char` or a string, which `===` compares by its
            // value; not a `bool`, which has two, its `if`'s.
            thir::PatKind::Constant { .. } => {
                let ty = pattern.ty.peel_refs();
                let scalar = ty.is_integral() || ty.is_floating_point() || ty.is_char() || ty.is_str();
                scalar.then(|| vec![value(Constant::Value(self.js_span(pattern.span)))])
            }
            // A `#[repr(transparent)]` struct's of such a constant, which is
            // its field in JS (ADR 0584), `EventPriority(2)`.
            thir::PatKind::Leaf { .. }
                if let Some(field) = body_queries::transparent_subpattern(self.tcx, pattern)
                    && let thir::PatKind::Constant { .. } = field.kind
                    && (field.ty.is_integral() || field.ty.is_floating_point() || field.ty.is_char()) =>
            {
                Some(vec![value(Constant::Value(self.js_span(pattern.span)))])
            }
            // The registry's symbol, `REACT_MEMO_TYPE`, which `===` compares
            // as it does its key (ADR 0634).
            thir::PatKind::Leaf { .. } if self.recognition().is_symbol_for(pattern.ty) => {
                Some(vec![value(Constant::Value(self.js_span(pattern.span)))])
            }
            // A `Some` of constants, where it's the value itself (ADR
            // 0030), whose test of each says it's there.
            thir::PatKind::Variant {
                adt_def, subpatterns, ..
            } if self.tcx.is_lang_item(adt_def.did(), LangItem::Option)
                && let [some] = &subpatterns[..]
                && !self.boxed_payload(some.pattern.ty) =>
            {
                self.valued_constants(&some.pattern, false)
            }
            // A variant of an untagged enum whose value is a constant,
            // `Kind::String("preload")`: that constant, the value itself
            // (ADR 0668), `case "preload":`, as a `Some`'s is (rd242).
            thir::PatKind::Variant {
                adt_def,
                variant_index,
                subpatterns,
                ..
            } if let [field] = &subpatterns[..]
                && let thir::PatKind::Constant { .. } = peeled(&field.pattern).kind
                && crate::lower::bindings::is_untagged(self.tcx, adt_def.did())
                && crate::lower::untagged::told_by_kind(self.tcx, adt_def.variant(*variant_index)) =>
            {
                self.valued_constants(&field.pattern, present)
            }
            // A variant of an untagged enum, by what `typeof` says of it, its
            // value bound or not, which is the value itself (ADR 0668), or
            // `undefined` or `null` (ADR 0676); or an or-pattern of them,
            // by what they are together, `Object | Array`'s `"object"`, as
            // their test is.
            thir::PatKind::Variant { .. } | thir::PatKind::Or { .. }
                if let Some((adt, args, variants)) = self.untagged_variants(pattern) =>
            {
                let subject = js::Expr::var(SUBJECT);
                let cases = (self.recognition()).untagged_cases(adt, args, &variants, &subject, present)?;
                Some(cases.into_iter().map(|case| (Constant::Kind(case), None)).collect())
            }
            thir::PatKind::Variant {
                adt_def,
                variant_index,
                subpatterns,
                ..
            } if subpatterns.is_empty() && adt_def.variant(*variant_index).fields.is_empty() => {
                let subject = js::Expr::var(SUBJECT);
                let test = self.variant_test(subject.clone(), pattern.ty, *adt_def, *variant_index, false);
                match test.kind {
                    js::ExprKind::Binary(js::Op::Eq, s, variant) if *s == subject => {
                        Some(vec![(Constant::Variant(*variant), None)])
                    }
                    _ => None,
                }
            }
            thir::PatKind::Or { pats } => {
                let each: Option<Vec<Vec<Valued<'tcx>>>> =
                    pats.iter().map(|p| self.valued_constants(p, present)).collect();
                Some(each?.concat())
            }
            _ => None,
        }
    }
}

/// `pattern` past each `&` and `*` of it, `"preload"` of `&str`'s.
fn peeled<'p, 'tcx>(mut pattern: &'p thir::Pat<'tcx>) -> &'p thir::Pat<'tcx> {
    while let thir::PatKind::Deref { subpattern, .. } = &pattern.kind {
        pattern = subpattern;
    }
    pattern
}
