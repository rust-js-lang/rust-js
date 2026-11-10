//! A combinator's closure: given what it doesn't use, called once, and
//! what's dropped of what it isn't run on.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Each argument a std call `known` is given by value with a destructor,
    /// a function or a value it may leave unused, of arguments of types
    /// `tys`, by its argument, with where it's dropped, `handled_argument`'s.
    /// A function what it gives holds, `map(f)`'s, is refused.
    pub(in crate::lower) fn given_unused(
        &self,
        known: Option<crate::lower::recognition::Std>,
        (tys, output): (&[Ty<'tcx>], Ty<'tcx>),
        callee: Option<rustc_span::def_id::DefId>,
        span: Span,
    ) -> R<Vec<(usize, Option<Unused>)>> {
        // How `callee` calls each it's given, by its own parameter's `Fn*`
        // bound, `any<P: FnMut(..)>`'s: what a boxed one's kind is, which its
        // type doesn't say, `Box<dyn FnMut(..)>`'s; none of one it keeps,
        // `push(T)`'s (#1220).
        let inputs = callee.map(|callee| self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs());
        let called = |at: usize| {
            let (callee, inputs) = (callee?, inputs?);
            crate::lower::recognition::function_kind(self.tcx, callee, *inputs.get(at)?)
        };
        // What it gives as the type it stands for, as each argument's is:
        // `Rc::new(f)`'s `Rc<impl Fn>` of a function that gives it back is
        // the `Rc` of `f`'s closure, which keeps it (#464).
        let mut given = Vec::new();
        for (at, &ty) in tys.iter().enumerate() {
            // What an `impl FnMut` a function gives is, its closure, which
            // carries its drop (#259).
            let Some(when) = self.handled_argument(known, at, ty, called(at)) else {
                continue;
            };
            // Kept by a lazy adapter it's given to, `map(f)`'s, it drops with it
            // (ADR 0485); by anything else, refused.
            if when.is_none() && self.recognition().held_function(output) == Some(ty) {
                continue;
            }
            // Kept by what drops it as it drops itself, `Box::new(f)`'s, of
            // one whose drop runs wherever it's dropped: a type parameter's,
            // given, a `dyn`'s, or one that carries it (ADRs 0484, 0521); or
            // of one boxed to hand to JS, which Rust never drops (ADR 0873).
            let facts = &self.krate.closure_facts;
            let dropped_anywhere = match ty.kind() {
                ty::Param(_) | ty::Dynamic(..) => true,
                ty::Closure(id, _) => facts.carries_its_drop(self.tcx, *id) || facts.handed_to_js(*id),
                _ => false,
            };
            let kept = output.walk().any(|part| part.as_type() == Some(ty));
            if when.is_none() && kept && dropped_anywhere {
                continue;
            }
            // Kept by an adapter closed to drop it, `scan(f)`'s, which drops
            // it as it's closed (`closed_on_drop`, #760).
            if when.is_none()
                && kept
                && (self.recognition().closed_on_drop(output))
                    .is_some_and(|held| held.walk().any(|part| part.as_type() == Some(ty)))
            {
                continue;
            }
            if when.is_none() && kept {
                return Err(self.unsupported(
                    span,
                    "a function that holds a value with a destructor, kept by what it's given to",
                ));
            }
            given.push((at, when));
        }
        Ok(given)
    }

    /// Where an argument std is given by value, `at` of `known`, of type
    /// `ty`, that holds a value with a destructor, is dropped, as std drops
    /// it: on the variant `known` leaves it unused on, a function called by
    /// `FnOnce` alone, whose body drops it where it's run (ADRs 0469, 0470),
    /// or a value, which it keeps where it's used (ADR 0490); a function
    /// called by `&` or `&mut`, as the call returns, `Some(None)`. `None` of
    /// one with nothing to drop, or that's left as it was, run once or kept
    /// by its call.
    pub(in crate::lower) fn handled_argument(
        &self,
        known: Option<crate::lower::recognition::Std>,
        at: usize,
        ty: Ty<'tcx>,
        called: Option<ty::ClosureKind>,
    ) -> Option<Option<Unused>> {
        let unused = || {
            (unused_arguments(known?).iter())
                .find(|&&(arg, _)| arg == at)
                .map(|&(_, when)| Some(when))
        };
        // `drop(f)` drops it, and `forget(f)` drops none of it, themselves.
        if matches!(
            known,
            Some(crate::lower::recognition::Std::Drop | crate::lower::recognition::Std::Forget)
        ) {
            return None;
        }
        // Or, of one whose type doesn't say, a boxed one's, as its callee
        // calls it (`given_unused`, #1220).
        let kind = crate::lower::recognition::function_kind(self.tcx, self.frame.owner, ty).or(called);
        let Some(kind) = kind else {
            return unused().filter(|_| self.drops_function(ty));
        };
        if !self.drops_function(ty) {
            return None;
        }
        match kind {
            ty::ClosureKind::FnOnce => unused(),
            ty::ClosureKind::Fn | ty::ClosureKind::FnMut => Some(None),
        }
    }

    /// A function called by `&` or `&mut` that holds a value with a
    /// destructor, given where `callee` calls it by `FnOnce` alone, as its
    /// argument: refused, as that call drops none of it, where Rust's drops
    /// it once it's run (ADR 0470); but a closure made where it's given to a
    /// binding that calls it once, whose body drops what it took as its
    /// call ends (`ClosureFacts::called_once`, ADR 1020).
    pub(in crate::lower) fn check_called_once(
        &self,
        callee: rustc_span::def_id::DefId,
        tys: &[Ty<'tcx>],
        span: Span,
    ) -> R<()> {
        let inputs = self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        let tcx = self.tcx;
        let once = |input: Ty<'tcx>| {
            crate::lower::recognition::function_kind(tcx, callee, input) == Some(ty::ClosureKind::FnOnce)
        };
        // Or one whose own bound takes such a one as it is, `impl Is<F>` of
        // an `F: FnOnce()`, whose value is that one's (#966).
        let through = |input: Ty<'tcx>| {
            (tcx.clauses_of(callee).instantiate_identity(tcx).into_iter())
                .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
                .map(|bound| bound.skip_binder().trait_ref)
                .filter(|tr| tr.self_ty() == input && tcx.fn_trait_kind_from_def_id(tr.def_id).is_none())
                .any(|tr| {
                    tr.args
                        .types()
                        .skip(1)
                        .any(|arg| matches!(arg.kind(), ty::Param(_)) && once(arg))
                })
        };
        for (&ty, &input) in tys.iter().zip(inputs) {
            let kind = crate::lower::recognition::function_kind(self.tcx, self.frame.owner, ty);
            if kind.is_some_and(|kind| kind != ty::ClosureKind::FnOnce)
                && !matches!(*ty.kind(), ty::Closure(id, _) if self.krate.closure_facts.called_once(id))
                && (once(input) || matches!(input.kind(), ty::Param(_)) && through(input))
                && self.drops_function(ty)
            {
                let name = self.tcx.item_name(callee);
                return Err(self.unsupported(
                    span,
                    &format!("giving `{name}`, which calls it once, a function called by `&` that holds a value with a destructor"),
                ));
            }
        }
        Ok(())
    }

    /// Whether a function of type `ty`, called by `FnOnce` alone, which a
    /// std call that writes a place or a map's entry may leave unrun, is
    /// dropped there, as `get_or_insert_with`'s is (ADR 0469).
    /// Or a boxed one's, `Box<dyn FnOnce()>`'s, whose kind its type doesn't
    /// say, which such a call takes as an `F: FnOnce` (#1220).
    pub(in crate::lower) fn drops_unrun(&self, ty: Ty<'tcx>) -> bool {
        let boxed = ty.boxed_ty().is_some_and(|inner| {
            inner.is_closure()
                || matches!(inner.kind(), ty::Dynamic(predicates, ..)
                    if predicates.principal_def_id().is_some_and(|p| self.tcx.fn_trait_kind_from_def_id(p).is_some()))
        });
        let kind = crate::lower::recognition::function_kind(self.tcx, self.frame.owner, ty);
        (kind == Some(ty::ClosureKind::FnOnce) || kind.is_none() && boxed) && self.drops_function(ty)
    }

    /// Whether a function, or a value, of type `ty` std is given is dropped
    /// where std drops it: one that holds a value with a destructor, or a
    /// type parameter's whose drop this function is given. Of one it isn't
    /// given, its callers give none with a destructor (ADR 0190).
    fn drops_function(&self, ty: Ty<'tcx>) -> bool {
        if self.drops(ty) == Drops::Nothing {
            return false;
        }
        // Of what holds only type parameters' values: its own drop given, or
        // theirs, a closure's of what it took (ADRs 0485, 0496).
        if self.drop_query().drops_but_params(ty) == Drops::Nothing
            && self.given_drop(ty).is_none()
            && !self.holds_given_drops(ty)
        {
            self.require_no_drops(ty);
            return false;
        }
        true
    }

    /// What of `given`, a std call's arguments `dropped_given` drops, it
    /// drops as a panic in the call unwinds: what it drops as it returns,
    /// `when` none, a function it calls by `&mut` or `&`, a fold's
    /// (`handled_argument`), which std's call took and drops as it unwinds
    /// (#719); not one called by `FnOnce`, whose body drops what it took.
    pub(in crate::lower) fn dropped_unwinding(
        &mut self,
        given: &[(Option<Unused>, Expr, Ty<'tcx>)],
        span: Span,
    ) -> R<Vec<Stmt>> {
        let always = given.iter().filter(|(when, ..)| when.is_none()).cloned().collect();
        let mut dropped = Vec::new();
        self.dropped_given(None, always, span, &mut dropped)?;
        Ok(dropped)
    }

    /// Each argument `f`, of type `ty`, a std call was given, dropped where
    /// std drops it: as it returns, or where it's left unused, by the
    /// variant of its `subject` it's left on: `if (option == null) {
    /// loudDrop_drop(a); }`.
    pub(in crate::lower) fn dropped_given(
        &mut self,
        subject: Option<Expr>,
        given: Vec<(Option<Unused>, Expr, Ty<'tcx>)>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let js_span = self.js_span(span);
        for (when, f, ty) in given {
            let mut dropped = Vec::new();
            self.drop_value(f, ty, span, &mut dropped)?;
            let Some(when) = when else {
                out.extend(dropped);
                continue;
            };
            let subject = subject.clone().expect("the subject it's left unrun by");
            let test = match when {
                Unused::None => Expr::bin(Op::LooseEq, subject, Expr::null()),
                Unused::Some => Expr::bin(Op::LooseNe, subject, Expr::null()),
                Unused::Ok => result::is(subject, true),
                Unused::Err => result::is(subject, false),
                Unused::False => Expr::unary(js::UnaryOp::Not, subject),
            };
            out.push(StmtKind::If(test, dropped, None).at(js_span));
        }
        Ok(())
    }
}
