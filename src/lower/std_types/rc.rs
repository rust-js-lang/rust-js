//! A counted `Rc` or `Arc` (ADR 0320): `{ value, strong, weak }`, which its
//! clones and `Weak`s share, where the crate reads its counts. Every other
//! is the value it points at (ADR 0023).

use rustc_middle::ty::{self, GenericArgsRef, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::recognition::{rc_pointee, weak_pointee};
use crate::lower::{FnCx, R};

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

    /// What a counted `Weak` points at, if `ty` is one: a `Weak` of a
    /// counted `Rc`'s. Every other is what its `Rc` is, the value itself,
    /// or `null` of a `Weak::new()` (ADR 0843).
    pub(in crate::lower) fn counted_weak(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        weak_pointee(self.tcx, ty).filter(|&pointee| self.krate.counted.counts(self.tcx, pointee))
    }

    /// Refused, a binding's call of a counted `Rc` or `Weak` among its type
    /// arguments, `WeakMap<K, Rc<T>>::get` (ADR 0850): JS's value is the
    /// counted box, which what JS gives back a Rust owner of, `get`'s
    /// `Rc`, would drop without the clone that counted it. An uncounted one
    /// is the value it points at, which JS holds as Rust shares it. And of a
    /// `RefCell` that's its `{ value }` box, not the object it holds, as
    /// `ref_cells` says (ADR 0962).
    pub(in crate::lower) fn check_given_to_js(
        &self,
        def_id: DefId,
        generic_args: GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<()> {
        if !crate::lower::bindings::is_binding(self.tcx, def_id) {
            return Ok(());
        }
        let refused = |what: String, why: &str| {
            Err(crate::lower::refusal::refuse(
                self.tcx,
                span,
                crate::lower::refusal::Refused::Because(&format!("{what} given to JS or taken from it"), why),
            ))
        };
        let counted = (generic_args.types())
            .flat_map(|ty| ty.walk())
            .filter_map(|part| part.as_type())
            .find(|&part| self.counted_rc(part).is_some() || self.counted_weak(part).is_some());
        if let Some(rc) = counted {
            return refused(
                format!("a counted `{rc}`"),
                "JS holds the box that counts it, which counts no clone JS makes; an `Rc` is the value it points at where the crate reads no count and drops nothing it points at (ADR 0844)",
            );
        }
        // A `RefCell` JS holds that's its `{ value }` box, not the object it
        // holds; not one a function JS is given captures, which JS holds
        // only the function of (ADR 0962).
        let boxed = generic_args.types().find_map(|ty| {
            let mut walk = ty.walk();
            while let Some(part) = walk.next() {
                let Some(part) = part.as_type() else { continue };
                match part.kind() {
                    ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..) | ty::Dynamic(..) | ty::Coroutine(..) => {
                        walk.skip_current_subtree()
                    }
                    ty::Adt(adt, _)
                        if crate::lower::recognition::is_std_def(
                            self.tcx,
                            adt.did(),
                            crate::lower::recognition::StdItem::RefCell,
                        ) && !self.is_value_cell(part) =>
                    {
                        return Some(part);
                    }
                    _ => {}
                }
            }
            None
        });
        if let Some(cell) = boxed {
            return refused(
                format!("a `{cell}`"),
                "JS would hold its `{ value }` box, not what it holds; a `RefCell` is the object it holds where that's an array, a map, a set or a struct's object no generic code or `dyn` boxes (ADR 0863)",
            );
        }
        Ok(())
    }

    /// What an uncounted `Weak` points at, if `ty` is one (ADR 0843).
    pub(in crate::lower) fn uncounted_weak(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        weak_pointee(self.tcx, ty).filter(|&pointee| !self.krate.counted.counts(self.tcx, pointee))
    }

    /// `Weak::new()`, and `Weak`'s `Default`, of a `Weak` type `weak`: a
    /// counted one's, `$weakNone`, the one each is, so two are `ptr_eq`;
    /// or an uncounted one's `null` (ADR 0843).
    pub(in crate::lower) fn weak_to_nothing(&self, weak: Ty<'tcx>) -> Expr {
        match self.counted_weak(weak) {
            Some(_) => Expr::helper("$weakNone"),
            None => Expr::null(),
        }
    }

    /// `Rc::new(value)` of a counted one: `{ value, strong: 1, weak: 0 }`.
    pub(in crate::lower) fn new_rc(value: Expr) -> Expr {
        Expr::object(vec![
            Prop::Field(crate::js::HELD.into(), value),
            Prop::Field("strong".into(), Expr::int(1)),
            Prop::Field("weak".into(), Expr::int(0)),
        ])
    }

    /// `Rc::make_mut(&mut place)` of an `Rc` of `item`: the place given the
    /// one `$makeMut` gives, and what it points to (ADR 0364).
    pub(in crate::lower) fn make_mut_values(
        &mut self,
        place: Expr,
        item: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut given = vec![place.clone(), self.clone_arg(item, span)?];
        given.extend(self.drop_function(item, span)?);
        let made = Expr::call(Expr::helper("$makeMut"), given);
        out.push(StmtKind::Assign(place.clone(), made).at(self.js_span(span)));
        Ok(self.through_rc(place, item))
    }

    /// An `Rc`'s or a `Weak`'s function, of its arguments' values, `tys`
    /// their types, giving an `output` (ADR 0364).
    pub(in crate::lower) fn rc_values(
        &mut self,
        op: RcOp,
        (values, tys, output): (Vec<Expr>, &[Ty<'tcx>], Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let pointee = tys.first().and_then(|first| {
            let first = first.peel_refs();
            self.counted_rc(first).or(self.counted_weak(first))
        });
        let mut values = values.into_iter();
        let helper = |name: &'static str, args: Vec<Expr>| Expr::call_helper(name, args);
        Ok(match op {
            // An uncounted `Weak` is what its `Rc` is, the value itself, and
            // one to nothing `null`: nothing asks whether it's alive, so it
            // needs no count; `Weak::ptr_eq` is `===` of either (ADR 0843).
            RcOp::Downgrade if self.uncounted_weak(output).is_some() => values.next().expect("an `Rc`"),
            RcOp::WeakNew => self.weak_to_nothing(output),
            RcOp::NewCyclic => {
                let f = values.next().expect("a function");
                helper("$newCyclic", vec![f])
            }
            RcOp::MakeMut => return Err(self.unsupported(span, "`Rc::make_mut`, of its value alone")),
            // The same box, a counted one's, or an uncounted one's JS object,
            // whose `===` is its identity (ADR 0582).
            RcOp::PtrEq => {
                let (rc, other) = (values.next().expect("an `Rc`"), values.next().expect("another"));
                Expr::bin(Op::Eq, rc, other)
            }
            _ => {
                let rc = values.next().expect("an `Rc`");
                // A `&mut` to one is its place's box (ADR 0523).
                let rc = match tys.first().map(|t| t.kind()) {
                    Some(ty::Ref(_, _, ty::Mutability::Mut)) => Expr::held(rc),
                    _ => rc,
                };
                let item = pointee.expect("a counted `Rc` or a `Weak`");
                match op {
                    RcOp::PtrEq => unreachable!("an arm of its own"),
                    RcOp::StrongCount | RcOp::WeakStrongCount => Expr::member(rc, "strong"),
                    RcOp::WeakCount => Expr::member(rc, "weak"),
                    RcOp::WeakWeakCount => helper("$weakCount", vec![rc]),
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
    /// What a counted `Rc` of `pointee` points at, as JS reads it: its
    /// `value`; of a `dyn`'s, its pair, the `Rc` itself, `{ value, impl,
    /// strong, weak }` (ADR 0520). The one answer, of each that reads it.
    pub(in crate::lower) fn counted_pointee(&self, rc: Expr, pointee: Ty<'tcx>) -> Expr {
        match pointee.kind() {
            ty::Dynamic(predicates, ..) if self.recognition().is_pair_dyn(predicates) => rc,
            _ => Expr::held(rc),
        }
    }

    fn through_rc(&self, rc: Expr, item: Ty<'tcx>) -> Expr {
        match self.is_boxable(item) {
            true => rc,
            false => Expr::held(rc),
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
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
            false => self.counted_pointee(value, pointee),
        })
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

    /// A value as `{}` and `{:?}` show it: through its references (ADR
    /// 0099), and its counted `Rc`s, guards and pins, `through_boxes`'s.
    pub(in crate::lower) fn shown_through(&self, value: Expr, ty: Ty<'tcx>) -> (Expr, Ty<'tcx>) {
        let (value, ty) = self.through_refs(value, ty);
        self.through_boxes(value, ty)
    }

    /// What a counted `Rc` points at, or a guard guards, through any others:
    /// its `value`, as `{}` and `{:?}` show it (ADRs 0320, 0328), and what a
    /// `Pin`'s pointer does (ADR 0329); a `Box`'s or another `Rc`'s, which is
    /// it in JS (ADR 0023). Any other value is itself.
    pub(in crate::lower) fn through_boxes(&self, mut value: Expr, mut ty: Ty<'tcx>) -> (Expr, Ty<'tcx>) {
        loop {
            if let Some(pointer) = self.recognition().pinned(ty) {
                (value, ty) = self.through_refs(value, pointer);
                continue;
            }
            let inside = match ty.kind() {
                ty::Adt(_, args) if self.is_guard(ty) => args.types().next().expect("what it guards"),
                ty::Adt(_, args) => match self.counted_rc(ty) {
                    Some(pointee) => pointee,
                    None if self.shows_inside(ty) => {
                        (value, ty) = self.through_refs(value, args.type_at(0));
                        continue;
                    }
                    None => return (value, ty),
                },
                _ => return (value, ty),
            };
            // A guard's `value`, an `Rc`'s what it points at (ADR 0520); a
            // reference it holds, an `Rc<&T>`'s, is shown as references are,
            // through it (ADR 0504, #228). A guard of a `RefCell` of an
            // object is the object (ADR 0863).
            value = match self.is_guard(ty) {
                true if self.guards_value_cell(ty) => value,
                true => Expr::held(value),
                false => self.counted_pointee(value, inside),
            };
            ty = inside;
        }
    }
}
