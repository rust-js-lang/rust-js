//! Which of std's conversions a `From`, an `Into`, a `TryFrom` or a
//! `TryInto` is, which `calls.rs`'s trait methods ask.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// A `From`'s or an `Into`'s conversion, of std's, if rust-js knows it:
    /// one list of what it converts into what, `conversion_sides`', the same
    /// of either.
    pub(super) fn classify_conversion(&self, trait_: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let into = tcx.is_diagnostic_item(sym::Into, trait_);
        if !into && !tcx.is_diagnostic_item(sym::From, trait_) {
            return None;
        }
        let (from_ty, to_ty) = conversion_sides(tcx, trait_, args)?;
        // `From<T> for T`, std's for every type, and the `Into` it gives:
        // the value itself, of an `impl Into<T>` that is a `T` too (ADR 0387).
        if tcx.erase_and_anonymize_regions(self.reveal(from_ty)) == tcx.erase_and_anonymize_regions(self.reveal(to_ty))
        {
            return Some(Std::Same);
        }
        // `VecDeque::from(v)` is a copy of `v`, which may be a clone that
        // was never made (ADR 0052), and so is a `Vec` of an array, of a
        // slice, its items cloned, or of a deque; `BinaryHeap::from(v)` puts
        // one in heap order.
        let vec = self.is_std_type(to_ty, StdItem::Vec);
        if let Some(known) = c_str_conversion(tcx, from_ty, to_ty, vec) {
            return Some(known);
        }
        let items =
            from_ty.is_array() || from_ty.peel_refs().is_slice() || self.is_std_type(from_ty, StdItem::VecDeque);
        if self.is_std_type(to_ty, StdItem::VecDeque) || self.is_std_type(to_ty, StdItem::LinkedList) || (vec && items)
        {
            return Some(Std::ToVec);
        }
        // A heap's `Vec` is the array it is, as `into_vec()` gives it.
        if vec && self.is_std_type(from_ty, StdItem::BinaryHeap) {
            return Some(Std::Same);
        }
        // A `Vec<u8>` of text, its bytes, as `into_bytes()` gives them.
        if vec && (from_ty.peel_refs().is_str() || self.is_lang_adt(from_ty, LangItem::String)) {
            return Some(Std::Text(TextOp::IntoBytes));
        }
        if self.is_std_type(to_ty, StdItem::BinaryHeap) {
            return Some(Std::Heap(HeapOp::From));
        }
        // `HashMap::from([(k, v)])`: `new Map([[k, v]])`.
        if self.is_map(to_ty) {
            return Some(Std::Map(MapOp::From {
                set: self.is_set(to_ty),
            }));
        }
        // `Option::from(x)`: `Some(x)`.
        if self.option_of(to_ty) == Some(from_ty) {
            return Some(Std::OptionFrom);
        }
        // A `Cow` of a reference to what it borrows, or to what owns it,
        // `Borrowed`, or of what owns, `Owned`.
        if let Some((borrowed, owned)) = self.cow_parts(to_ty) {
            let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
            let of_owned = erased(from_ty) == erased(owned);
            let lent = matches!(from_ty.kind(), ty::Ref(_, to, Mutability::Not)
                if erased(*to) == erased(borrowed) || erased(*to) == erased(owned));
            if of_owned || lent {
                return Some(Std::Cow(CowOp::From { owned: of_owned }));
            }
        }
        // A `String` of a `Box<str>`, its text, or of a `Cow<str>`, what it
        // owns or borrows, owned (ADR 0063).
        if self.is_lang_adt(to_ty, LangItem::String) {
            if from_ty.is_box() && from_ty.boxed_ty().is_some_and(|text| text.is_str()) {
                return Some(Std::Same);
            }
            if self.cow_parts(from_ty).is_some_and(|(borrowed, _)| borrowed.is_str()) {
                return Some(Std::Cow(CowOp::IntoOwned));
            }
        }
        // An `Rc` or an `Arc` of the value, which `rc_same` says it makes, as
        // `Rc::new(x)`: of a `str` its text, of a `[T]` a `Vec<T>`'s items.
        if rc_made_of(tcx, from_ty, to_ty).is_some() {
            return Some(Std::Same);
        }
        let (from_ty, to_ty) = (Some(from_ty), Some(to_ty));
        // std's own conversions that change nothing in JS (ADR 0063): to a
        // `String` or a `Box<str>` (ADR 0191) from a `&str` or a `char`, and
        // between numbers, which only widen.
        if let (Some(from_ty), Some(to_ty)) = (from_ty, to_ty) {
            let boxed_str = to_ty.is_box() && to_ty.boxed_ty().is_some_and(|inner| inner.is_str());
            if (self.is_lang_adt(to_ty, LangItem::String) || self.is_path_like(to_ty) || boxed_str)
                && self.is_string_like(from_ty)
            {
                return Some(Std::Same);
            }
            // A `char` from a `u8`: the code point it is (ADR 0157).
            if to_ty.is_char() && matches!(from_ty.kind(), ty::Uint(ty::UintTy::U8)) {
                return Some(Std::Text(TextOp::CharFromByte));
            }
            // Into a `Box`, which is its value (ADR 0023): `Box::from(x)`, and
            // a `Vec`'s items as a boxed slice.
            if let ty::Adt(_, boxed) = to_ty.kind()
                && to_ty.is_box()
                && (boxed.type_at(0) == from_ty
                    || matches!((boxed.type_at(0).kind(), from_ty.kind()), (ty::Slice(item), ty::Adt(_, vec))
                        if self.is_std_type(from_ty, StdItem::Vec) && vec.type_at(0) == *item))
            {
                return Some(Std::Same);
            }
            // A number from a `bool` or a `char`, as `as` makes it (ADR 0358).
            if (from_ty.is_bool() || from_ty.is_char()) && Num::of(to_ty).is_some() {
                return Some(Std::Cast { into });
            }
            // Into a BigInt from a number (ADR 0086), else the same.
            if let (Some(from), Some(to)) = (Num::of(from_ty.peel_refs()), Num::of(to_ty)) {
                return Some(if to.big() && !from.big() { Std::ToBig } else { Std::Same });
            }
        }
        None
    }
}

/// A `TryFrom` or a `TryInto` of `from` into `to` std makes, `into` of
/// `try_into()`: between integers, which may not fit; a `u32` into a
/// `char`, or a `char` into a `u8`.
pub(super) fn try_conversion<'tcx>(from: Ty<'tcx>, to: Ty<'tcx>, into: bool) -> Option<Std> {
    if let (Some(a), Some(b)) = (Num::of(from), Num::of(to))
        && !a.float()
        && !b.float()
    {
        return Some(Std::TryFromInt { into });
    }
    match (from.kind(), to.kind()) {
        (ty::Uint(ty::UintTy::U32), ty::Char) => Some(Std::TryFromChar { to_char: true }),
        (ty::Char, ty::Uint(ty::UintTy::U8)) => Some(Std::TryFromChar { to_char: false }),
        _ => None,
    }
}
