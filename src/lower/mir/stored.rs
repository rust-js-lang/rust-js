//! Where a place's objects are stored, for the addresses a pointer to it
//! asks (ADR 0562): the records a borrow of it makes (#1113), and the root a
//! nested place's pointer is given, its offset there (#1089), each place
//! read more than once with its temporaries made first (#1110).

use super::*;
use crate::lower::lent::Within;
use crate::lower::pointers::{Offset, checked_item};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of `place`, which a reference or a raw pointer is made to, where each
    /// object it's in, or is, is stored, recorded, `$stored($ptr(other,
    /// 0, 0))`, of the types whose objects say so (`StoredPlaces`, #1113):
    /// each field or item from its last `Deref`, whose referent was recorded
    /// where that reference was made, or from its local, an allocation of its
    /// own; and of a `Vec`'s or an array's, which iterators and slices of it
    /// give references into, its items, `$storedItems(v)`. Every address is
    /// a pointer's, and every pointer is made of a place here, or of one a
    /// reference made here refers to.
    pub(super) fn record_stored(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let tcx = self.tcx;
        let stored = self.krate.stored_places;
        let decls = &state.body.local_decls;
        let start = (place.projection.iter())
            .rposition(|elem| elem == PlaceElem::Deref)
            .map_or(0, |at| at + 1);
        let steps: Vec<usize> = (start..place.projection.len())
            .filter(|&at| {
                matches!(
                    place.projection[at],
                    PlaceElem::Field(..) | PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. }
                ) && {
                    // Not a field that's its place's own JS value, an
                    // `Option`'s payload, which its place's record says is
                    // `within` it (#1114). Nor a slice's item, a shared
                    // subslice being a copy of its items (ADR 0063), whose
                    // objects say where they're stored where the borrow of
                    // their `Vec` or array was made.
                    let from = Place::ty_from(place.local, &place.projection[..at], decls, tcx);
                    let to = Place::ty_from(place.local, &place.projection[..=at], decls, tcx);
                    !self.field_is_its_base(from) && !from.ty.is_slice() && stored.records(to.ty)
                }
            })
            .collect();
        let whole = place.ty(decls, tcx).ty;
        let item = match whole.kind() {
            ty::Adt(adt, args) if crate::lower::recognition::is_vec_like_def(tcx, adt.did()) => args.types().next(),
            ty::Array(item, _) => Some(*item),
            _ => None,
        }
        .filter(|&item| stored.records(item));
        let items = item.is_some();
        // A cell field that's the property it holds, a `RefCell`'s of a
        // number, borrowed whole in a crate that asks an address: where it
        // keeps it, past its borrow flag (#1114). A `{ value }`'s says so
        // where std's call is given it (`record_lent`).
        let inline = stored.asks()
            && matches!(place.projection.last(), Some(PlaceElem::Field(..)))
            && self.held_inline(whole).is_some()
            && self.lends_at_block_end(state, whole);
        // A local held in a cell of its own, `{ value }`, as one whose place
        // a pointer is made of is, whose value is the object it holds past
        // its start, an `Option`'s payload: there in its cell (#1114).
        let held_local = start == 0
            && state.facts.pointed().locals.contains(&place.local)
            && stored.records(decls[place.local].ty)
            && matches!(self.object_within(decls[place.local].ty), Within::At(1..));
        // An item's size, the stride its array's items are at, where it's
        // known here: the records say where in the array each is.
        let stride =
            |cx: &Self, item: Ty<'tcx>| cx.layout(item).map(|layout| Expr::int(i128::from(layout.size.bytes())));
        if steps.is_empty() && !items && !inline && !held_local {
            return Ok(());
        }
        self.read_more_than_once(state, place, out)?;
        let js_span = self.js_span(span);
        let parts = (held_local.then_some(None))
            .into_iter()
            .chain(steps.into_iter().map(Some));
        for at in parts {
            let part = Place {
                local: place.local,
                projection: tcx.mk_place_elems(at.map_or(&[][..], |at| &place.projection[..=at])),
            };
            let target = self.mir_place(state, part, span, out)?;
            let offset = match at {
                Some(_) => self.field_offset(part, &state.body.local_decls),
                None => Offset::At(0),
            };
            let held = part.ty(&state.body.local_decls, tcx).ty;
            // Not what's an allocation of its own, a box's value, which no
            // slot holds.
            let Some(within) = self.stored_within(held, span)? else {
                continue;
            };
            if let Some(handle) = self.stored_handle(target, offset) {
                let size = match at.map(|at| place.projection[at]) {
                    Some(PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. }) => stride(self, held),
                    _ => None,
                };
                let given = given_record(handle, size, within);
                out.push(StmtKind::Expr(Expr::call_helper("$stored", given)).at(js_span));
            }
        }
        if let Some(item) = item
            && let Some(within) = self.stored_within(item, span)?
        {
            let array = self.mir_place(state, place, span, out)?;
            let given = given_record(array, stride(self, item), within);
            out.push(StmtKind::Expr(Expr::call_helper("$storedItems", given)).at(js_span));
        }
        if inline {
            let cell = self.mir_place(state, place, span, out)?;
            let field = self.field_offset(place, &state.body.local_decls);
            // Not again where the one just before says so.
            if let Some(record) = self.property_record(whole, &cell, field, span, out)
                && out.last().is_none_or(|last| last.kind != record.kind)
            {
                out.push(record);
            }
        }
        Ok(())
    }

    /// Whether the call that ends the block being lowered lends from inside
    /// a `container` it's given first, `counter.count.borrow()`'s, where a
    /// borrow of one in it is what it's given (#1114).
    fn lends_at_block_end(&self, state: &State<'_, 'tcx>, container: Ty<'tcx>) -> bool {
        let mir::TerminatorKind::Call { func, .. } = &state.body[state.at].terminator().kind else {
            return false;
        };
        crate::lower::fn_def(func.ty(&state.body.local_decls, self.tcx))
            .and_then(|callee| self.lent_from_receiver(callee))
            .is_some_and(|(receiver, _)| {
                self.tcx.erase_and_anonymize_regions(receiver) == self.tcx.erase_and_anonymize_regions(container)
            })
    }

    /// Of `place`, which is read more than once, a rooted pointer's and its
    /// root's, or a borrow's and the records of where it's stored: the
    /// temporaries it reads not yet made, its local's and its indices', made
    /// first, `const`s, as `flush` makes what's pending before what mustn't
    /// run first, so each read is of the one value, never of a name no JS
    /// declares (#1110).
    pub(super) fn read_more_than_once(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let pending = std::iter::once(place.local)
            .chain(place.projection.iter().filter_map(|elem| match elem {
                PlaceElem::Index(local) => Some(local),
                _ => None,
            }))
            .any(|local| state.pending.iter().any(|(l, _)| *l == local));
        if pending {
            self.flush(state, out)?;
        }
        Ok(())
    }

    /// Where `place`'s root ends, the projection its steps start at, past
    /// its last `Deref`, of a place reached through more than one field or
    /// item from there, which `place_in_root` roots; `None` of one step.
    pub(super) fn rooted_from(place: Place<'tcx>) -> Option<usize> {
        let start = (place.projection.iter())
            .rposition(|elem| elem == PlaceElem::Deref)
            .map_or(0, |at| at + 1);
        (place.projection.len() - start >= 2).then_some(start)
    }

    /// Of `place`, a raw pointer's, reached through more than one field or
    /// item from where its projections start, its local or what its last
    /// `Deref` reads: that root's JS, an object, and the place's byte offset
    /// in it, rustc's layout for wasm32, so its address is in the root's
    /// allocation, not the inner object's own (#1089). `None` of a place of
    /// one step, or of a step whose layout isn't known here, a variant's,
    /// or an index whose value is pending.
    pub(super) fn place_in_root(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<(Expr, Expr)>> {
        let Some(start) = Self::rooted_from(place) else {
            return Ok(None);
        };
        let steps = &place.projection[start..];
        let root = Place {
            local: place.local,
            projection: self.tcx.mk_place_elems(&place.projection[..start]),
        };
        let decls = &state.body.local_decls;
        let mut ty = root.ty(decls, self.tcx);
        if !self.is_object(ty.ty) {
            return Ok(None);
        }
        let (mut bytes, mut indices): (u64, Vec<(Local, u64)>) = (0, Vec::new());
        for &elem in steps {
            let size = |cx: &Self, ty: Ty<'tcx>| cx.layout(ty).map(|layout| layout.size.bytes());
            match elem {
                PlaceElem::Field(field, _) if ty.variant_index.is_none() => {
                    let Some(layout) = self.layout(ty.ty) else {
                        return Ok(None);
                    };
                    bytes += layout.fields.offset(field.index()).bytes();
                }
                PlaceElem::ConstantIndex {
                    offset,
                    from_end: false,
                    ..
                } => {
                    let Some(item) = size(self, ty.projection_ty(self.tcx, elem).ty) else {
                        return Ok(None);
                    };
                    bytes += offset * item;
                }
                PlaceElem::Index(local) if !state.pending.iter().any(|(l, _)| *l == local) => {
                    let Some(item) = size(self, ty.projection_ty(self.tcx, elem).ty) else {
                        return Ok(None);
                    };
                    indices.push((local, item));
                }
                _ => return Ok(None),
            }
            ty = ty.projection_ty(self.tcx, elem);
        }
        let root = self.mir_place(state, root, span, out)?;
        // An index that's a number is in the offset, `8`; another, its
        // value times its item's size, `i * 8 + 4`.
        let mut scaled = Vec::new();
        for (local, item) in indices {
            let index = self.read_local(state, local, out)?;
            match index.kind {
                js::ExprKind::Num(n) if n >= 0.0 && n.fract() == 0.0 => bytes += n as u64 * item,
                _ => scaled.push(Expr::bin(js::Op::Mul, index, Expr::int(i128::from(item)))),
            }
        }
        let at = (scaled.into_iter())
            .reduce(|all, part| Expr::bin(js::Op::Add, all, part))
            .map_or_else(
                || Expr::int(i128::from(bytes)),
                |parts| match bytes {
                    0 => parts,
                    _ => Expr::bin(js::Op::Add, parts, Expr::int(i128::from(bytes))),
                },
            );
        Ok(Some((root, at)))
    }

    /// The handle on `place`, a field's or an item's, whatever its value
    /// is, an object's too, which `pointer_to` gives as itself: where it's
    /// stored, its container, key and offset, which `$stored` records of
    /// what's there (#1113). `None` of a place that's no field or item.
    pub(in crate::lower) fn stored_handle(&self, place: Expr, offset: Offset) -> Option<Expr> {
        let (object, key) = match place.kind {
            _ if let Some(args) = checked_item(&place) => (args[0].clone(), args[1].clone()),
            js::ExprKind::Index(object, index) => (*object, *index),
            js::ExprKind::Member(object, key) => (*object, Expr::str(key)),
            _ => return None,
        };
        let mut args = vec![object, key];
        if let Offset::At(offset) = offset {
            args.push(Expr::int(i128::from(offset)));
        }
        Some(Expr::call(Expr::helper("$ptr"), args))
    }
}

/// What a record of where an object is stored is given: where, a field's or
/// an item's handle, or an array; an item's `stride`, where it's known; and
/// `within`, where in the slot the object is, past what holds it (#1114).
fn given_record(at: Expr, stride: Option<Expr>, within: u64) -> Vec<Expr> {
    let within = (within > 0).then_some(within);
    let mut given = vec![at];
    match (stride, within) {
        (stride, Some(within)) => given.extend([stride.unwrap_or_else(Expr::undefined), Expr::int(i128::from(within))]),
        (Some(stride), None) => given.push(stride),
        (None, None) => {}
    }
    given
}
