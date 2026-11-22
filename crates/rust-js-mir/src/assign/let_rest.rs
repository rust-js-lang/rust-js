//! A `let`'s rest of props, `let P { href, rest, .. } = p`: what's left
//! of them, each of their other fields named. Split from `assign.rs` by
//! concept (ADR 0572).

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// `rest = p.rest` of a `let`'s pattern, `let P { href, rest, .. } = p`:
    /// what's left of the props, each of their other fields named,
    /// `const { href: _href, title: _title, ...rest } = p;`, whose names tidy
    /// makes the pattern's where it binds them.
    pub(super) fn mir_let_rest(
        &mut self,
        state: &mut State<'_, 'tcx>,
        local: Local,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let decls = &state.body.local_decls;
        let props = match rvalue {
            Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) => match read.as_ref().last_projection() {
                Some((base, PlaceElem::Field(field, _))) => {
                    let ty = Place::ty_from(base.local, base.projection, decls, self.tcx).ty;
                    let place = Place {
                        local: base.local,
                        projection: self.tcx.mk_place_elems(base.projection),
                    };
                    bindings::is_rest_field(self.tcx, ty, field.as_usize()).then_some((place, ty))
                }
                _ => None,
            },
            _ => None,
        };
        let Some((place, ty)) = props.filter(|_| !state.locals.declared[local]) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let Shape::Object(fields) = self.shape(ty) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let value = self.mir_place(state, place, span, out)?;
        let tcx = self.tcx;
        let keys: Vec<&str> = (fields.iter().enumerate())
            .filter(|&(i, _)| !bindings::is_rest_field(tcx, ty, i))
            .map(|(_, (key, _))| key.text())
            .collect();
        let named = (keys.iter().zip(self.left_out(&keys)))
            .map(|(key, unused)| (key.to_string(), js::Pattern::Name(unused), None))
            .collect();
        self.flush(state, out)?;
        state.locals.declared[local] = true;
        out.push(
            StmtKind::Destructure {
                pattern: js::Pattern::Object(named, Some(state.locals.names[local].clone())),
                value,
                mutable: false,
            }
            .at(self.js_span(span)),
        );
        Ok(())
    }
}
