//! What rust-js makes in turn, a derived or std `Clone`'s and `Default`'s
//! parts and a clone's items: whether making a part may panic, and which
//! parts made before it are dropped as it unwinds, `$madeInTurn` and
//! `$madeEach` (ADR 1102, #897, #902).

use crate::js::{self, Expr, StmtKind};
use crate::lower::drops::Drops;
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a clone of `ty` may panic: a `Copy` value's never does, nor
    /// an `Rc`'s or a `Weak`'s, which counts one more; a hand-written
    /// `Clone`'s may, as its impl's methods may unwind, all but the crate's
    /// that never do (ADR 0902); a derived one's, as its fields' do; std's
    /// other types', a tuple's and an array's, as what they hold's do, and
    /// a closure's as its captures'; a type parameter's, a `dyn`'s or an
    /// opaque type's, anyone's (#708, #897). The one answer, which what
    /// clones as it steps and what's made in turn ask.
    pub(in crate::lower) fn clone_may_panic(&self, ty: Ty<'tcx>) -> bool {
        self.clone_may_panic_in(ty, &mut Vec::new())
    }

    fn clone_may_panic_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if self.is_copy(ty) || seen.contains(&ty) {
            return false;
        }
        if let ty::Param(_) | ty::Dynamic(..) | ty::Alias(..) = ty.kind() {
            return true;
        }
        seen.push(ty);
        let tr = ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]);
        let may = match ty.kind() {
            _ if let Some(imp) = self.recognition().user_impl(tr) => self.impl_may_unwind(imp),
            ty::Adt(..) if self.is_rc(ty) || self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some() => false,
            ty::Adt(adt, args) if self.krate.foreign.compiled_by_rust_js(adt.did()) => {
                (adt.all_fields()).any(|f| self.clone_may_panic_in(self.field_ty(f, args), seen))
            }
            ty::Adt(_, args) => args.types().any(|arg| self.clone_may_panic_in(arg, seen)),
            ty::Tuple(parts) => parts.iter().any(|part| self.clone_may_panic_in(part, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.clone_may_panic_in(*item, seen),
            ty::Closure(_, args) => {
                (args.as_closure().upvar_tys().iter()).any(|upvar| self.clone_may_panic_in(upvar, seen))
            }
            _ => false,
        };
        seen.pop();
        may
    }

    /// Which of a value's parts, made in turn, each its type and whether
    /// making it may panic, a later part's making may unwind past, so that
    /// it's dropped as that one unwinds: each made before the last whose
    /// making may panic, of a type with a destructor. As Rust drops an
    /// aggregate's operands made so far, the latest first, as a later one
    /// unwinds: a derived `Clone`'s or `Default`'s fields, a tuple's, an
    /// array's `Default`'s items (#897, #902).
    pub(super) fn held_in_turn(&self, parts: &[(Ty<'tcx>, bool)]) -> Vec<bool> {
        let last = parts.iter().rposition(|&(_, may_panic)| may_panic);
        (parts.iter().enumerate())
            .map(|(i, &(ty, _))| last.is_some_and(|last| i < last) && self.drops(ty) != Drops::Nothing)
            .collect()
    }

    /// Each of a value's `parts`, its type and whether making it may panic,
    /// made by `make` in turn, each `held_in_turn` says is held given to
    /// `made`, and `made`'s name where any is, which `in_turn` wraps the
    /// value in.
    pub(super) fn parts_in_turn(
        &mut self,
        parts: &[(Ty<'tcx>, bool)],
        span: Span,
        mut make: impl FnMut(&mut Self, usize) -> R<Expr>,
    ) -> R<(Vec<Expr>, Option<String>)> {
        let held = self.held_in_turn(parts);
        let made = held.contains(&true).then(|| self.fresh("made"));
        let mut out = Vec::new();
        for (i, &(ty, _)) in parts.iter().enumerate() {
            let part = make(self, i)?;
            out.push(match (&made, held[i]) {
                (Some(made), true) => self.made_part(made, part, ty, span)?,
                _ => part,
            });
        }
        Ok((out, made))
    }

    /// `part`, a `ty` made in turn that `held_in_turn` says is held, given
    /// to `made`, `made(part, drop)`, which `in_turn`'s `$madeInTurn` drops
    /// by its `drop` as a later part unwinds.
    pub(super) fn made_part(&mut self, made: &str, part: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let drop = self.drop_function(ty, span)?.unwrap_or_else(Expr::undefined);
        Ok(Expr::call(Expr::var(made), vec![part, drop]))
    }

    /// `value`, made of parts in turn, of which those `held_in_turn` says
    /// were given to `made`: `$madeInTurn((made) => value)`, where each is
    /// dropped as a later one unwinds; or, of none, `value` itself.
    pub(super) fn in_turn(value: Expr, made: Option<String>) -> Expr {
        match made {
            Some(name) => Expr::call_helper(
                "$madeInTurn",
                vec![Expr::arrow(
                    vec![name.as_str().into()],
                    vec![StmtKind::Return(Some(value)).at(js::Span::NONE)],
                )],
            ),
            None => value,
        }
    }

    /// `items`, each made by `make`, in an array: `items.map(make)`, where
    /// `make`'s `ty` may panic and has a destructor, `$madeEach(items, make,
    /// drop)`, which drops what it made, in order, as a later one unwinds,
    /// as std's clone of a slice, a `VecDeque` or a map drops what its guard
    /// holds (#897).
    pub(super) fn made_each(
        &mut self,
        items: Expr,
        make: Expr,
        (ty, may_panic): (Ty<'tcx>, bool),
        span: Span,
    ) -> R<Expr> {
        if may_panic && self.drops(ty) != Drops::Nothing {
            let drop = self.drop_function(ty, span)?.unwrap_or_else(Expr::undefined);
            return Ok(Expr::call_helper("$madeEach", vec![items, make, drop]));
        }
        Ok(Expr::call(Expr::member(items, "map"), vec![make]))
    }

    /// `made_each` of a map that may be std's nodes, `map`'s `args` its key
    /// and value: `$madeEach(m, make, drop, dropKey, dropValue)`, each node's
    /// clone a frame of its own, whose key and value made before a later
    /// edge's panic are dropped as its locals are, the value first (#926).
    pub(super) fn made_each_of_nodes(
        &mut self,
        items: Expr,
        make: Expr,
        entry: Ty<'tcx>,
        map: ty::GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let mut list = vec![items, make];
        list.push(self.drop_function(entry, span)?.unwrap_or_else(Expr::undefined));
        let parts = [map.type_at(0), map.type_at(1)]
            .into_iter()
            .map(|part| self.std_drop_function(part, span))
            .collect::<R<Vec<_>>>()?;
        Ok(Expr::call_helper("$madeEach", Expr::optional_args(list, parts)))
    }
}
