//! `a == b` of `Option`s: of two `Option`s of `Option`s each held as its
//! payload, its three kinds of value compared apart, and whether an `==`
//! of one is `===`, where either answers the same (ADRs 0030, 0771, 1017).
//! Split from `dispatch.rs` by concept, as a file is at most 1350 lines
//! (#1334).

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// `a == b` and `a != b` of two `Option`s of `Option`s each held as its
    /// payload (`OptionWrites::nested`, ADR 1017), whose three kinds of
    /// value, `undefined`, `null` and the payload's, are each one of them:
    /// `a === b` of a payload `===` compares as its `==` does, `instance
    /// !== previousInstance`; else `a === undefined || b === undefined ? a
    /// === b : <a == b of their payloads' `Option`s>`. `None` of any other
    /// call.
    pub(super) fn nested_equality(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        operands: &[&Operand<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let nested = |operand: &Operand<'tcx>| {
            operand
                .place()
                .and_then(|place| place.as_local())
                .is_some_and(|local| state.facts.options().nested.contains(&local))
        };
        // `flatten()` of one `mir_nested` says may be, or of a nested field:
        // what it holds, its `None` `null` as the field's is, `entry.value`
        // (#470).
        if let [x] = operands
            && self.holds_nested_operand(state, x)
            && self.recognition().classify(def_id, generic_args) == Some(Std::OptionFlatten)
        {
            let place = x.place().expect("a place");
            return Ok(Some(match place.projection.is_empty() {
                true => self.nested_operand(state, x, span, out)?,
                false => self.mir_place(state, place, span, out)?,
            }));
        }
        // `is_some()` and `is_none()` of one, through a reference: a test of
        // its `None`, `last !== undefined` (ADR 1017, amended), `null` of a
        // value never `null` (rd645).
        if let [x] = operands
            && nested(x)
            && let Some(known @ (Std::IsSome | Std::IsNone)) = self.recognition().classify(def_id, generic_args)
        {
            let none = self.held_none(x.ty(state.body, self.tcx));
            let x = self.nested_operand(state, x, span, out)?;
            let op = if known == Std::IsSome { Op::Ne } else { Op::Eq };
            return Ok(Some(Expr::bin(op, x, none)));
        }
        let negate = match self
            .tcx
            .trait_of_assoc(def_id)
            .and_then(|t| self.recognition().trait_call(def_id, t))
        {
            Some(TraitCall::Equality { negate }) => negate,
            _ => return Ok(None),
        };
        let [a, b] = operands else { return Ok(None) };
        // Of one with a constant `Some(None)`, `null`: `last === null` (ADR
        // 1017, amended).
        let constant = match (nested(a), nested(b)) {
            (true, false) if self.nested_some_none(state.body, b) => Some(0),
            (false, true) if self.nested_some_none(state.body, a) => Some(1),
            _ => None,
        };
        if let Some(at) = constant {
            let value = (self.take_operands(state, operands, out)?.into_iter().nth(at)).expect("two operands");
            let x = self.value_expr(value, span)?;
            let op = if negate { Op::Ne } else { Op::Eq };
            return Ok(Some(Expr::bin(op, x, Expr::null())));
        }
        if !nested(a) || !nested(b) {
            return Ok(None);
        }
        let ty = a.ty(state.body, self.tcx).peel_refs();
        let payload = self.nested_payload(ty).expect("an `Option` of an `Option`");
        let mut values = self.take_operands(state, operands, out)?.into_iter();
        let mut next = |cx: &mut Self, out: &mut Vec<Stmt>| -> R<Expr> {
            let value = cx.value_expr(values.next().expect("two operands"), span)?;
            Ok(if value.reads_same() {
                value
            } else {
                cx.spill(Role::Option, value, out)
            })
        };
        let (a, b) = (next(self, out)?, next(self, out)?);
        // A `Box` and an uncounted `Rc` are what they point at, as `==`
        // compares them (ADR 0023).
        let mut held = payload;
        while let ty::Adt(_, args) = held.kind()
            && (held.is_box() || self.is_rc(held) && self.counted_rc(held).is_none())
        {
            held = args.type_at(0);
        }
        let equal = if self.js_eq(held).is_some() || self.eq_by_address(held) {
            Expr::bin(Op::Eq, a, b)
        } else {
            let held = Ty::new_option(self.tcx, payload);
            let none = Expr::bin(
                Op::Or,
                Expr::bin(Op::Eq, a.clone(), Expr::js_undefined()),
                Expr::bin(Op::Eq, b.clone(), Expr::js_undefined()),
            );
            let both = self.eq_value(a.clone(), b.clone(), held, false, span, out)?;
            Expr::cond(none, Expr::bin(Op::Eq, a, b), both)
        };
        Ok(Some(match negate {
            true => equal.negated_test(),
            false => equal,
        }))
    }

    /// The payload `operand` holds, through a reference, read once.
    fn nested_operand(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let value = (self.take_operands(state, &[operand], out)?.into_iter().next()).expect("one operand");
        self.value_expr(value, span)
    }

    /// Whether an `==` of an `Option` is `===`: of one side only ever
    /// `Some(..)`, `p == Some(s)`, never `None`, so either answers the same
    /// where `==` is written, of a payload that's its `Some` (`eq_value`),
    /// and `===` is what a person writes (ADR 0030); and one of a value
    /// read as the registry's symbol, itself, which only `===` tells from
    /// what turns into one (ADR 0771).
    pub(super) fn strictly_equal(
        &self,
        state: &State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        operands: &[&Operand<'tcx>],
        span: Span,
    ) -> bool {
        self.tcx.trait_of_assoc(def_id).is_some_and(|trait_id| {
            matches!(
                self.recognition().trait_call(def_id, trait_id),
                Some(TraitCall::Equality { .. })
            )
        }) && (operands.iter().any(|operand| {
            operand.place().is_some_and(|place| {
                place.projection.is_empty() && state.facts.options().made_some.contains(&place.local)
            })
        }) || rust_js_lower::body_queries::the_call_at(self.frame.thir, span)
            .is_some_and(|equality| self.frame.facts.registered_equalities.contains(&equality)))
    }
}
