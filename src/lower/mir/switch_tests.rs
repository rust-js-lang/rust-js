//! The tests a switch makes of its subject, each case's: of an enum's
//! discriminant its variant's, of an `Option` whose test is known that
//! test, of what `?` tried whether it returns early. Split from
//! structure.rs by concept, as it reached its cap (#2144).

use super::*;

/// Where in what a pattern matches a switch's subject is, which names its
/// constants (`body_queries::pattern_places`).
type Tested<'a, 'tcx> = Option<&'a [(Ty<'tcx>, Vec<crate::lower::body_queries::PatternStep>)]>;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The test that `subject`, of type `ty`, is `value`: of an enum's
    /// discriminant, its variant. `tested` is where in what a pattern
    /// matches it is, of MIR's place of it (`body_queries::pattern_places`).
    pub(super) fn switch_test(
        &mut self,
        subject: &Value<'tcx>,
        ty: Ty<'tcx>,
        value: u128,
        span: Span,
        (tested, tag): (Tested<'_, 'tcx>, Option<&Expr>),
    ) -> R<Expr> {
        match subject {
            // Of an `Option` whose test is known: it, of its `Some` (ADR 0642).
            Value::Tested(test) => Ok(match value {
                0 => test.clone().negated_test(),
                _ => test.clone(),
            }),
            Value::Branch(tried, tried_ty) => {
                let failed = self.branch_failed(tried.clone(), *tried_ty);
                Ok(match value {
                    0 => failed.negated_test(),
                    _ => failed,
                })
            }
            Value::Discriminant(enum_value, enum_ty, present, none) => {
                let ty::Adt(adt, _) = enum_ty.kind() else {
                    return Err(self.unsupported(span, "a discriminant of this type, from its MIR"));
                };
                let variant = adt
                    .discriminants(self.tcx)
                    .find(|(_, d)| d.val == value)
                    .map(|(v, _)| v)
                    .ok_or_else(|| self.unsupported(span, "this discriminant, from its MIR"))?;
                // An untagged enum's variants JS can't tell apart: refused (ADR 0214).
                if bindings::is_untagged(self.tcx, adt.did())
                    && let Some((other, classes)) = self.untagged_alike(*enum_ty, adt.variant(variant))
                {
                    // Two classes of one member, refused once the arms are
                    // known to differ (`untold_class`, #555).
                    if classes.is_none() {
                        let message = format!(
                            "rust-js: `{}` and `{other}` hold values of one kind, which JS can't tell apart in an untagged enum (ADR 0214)",
                            adt.variant(variant).name
                        );
                        return Err(crate::lower::refusal::error(self.tcx, span, message));
                    }
                }
                Ok(self.mir_variant_test(
                    (enum_value.clone(), tag),
                    *enum_ty,
                    *adt,
                    &[variant],
                    &[],
                    (*present, *none),
                ))
            }
            _ => {
                let subject = self.value_expr(subject.clone(), span)?;
                // An `Option<bool>`'s `Some(true)` or `Some(false)`, as a
                // pattern tests it: that value exactly, `o === true`, as
                // `o == Some(true)` is (the user's decision, ADR 0298).
                // Of `false`, `o === false` where a pattern there writes
                // `Some(false)`, else `o !== true`, which a `Some(true)`'s
                // branch negates.
                if ty.is_bool() && self.option_bool_payload(tested) {
                    let value_of = crate::lower::body_queries::PatternValue::Bits(ty, 0);
                    let written_false =
                        value != 0 || (self.frame.facts.constant_patterns).tests(self.tcx, &value_of, span, tested);
                    return Ok(match written_false {
                        true => Expr::bin(Op::Eq, subject, Expr::bool(value != 0)),
                        false => Expr::bin(Op::Ne, subject, Expr::bool(true)),
                    });
                }
                if ty.is_bool() {
                    return Ok(match value {
                        0 => Expr::unary(js::UnaryOp::Not, subject),
                        _ => subject,
                    });
                } // A value the program's patterns name by a `const` where it's
                // tested, as that reads (ADR 0590, amended): `priorityLevel
                // === ImmediatePriority`.
                let value_of = crate::lower::body_queries::PatternValue::Bits(ty, value);
                if let Some(named) = (self.frame.facts.constant_patterns).named(self.tcx, &value_of, span, tested) {
                    let named = self.named_const(named, ty::List::empty(), ty, span, self.stored_copied(ty))?;
                    return Ok(Expr::bin(Op::Eq, subject, named));
                }
                if ty.is_char() {
                    let c = char::from_u32(value as u32).ok_or_else(|| self.unsupported(span, "this `char`"))?;
                    return Ok(Expr::bin(Op::Eq, subject, Expr::str(c.to_string())));
                }
                let num = self.num(ty, span)?;
                let literal = match num.signed() {
                    true => {
                        let bits = num.bits();
                        let v = ((value << (128 - bits)) as i128) >> (128 - bits);
                        num.literal(v)
                    }
                    false => num.literal(value as i128),
                };
                Ok(Expr::bin(Op::Eq, subject, literal))
            }
        }
    }

    /// Whether what a pattern tests at `tested` (`body_queries::pattern_places`)
    /// is an `Option<bool>`'s `Some`'s value.
    pub(super) fn option_bool_payload(
        &self,
        tested: Option<&[(Ty<'tcx>, Vec<crate::lower::body_queries::PatternStep>)]>,
    ) -> bool {
        use crate::lower::body_queries::PatternStep;
        tested.is_some_and(|places| {
            places.iter().any(|(of, steps)| {
                self.option_of(*of).is_some_and(|inner| inner.is_bool())
                    && matches!(steps[..], [PatternStep::Variant(v), PatternStep::Field(f)] if v.as_u32() == 1 && f.as_u32() == 0)
            })
        })
    }

    /// Whether what `?` tried returns early; a write's never does (ADRs
    /// 0054, 0132).
    fn branch_failed(&self, tried: Expr, ty: Ty<'tcx>) -> Expr {
        match self.is_write_result(ty) {
            true => Expr::bool(false),
            false => self.tried_failed(tried, ty),
        }
    }

    /// That `subject` is one of `variants` of its enum, as a pattern tests
    /// it (`variants_test`): a test function an untagged one's calls,
    /// `isValidElement(child)`, gives a `bool`, which tidy writes a test of
    /// as one. `present`, as `variant_test` has it; each of `variants`
    /// `written` where it's given.
    #[track_caller]
    pub(in crate::lower) fn mir_variant_test(
        &mut self,
        (subject, tag): (Expr, Option<&Expr>),
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variants: &[VariantIdx],
        written: &[js::Span],
        (present, none): (bool, Option<js::OnlyNone>),
    ) -> Expr {
        let test = self.variants_test_by((subject, tag), ty, adt, variants, written, (present, none));
        if bindings::is_untagged(self.tcx, adt.did()) {
            test.clone().each_mut(&mut |e| {
                if let Some(path) = e.called_path() {
                    self.locals.mir.paths.entry(path).or_insert(true);
                }
            });
        }
        test
    }
}
