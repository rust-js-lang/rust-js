//! A combinator's closure: given what it doesn't use, called once, and
//! what's dropped of what it isn't run on.

use super::*;
use crate::drops::call_once::{OnceDrop, boxed_call_once_drop, call_once_drop};
use rustc_middle::mir;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Each argument a std call `known` is given by value with a destructor,
    /// a function or a value it may leave unused, of arguments of types
    /// `tys`, by its argument, with where it's dropped, `handled_argument`'s.
    /// A function what it gives holds, `map(f)`'s, is refused.
    pub fn given_unused(
        &self,
        known: Option<crate::recognition::Std>,
        (tys, output): (&[Ty<'tcx>], Ty<'tcx>),
        callee: Option<rustc_span::def_id::DefId>,
        span: Span,
    ) -> R<Vec<(usize, Option<Unused>)>> {
        // Of a boxed function, `Box<dyn Fn(..)>`, whose kind its type
        // doesn't say: its `dyn`'s, where `callee` calls what it's given, its
        // own parameter bound by an `Fn*`, `any<P: FnMut(..)>`'s; none where
        // it keeps it, `push(T)`'s (#1220).
        let inputs = callee.map(|callee| self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs());
        let called = |at: usize| {
            let (callee, inputs) = (callee?, inputs?);
            call_once_drop(self.tcx, callee, *inputs.get(at)?)?;
            boxed_call_once_drop(self.tcx, tys[at])
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
    pub(crate) fn handled_argument(
        &self,
        known: Option<crate::recognition::Std>,
        at: usize,
        ty: Ty<'tcx>,
        called: Option<OnceDrop>,
    ) -> Option<Option<Unused>> {
        let unused = || {
            (unused_arguments(known?).iter())
                .find(|&&(arg, _)| arg == at)
                .map(|&(_, when)| Some(when))
        };
        // `drop(f)` drops it, and `forget(f)` drops none of it, themselves.
        if matches!(
            known,
            Some(crate::recognition::Std::Drop | crate::recognition::Std::Forget)
        ) {
            return None;
        }
        // Or, of one whose type doesn't say, a boxed one's, as its callee
        // calls it (`given_unused`, #1220).
        let kind = call_once_drop(self.tcx, self.frame.owner, ty).or(called);
        let Some(kind) = kind else {
            return unused().filter(|_| self.drops_function(ty));
        };
        if !self.drops_function(ty) {
            return None;
        }
        match kind {
            OnceDrop::Body => unused(),
            OnceDrop::Whole => Some(None),
        }
    }

    /// Whether a function of type `ty`, called by `FnOnce` alone, which a
    /// std call that writes a place or a map's entry may leave unrun, is
    /// dropped there, as `get_or_insert_with`'s is (ADR 0469).
    /// Or a boxed one's, `Box<dyn FnOnce()>`'s, whose kind its type doesn't
    /// say, which such a call takes as an `F: FnOnce` (#1220).
    pub fn drops_unrun(&self, ty: Ty<'tcx>) -> bool {
        let kind = call_once_drop(self.tcx, self.frame.owner, ty);
        (kind == Some(OnceDrop::Body) || kind.is_none() && boxed_call_once_drop(self.tcx, ty).is_some())
            && self.drops_function(ty)
    }

    /// Each of a call's arguments, by its place, that's a boxed function
    /// called by `&` or `&mut`, `Box<dyn Fn()>`'s, holding a value with a
    /// destructor, given where `callee`, of `generic_args`, calls it by value
    /// and never by `&`, a type parameter it's given for
    /// (`ClosureFacts::param_called_once`): std's `call_once` of a `Box<F>`
    /// calls it and then drops the box, as it returns or unwinds, which the
    /// call of its JS function doesn't, so it's given as one that does,
    /// `$droppingOnce` (#2033). A closure's own call by value is refused
    /// where it's made (`ClosureFacts::called_by_value`, #2031).
    pub fn boxes_called_once(
        &self,
        (callee, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        tys: &[Ty<'tcx>],
    ) -> Vec<usize> {
        let facts = &self.krate.closure_facts;
        (tys.iter().enumerate())
            .filter(|&(_, &ty)| {
                call_once_drop(self.tcx, self.frame.owner, ty).is_none()
                    && boxed_call_once_drop(self.tcx, ty) == Some(OnceDrop::Whole)
            })
            .filter(|&(_, &ty)| {
                let ty = self.tcx.erase_and_anonymize_regions(ty);
                (generic_args.iter().enumerate()).any(|(index, arg)| {
                    arg.as_type().map(|arg| self.tcx.erase_and_anonymize_regions(arg)) == Some(ty)
                        && facts.param_called_once(self.tcx, callee, index as u32)
                })
            })
            .filter(|&(_, &ty)| self.drops_function(ty))
            .map(|(at, _)| at)
            .collect()
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
    pub fn dropped_unwinding(&mut self, given: &[(Option<Unused>, Expr, Ty<'tcx>)], span: Span) -> R<Vec<Stmt>> {
        let always = given.iter().filter(|(when, ..)| when.is_none()).cloned().collect();
        let mut dropped = Vec::new();
        self.dropped_given(None, always, span, &mut dropped)?;
        Ok(dropped)
    }

    /// Each argument `f`, of type `ty`, a std call was given, dropped where
    /// std drops it: as it returns, or where it's left unused, by the
    /// variant of its `subject` it's left on: `if (option == null) {
    /// loudDrop_drop(a); }`.
    pub fn dropped_given(
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The function whose call the closure `def_id` gives back, MIR's:
    /// its return place's one write a call's, `|i| try_hydrate_activity(fiber,
    /// i)`'s, or a copy of one; `None` of anything else.
    // helper: what a closure gives back the call of, `and_then_given`'s
    pub(crate) fn closure_returned_callee(
        &self,
        def_id: rustc_span::def_id::DefId,
    ) -> Option<rustc_span::def_id::DefId> {
        let body = &(self.krate.closures.get(&def_id)?.mir.as_ref())?.body;
        let mut at = mir::RETURN_PLACE;
        for _ in 0..4 {
            let mut calls = (body.basic_blocks.iter()).filter_map(|data| match &data.terminator().kind {
                mir::TerminatorKind::Call { func, destination, .. }
                    if destination.local == at && destination.projection.is_empty() =>
                {
                    Some(func)
                }
                _ => None,
            });
            let mut assigns = (body.basic_blocks.iter())
                .flat_map(|data| &data.statements)
                .filter_map(|statement| match &statement.kind {
                    mir::StatementKind::Assign(assign) if assign.0.local == at => Some(assign),
                    _ => None,
                });
            match (calls.next(), calls.next(), assigns.next()) {
                (Some(func), None, None) => return func.const_fn_def().map(|(callee, _)| callee),
                (None, None, Some(assign))
                    if assigns.next().is_none()
                        && assign.0.projection.is_empty()
                        && let mir::Rvalue::Use(mir::Operand::Copy(from) | mir::Operand::Move(from), _) = &assign.1
                        && from.projection.is_empty() =>
                {
                    at = from.local;
                }
                _ => return None,
            }
        }
        None
    }

    /// Whether what a call of `def_id` gives is `null` or a value: a
    /// `#[rust_js::nullable]` function's, a binding's too, its `None` `null`
    /// (ADRs 0578, 0638), or one of the crate's, or a closure, that returns
    /// so at each return (rd632, rd16): what an `and_then` of a closure
    /// gives where what it's of is `None`, `null` (`calls::and_then_given`,
    /// #2112 S4d).
    // helper: asked by `mir` and the base alike, the base's since #2299 moved it from `mir::nulled`
    pub fn returns_null_or_value(&self, def_id: rustc_span::def_id::DefId) -> bool {
        crate::calls::gives_null_for_none(self.tcx, def_id)
            || (crate::analysis::nulled_params::may_return_nulled(self.tcx, def_id)
                && (self.settled.nulled).returns_nulled(&mut self.nulled_seen.borrow_mut(), self.item, def_id))
    }

    /// `f`, an `and_then`'s closure `FoundIndexes::through` says gives a
    /// slice's `position`, `(e) => $position(DAYS, (d) => d === e)`: its
    /// value JS's own search of the array, `(e) => DAYS.indexOf(e)`, whose
    /// `-1` is its `None` (#2074).
    pub(crate) fn found_index_closure(&self, f: Expr, span: Span) -> R<Expr> {
        let js::ExprKind::Arrow(params, body) = f.kind else {
            return Err(self.unsupported(span, "this `and_then`'s closure of a slice's `position`, from its MIR"));
        };
        let [
            js::Stmt {
                kind: StmtKind::Return(Some(value)),
                span: at,
                ..
            },
        ] = body.as_slice()
        else {
            return Err(self.unsupported(span, "this `and_then`'s closure of a slice's `position`, from its MIR"));
        };
        let searched = self.found_index(value.clone(), span)?;
        Ok(Expr {
            kind: js::ExprKind::Arrow(params, vec![StmtKind::Return(Some(searched)).at(*at)]),
            ..f
        })
    }

    /// `call`, `$position(it, f)` of a `position` `FoundIndexes::call`
    /// says, as JS's own search of the array `it` is, lent, which its
    /// `iter()` gave as it is (`found_index_items`): `items.indexOf(x)`
    /// where `f` is `(c) => c === x`, else `items.findIndex(f)`.
    pub fn found_index(&self, call: Expr, span: Span) -> R<Expr> {
        let searched = match &call.kind {
            js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$position") => match &args[..] {
                [it, f] => Some((crate::iterators::unlent(it).clone(), f.clone())),
                _ => None,
            },
            _ => None,
        };
        let Some((items, f)) = searched else {
            return Err(self.unsupported(span, "this `position` of a slice's items, from its MIR"));
        };
        Ok(match crate::iterators::sought(&f) {
            Some(x) => Expr::builtin(items, "indexOf", vec![x]),
            None => Expr::call(Expr::member(items, "findIndex"), vec![f]),
        })
    }
}
