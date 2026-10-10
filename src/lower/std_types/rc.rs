//! A counted `Rc` or `Arc` (ADR 0320): `{ value, strong, weak }`, which its
//! clones and `Weak`s share, where the crate reads its counts. Every other
//! is the value it points at (ADR 0023).

use rustc_middle::thir::ExprId;
use rustc_middle::ty::{self, GenericArgsRef, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::recognition::{rc_pointee, weak_pointee};
use crate::lower::{FnCx, R, fn_def};
use crate::runtime::Helper;

/// What only a counted `Rc` or its `Weak` answers.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum RcOp {
    StrongCount,
    WeakCount,
    PtrEq,
    Downgrade,
    GetMut,
    MakeMut,
    TryUnwrap,
    IntoInner,
    UnwrapOrClone,
    NewCyclic,
    WeakNew,
    Upgrade,
    WeakStrongCount,
    WeakWeakCount,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a counted `Rc` points at, if `ty` is one.
    pub(in crate::lower) fn counted_rc(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        rc_pointee(self.tcx, ty).filter(|&pointee| self.krate.counted.counts(self.tcx, pointee))
    }

    /// What a `Weak` points at, if `ty` is one: always counted.
    pub(in crate::lower) fn weak_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        weak_pointee(self.tcx, ty)
    }

    /// `Rc::new(value)` of a counted one: `{ value, strong: 1, weak: 0 }`.
    pub(in crate::lower) fn new_rc(value: Expr) -> Expr {
        Expr::object(vec![
            Prop::Field("value".into(), value),
            Prop::Field("strong".into(), Expr::int(1)),
            Prop::Field("weak".into(), Expr::int(0)),
        ])
    }

    /// One of `RcOp`'s, of a counted `Rc` or a `Weak`.
    pub(in crate::lower) fn rc_call(&mut self, op: RcOp, args: &[ExprId], span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let first = args.first().map(|&first| self.thir[first].ty);
        // `make_mut` changes which `Rc` its place holds: the place is
        // given the one `$makeMut` gives.
        if op == RcOp::MakeMut {
            let item = first
                .and_then(|first| self.counted_rc(first.peel_refs()))
                .expect("a counted `Rc`");
            self.counted_here(item, span)?;
            self.runtime.insert(Helper::Rc);
            let Some((place, _)) = self.ref_place(args[0]) else {
                return Err(self.unsupported(span, "`Rc::make_mut` of an `Rc` that isn't a place here"));
            };
            let mut given = vec![place.clone(), self.clone_arg(item, span)?];
            given.extend(self.drop_function(item, span)?);
            let made = Expr::call(Expr::var("$makeMut"), given);
            out.push(StmtKind::Assign(place.clone(), made).at(self.js_span(span)));
            return Ok(self.through_rc(place, item));
        }
        let values = self.operands(args, out)?;
        let tys: Vec<Ty<'tcx>> = args.iter().map(|&a| self.thir[a].ty).collect();
        self.rc_values(op, (values, &tys), span, out)
    }

    /// An `Rc`'s or a `Weak`'s function, of its arguments' values, `tys`
    /// their types: what THIR and MIR both lower it to (ADR 0364).
    pub(in crate::lower) fn rc_values(
        &mut self,
        op: RcOp,
        (values, tys): (Vec<Expr>, &[Ty<'tcx>]),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let pointee = tys.first().and_then(|first| {
            let first = first.peel_refs();
            self.counted_rc(first).or(self.weak_of(first))
        });
        let mut values = values.into_iter();
        if let Some(pointee) = pointee {
            self.counted_here(pointee, span)?;
        }
        self.runtime.insert(Helper::Rc);
        let helper = |name: &str, args: Vec<Expr>| Expr::call(Expr::var(name), args);
        Ok(match op {
            RcOp::WeakNew => Expr::object(vec![
                Prop::Field("strong".into(), Expr::int(0)),
                Prop::Field("weak".into(), Expr::int(0)),
            ]),
            RcOp::NewCyclic => {
                let f = values.next().expect("a function");
                helper("$newCyclic", vec![f])
            }
            RcOp::MakeMut => return Err(self.unsupported(span, "`Rc::make_mut`, of its value alone")),
            _ => {
                let rc = values.next().expect("an `Rc`");
                let item = pointee.expect("a counted `Rc` or a `Weak`");
                match op {
                    RcOp::StrongCount | RcOp::WeakStrongCount => Expr::member(rc, "strong"),
                    RcOp::WeakCount => Expr::member(rc, "weak"),
                    RcOp::WeakWeakCount => helper("$weakCount", vec![rc]),
                    RcOp::PtrEq => {
                        let other = values.next().expect("another");
                        Expr::bin(Op::Eq, rc, other)
                    }
                    RcOp::Downgrade => helper("$downgrade", vec![rc]),
                    RcOp::Upgrade => helper("$upgrade", vec![rc]),
                    RcOp::TryUnwrap => helper("$tryUnwrap", vec![rc]),
                    RcOp::IntoInner => helper("$intoInner", vec![rc]),
                    RcOp::UnwrapOrClone => {
                        let mut given = vec![rc, self.clone_arg(item, span)?];
                        given.extend(self.drop_function(item, span)?);
                        helper("$unwrapOrClone", given)
                    }
                    // `Some` of a `&mut` to what it points at, if it's the only
                    // `Rc` and nothing's weak to it.
                    RcOp::GetMut if self.boxed_payload(item) => {
                        return Err(self.unsupported(span, &format!("`Rc::get_mut` of an `Rc<{item}>`")));
                    }
                    RcOp::GetMut => {
                        let rc = if rc.reads_same() { rc } else { self.spill("rc", rc, out) };
                        let only = Expr::bin(
                            Op::And,
                            Expr::bin(Op::Eq, Expr::member(rc.clone(), "strong"), Expr::int(1)),
                            Expr::bin(Op::Eq, Expr::member(rc.clone(), "weak"), Expr::int(0)),
                        );
                        let given = self.through_rc(rc, item);
                        Expr::cond(only, given, Expr::undefined())
                    }
                    RcOp::WeakNew | RcOp::NewCyclic | RcOp::MakeMut => unreachable!("lowered above"),
                }
            }
        })
    }

    /// A `&mut` to what a counted `Rc` points at: the object itself, or for
    /// a number or text, the `Rc`, whose `value` it is (ADR 0074).
    fn through_rc(&self, rc: Expr, item: Ty<'tcx>) -> Expr {
        match self.is_boxable(item) {
            true => rc,
            false => Expr::member(rc, "value"),
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A counted `Rc` of `pointee` here, which only this crate counts: not
    /// one another crate may hold, a library's, or this one's if it's a
    /// library, which would be its value there.
    pub(in crate::lower) fn counted_here(&self, pointee: Ty<'tcx>, span: Span) -> R<()> {
        match self.krate.library || self.crosses_crates(pointee) {
            true => Err(self.unsupported(span, &format!("a counted `Rc<{pointee}>` another crate may share"))),
            false => Ok(()),
        }
    }

    /// `Rc::new(x)` or `*rc` of a counted `Rc`, each `Std::Same` of one that
    /// isn't: `{ value: x, strong: 1, weak: 0 }`, or `rc.value`.
    pub(in crate::lower) fn counted_same(
        &self,
        def_id: DefId,
        generic_args: GenericArgsRef<'tcx>,
        value: Expr,
    ) -> Option<Expr> {
        let (new, pointee) = self.recognition().rc_same(def_id, generic_args)?;
        self.krate.counted.counts(self.tcx, pointee).then(|| match new {
            true => Self::new_rc(value),
            false => Expr::member(value, "value"),
        })
    }

    /// Is `fun` `Rc::new`, `Some(true)`, or an `Rc`'s deref, `Some(false)`, of a
    /// counted one?
    pub(in crate::lower) fn counted_same_of(&self, fun: ExprId) -> Option<bool> {
        let (def_id, generic_args) = fn_def(self.thir[self.strip(fun)].ty)?;
        self.counted_same_by(def_id, generic_args)
    }

    /// `counted_same_of`, of the function `def_id` of `generic_args`.
    pub(in crate::lower) fn counted_same_by(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<bool> {
        let (new, pointee) = self.recognition().rc_same(def_id, generic_args)?;
        self.krate.counted.counts(self.tcx, pointee).then_some(new)
    }

    /// What a counted `Rc` points at, or a guard guards, through any others:
    /// its `value`, as `{}` and `{:?}` show it (ADRs 0320, 0328), and what a
    /// `Pin`'s pointer does (ADR 0329). Any other value is itself.
    pub(in crate::lower) fn through_boxes(&self, mut value: Expr, mut ty: Ty<'tcx>) -> (Expr, Ty<'tcx>) {
        loop {
            if let Some(pointer) = self.recognition().pinned(ty) {
                (value, ty) = self.through_refs(value, pointer);
                continue;
            }
            let inside = match ty.kind() {
                ty::Adt(_, args) if self.is_guard(ty) => args.types().next().expect("what it guards"),
                _ => match self.counted_rc(ty) {
                    Some(pointee) => pointee,
                    None => return (value, ty),
                },
            };
            value = Expr::member(value, "value");
            ty = inside;
        }
    }
}
