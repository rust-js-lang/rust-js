//! What field a place is: its struct's field, its byte offset in rustc's
//! layout, and the forms a field's cell takes, a property, a nullable
//! item's. Split from `rvalues.rs` by concept (#1334).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The byte offset of `place`'s last field in its struct, its tuple or
    /// its variant, rustc's layout for wasm32 (ADR 0562; #1114): where its address is in the JS
    /// object or array the field is on (#1081). `Unsaid` of a place that
    /// isn't a field's; `LaidOutByCaller` of a type a caller lays out.
    pub(in crate::lower::mir) fn field_offset(
        &self,
        place: Place<'tcx>,
        decls: &IndexVec<Local, mir::LocalDecl<'tcx>>,
    ) -> Offset {
        let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection() else {
            return Offset::Unsaid;
        };
        let parent = Place::ty_from(base.local, base.projection, decls, self.tcx);
        match self.place_field_offset(parent, field) {
            Some(offset) => Offset::At(offset),
            None => Offset::LaidOutByCaller,
        }
    }

    /// The struct's field `place` is, `p.code`, if it's one: of a struct,
    /// or of an enum's variant it's read as.
    pub(in crate::lower::mir) fn place_field(
        &self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
    ) -> Option<&'tcx ty::FieldDef> {
        let (PlaceElem::Field(field, _), parent) = place.projection.split_last()? else {
            return None;
        };
        let parent = mir::PlaceRef {
            local: place.local,
            projection: parent,
        };
        let mir::PlaceTy { ty, variant_index } = parent.ty(state.body, self.tcx);
        match ty.kind() {
            ty::Adt(adt, _) if !adt.is_enum() || variant_index.is_some() => {
                Some(&adt.variant(variant_index.unwrap_or(VariantIdx::ZERO)).fields[*field])
            }
            _ => None,
        }
    }

    /// Whether `place` is a whole field that's the property its cell holds,
    /// a `Cell`'s or a struct's `RefCell`'s (`property_cell`): read as a
    /// handle on it, written and moved as what it holds.
    pub(in crate::lower::mir) fn whole_property_cell(&self, state: &State<'_, 'tcx>, place: Place<'tcx>) -> bool {
        let ty = place.ty(state.body, self.tcx).ty;
        (self.is_std_type(ty, StdItem::Cell) || self.is_std_type(ty, StdItem::RefCell))
            && self
                .place_field(state, place)
                .is_some_and(|field| self.property_cell(Some(field), ty).is_some())
    }

    /// The field `place`, `*r`, is where `r` is what a `RefCell`'s `get_mut`
    /// gave of one, `r = x.f.get_mut()`: a `#[rust_js::nullable]` field's
    /// `RefCell<Option<T>>`, which `*r = None` writes `null` (ADR 0863).
    pub(in crate::lower::mir) fn field_got_mut(
        &self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
    ) -> Option<&'tcx ty::FieldDef> {
        let [PlaceElem::Deref] = place.projection.as_slice() else {
            return None;
        };
        let body = state.body;
        let lent = body
            .basic_blocks
            .iter()
            .find_map(|data| match &data.terminator().kind {
                mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } if destination.local == place.local && destination.projection.is_empty() => {
                    let (id, generic_args) = crate::lower::fn_def(func.ty(&body.local_decls, self.tcx))?;
                    (self.recognition().classify(id, generic_args) == Some(crate::lower::Std::CellGetMut))
                        .then(|| args.first().and_then(|arg| arg.node.place()))
                        .flatten()
                }
                _ => None,
            })?;
        let [] = lent.projection.as_slice() else { return None };
        body.basic_blocks
            .iter()
            .flat_map(|data| &data.statements)
            .find_map(|statement| match &statement.kind {
                mir::StatementKind::Assign(assigned)
                    if assigned.0.local == lent.local && assigned.0.projection.is_empty() =>
                {
                    match &assigned.1 {
                        Rvalue::Ref(_, BorrowKind::Mut { .. }, of) => self.place_field(state, *of),
                        _ => None,
                    }
                }
                _ => None,
            })
    }

    /// The field `place`, `*r`, is an item of, where `r` is what an
    /// `index_mut` gave of a `#[rust_js::nullable]` field's
    /// `Vec<Option<T>>`, `r = v.index_mut(i)` of `v[i] = None`: whose
    /// items' `None` is `null` (ADR 0586, amended).
    pub(in crate::lower::mir) fn field_item(
        &self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
    ) -> Option<&'tcx ty::FieldDef> {
        let [PlaceElem::Deref] = place.projection.as_slice() else {
            return None;
        };
        let body = state.body;
        body.basic_blocks.iter().find_map(|data| match &data.terminator().kind {
            mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } if destination.local == place.local && destination.projection.is_empty() => {
                let (id, _) = crate::lower::fn_def(func.ty(&body.local_decls, self.tcx))?;
                (self.tcx.trait_of_assoc(id)).filter(|&t| self.tcx.is_lang_item(t, LangItem::IndexMut))?;
                self.nullable_items_field(state, &args.first()?.node)
            }
            _ => None,
        })
    }

    /// The `#[rust_js::nullable]` field whose `Vec<Option<T>>` the items
    /// `operand` gives are, if they are (ADR 0586, amended): the field,
    /// lent, `&mut root.f`, or borrowed of its `RefCell` and dereferenced,
    /// `root.f.borrow_mut()`, held in a variable or not. What an index of
    /// it writes is `null` for `None`, as the field's elements are.
    pub(in crate::lower::mir) fn nullable_items_field(
        &self,
        state: &State<'_, 'tcx>,
        operand: &Operand<'tcx>,
    ) -> Option<&'tcx ty::FieldDef> {
        let body = state.body;
        let mut place = operand.place()?;
        // Each step back is a reference's or a call's, of which a body has
        // few: as many as its locals bound the walk.
        for _ in 0..body.local_decls.len() {
            if let Some(field) = self.place_field(state, place) {
                let ty = Place::ty_from(place.local, place.projection, &body.local_decls, self.tcx).ty;
                let nullable = crate::lower::representation::none_form(
                    self.tcx,
                    crate::lower::representation::NonePlace::Field(field, None),
                )
                .stores_null();
                return (nullable && bindings::nullable_items(self.tcx, ty).is_some()).then_some(field);
            }
            if !matches!(place.projection.as_slice(), [] | [PlaceElem::Deref]) {
                return None;
            }
            let local = place.local;
            let assigned = (body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
                mir::StatementKind::Assign(assign) if assign.0 == local.into() => Some(&assign.1),
                _ => None,
            });
            place = match assigned {
                Some(Rvalue::Ref(_, _, lent)) => *lent,
                Some(Rvalue::Use(Operand::Copy(given) | Operand::Move(given), _)) => *given,
                Some(_) => return None,
                // What a `RefCell`'s `borrow_mut` gives, and a guard's
                // `deref_mut`, of what they're given.
                None => body
                    .basic_blocks
                    .iter()
                    .find_map(|data| match &data.terminator().kind {
                        mir::TerminatorKind::Call {
                            func,
                            args,
                            destination,
                            ..
                        } if *destination == local.into() => {
                            let (id, generic_args) = crate::lower::fn_def(func.ty(&body.local_decls, self.tcx))?;
                            let derefs = (self.tcx.trait_of_assoc(id)).is_some_and(|t| {
                                self.tcx.is_lang_item(t, LangItem::Deref)
                                    || self.tcx.is_lang_item(t, LangItem::DerefMut)
                            });
                            let borrows = matches!(
                                self.recognition().classify(id, generic_args),
                                Some(crate::lower::Std::Borrow { .. })
                            );
                            (derefs || borrows).then(|| args.first()?.node.place()).flatten()
                        }
                        _ => None,
                    })?,
            };
        }
        None
    }
}
