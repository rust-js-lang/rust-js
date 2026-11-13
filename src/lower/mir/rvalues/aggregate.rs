//! An aggregate MIR makes, as JS: a struct, a tuple, an array, an enum's
//! variant, a union, a closure, a coroutine's or an `async` closure's body,
//! each of its operands in order.

use super::*;
use crate::lower::aggregates::Given;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a struct's or a variant's `field` of `ty` is given, `value`:
    /// where it's held as generic code's box, the object's in-place cell,
    /// `Slot { at: $cellOf(point) }` (`field_crossing`, #1405).
    fn given_boxed_field(&self, ty: Ty<'tcx>, variant: VariantIdx, field: usize, value: Expr, span: Span) -> R<Expr> {
        Ok(match self.field_crossing(ty, variant, FieldIdx::from_usize(field)) {
            Some(crossing) => self.form_given(Form::Box, crossing, value, span)?,
            None => value,
        })
    }

    pub(super) fn mir_aggregate(
        &mut self,
        state: &mut State<'_, 'tcx>,
        kind: &AggregateKind<'tcx>,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        // `P { x, ..base }`, where THIR says it is (`written_base`): its other
        // fields `base`'s, `{ ...base, x }` (ADR 0250).
        if let AggregateKind::Adt(did, VariantIdx::ZERO, args, None, None) = *kind
            && self.tcx.adt_def(did).is_struct()
            && let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(did), args)
            && let Some(spread) = self.written_base(state, ty, operands, span)
        {
            let Shape::Object(names) = self.shape(ty) else {
                unreachable!("a struct of named fields")
            };
            let base = self.mir_place(state, spread, span, out)?;
            let of_base = |i: usize, operand: &Operand<'tcx>| {
                operand.place().is_some_and(|p| {
                    p.local == spread.local
                        && p.projection.split_last().is_some_and(|(last, rest)| {
                            matches!(last, PlaceElem::Field(f, _) if f.as_usize() == i)
                                && rest == spread.projection.as_slice()
                        })
                })
            };
            // The fields given, as written, after the spread, which JS's
            // fields after it override.
            let order = self.written_order(&self.tcx.adt_def(did), VariantIdx::ZERO, span);
            let given: Vec<usize> = order
                .into_iter()
                .filter(|&i| !of_base(i, &operands[FieldIdx::from_usize(i)]))
                .collect();
            let given_operands: Vec<&Operand<'tcx>> =
                given.iter().map(|&i| &operands[FieldIdx::from_usize(i)]).collect();
            let values = self.take_operands(state, &given_operands, out)?;
            let mut props = vec![Prop::Spread(base)];
            for (i, value) in given.into_iter().zip(values) {
                let value = self.value_expr(value, span)?;
                let field = &self.tcx.adt_def(did).non_enum_variant().fields[FieldIdx::from_usize(i)];
                let value = self.given_field(state, field, &operands[FieldIdx::from_usize(i)], value, out);
                let value = self.holding(value, Some(field), names[i].1);
                let value = self.given_boxed_field(ty, VariantIdx::ZERO, i, value, span)?;
                // One a symbol names, `[SYMBOL]: ..` (ADR 1051).
                props.push(match names[i].0.symbol_item() {
                    Some((item, span)) => Prop::Keyed(self.field_symbol(item, span), value),
                    None => Prop::Field(names[i].0.text().to_owned(), value),
                });
            }
            let mut made = Expr::object(props);
            if let js::ExprKind::Object(_, layout) = &mut made.kind {
                *layout = self.object_layout(&self.tcx.adt_def(did), VariantIdx::ZERO, span);
            }
            return Ok(Value::Expr(made));
        }
        // A struct's or a variant's fields, read in the order its object
        // lists them, as written (ADR 0674).
        let order = match kind {
            AggregateKind::Adt(did, variant_index, _, _, None) => {
                Some(self.written_order(&self.tcx.adt_def(*did), *variant_index, span))
            }
            _ => None,
        };
        // A tuple's field of no value, a value cell's clone no JS makes,
        // `undefined` (#641).
        let unwritten = |operand: &Operand<'tcx>| {
            matches!(kind, AggregateKind::Tuple)
                && matches!(operand, Operand::Move(p) if p.projection.is_empty() && state.facts.unwritten(p.local))
        };
        let declared: Vec<usize> = (0..operands.len()).collect();
        let read: Vec<usize> = (order.as_deref().unwrap_or(&declared).iter().copied())
            .filter(|&i| !unwritten(&operands[FieldIdx::from_usize(i)]))
            .collect();
        let in_order: Vec<&Operand<'tcx>> = read.iter().map(|&i| &operands[FieldIdx::from_usize(i)]).collect();
        let mut taken: Vec<Option<Value<'tcx>>> = (operands.iter())
            .map(|operand| unwritten(operand).then(|| Value::Expr(Expr::undefined())))
            .collect();
        for (&i, value) in read.iter().zip(self.take_operands(state, &in_order, out)?) {
            taken[i] = Some(value);
        }
        let values: Vec<Value<'tcx>> = taken.into_iter().map(|v| v.expect("each operand read")).collect();
        let operands: Vec<&Operand<'tcx>> = operands.iter().collect();
        match kind {
            AggregateKind::Array(_) => Ok(Value::List(values, js::Span::NONE)),
            AggregateKind::Tuple if values.is_empty() => Ok(Value::Expr(Expr::undefined())),
            AggregateKind::Tuple => Ok(Value::List(values, js::Span::NONE)),
            // A union, holding its one field (ADR 0406).
            AggregateKind::Adt(did, _, args, _, Some(field)) => {
                let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(*did), args);
                let value = values.into_iter().next().expect("a union's field");
                let value = self.value_expr(value, span)?;
                Ok(Value::Expr(self.union_value(ty, *field, value)))
            }
            AggregateKind::Adt(did, variant_index, args, _, _) => {
                let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(*did), args);
                let nulled: Vec<bool> = operands.iter().map(|o| self.nulled(state, o)).collect();
                // What's `nulled` is never `undefined`, which a field of a
                // guard its `Drop` gives a nullable place, read where it's
                // dropped as what it was made of, says (ADR 0841, amended).
                let exprs = (values.into_iter().zip(&nulled))
                    .map(|(v, &nulled)| match nulled {
                        true => self.value_expr(v, span).map(Expr::defined),
                        false => self.value_expr(v, span),
                    })
                    .collect::<R<Vec<_>>>()?;
                let exprs: Vec<Expr> = (exprs.into_iter().enumerate())
                    .map(|(i, e)| self.given_boxed_field(ty, *variant_index, i, e, span))
                    .collect::<R<Vec<_>>>()?;
                // A nested field's, held as its payload (#470).
                let exprs = match self.tcx.adt_def(*did).is_struct() {
                    true => {
                        let fields = &self.tcx.adt_def(*did).non_enum_variant().fields;
                        let mut given = Vec::with_capacity(exprs.len());
                        for ((field, operand), e) in fields.iter().zip(&operands).zip(exprs) {
                            given.push(self.given_field(state, field, operand, e, out));
                        }
                        given
                    }
                    false => exprs,
                };
                let given: Vec<Given> = (operands.iter())
                    .map(|o| match self.written_none(state.body, &state.locals, o) {
                        true => Given::WrittenNone,
                        false => self.stored_given(state, o),
                    })
                    .collect();
                // `FromFn(f)`, `iter::from_fn(f)` inlined into std's bodies:
                // what that makes, `$fromFn(f)`, its `Some` boxed where it
                // looks like `None` (ADRs 0051, 0917).
                if crate::lower::recognition::is_from_fn(self.tcx, ty)
                    && let [f] = &exprs[..]
                {
                    let mut given = vec![f.clone()];
                    if self.iterator_item(ty).is_some_and(|item| self.boxed_payload(item)) {
                        given.push(Expr::bool(true));
                    }
                    return Ok(Value::Expr(Expr::call_helper("$fromFn", given)));
                }
                let value = self.adt_value(ty, *variant_index, exprs, &given, order.as_deref(), span)?;
                // A compiled std iterator's struct, `Peekable { iter, peeked }`
                // in std's own `peekable`, is the JS iterator it is where
                // it's made, stepped by the compiled impl's `next` and sized
                // by its `size_hint`, given this body's dictionaries; in its
                // `Clone`, which has none of `I: Iterator`'s, as the one it
                // clones is, `self.$steps` (ADR 0561). Where it's no iterator,
                // in `StepBy::new`, the struct, which its caller makes one
                // (`mir_call_values`, ADR 0917).
                let cloning = self.tcx.trait_impl_of_assoc(self.item).is_some_and(|imp| {
                    self.tcx.is_lang_item(self.tcx.impl_trait_id(imp), LangItem::Clone)
                        && self.tcx.type_of(imp).instantiate_identity().skip_normalization() == ty
                });
                if crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty)
                    && (cloning || crate::lower::recognition::std_struct_steps_in(self.tcx, self.typing_env, ty))
                {
                    let steps = match cloning {
                        true => Expr::member(Expr::var(&state.locals.names[Local::from_u32(1)]), "$steps"),
                        false => self.std_iterator_steps(ty, span)?,
                    };
                    return Ok(Value::Expr(Expr::call_helper("$stdIterator", vec![value, steps])));
                }
                Ok(Value::Expr(value))
            }
            // A closure that's the function it calls, where it's made, that
            // function (`passed_on`, ADR 0605, amended); but an `Rc`'s, whose
            // pointer is its own (ADR 0959).
            AggregateKind::Closure(def_id, args) => {
                let ty = ty::Ty::new_closure(self.tcx, *def_id, args);
                let values = self.held_captures(state, *def_id, &operands, values, out)?;
                let arrow = self.mir_closure(state, ty, values, span, out)?;
                let taker = crate::lower::passed_on::Taker::Closure;
                let made = (!self.counted_closure(span))
                    .then(|| crate::lower::passed_on::passed_on(&arrow, taker, self.forwarding()))
                    .flatten();
                // An `Rc`'s, the function, only as the crate compares no
                // `Rc<dyn Fn>`: one a consumer that does compiles its own of
                // (#1052).
                if made.is_some() && self.given_rc_new(span) {
                    self.dependencies.borrow_mut().merges_identities = true;
                }
                Ok(Value::Expr(made.unwrap_or(arrow)))
            }
            // An `async` body, what it holds as a closure's environment.
            AggregateKind::Coroutine(def_id, args) if self.tcx.coroutine_is_async(*def_id) => {
                let ty = ty::Ty::new_coroutine(self.tcx, *def_id, args);
                Ok(Value::Expr(self.mir_closure(state, ty, values, span, out)?))
            }
            // An `async` closure, which makes its body each call.
            AggregateKind::CoroutineClosure(def_id, args) => {
                let ty = ty::Ty::new_coroutine_closure(self.tcx, *def_id, args);
                Ok(Value::Expr(self.mir_closure(state, ty, values, span, out)?))
            }
            _ => Err(self.unsupported(span, "this aggregate, from its MIR")),
        }
    }
}
