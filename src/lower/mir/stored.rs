//! Where a place's objects are stored, for the addresses a pointer to it
//! asks (ADR 0562): the records a borrow of it makes (#1113), and the root a
//! nested place's pointer is given, its offset there (#1089), each place
//! read more than once with its temporaries made first (#1110).

use super::*;
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
                    // Not a variant's field, whose offset its enum's layout
                    // doesn't say: its object's own. Nor a slice's item, a
                    // shared subslice being a copy of its items (ADR 0063),
                    // whose objects say where they're stored where the
                    // borrow of their `Vec` or array was made.
                    let from = Place::ty_from(place.local, &place.projection[..at], decls, tcx);
                    let to = Place::ty_from(place.local, &place.projection[..=at], decls, tcx);
                    from.variant_index.is_none() && !from.ty.is_slice() && stored.records(to.ty)
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
        // An item's size, the stride its array's items are at, where it's
        // known here: the records say where in the array each is.
        let stride =
            |cx: &Self, item: Ty<'tcx>| cx.layout(item).map(|layout| Expr::int(i128::from(layout.size.bytes())));
        if steps.is_empty() && !items {
            return Ok(());
        }
        self.read_more_than_once(state, place, out)?;
        let js_span = self.js_span(span);
        for at in steps {
            let part = Place {
                local: place.local,
                projection: tcx.mk_place_elems(&place.projection[..=at]),
            };
            let target = self.mir_place(state, part, span, out)?;
            let offset = self.field_offset(part, &state.body.local_decls);
            if let Some(handle) = self.stored_handle(target, offset) {
                let mut given = vec![handle];
                if !matches!(place.projection[at], PlaceElem::Field(..))
                    && let Some(size) = stride(self, part.ty(&state.body.local_decls, tcx).ty)
                {
                    given.push(size);
                }
                out.push(StmtKind::Expr(Expr::call_helper("$stored", given)).at(js_span));
            }
        }
        if items {
            let array = self.mir_place(state, place, span, out)?;
            let mut given = vec![array];
            given.extend(item.and_then(|item| stride(self, item)));
            out.push(StmtKind::Expr(Expr::call_helper("$storedItems", given)).at(js_span));
        }
        Ok(())
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
    pub(super) fn stored_handle(&self, place: Expr, offset: Offset) -> Option<Expr> {
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
