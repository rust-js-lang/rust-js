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
use crate::lower::temporary_names::Role;
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
        // Of the receiver itself, `by_ref()`'s `&mut Self`, nothing inside it;
        // of what generic code lends, each instance its callers give; and
        // lent shared, a value that holds no JS object, a number's, an
        // `Option` of one, which is a copy no pointer is made of, refused
        // where one would be (#1293).
        let owner = tcx.typeck_root_def_id(self.item);
        let addressable = |(lent, shared): (Ty<'tcx>, bool)| {
            // And a `&mut` the lowering makes a handle on its place (#1474),
            // and an object it records as stored in its slot (#1475).
            if (!shared && self.lend_placed(def_id, receiver, lent))
                || ((shared || !self.mut_ref_form(lent).is_cell()) && self.lend_recorded(def_id, receiver))
            {
                return false;
            }
            let asked = |lent: Ty<'tcx>| stored.addressable(tcx, lent) && !(shared && self.holds_no_object(lent));
            match lent.has_non_region_param() {
                false => asked(lent),
                true => stored
                    .instances(tcx, lent, owner)
                    .is_none_or(|instances| instances.into_iter().any(asked)),
            }
        };
        let Some((lent, shared)) = lent
            .into_iter()
            .find(|&(lent, shared)| lent != receiver && addressable((lent, shared)))
        else {
            return Ok(());
        };
        // What no handle or record can say is inside the receiver, a value
        // in a `$some` box, a lend std's catalog here doesn't list, is
        // #1513's; a `&mut` of generic code whose instances keep it at
        // different offsets, #1478's.
        let issue = match !shared && self.mut_ref_form(lent).is_cell() {
            // Of generic code, whose instances keep it at different offsets,
            // #1478's, a fact its callers would give.
            true if self.lends_mut_of(def_id, receiver).is_some() && receiver.has_non_region_param() => 1478,
            true => 1513,
            false => 1513,
        };
        let what = format!(
            "`{}` in a crate that asks an address (#{issue})",
            tcx.def_path_str(def_id)
        );
        let why = format!(
            "it lends `{lent}` from inside its `{receiver}`, whose storage JS keeps apart from it, so its address \
             wouldn't be in the `{receiver}`'s; take what it holds out by value, keep it where rust-js \
             knows where it's stored, a `RefCell`, an `Option` or a struct's field, or reach it by a pattern, \
             `if let Ok(x) = &value`"
        );
        Err(refuse(tcx, span, Refused::Because(&what, &why)))
    }

    /// Whether a value of `ty` holds no JS object: a number, a `bool`, a
    /// `char`, `()`, or an `Option` of one, whose JS is a copy, which no
    /// pointer can be made of but at its place (#1293).
    fn holds_no_object(&self, ty: Ty<'tcx>) -> bool {
        match self.option_of(ty) {
            Some(payload) => self.holds_no_object(payload),
            None => ty.is_numeric() || ty.is_bool() || ty.is_char() || ty.is_unit(),
        }
    }

    /// Of a function whose first argument is a reference to its receiver
    /// and whose output holds that reference's lifetime, `RefCell::borrow`,
    /// `Option::as_ref`, `OnceCell::get`: the receiver's type, and what it
    /// lends, each type a reference of that lifetime, or a raw pointer, in
    /// the output refers to, and whether it's lent shared, `&T`, not `&mut`,
    /// a pointer, or what a type of that lifetime holds; a guard's,
    /// `Ref<'_, T>`, its `T`. `None` of
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
    ) -> Option<(Ty<'tcx>, Vec<(Ty<'tcx>, bool)>)> {
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
        // lifetimes erased, and normalized where the caller's bounds say
        // what it is: `<PathBuf as Deref>::Target` is `Path`, which isn't
        // an unknown type that may be any recorded one (#1293).
        let given = |ty: Ty<'tcx>| {
            let ty = ty::EarlyBinder::bind(tcx, ty)
                .instantiate(tcx, args)
                .skip_normalization();
            let ty = tcx.instantiate_bound_regions_with_erased(ty::Binder::bind_with_vars(ty, declared.bound_vars()));
            tcx.try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
                .unwrap_or(ty)
        };
        let mut lent = Vec::new();
        let mut holds = false;
        for part in sig.output().walk() {
            if part.as_region() == Some(region) {
                holds = true;
            }
            let Some(ty) = part.as_type() else { continue };
            if let Some((pointee, _)) = raw_pointee(tcx, ty) {
                lent.push((given(pointee), false));
                continue;
            }
            match *ty.kind() {
                ty::Ref(r, pointee, mutability) if r == region => lent.push((given(pointee), mutability.is_not())),
                ty::Adt(_, args) if args.regions().any(|r| r == region) => {
                    lent.extend(args.types().map(|ty| (given(ty), false)));
                }
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
            false => self.spill(Role::Object, object, out),
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

/// What std lends a `&mut` to from inside its receiver's storage, of which
/// the receiver's own place is known (#1474): a `OnceCell`'s value, a
/// `LazyCell`'s, a `Peekable`'s item it peeked at, and a `Result`'s `Ok` or
/// `Err` payload.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Lent {
    OnceCell,
    LazyCell,
    Peekable,
    Ok,
    Err,
}

impl Lent {
    /// The property of the receiver's JS that holds it: a cell's `value`, a
    /// `Peekable`'s `peeked`, a `Result`'s `_0`.
    fn key(self) -> &'static str {
        match self {
            Lent::OnceCell | Lent::LazyCell => "value",
            Lent::Peekable => "peeked",
            Lent::Ok | Lent::Err => "_0",
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What std's `def_id` lends a `&mut` to from inside a `receiver`, where
    /// the lowering makes its place a handle the pointer machinery reads
    /// (`lent_place`): a `OnceCell`'s `get_mut`, a `LazyCell`'s `force_mut`,
    /// a `Peekable`'s `peek_mut`, a `Result`'s `as_mut`, of either variant,
    /// and its `iter_mut`, of its `Ok`. `None` of another call.
    pub(in crate::lower) fn lends_mut_of(&self, def_id: DefId, receiver: Ty<'tcx>) -> Option<&'static [Lent]> {
        let name = self.tcx.item_name(def_id);
        let result = self.is_std_type(receiver, StdItem::Result);
        match name.as_str() {
            "get_mut"
                if self.is_std_type(receiver, StdItem::OnceCell) && !self.recognition().is_lock_cell(receiver) =>
            {
                Some(&[Lent::OnceCell])
            }
            "force_mut"
                if self.is_std_type(receiver, StdItem::LazyCell) && !self.recognition().is_lock_cell(receiver) =>
            {
                Some(&[Lent::LazyCell])
            }
            "peek_mut" if self.is_peekable(receiver) => Some(&[Lent::Peekable]),
            "as_mut" if result => Some(&[Lent::Ok, Lent::Err]),
            "iter_mut" if result => Some(&[Lent::Ok]),
            _ => None,
        }
    }

    /// Which of a `receiver`'s slots std's `def_id` lends an object from,
    /// shared or `&mut`, which JS gives as the object: a `OnceCell`'s value,
    /// `get`'s, `get_or_init`'s and `get_mut`'s; a `LazyCell`'s, `force`'s and
    /// `force_mut`'s; a `Peekable`'s peeked item, `peek`'s and `peek_mut`'s;
    /// a `Result`'s payload, `as_ref`'s and `as_mut`'s, its `Ok`'s or its
    /// `Err`'s. The one catalog of what #1475 records.
    pub(in crate::lower) fn lends_object_of(&self, def_id: DefId, receiver: Ty<'tcx>) -> Option<&'static [Lent]> {
        let name = self.tcx.item_name(def_id);
        let lock = self.recognition().is_lock_cell(receiver);
        match name.as_str() {
            "get" | "get_or_init" | "get_mut" if self.is_std_type(receiver, StdItem::OnceCell) && !lock => {
                Some(&[Lent::OnceCell])
            }
            "force" | "force_mut" if self.is_std_type(receiver, StdItem::LazyCell) && !lock => Some(&[Lent::LazyCell]),
            "peek" | "peek_mut" if self.is_peekable(receiver) => Some(&[Lent::Peekable]),
            "as_ref" | "as_mut" if self.is_std_type(receiver, StdItem::Result) => Some(&[Lent::Ok, Lent::Err]),
            _ => None,
        }
    }

    /// Whether std's `def_id` lends an object from inside a `receiver` the
    /// lowering records as stored there, at its offset (`lent_record`,
    /// #1475): of a type known here, each slot's offset known and its
    /// payload no `$some` box, whose object is where the record says.
    pub(in crate::lower) fn lend_recorded(&self, def_id: DefId, receiver: Ty<'tcx>) -> bool {
        !receiver.has_non_region_param()
            && self.lends_object_of(def_id, receiver).is_some_and(|lents| {
                lents.iter().all(|&l| {
                    self.lent_offset(receiver, l).is_some()
                        && self.lent_within(receiver, l).is_some()
                        && !(matches!(l, Lent::OnceCell | Lent::Peekable) && self.boxed_lent(receiver, l))
                })
            })
    }

    /// Where in a `receiver`'s slot `lent` the object its payload holds is,
    /// of the receiver's layout: a struct's at its start, an `Option`'s of
    /// one past its tag; a payload that holds no object, a number's, nothing
    /// to record, at 0. None of one elsewhere, an allocation of its own, or
    /// not known.
    fn lent_within(&self, receiver: Ty<'tcx>, lent: Lent) -> Option<u64> {
        let payload = self.lent_payload_in(receiver, lent)?;
        if self.holds_no_object(payload) {
            return Some(0);
        }
        match self.object_within(payload) {
            Within::At(at) => Some(at),
            Within::Elsewhere | Within::Unknown => None,
        }
    }

    /// Of a call of std's that lends an object from inside its receiver,
    /// `lend_recorded`'s, in a crate that asks an address: what records the
    /// object as stored in its slot once the call's made, which the call's JS
    /// is wrapped in, `$lentFrom(once, "value", 4, 0, once.value)`, after it
    /// since `get_or_init` and `peek` fill their slot as they're called. Its
    /// receiver, `values`' first, held first where it reads otherwise twice.
    pub(in crate::lower) fn lent_record(
        &mut self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
        values: &mut [Expr],
        out: &mut Vec<Stmt>,
    ) -> R<Option<impl FnOnce(Expr) -> Expr + use<'tcx>>> {
        if !self.krate.stored_places.asks() {
            return Ok(None);
        }
        let Some((receiver, lent)) = self.lent_from_receiver((def_id, args)) else {
            return Ok(None);
        };
        // An object, shared or `&mut`, among what it lends: a `&mut` the
        // lowering holds as a handle, a number's, is #1474's, and what holds
        // no object, a number lent shared, has nothing to record.
        if !lent
            .iter()
            .any(|&(lent, shared)| (shared || !self.mut_ref_form(lent).is_cell()) && !self.holds_no_object(lent))
        {
            return Ok(None);
        }
        if !self.lend_recorded(def_id, receiver) || values.is_empty() {
            return Ok(None);
        }
        let lents = self.lends_object_of(def_id, receiver).expect("lend_recorded's");
        if !values[0].reads_same() {
            let given = std::mem::replace(&mut values[0], Expr::undefined());
            values[0] = self.spill(Role::Receiver, given, out);
        }
        let held = values[0].clone();
        // Of each slot, or of a `Result`'s two by its variant where they differ.
        let by_slot = |of: &dyn Fn(Lent) -> u64| match lents {
            [one] => Expr::int(i128::from(of(*one))),
            [ok, err] if of(*ok) == of(*err) => Expr::int(i128::from(of(*ok))),
            [ok, err] => Expr::cond(
                crate::js::result::is(held.clone(), true),
                Expr::int(i128::from(of(*ok))),
                Expr::int(i128::from(of(*err))),
            ),
            _ => unreachable!("one slot or a `Result`'s two"),
        };
        let offset = by_slot(&|l| self.lent_offset(receiver, l).expect("lend_recorded's"));
        let within = by_slot(&|l| self.lent_within(receiver, l).expect("lend_recorded's"));
        let key = Expr::str(lents[0].key());
        Ok(Some(move |made: Expr| {
            Expr::call_helper("$lentFrom", vec![held, key, offset, within, made])
        }))
    }

    /// Whether std's `def_id` lends a `&mut` to a `lent` from inside a
    /// `receiver` that the lowering makes a handle on its place there, which
    /// the pointer machinery reads (#1474): of what's a box's, a number's or
    /// generic code's, past no `$some` box, at an offset known
    /// (`lent_place`); or a heap's `PeekMut`'s, whose top's handle is one
    /// on the heap's first item, `$ptr(heap, 0)`, in its buffer's
    /// allocation (`$peekMutTop`). The one answer of whether one is placed.
    pub(in crate::lower) fn lend_placed(&self, def_id: DefId, receiver: Ty<'tcx>, lent: Ty<'tcx>) -> bool {
        if !self.mut_ref_form(lent).is_cell() {
            return false;
        }
        if self.recognition().peek_mut_of(receiver).is_some() {
            return true;
        }
        self.lends_mut_of(def_id, receiver).is_some_and(|lents| {
            !lents
                .iter()
                .any(|&l| matches!(l, Lent::OnceCell | Lent::Peekable) && self.boxed_lent(receiver, l))
                && lents.iter().all(|&l| self.lent_offset(receiver, l).is_some())
        })
    }

    /// Whether `local` of `body`, a `&mut`, is one std lent from inside
    /// its receiver as a handle on its place there, in a crate that asks an
    /// address (`lend_placed`): the lending call's own value, or the
    /// payload of the `Option` or `Result` it gives, `Some(x)`, `Ok(x)`.
    /// A raw pointer made of it is that handle (#1474).
    pub(in crate::lower) fn placed_by_lend(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        local: rustc_middle::mir::Local,
    ) -> bool {
        use rustc_middle::mir::{Operand, PlaceElem, Rvalue, StatementKind, TerminatorKind};
        if !self.krate.stored_places.asks() {
            return false;
        }
        let ty::Ref(_, lent, ty::Mutability::Mut) = *body.local_decls[local].ty.kind() else {
            return false;
        };
        let writes: Vec<&Rvalue<'tcx>> = (body.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.local == local && assign.0.projection.is_empty() => {
                    Some(&assign.1)
                }
                _ => None,
            })
            .collect();
        let made = match writes.as_slice() {
            [] => local,
            [Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)] if matches!(from.projection.as_slice(), [PlaceElem::Downcast(..), PlaceElem::Field(field, _)] if field.as_u32() == 0) => {
                from.local
            }
            _ => return false,
        };
        body.basic_blocks.iter().any(|data| match &data.terminator().kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } if destination.local == made && destination.projection.is_empty() => {
                let receiver = args
                    .first()
                    .map(|arg| arg.node.ty(&body.local_decls, self.tcx).peel_refs());
                crate::lower::fn_def(func.ty(&body.local_decls, self.tcx))
                    .zip(receiver)
                    .is_some_and(|((id, _), receiver)| self.lend_placed(id, receiver, lent))
            }
            _ => false,
        })
    }

    /// Whether what a cell or a `Peekable` of type `receiver` lends, its
    /// `lent`, may be held in its slot in a `$some` box, as one that may look
    /// like `None` is (ADR 0051): of generic code's, in any instance its
    /// callers give, or where they can't all be seen. A handle on the slot
    /// would read the box.
    pub(in crate::lower) fn boxed_lent(&self, receiver: Ty<'tcx>, lent: Lent) -> bool {
        let boxed = |receiver: Ty<'tcx>| {
            self.lent_payload_in(receiver, lent)
                .is_none_or(|t| self.boxed_payload(t))
        };
        if !receiver.has_non_region_param() {
            return boxed(receiver);
        }
        let owner = self.tcx.typeck_root_def_id(self.item);
        (self.krate.stored_places.instances(self.tcx, receiver, owner))
            .is_none_or(|instances| instances.into_iter().any(boxed))
    }

    /// What a receiver of type `receiver` lends of `lent`, its type there:
    /// a `OnceCell`'s or a `Peekable`'s payload past its `Option`s.
    fn lent_payload_in(&self, receiver: Ty<'tcx>, lent: Lent) -> Option<Ty<'tcx>> {
        let held = match lent {
            Lent::OnceCell => self.option_of(self.along(receiver, Some(&["inner", "value"]))?.0)?,
            Lent::Peekable => self.option_of(self.option_of(self.along(receiver, Some(&["peeked"]))?.0)?)?,
            // Its `T`, and a `Result`'s side.
            Lent::LazyCell | Lent::Ok | Lent::Err => {
                let ty::Adt(_, args) = receiver.kind() else { return None };
                args.type_at(usize::from(lent == Lent::Err))
            }
        };
        // As the instance says it is, `u32` of an `IntoIter<u32, 2>`'s `Item`.
        Some(
            (self.tcx)
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(held))
                .unwrap_or(held),
        )
    }

    /// The byte offset in a `receiver` of what std lends of it, `lent`,
    /// rustc's layout of the instance for wasm32: a `OnceCell`'s value past
    /// its `Option`'s tag, a `LazyCell`'s past its state's, a `Peekable`'s
    /// item past its two `Option`s' tags, a `Result`'s payload at its
    /// variant's offset (#1474). Of generic code's receiver, the one each
    /// instance its callers give shares; `None` where they differ, or where
    /// a layout isn't known.
    pub(in crate::lower) fn lent_offset(&self, receiver: Ty<'tcx>, lent: Lent) -> Option<u64> {
        if !receiver.has_non_region_param() {
            return self.lent_offset_in(receiver, lent);
        }
        let owner = self.tcx.typeck_root_def_id(self.item);
        let instances = self.krate.stored_places.instances(self.tcx, receiver, owner)?;
        let mut offsets = instances
            .into_iter()
            .map(|instance| self.lent_offset_in(instance, lent));
        let first = offsets.next()??;
        offsets.all(|offset| offset == Some(first)).then_some(first)
    }

    fn lent_offset_in(&self, receiver: Ty<'tcx>, lent: Lent) -> Option<u64> {
        let some = |option: Ty<'tcx>| self.variant_field_offset(option, VariantIdx::from_u32(1), FieldIdx::ZERO);
        match lent {
            Lent::OnceCell => {
                let (option, at) = self.along(receiver, Some(&["inner", "value"]))?;
                Some(at + some(option)?)
            }
            Lent::LazyCell => {
                let (state, at) = self.along(receiver, Some(&["state", "value"]))?;
                let ty::Adt(adt, _) = state.kind() else {
                    return None;
                };
                let init = adt
                    .variants()
                    .iter_enumerated()
                    .find(|(_, v)| v.name.as_str() == "Init")?
                    .0;
                Some(at + self.variant_field_offset(state, init, FieldIdx::ZERO)?)
            }
            Lent::Peekable => {
                let (outer, at) = self.along(receiver, Some(&["peeked"]))?;
                let inner = self.option_of(outer)?;
                Some(at + some(outer)? + some(inner)?)
            }
            Lent::Ok => self.variant_field_offset(receiver, VariantIdx::from_u32(0), FieldIdx::ZERO),
            Lent::Err => self.variant_field_offset(receiver, VariantIdx::from_u32(1), FieldIdx::ZERO),
        }
    }

    /// Of a `&mut` std lends from inside `receiver`, of type `receiver_ty`,
    /// in a crate that asks an address: a handle on its place there,
    /// `$ptr(cell, "value", 4)`, which the pointer machinery reads as in the
    /// receiver's allocation at its offset (#1474); of a `Result`'s, whose
    /// payload's offset is its variant's, that variant's as it is,
    /// `ok` a test that it's an `Ok`. `None` in a crate that asks no
    /// address, which reads the handle the lowering makes otherwise.
    pub(in crate::lower) fn lent_place(
        &self,
        (receiver, receiver_ty): (Expr, Ty<'tcx>),
        lent: Lent,
        ok: Option<Expr>,
    ) -> Option<Expr> {
        if !self.krate.stored_places.asks() {
            return None;
        }
        let at = |lent: Lent| self.lent_offset(receiver_ty, lent);
        // Of both variants at one offset, that offset, no test.
        let offset = match ok {
            Some(ok) => match (at(Lent::Ok)?, at(Lent::Err)?) {
                (ok_at, err_at) if ok_at == err_at => Expr::int(i128::from(ok_at)),
                (ok_at, err_at) => Expr::cond(ok, Expr::int(i128::from(ok_at)), Expr::int(i128::from(err_at))),
            },
            None => Expr::int(i128::from(at(lent)?)),
        };
        Some(Expr::call_helper("$ptr", vec![receiver, Expr::str(lent.key()), offset]))
    }
}
