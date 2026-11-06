//! When a collect asks its iterator's `size_hint`, and for what room
//! (#820): one model of std's paths, `SpecFromIter` of a `TrustedLen`
//! iterator, its hint first, `SpecFromIterNested` of another, its first
//! item then its hint, and `extend_desugared` as the `Vec` fills; and the
//! reserves of a `String`'s and a hash table's `extend`, by destination
//! and source. Every collect asks it, and so does the analysis of what a
//! grown `Vec`'s items' size is asked of a caller (`grows_by_callers_size`).
//! What the runtime's `$roomForHint` and `$reserveHint` are given is made
//! here alone.

use super::*;
use crate::lower::recognition::{Recognition, TypeFact, is_trusted_len};
use rustc_middle::ty::TypeVisitableExt;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether a collect into `output` of an iterator of `iter_ty` grows a
    /// `Vec` of `item`s whose size is its caller's, a type parameter's or
    /// one built of them, `(T, u8)`, where the hint it asks as that `Vec`
    /// fills may run the crate's own (`hint_reaches_own`): so when it's
    /// asked is seen, the size its caller gives, `TSize` (#796). The one
    /// answer of the lowering's `grown_item_size` and the analysis's
    /// `grown_asked`.
    /// Of a collect into a `Result` or an `Option`, the collection std's
    /// `GenericShunt` collects and its values', `T` of `Result<T, E>`
    /// (#848).
    pub(in crate::lower) fn grows_by_callers_size(&self, output: Ty<'tcx>, iter_ty: Ty<'tcx>, item: Ty<'tcx>) -> bool {
        let (output, item) = self.shunted(output, item).unwrap_or((output, item));
        self.grows_as_vec(output) && item.has_param() && self.hint_reaches_own(iter_ty)
    }

    /// Of a collect into `output`, a `Result` or an `Option` of a
    /// collection, of `item`s, `Result<T, E>` or `Option<T>`: what std's
    /// `GenericShunt` collects, that collection of `T`s, as `try_process`
    /// makes it (#848). `None` of another.
    fn shunted(&self, output: Ty<'tcx>, item: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let first = |ty: Ty<'tcx>| match ty.kind() {
            ty::Adt(_, args) => args.types().next(),
            _ => None,
        };
        let collection = match self.option_of(output) {
            Some(collection) => collection,
            None if self.is_result(output) => first(output)?,
            None => return None,
        };
        Some((collection, first(item)?))
    }
}

/// What a reserve by an iterator's hint makes room for (`reserved_by_hint`).
pub(in crate::lower) enum Room<'tcx> {
    /// A `String`'s bytes.
    Bytes,
    /// A hash table's entries of this type, of a type parameter's of their
    /// count alone, as `table_room_for` asks it.
    Entries(Ty<'tcx>),
    /// None: the hint of a `GenericShunt`, whose lower bound is none (#848).
    Nothing,
}

/// What a collect into a `Result` or an `Option` of a collection asks of
/// its iterator's hint through std's `GenericShunt`, whose hint is its
/// source's, its lower bound none, `(0, upper)`, asked while no `Err` or
/// `None` has stopped it (#848).
pub(in crate::lower) enum Shunted {
    /// Nothing the program sees.
    Never,
    /// As a `Vec` of values of this size grows (`$shuntGrowth`).
    Grown(Expr),
    /// Once, before it's stepped, as a `String`'s or a hash table's
    /// `extend` reserves, room for none (`$reserveHint(it)`).
    Reserved,
}

/// What a collect into a `Vec` std grows asks of its iterator's hint, as
/// std's `SpecFromIter` does, where it asks any (`FnCx::grown_ask`).
pub(in crate::lower) struct Grown<'tcx> {
    /// What it grows, a `Vec` std makes, of `item`s.
    output: Ty<'tcx>,
    item: Ty<'tcx>,
    /// Whether it's `TrustedLen`, its hint asked before it's stepped.
    trusted: bool,
    /// Whether it has no end its type knows, or none once it gives an
    /// item, a `cycle()`'s: a `capacity overflow` panic (ADR 0940).
    endless: bool,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `collect()` of an iterator of type `iter_ty` into `output`, a
    /// `Vec` std grows (`grows_as_vec`), of `item`s, asks of its hint: of
    /// std's struct made here, `std_struct`, not bounded by an array's
    /// items, the room its hint asks first, as std's `SpecFromIter` does;
    /// of one whose type has no end, the `capacity overflow` a
    /// `TrustedLen`'s none asks (ADR 0940); of one whose hint may run the
    /// crate's own, when it's asked (#733). `None` where it asks nothing
    /// the program sees, a `LinkedList`'s, whose `from_iter` extends a new
    /// one, among them. Of another whose hint has no end once it gives an
    /// item, a `cycle()`'s, std steps it once, then asks room for its lower
    /// bound and one more, `usize::MAX`: a `capacity overflow` of items of
    /// any size, the item it made dropped (ADR 0940, amended); of items of
    /// no size, room for any number, so it goes on as JS does. Refused
    /// where its type doesn't say which.
    pub(in crate::lower) fn grown_ask(
        &self,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Ty<'tcx>,
        std_struct: bool,
        span: Span,
    ) -> R<Option<Grown<'tcx>>> {
        let trusted = is_trusted_len(self.tcx, self.typing_env, iter_ty);
        let endless = trusted && self.recognition().endless_by_type(iter_ty);
        let sized = |item: Ty<'tcx>| self.layout(item).is_some_and(|layout| layout.size.bytes() > 0);
        let asks_room = self.recognition().grows_as_vec(output);
        let endless_once = match !trusted && asks_room && item.is_some_and(sized) {
            true => self.recognition().endless_once_stepped(iter_ty).ok_or_else(|| {
                self.unsupported(
                    span,
                    "`collect()` into a `Vec` of what may have no end only as what it holds says, a `cycle()` chained with what has an end, a `cycle()` of what's partly filtered or a `step_by(n)` of what has no end, which panics `capacity overflow` only then",
                )
            })?,
            false => false,
        };
        Ok(item
            .filter(|_| {
                asks_room
                    && (endless
                        || endless_once
                        || std_struct && !self.recognition().bounded_by_items(iter_ty)
                        || self.hint_reaches_own(iter_ty))
            })
            .map(|item| Grown {
                output,
                item,
                trusted,
                endless: endless || endless_once,
            }))
    }

    /// `it` grown as `grown` says, `$roomForHint(it, size, trusted)`: the
    /// size it's grown by (`grown_item_size`), whether it's `TrustedLen`,
    /// a caller's `ITrustedLen` where that's its caller's, `callers_trusted`
    /// (#670); and of one that's endless, or not `TrustedLen` of items with
    /// a destructor, whose first item std takes by `next` before it asks
    /// room, that item's drop, as a panic asking it unwinds (#724).
    pub(in crate::lower) fn grown_by_hint(
        &mut self,
        it: Expr,
        grown: Grown<'tcx>,
        iter_ty: Ty<'tcx>,
        callers_trusted: Option<Expr>,
        span: Span,
    ) -> R<Expr> {
        let Grown {
            output,
            item,
            trusted,
            endless,
        } = grown;
        let size = self.grown_item_size(output, iter_ty, item, span)?;
        let mut given = vec![it, size, callers_trusted.unwrap_or_else(|| Expr::bool(trusted))];
        let drop = match !trusted && self.drops(item) != crate::lower::drops::Drops::Nothing {
            true => self.drop_function(item, span)?,
            false => None,
        };
        if endless || drop.is_some() {
            given.push(Expr::bool(endless));
        }
        given.extend(drop);
        Ok(Expr::call_helper("$roomForHint", given))
    }

    /// The size of each of `item`s a `Vec` std makes of an iterator of
    /// `iter_ty` grows by, which says when it's full and asks the hint
    /// again (`$roomForHint`'s, #728): `room_item_size`; but of items whose
    /// size is its caller's where that hint may run the crate's own
    /// (`grows_by_callers_size`), the caller's, `TSize` (#796).
    fn grown_item_size(&mut self, output: Ty<'tcx>, iter_ty: Ty<'tcx>, item: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.recognition().grows_by_callers_size(output, iter_ty, item) {
            true => self.type_fact_value(item, TypeFact::Size, span),
            false => Ok(self.room_item_size(item)),
        }
    }

    /// What a collect of an iterator of `iter_ty` into `target`, a `Result`
    /// or an `Option` of a collection, of `item`s, asks of its hint
    /// through std's `GenericShunt` (`Shunted`): of a collection std grows
    /// as a `Vec`, as it grows, by its values' size, a caller's `TSize`
    /// where it's its caller's (`grows_by_callers_size`); of a `String` of
    /// `char`s or a hash table, which reserve first, once; of another, a
    /// `LinkedList` or a `String` of strings, none. Only where asking it
    /// may run the crate's own (`hint_reaches_own`).
    pub(in crate::lower) fn shunted_ask(
        &mut self,
        target: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Shunted> {
        let (Some(item), Some(iter_ty)) = (item, iter_ty) else {
            return Ok(Shunted::Never);
        };
        let Some((collection, value)) = self.recognition().shunted(target, item) else {
            return Ok(Shunted::Never);
        };
        if !self.hint_reaches_own(iter_ty) {
            return Ok(Shunted::Never);
        }
        if self.recognition().grows_as_vec(collection) {
            return Ok(Shunted::Grown(
                match self.recognition().grows_by_callers_size(target, iter_ty, item) {
                    true => self.type_fact_value(value, TypeFact::Size, span)?,
                    false => self.room_item_size(value),
                },
            ));
        }
        let chars =
            self.is_lang_adt(collection, rustc_hir::attrs::lang_items::LangItem::String) && value.peel_refs().is_char();
        Ok(
            match chars || (self.is_map(collection) && !self.is_sorted(collection)) {
                true => Shunted::Reserved,
                false => Shunted::Never,
            },
        )
    }

    /// `it`, of type `iter_ty`, asked its `size_hint` before it's stepped,
    /// as std's `extend` of a new `String` or hash table asks it, and room
    /// for its lower bound reserved as `room` says, `$reserveHint(it, size,
    /// align)`. Only where asking it may run what the program sees
    /// (`hint_reaches_own`), as std's own hints change nothing but that
    /// room, which no JS string or table reaches (#733).
    pub(in crate::lower) fn reserved_by_hint(&self, it: Expr, iter_ty: Option<Ty<'tcx>>, room: Room<'tcx>) -> Expr {
        if !iter_ty.is_some_and(|iter_ty| self.hint_reaches_own(iter_ty)) {
            return it;
        }
        let mut args = vec![it];
        match room {
            Room::Nothing => {}
            Room::Bytes => args.push(Expr::int(1)),
            Room::Entries(entry) => {
                let (size, align) = match self.layout(entry) {
                    Some(layout) => (i128::from(layout.size.bytes()), i128::from(layout.align.abi.bytes())),
                    None => (0, 1),
                };
                args.extend([Expr::int(size), Expr::int(align)]);
            }
        }
        Expr::call_helper("$reserveHint", args)
    }
}
