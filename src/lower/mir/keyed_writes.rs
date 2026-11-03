//! A whole write of a flattened keyed enum, `node.kind = ..` (#504): its
//! parent's keys, the other variants' deleted, as `in` tells a variant by
//! a key of its own.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `node.kind = kind` of a field a keyed enum is flattened in: each key
    /// of a variant the value isn't deleted from the parent, then each of
    /// its own set, `delete node.currentValue; ..; node.value = undefined;
    /// node.getter = getter;`; of a value no literal says, each key
    /// deleted, then its own given, `Object.assign(node, kind)`. What it's
    /// made of is computed first, as Rust's operands are. `None` where
    /// `place` is no such field.
    pub(super) fn keyed_write(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: &Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> Option<R<()>> {
        let (base, PlaceElem::Field(field, _)) = place.as_ref().last_projection()? else {
            return None;
        };
        let parent_ty = Place::ty_from(base.local, base.projection, state.body, self.tcx).ty;
        if !bindings::is_flatten_field(self.tcx, parent_ty, field.as_usize()) {
            return None;
        }
        let ty::Adt(adt, _) = place.ty(state.body, self.tcx).ty.kind() else {
            return None;
        };
        let keyed = self.krate.flattened.keyed(self.tcx, *adt)?;
        let parent = Place {
            local: base.local,
            projection: self.tcx.mk_place_elems(base.projection),
        };
        Some(self.keyed_written(state, parent, rvalue, &keyed.keys, span, out))
    }

    fn keyed_written(
        &mut self,
        state: &mut State<'_, 'tcx>,
        parent: Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
        keys: &[String],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let js_span = self.js_span(span);
        let value = self.mir_rvalue(state, rvalue, span, out)?;
        let value = self.value_expr(value, span)?;
        let parent = self.mir_place(state, parent, span, out)?;
        let parent = match parent.reads_same() {
            true => parent,
            false => self.spill("parent", parent, out),
        };
        let delete = |key: &str| StmtKind::Expr(Expr::unary(js::UnaryOp::Delete, Expr::member(parent.clone(), key)));
        match value.kind {
            js::ExprKind::Object(props, _) if props.iter().all(|prop| matches!(prop, Prop::Field(..))) => {
                let mut set = Vec::new();
                for prop in props {
                    let Prop::Field(key, value) = prop else {
                        unreachable!("fields alone")
                    };
                    let value = match value.has_effects() {
                        true => self.spill(&key, value, out),
                        false => value,
                    };
                    set.push((key, value));
                }
                for key in keys.iter().filter(|key| !set.iter().any(|(own, _)| own == *key)) {
                    out.push(delete(key).at(js_span));
                }
                for (key, value) in set {
                    out.push(StmtKind::Assign(Expr::member(parent.clone(), &key), value).at(js_span));
                }
            }
            _ => {
                let value = match value.reads_same() {
                    true => value,
                    false => self.spill("value", value, out),
                };
                for key in keys {
                    out.push(delete(key).at(js_span));
                }
                let assign = Expr::member(Expr::var("Object"), "assign");
                out.push(StmtKind::Expr(Expr::call(assign, vec![parent, value])).at(js_span));
            }
        }
        Ok(())
    }
}
