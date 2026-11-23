//! Rust's dispatch of generic code's call of an `Iterator`'s provided
//! method an impl not std's writes, on the steps it holds (#2414, #2421):
//! where the call is the impl's own, else what `mir_call_values_of` makes.

use super::*;
use rust_js_lower::temporary_names::Role;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// `mir_call_values_of`'s, or, of generic code's call of `Iterator`'s
    /// provided method an impl not std's writes (`provided_dispatches`),
    /// Rust's dispatch, `provided_dispatched`'s. The one place every call
    /// MIR lowers asks.
    pub(super) fn mir_call_values_dispatched(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        call: (Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        if self.tcx.trait_of_assoc(def_id).is_some()
            && let Some(self_ty) = generic_args.types().next()
            && self.provided_dispatches(def_id, self_ty)
        {
            return self.provided_dispatched(state, (def_id, generic_args), call, output, span, out);
        }
        self.mir_call_values_of(state, (def_id, generic_args), call, output, span, out)
    }

    /// Generic code's call of `Iterator`'s provided method `def_id` that an
    /// impl not std's writes (`provided_dispatches`): `$provided(steps,
    /// "own<Name>", [..], () => { ..; return v; })`, the impl's own the steps
    /// carry where it writes one, else what std's lowering makes of the call,
    /// `mir_call_values_undispatched`'s, statements and all, what it drops
    /// after too, made only then (#2414, #2421). What it's given is held
    /// first where it doesn't read the same, as both read it.
    fn provided_dispatched(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (values, arg_tys, nulled): (Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        self.flush(state, out)?;
        let mut held = Vec::with_capacity(values.len());
        for (at, value) in values.into_iter().enumerate() {
            held.push(match value {
                Value::Ref(place) if place.reads_same() => Value::Ref(place),
                value => {
                    let e = self.value_expr(value, span)?;
                    // The iterator, named as what consumes one holds it.
                    let role = if at == 0 { Role::Iter } else { Role::Arg };
                    match e.reads_same() {
                        true => Value::Expr(e),
                        false => Value::Expr(self.spill(role, e, out)),
                    }
                }
            });
        }
        // A `&mut Self` it's given as a value, not `Self`'s place, a
        // `by_ref()`'s: what that lends, `$referent`'s, whose own Rust's
        // `I::position` runs; never where `Self` is the `&mut` itself, whose
        // own is std's default over `&mut I` (#2421).
        let lent = matches!(arg_tys.first().map(|ty| ty.kind()),
            Some(ty::Ref(_, referent, ty::Mutability::Mut)) if *referent == generic_args.type_at(0));
        let receiver = match &held[0] {
            Value::Ref(place) => place.clone(),
            value if lent => Expr::call_helper("$referent", vec![self.value_expr(value.clone(), span)?]),
            value => self.value_expr(value.clone(), span)?,
        };
        let steps = self.provided_steps(receiver, generic_args.type_at(0));
        let given = (held[1..].iter().cloned())
            .map(|value| self.value_expr(value, span))
            .collect::<R<Vec<_>>>()?;
        let mut made = Vec::new();
        let call = (held, arg_tys, nulled);
        let value = self.mir_call_values_of(state, (def_id, generic_args), call, output, span, &mut made)?;
        let value = self.value_expr(value, span)?;
        self.flush(state, &mut made)?;
        let called = self.provided_on_steps((def_id, generic_args), steps, given, (made, value), span)?;
        Ok(Value::Expr(called))
    }
}
