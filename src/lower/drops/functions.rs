//! A drop as a function: std's, a type's own, and one held.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `drop_function` of a `ty` a std function drops itself, `truncate`'s
    /// items or `insert`'s old value: of a type parameter whose drop this
    /// function isn't given, none, one its callers give none with a
    /// destructor (ADR 0190).
    pub(in crate::lower) fn std_drop_function(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let drop = self.drop_function(ty, span)?;
        if drop.is_none() {
            self.require_no_drops(ty);
        }
        Ok(drop)
    }

    /// The function that drops a `ty`, which a generic function is given for
    /// its type parameter: its `drop` itself, when that's all its drop is,
    /// `noisyDrop_drop`, or an arrow; a type parameter's is the one this
    /// function was given. None for a type with nothing to drop.
    pub(in crate::lower) fn drop_function(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        if let ty::Param(_) = ty.kind() {
            let index = self.given_drop(ty);
            if let Some(index) = index {
                self.drop_state.used_drops.insert(index);
            }
            return Ok(index.map(|index| Expr::var(&self.drop_state.param_drops[&index])));
        }
        if let Some(index) = self.given_drop(ty) {
            self.drop_state.used_drops.insert(index);
            return Ok(Some(Expr::var(&self.drop_state.param_drops[&index])));
        }
        match self.drops(ty) {
            Drops::Nothing => return Ok(None),
            Drops::Unsupported(t, what) => return Err(self.unsupported(span, &describe(t, what))),
            Drops::Runs => {}
        }
        if let Some(drop) = self.item_drop(ty) {
            return Ok(Some(drop));
        }
        let base = match ty.kind() {
            ty::Adt(adt, _) => lower_first(self.tcx.item_name(adt.did()).as_str()),
            _ => "value".to_string(),
        };
        let param = self.fresh(&base);
        let mut body = Vec::new();
        self.drop_value(Expr::var(&param), ty, span, &mut body)?;
        // `(noisy) => noisyDrop_drop(noisy)` is `noisyDrop_drop`.
        if let [
            Stmt {
                kind: StmtKind::Expr(call),
                ..
            },
        ] = body.as_slice()
            && let js::ExprKind::Call(callee, args) = &call.kind
            && let [arg] = args.as_slice()
            && matches!(&arg.kind, js::ExprKind::Var(name) if *name == param)
            && matches!(&callee.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_))
        {
            return Ok(Some((**callee).clone()));
        }
        // `(intoIter) => { for (const item of intoIter) nDrop_drop(item); }`,
        // what drops each an iterator hasn't given, is `$dropEach(nDrop_drop)`.
        if let [
            Stmt {
                kind:
                    StmtKind::ForOf {
                        label: None,
                        pattern: js::Pattern::Name(item),
                        iterable,
                        body: each,
                        ..
                    },
                ..
            },
        ] = body.as_slice()
            && matches!(&iterable.kind, js::ExprKind::Var(name) if *name == param)
            && let [
                Stmt {
                    kind: StmtKind::Expr(call),
                    ..
                },
            ] = each.as_slice()
            && let js::ExprKind::Call(callee, args) = &call.kind
            && let [arg] = args.as_slice()
            && matches!(&arg.kind, js::ExprKind::Var(name) if name == item)
            && matches!(&callee.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_))
        {
            return Ok(Some(Expr::call_helper("$dropEach", vec![(**callee).clone()])));
        }
        Ok(Some(Expr::arrow(vec![param.into()], body)))
    }

    /// The function that drops what generic code holds of a `ty`: `ty`'s
    /// drop function, or where its caller made it a JS iterator, `iterated`,
    /// `$iterator(value, next)`, one that drops `value`, its `of` (ADR 0421).
    pub(in crate::lower) fn held_drop_function(&mut self, ty: Ty<'tcx>, iterated: bool, span: Span) -> R<Option<Expr>> {
        if !iterated || !self.has_drops(ty) {
            return self.drop_function(ty, span);
        }
        let param = self.fresh("it");
        let mut body = Vec::new();
        self.drop_value(Expr::member(Expr::var(&param), "of"), ty, span, &mut body)?;
        Ok(Some(Expr::arrow(vec![param.into()], body)))
    }

    /// Whether dropping a `ty` reads it once, so a value that isn't a
    /// variable can be dropped where it's made: `noisyDrop_drop(["x"])`.
    pub(in crate::lower) fn drops_once(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            // Its drop drops what it holds, not the function.
            ty::Closure(..) | ty::Coroutine(..) | ty::CoroutineClosure(..) => true,
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                self.drops_once(args.type_at(0))
            }
            ty::Adt(adt, args) => {
                self.tcx
                    .adt_destructor(adt.did())
                    .is_some_and(|d| self.runs_drop(d.did))
                    && adt.is_struct()
                    && adt.all_fields().all(|f| !self.has_drops(self.field_ty(f, args)))
            }
            _ => false,
        }
    }

    /// `value`, which `drop_value` may read more than once, in a `const`
    /// first unless it reads the same each time, or is read once.
    pub(in crate::lower) fn droppable(&mut self, value: Expr, ty: Ty<'tcx>, out: &mut Vec<Stmt>) -> Expr {
        if value.reads_same() || self.drops_once(ty) {
            value
        } else {
            self.spill("value", value, out)
        }
    }
}
