//! Where a value keeps what it holds inline, where JS holds that as another
//! place (#1114, ADR 0562, amended): std's containers that lend what they
//! hold, a `RefCell`'s value past its borrow flag, a lock's past its state,
//! a `Cell`'s, an atomic's; and the containers JS holds as what they hold,
//! an `Option` of an object, past its tag, a `RefCell` of an object (ADR
//! 0863). Each offset is rustc's layout of the instance, for wasm32, never
//! a number written here.

use crate::js::{Expr, Stmt, StmtKind};
use crate::lower::pointers::{Offset, raw_pointee};
use crate::lower::recognition::{StdItem, is_std_crate, is_vec_like_def};
use crate::lower::refusal::{Refused, refuse};
use crate::lower::representation::property_of_cell;
use crate::lower::{FnCx, R, bindings};
use rustc_abi::{FieldIdx, VariantIdx};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::PlaceTy;
use rustc_middle::ty::layout::LayoutCx;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// Each `{ value }` container std lends what it holds from, and the path of
/// its fields, by std's names, to it: `RefCell<T>`'s `value.value`, past
/// its borrow flag; a `Mutex`'s and an `RwLock`'s `data.value`, past their
/// state; a `Cell`'s `value.value`. An atomic's is its one field, at its
/// start, whatever std names it.
const HELD_INLINE: [(StdItem, &[&str]); 4] = [
    (StdItem::RefCell, &["value", "value"]),
    (StdItem::Cell, &["value", "value"]),
    (StdItem::Mutex, &["data", "value"]),
    (StdItem::RwLock, &["data", "value"]),
];

/// The std types whose value, in JS an object, is an allocation of its own
/// natively, wherever the value that points to it is: a box's, an `Rc`'s
/// and an `Arc`'s value, a collection's items.
const HELD_ELSEWHERE: [StdItem; 6] = [
    StdItem::Rc,
    StdItem::Arc,
    StdItem::HashMap,
    StdItem::HashSet,
    StdItem::BTreeMap,
    StdItem::BTreeSet,
];

/// Where in a value of a type the object its JS is sits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Within {
    /// This many bytes into it: 0 of most, which are their own; an
    /// `Option`'s payload's offset, of one that's its payload.
    At(u64),
    /// Nowhere in it: in an allocation of its own natively, a box's value,
    /// or a reference's referent, so no slot holds it.
    Elsewhere,
    /// Not known here: of a type its caller lays out, or of an untagged
    /// enum whose variants hold theirs at different offsets.
    Unknown,
}

impl Within {
    fn and(self, then: impl FnOnce() -> Within) -> Within {
        match self {
            Within::At(at) => match then() {
                Within::At(more) => Within::At(at + more),
                other => other,
            },
            other => other,
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of a value of `ty` in the body being lowered, where in it the object
    /// its JS is sits (`object_within`); of a type parameter's, where in
    /// each of the crate's instances of it, which must agree, as generic
    /// code has one JS for them all. Refused where they don't, or where an
    /// untagged enum's variants hold theirs at different offsets (#1295);
    /// `what` is what's made of it. Of instances the crate can't all see, a
    /// `pub` function's, 0. `None`, held elsewhere.
    pub(in crate::lower) fn known_within(&self, ty: Ty<'tcx>, span: Span, what: &str) -> R<Option<u64>> {
        let refused = |why: String| Err(refuse(self.tcx, span, Refused::Because(what, &why)));
        // An untagged enum's variants at different offsets, #1295's.
        let varies = |ty: Ty<'tcx>| {
            let why = format!(
                "`{ty}`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say"
            );
            Err(refuse(
                self.tcx,
                span,
                Refused::Because(&format!("{what} (#1295)"), &why),
            ))
        };
        match self.object_within(ty) {
            Within::At(at) => return Ok(Some(at)),
            Within::Elsewhere => return Ok(None),
            Within::Unknown if !ty.has_non_region_param() => return varies(ty),
            Within::Unknown => {}
        }
        let owner = self.tcx.typeck_root_def_id(self.item);
        let Some(instances) = self.krate.stored_places.instances(self.tcx, ty, owner) else {
            return Ok(Some(0));
        };
        let mut found: Option<(Ty<'tcx>, Within)> = None;
        for instance in instances {
            let at = self.object_within(instance);
            if at == Within::Unknown {
                return varies(instance);
            }
            match found {
                Some((other, before)) if before != at => {
                    let (held, by) = match at {
                        Within::At(at) if at > 0 => (instance, other),
                        _ => (other, instance),
                    };
                    return refused(format!(
                        "generic code has one JS for each of `{ty}`'s instances, and a `{held}` is the object it \
                         holds, which isn't at its start, where a `{by}` isn't (#1114); make it where the type is \
                         known, in a function of `{held}`'s own, or of what it holds, its payload or its value"
                    ));
                }
                _ => found = Some((instance, at)),
            }
        }
        Ok(match found {
            Some((_, Within::At(at))) => Some(at),
            Some((_, _)) => None,
            None => Some(0),
        })
    }

    /// A pointer to `object`, a `pointee`'s JS: `$ptrTo(object)`; of a
    /// pointee JS holds as the object it holds, past its start, an
    /// `Option`'s payload past its tag, a `RefCell` of an object (ADR 0863),
    /// a pointer to what holds it, `$ptrOuter(object, 4)`, whose address is
    /// the object's less that (#1114). Of a type parameter's, in a crate
    /// that asks an address, so in each instance of it, refused where they
    /// differ (`known_within`).
    pub(in crate::lower) fn object_pointer(&self, object: Expr, pointee: Ty<'tcx>, span: Span) -> R<Expr> {
        let within = match self.object_within(pointee) {
            Within::At(at) => at,
            Within::Unknown if self.krate.stored_places.asks() => {
                (self.known_within(pointee, span, "a pointer to a value of a type parameter")?).unwrap_or(0)
            }
            Within::Unknown | Within::Elsewhere => 0,
        };
        Ok(match within {
            0 => Expr::call(Expr::helper("$ptrTo"), vec![object]),
            within => Expr::call(Expr::helper("$ptrOuter"), vec![object, Expr::int(i128::from(within))]),
        })
    }

    /// Where a record of an object stored in a slot of type `ty` says it is
    /// in the slot, `known_within`'s: `None`, held elsewhere, which no
    /// record is made of.
    pub(in crate::lower) fn stored_within(&self, ty: Ty<'tcx>, span: Span) -> R<Option<u64>> {
        self.known_within(ty, span, "a borrow of where an object is stored, in generic code")
    }

    /// Of a call of std's given a `{ value }` container that keeps what it
    /// lends inline, `cell.borrow()`, `m.lock()`, `c.get_mut()`, in a crate
    /// that asks an address: the container, where it keeps that recorded as
    /// it's given, `$borrow($storedValue(cell, 4))`, so a pointer to what it
    /// lends, or into it, is in the container's allocation, at its offset
    /// there (#1114); with where in that slot the object it holds is,
    /// `within`, or `null` of one that's an allocation of its own, a `Vec`'s
    /// items. Of a cell field that's the property it holds, which its
    /// borrow records (`property_record`), nothing.
    pub(in crate::lower) fn record_lent(
        &self,
        callee: (DefId, ty::GenericArgsRef<'tcx>),
        values: &mut std::vec::IntoIter<Expr>,
        span: Span,
    ) -> R<()> {
        if !self.krate.stored_places.asks() {
            return Ok(());
        }
        let Some((container, _)) = self.lent_from_receiver(callee) else {
            return Ok(());
        };
        let Some((held, at)) = self.held_inline(container) else {
            return Ok(());
        };
        let mut given: Vec<Expr> = values.collect();
        if property_of_cell(&given[0]).is_none() {
            let mut record = vec![given[0].clone(), Expr::int(i128::from(at))];
            match self.stored_within(held, span)? {
                Some(0) => {}
                Some(within) => record.push(Expr::int(i128::from(within))),
                None => record.push(Expr::null()),
            }
            given[0] = Expr::call_helper("$storedValue", record).never_throws();
        }
        *values = given.into_iter();
        Ok(())
    }

    /// A call of std's that lends a reference into its receiver's own
    /// storage, `once.get()`, `r.as_ref()`, `it.peek()`: one whose first
    /// argument is a reference to its receiver and whose output holds that
    /// reference's lifetime. Of a receiver whose storage this module knows,
    /// a cell's, a lock's, an `Option`'s, a guard's, or one whose value is an
    /// allocation of its own, a box's or a collection's, nothing; of
    /// another, in a crate that asks an address of what it lends, refused,
    /// naming #1114, as where JS keeps that isn't where Rust does.
    pub(in crate::lower) fn check_lent(&self, (def_id, args): (DefId, ty::GenericArgsRef<'tcx>), span: Span) -> R<()> {
        let tcx = self.tcx;
        let stored = self.krate.stored_places;
        if !stored.asks()
            || !is_std_crate(tcx, def_id)
            || !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
        {
            return Ok(());
        }
        let Some((receiver, lent)) = self.lent_from_receiver((def_id, args)) else {
            return Ok(());
        };
        // Of std's type alone: the crate's own, a `Deref` of its own say,
        // records where it stores what it lends where it's borrowed.
        let ty::Adt(adt, _) = receiver.kind() else {
            return Ok(());
        };
        if !is_std_crate(tcx, adt.did()) || self.lends_known(receiver) {
            return Ok(());
        }
        let Some(lent) = lent.into_iter().find(|&lent| stored.addressable(tcx, lent)) else {
            return Ok(());
        };
        let what = format!("`{}` in a crate that asks an address (#1293)", tcx.def_path_str(def_id));
        let why = format!(
            "it lends `{lent}` from inside its `{receiver}`, whose storage JS keeps apart from it, so its address \
             wouldn't be in the `{receiver}`'s; take what it holds out by value, keep it where rust-js \
             knows where it's stored, a `RefCell`, an `Option` or a struct's field, or reach it by a pattern, \
             `if let Ok(x) = &value`"
        );
        Err(refuse(tcx, span, Refused::Because(&what, &why)))
    }

    /// Of a function whose first argument is a reference to its receiver
    /// and whose output holds that reference's lifetime, `RefCell::borrow`,
    /// `Option::as_ref`, `OnceCell::get`: the receiver's type, and what it
    /// lends, each type a reference of that lifetime, or a raw pointer, in
    /// the output refers to; a guard's, `Ref<'_, T>`, its `T`. `None` of
    /// another, `Cell::set`. Its lifetime and its raw pointers are the
    /// function's own, as its signature declares them, not ones of what its
    /// caller gives: a `skip_while` given a `&mut` iterator, `Self` a
    /// reference, lends nothing, though its closure's captures are
    /// references too, whose lifetimes, erased, look the same as the
    /// receiver's (#1199); a `Cell<*const T>`'s `get()` copies a pointer
    /// out, lending nothing of the cell's.
    pub(in crate::lower) fn lent_from_receiver(
        &self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Option<(Ty<'tcx>, Vec<Ty<'tcx>>)> {
        let tcx = self.tcx;
        if !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
            return None;
        }
        let declared = tcx.fn_sig(def_id).instantiate_identity().skip_normalization();
        let sig = declared.skip_binder();
        let &ty::Ref(region, receiver, _) = sig.inputs().first()?.kind() else {
            return None;
        };
        // A type of the signature as the caller's `args` make it, its own
        // lifetimes erased.
        let given = |ty: Ty<'tcx>| {
            let ty = ty::EarlyBinder::bind(tcx, ty)
                .instantiate(tcx, args)
                .skip_normalization();
            tcx.instantiate_bound_regions_with_erased(ty::Binder::bind_with_vars(ty, declared.bound_vars()))
        };
        let mut lent = Vec::new();
        let mut holds = false;
        for part in sig.output().walk() {
            if part.as_region() == Some(region) {
                holds = true;
            }
            let Some(ty) = part.as_type() else { continue };
            if let Some((pointee, _)) = raw_pointee(tcx, ty) {
                lent.push(given(pointee));
                continue;
            }
            match *ty.kind() {
                ty::Ref(r, pointee, _) if r == region => lent.push(given(pointee)),
                ty::Adt(_, args) if args.regions().any(|r| r == region) => lent.extend(args.types().map(given)),
                _ => {}
            }
        }
        (holds || !lent.is_empty()).then_some((given(receiver), lent))
    }

    /// Whether `local` of `body` is a reference std's call lent from a
    /// receiver JS holds as the object it lends: past its start, an
    /// `Option`'s `get_or_insert` or `insert`'s; or elsewhere, an `Rc`'s
    /// or an `Arc`'s `deref`'s. Its JS reads the receiver's place, where a
    /// pointer to what it refers to isn't (#1114).
    pub(in crate::lower) fn lent_from_a_holder(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        local: rustc_middle::mir::Local,
    ) -> bool {
        body.basic_blocks.iter().any(|block| {
            let rustc_middle::mir::TerminatorKind::Call { func, destination, .. } = &block.terminator().kind else {
                return false;
            };
            destination.local == local
                && destination.projection.is_empty()
                && crate::lower::fn_def(func.ty(&body.local_decls, self.tcx))
                    .and_then(|callee| self.lent_from_receiver(callee))
                    .is_some_and(|(receiver, _)| {
                        matches!(self.object_within(receiver), Within::At(1..))
                            || self.is_rc(receiver) && self.counted_rc(receiver).is_none()
                    })
        })
    }

    /// Whether what std lends from inside a `receiver` is where this module
    /// knows it is: a `{ value }` container's (`held_inline`), an `Option`'s,
    /// a `RefCell`'s that's the object it holds, a guard's, which is its
    /// cell's, a type's that's the value it holds (`transparent`); or
    /// nowhere in it, of one whose value is an allocation of its own.
    fn lends_known(&self, receiver: Ty<'tcx>) -> bool {
        const KNOWN: [StdItem; 8] = [
            StdItem::RefCell,
            StdItem::Cell,
            StdItem::Mutex,
            StdItem::RwLock,
            StdItem::RefCellRef,
            StdItem::RefCellRefMut,
            StdItem::MutexGuard,
            StdItem::RwLockReadGuard,
        ];
        KNOWN.iter().any(|&item| self.is_std_type(receiver, item))
            || self.is_std_type(receiver, StdItem::RwLockWriteGuard)
            || self.is_std_type(receiver, StdItem::Atomic)
            || self.option_of(receiver).is_some()
            || self.transparent(receiver).is_some()
            || self.recognition().pinned(receiver).is_some()
            || self.is_std_type(receiver, StdItem::LocalKey)
            || self.is_lang_adt(receiver, LangItem::UnsafeCell)
            || self.is_lang_adt(receiver, LangItem::String)
            || self.object_within(receiver) == Within::Elsewhere
    }

    /// Of `place`, an `Option<item>`'s JS a payload was just written to, by
    /// `insert` or `get_or_insert` before it lends it: where that's stored,
    /// recorded again, as the borrow of the place given the call recorded
    /// what it held before, `$stored($ptr(o, "value"), undefined, 4)`, the
    /// handle that borrow made (#1114). Nothing in a crate whose records
    /// don't say where an `Option` is.
    pub(in crate::lower) fn record_inserted(
        &self,
        place: &Expr,
        item: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let option = Ty::new_option(self.tcx, item);
        if !self.krate.stored_places.records(option) {
            return Ok(());
        }
        let Some(within) = self.stored_within(option, span)? else {
            return Ok(());
        };
        let Some(handle) = self.stored_handle(place.clone(), Offset::Unsaid) else {
            return Ok(());
        };
        // Its size, the stride an array's items are at, where the place is
        // one, as the borrow's record says.
        let size = self
            .layout(option)
            .map(|layout| Expr::int(i128::from(layout.size.bytes())));
        let mut given = vec![handle, size.unwrap_or_else(Expr::undefined)];
        if within > 0 {
            given.push(Expr::int(i128::from(within)));
        }
        out.push(StmtKind::Expr(Expr::call_helper("$stored", given)).at(self.js_span(span)));
        Ok(())
    }

    /// Of `cell`, a cell field's JS that's the property it holds, a handle
    /// on it or its one cell (ADRs 0288, 0863), of a container of type
    /// `container` that keeps what it holds inline (`held_inline`), borrowed
    /// in a crate that asks an address: the record of where it keeps that,
    /// `field` the field's offset in its struct, `$storedIn(x, "f", 8, 4)`,
    /// so a pointer to what it lends, which is the property's, is there
    /// (#1114). `None` of another, and of a field whose offset isn't known.
    pub(in crate::lower) fn property_record(
        &mut self,
        container: Ty<'tcx>,
        cell: &Expr,
        field: Offset,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> Option<Stmt> {
        let (_, at) = self.held_inline(container)?;
        let (object, key) = property_of_cell(cell)?;
        let Offset::At(field) = field else {
            return None;
        };
        let object = match object.reads_same() && !object.has_effects() {
            true => object,
            false => self.spill("object", object, out),
        };
        let record = vec![
            object,
            Expr::str(&key),
            Expr::int(i128::from(field)),
            Expr::int(i128::from(at)),
        ];
        Some(StmtKind::Expr(Expr::call_helper("$storedIn", record)).at(self.js_span(span)))
    }

    /// Of a container `ty` JS holds as a `{ value }`, a `RefCell`'s that
    /// isn't the object it holds, a lock's, a `Cell`'s or an atomic's: what
    /// it holds, and where, its offset in it (`HELD_INLINE`). `None` of
    /// another type, or of one whose layout its caller says.
    pub(in crate::lower) fn held_inline(&self, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, u64)> {
        if self.is_std_type(ty, StdItem::Atomic) {
            return self.along(ty, None);
        }
        let (_, path) = HELD_INLINE.iter().find(|(item, _)| self.is_std_type(ty, *item))?;
        if self.is_std_type(ty, StdItem::RefCell) && self.is_value_cell(ty) {
            return None;
        }
        self.along(ty, Some(path))
    }

    /// Of a value of `ty`, where in it the object its JS is sits (`Within`):
    /// 0 of most, which are their own; an `Option`'s payload's, past its
    /// tag, of one that's its payload (ADR 0051); a `RefCell`'s value's, of
    /// one that's the object it holds (ADR 0863); a type that's the value it
    /// holds, its value's (`transparent`); an untagged enum's variant's,
    /// where each is at one; elsewhere, a box's, an `Rc`'s, a reference's or
    /// a collection's, whose object is an allocation of its own.
    pub(in crate::lower) fn object_within(&self, ty: Ty<'tcx>) -> Within {
        if let Some(inner) = self.transparent(ty) {
            return self.object_within(inner);
        }
        // A pointer's JS is a handle, apart from what holds it.
        if raw_pointee(self.tcx, ty).is_some() {
            return Within::Elsewhere;
        }
        let ty::Adt(adt, args) = *ty.kind() else {
            return match ty.kind() {
                ty::Param(_) | ty::Alias(..) => Within::Unknown,
                ty::Ref(..) | ty::FnPtr(..) | ty::Dynamic(..) | ty::Str | ty::Slice(_) => Within::Elsewhere,
                _ => Within::At(0),
            };
        };
        if adt.is_box()
            || is_vec_like_def(self.tcx, adt.did())
            || HELD_ELSEWHERE.iter().any(|&item| self.is_std_type(ty, item))
        {
            return Within::Elsewhere;
        }
        if let Some(payload) = self.option_of(ty) {
            if self.boxed_payload(payload) {
                return Within::At(0);
            }
            let Some(some) = self.variant_field_offset(ty, VariantIdx::from_u32(1), FieldIdx::ZERO) else {
                return Within::Unknown;
            };
            return Within::At(some).and(|| self.object_within(payload));
        }
        if self.is_std_type(ty, StdItem::RefCell) && self.is_value_cell(ty) {
            let Some((held, at)) = self.along(ty, Some(HELD_INLINE[0].1)) else {
                return Within::Unknown;
            };
            return Within::At(at).and(|| self.object_within(held));
        }
        // An untagged enum is the value its variant holds (ADR 0214); a
        // write's result, its `Err`'s error (ADR 0462).
        let untagged = adt.is_enum() && bindings::is_untagged(self.tcx, adt.did());
        if untagged || self.is_write_result(ty) {
            let mut found = None;
            for (variant, def) in adt.variants().iter_enumerated() {
                let Some(field) = def.fields.iter().next() else {
                    continue;
                };
                let held = self
                    .tcx
                    .erase_and_anonymize_regions(field.ty(self.tcx, args).skip_normalization());
                let Some(offset) = self.variant_field_offset(ty, variant, FieldIdx::ZERO) else {
                    return Within::Unknown;
                };
                let here = Within::At(offset).and(|| self.object_within(held));
                if found.is_some_and(|found| found != here) {
                    return Within::Unknown;
                }
                found = Some(here);
            }
            return found.unwrap_or(Within::At(0));
        }
        Within::At(0)
    }

    /// The byte offset of field `field` of variant `variant` of `ty`, an
    /// enum, rustc's layout of that variant; `None` of a type its caller
    /// lays out.
    pub(in crate::lower) fn variant_field_offset(
        &self,
        ty: Ty<'tcx>,
        variant: VariantIdx,
        field: FieldIdx,
    ) -> Option<u64> {
        let layout = self.layout(ty)?;
        let cx = LayoutCx::new(self.tcx, ty::TypingEnv::fully_monomorphized());
        Some(layout.for_variant(&cx, variant).fields.offset(field.as_usize()).bytes())
    }

    /// The byte offset of `field` of a place of type `ty`, a struct's, a
    /// tuple's or a variant's, rustc's layout; `None` of a type its caller
    /// lays out.
    pub(in crate::lower) fn place_field_offset(&self, ty: PlaceTy<'tcx>, field: FieldIdx) -> Option<u64> {
        match ty.variant_index {
            Some(variant) => self.variant_field_offset(ty.ty, variant, field),
            None => Some(self.layout(ty.ty)?.fields.offset(field.as_usize()).bytes()),
        }
    }

    /// What `ty` holds along `path`, its fields by name, or along its first
    /// fields to a value of no fields, and where: the sum of each field's
    /// offset in its own type, rustc's layout.
    fn along(&self, mut ty: Ty<'tcx>, path: Option<&[&str]>) -> Option<(Ty<'tcx>, u64)> {
        let mut at = 0;
        let mut step = 0;
        loop {
            if path.is_some_and(|path| step == path.len()) {
                return Some((ty, at));
            }
            let ty::Adt(adt, args) = *ty.kind() else {
                return path.is_none().then_some((ty, at));
            };
            let fields = &adt.non_enum_variant().fields;
            let (i, field) = match path {
                Some(path) => fields.iter_enumerated().find(|(_, f)| f.name.as_str() == path[step])?,
                None if adt.is_enum() || self.is_lang_adt(ty, LangItem::UnsafeCell) && fields.is_empty() => {
                    return None;
                }
                None => fields.iter_enumerated().next()?,
            };
            at += self.layout(ty)?.fields.offset(i.as_usize()).bytes();
            ty = self
                .tcx
                .erase_and_anonymize_regions(field.ty(self.tcx, args).skip_normalization());
            step += 1;
        }
    }
}
