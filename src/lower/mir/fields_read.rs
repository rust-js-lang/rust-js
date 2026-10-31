//! A function that gives a reference to a field of what it's given, read
//! as that field where it's called: a trait of classes' default, compiled
//! for each class (rd283, ADR 1035).

use super::*;

impl<'tcx> Mir<'tcx> {
    /// Of a function of one reference that gives a reference to a field of
    /// what it's given, `fn internal_root(&self) -> &RefCell<..> {
    /// &self.internal_root }`: that field's path, past the reference's
    /// deref (rd283).
    pub(in crate::lower) fn field_given(&self) -> Option<Vec<PlaceElem<'tcx>>> {
        let body = &self.body;
        if body.arg_count != 1 || body.basic_blocks.len() != 1 {
            return None;
        }
        let block = &body.basic_blocks[mir::START_BLOCK];
        if !matches!(block.terminator().kind, TerminatorKind::Return) {
            return None;
        }
        // Each local a borrow of a field of `*_1` is, through reborrows.
        let mut fields: HashMap<Local, Vec<PlaceElem<'tcx>>> = HashMap::new();
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    let Rvalue::Ref(_, BorrowKind::Shared, of) = &assign.1 else {
                        return None;
                    };
                    let path = match of.projection.as_slice() {
                        [PlaceElem::Deref, rest @ ..] if of.local == Local::from_u32(1) => rest.to_vec(),
                        [PlaceElem::Deref, rest @ ..] => {
                            let mut path = fields.get(&of.local)?.clone();
                            path.extend_from_slice(rest);
                            path
                        }
                        _ => return None,
                    };
                    if path.is_empty() || !path.iter().all(|elem| matches!(elem, PlaceElem::Field(..))) {
                        return None;
                    }
                    let local = assign.0.as_local()?;
                    fields.insert(local, path);
                }
                StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
                _ => return None,
            }
        }
        fields.remove(&mir::RETURN_PLACE)
    }

    /// This, each call it makes of a function `field_given` says of, that
    /// `given` finds, its field borrowed in its place, `&(*self).internal_root`
    /// (rd283).
    pub(in crate::lower) fn fields_read(
        &self,
        tcx: ty::TyCtxt<'tcx>,
        given: &dyn Fn(rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>) -> Option<Vec<PlaceElem<'tcx>>>,
    ) -> Self {
        let mut mir = Mir {
            body: self.body.clone(),
            promoted: self.promoted.clone(),
            awaits: self.awaits.clone(),
        };
        for data in mir.body.basic_blocks.as_mut() {
            let TerminatorKind::Call {
                func,
                args,
                destination,
                target: Some(target),
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let Some((callee, generic_args)) = func.const_fn_def() else {
                continue;
            };
            let [arg] = &args[..] else { continue };
            let Some(of) = arg.node.place() else { continue };
            let Some(path) = given(callee, generic_args) else {
                continue;
            };
            let mut projection = of.projection.to_vec();
            projection.push(PlaceElem::Deref);
            projection.extend(path);
            let field = Place {
                local: of.local,
                projection: tcx.mk_place_elems(&projection),
            };
            let (destination, target) = (*destination, *target);
            let source_info = data.terminator().source_info;
            data.statements.push(mir::Statement::new(
                source_info,
                StatementKind::Assign(Box::new((
                    destination,
                    Rvalue::Ref(tcx.lifetimes.re_erased, BorrowKind::Shared, field),
                ))),
            ));
            data.terminator_mut().kind = TerminatorKind::Goto { target };
        }
        mir
    }
}
