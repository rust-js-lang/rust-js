//! A std function's call, of its arguments' values (ADR 0364), each
//! handed to its type's module.

use crate::js::{Expr, Stmt};
use crate::lower::calls::Call;
use crate::lower::combinators::IterSource;
use crate::lower::combinators::{Comb, StepOp};
use crate::lower::drops::Drops;
use crate::lower::pointers::PointerOp;
use crate::lower::recognition::{ChannelOp, Std};
use crate::lower::std_types::any::AnyOp;
use crate::lower::std_types::heap::HeapOp;
use crate::lower::std_types::lazy::LazyOp;
use crate::lower::std_types::map::MapOp;
use crate::lower::std_types::once::OnceOp;
use crate::lower::std_types::option::OptionPlaceOp;
use crate::lower::std_types::slice::SliceOp;
use crate::lower::std_types::text::TextOp;
use crate::lower::{FnCx, R};
use rustc_ast::Mutability;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std function that takes a value with a destructor, or changes a
    /// place that holds one, must keep or give back what it takes: these
    /// do. Another might drop it, which JS wouldn't (ADR 0098); one of a type
    /// parameter's only is one its callers give none of (ADR 0190). `tys`:
    /// its arguments' types; `drained`: a chain it takes owns its items.
    pub(in crate::lower) fn check_takes_drops(
        &mut self,
        known: Std,
        def_id: DefId,
        tys: &[Ty<'tcx>],
        drained: bool,
        span: Span,
    ) -> R<()> {
        let holds_drops = |ty: Ty<'tcx>| self.drops(ty) == Drops::Runs;
        let takes_drops = tys.iter().any(|&ty| match *ty.kind() {
            ty::Ref(_, inner, Mutability::Mut) => holds_drops(inner),
            ty::Ref(..) => false,
            _ => holds_drops(ty),
        });
        // Its value moves into the `Ok`; its function, run only for a `None`,
        // would be dropped unrun, so it mustn't hold one.
        let keeps_value = known == Std::Comb(Comb::OkOrElse) && !holds_drops(tys[1]);
        // `v.extend(items)` moves each into `v`, and drops `items` once it's
        // given them all: that, of what holds nothing then, is nothing.
        let moves_all = known == Std::Comb(Comb::Extend) && self.ends_holding_nothing(tys[1]);
        if takes_drops
            && !drained
            && !keeps_value
            && !moves_all
            && !matches!(
                known,
                Std::Drop
                    | Std::Forget
                    | Std::Swap
                    | Std::Replace
                    | Std::Push
                    | Std::Same
                    | Std::Leak
                    // Moves it into its iterator, whose drop is its item's.
                    | Std::OptionIter
                    | Std::IterSource(IterSource::Once)
                    // Holds it, a clone each step, and drops it when it's
                    // closed (ADR 0422).
                    | Std::IterSource(IterSource::Repeat)
                    // Counts what it takes, or gives it back (ADR 0320).
                    | Std::Rc(_)
                    // A map keeps what it takes, and gives back what it replaces or
                    // removes; `or_insert_with` runs its function only to insert
                    // (ADR 0321).
                    | Std::Map(
                        MapOp::Insert
                            | MapOp::Remove
                            | MapOp::Get
                            | MapOp::Index
                            | MapOp::Has
                            | MapOp::Iter(_)
                            | MapOp::Entry
                            | MapOp::OrInsertWith
                            | MapOp::OrDefault
                            | MapOp::Len
                            | MapOp::IsEmpty
                    )
                    // A cell keeps it, and gives the old one back; a `Cell` of one is
                    // refused by its type (ADR 0320).
                    | Std::CellNew
                    | Std::CellReplace
                    | Std::CellTake
                    | Std::CellReplaceWith
                    // Moves its value into the function, which owns it then, and
                    // drops the other variant's, or passes it on (ADR 0179).
                    | Std::OptionMap
                    | Std::Comb(
                        Comb::ResultMap
                            | Comb::Filter
                            | Comb::MapOr
                            | Comb::MapOrElse
                            | Comb::AndThen
                            | Comb::UnwrapOrElse
                            | Comb::IsSomeAnd
                            | Comb::IsNoneOr
                            | Comb::MapErr
                            | Comb::ResultMapOr
                            | Comb::ResultMapOrElse
                            | Comb::ResultAndThen
                            | Comb::ResultUnwrapOrElse
                            | Comb::IsOkAnd
                            | Comb::IsErrAnd
                            | Comb::Err
                    )
                    | Std::ResultOk
                    | Std::VecMacro
                    | Std::Unwrap
                    | Std::UnwrapErr
                    | Std::UnwrapUnchecked
                    | Std::UnwrapOk
                    | Std::Method("pop")
                    | Std::Index
                    | Std::Len
                    | Std::IsEmpty
                    // What a guard guards, through its `&mut` (ADR 0328).
                    | Std::GuardValue { .. }
                    // A heap's top, through its `PeekMut`, and the top it pops (ADR 0333).
                    | Std::Heap(HeapOp::PeekTop { .. } | HeapOp::PeekPop)
                    // A raw pointer's: what it writes stays where it points,
                    // what it reads or replaces it gives back, and what it
                    // drops in place, it drops (ADR 0407).
                    | Std::Pointer(_)
                    // A write, through a `&mut` to the crate's writer, which it
                    // writes to and doesn't drop (ADRs 0166, 0392).
                    | Std::UserWrite
                    | Std::UserIoWrite
                    // What moves or lends items in place, or adds clones, and
                    // drops none: `swap`, a slice's item or a generic index's
                    // as a place, `extend_from_slice`; and the allocator's,
                    // `set_len` leaving what's past it (ADRs 0417, 0420).
                    | Std::Comb(Comb::Swap | Comb::ExtendFromSlice)
                    | Std::SliceGet
                    | Std::Slice(SliceOp::GenericIndex { mutable: true })
                    | Std::Alloc(_)
                    // A channel's queue, which its receiver's drop drops what's
                    // left of, and a failed send gives back (ADR 0142).
                    | Std::Channel(ChannelOp::Send)
                    // What moves a vector's items, in it, out or in, and drops
                    // none: what it takes out it gives back, `remove`'s item,
                    // `split_off`'s vector, `mem::take`'s old value (ADR 0098).
                    | Std::Comb(
                        Comb::Insert | Comb::Remove | Comb::SwapRemove | Comb::SplitOff | Comb::Rotate { .. }
                    )
                    | Std::Append
                    | Std::Method("reverse")
                    | Std::Sort
                    | Std::SortBy
                    | Std::SortByKey
                    // A `VecDeque`'s at its front, and `insert` and `remove`.
                    | Std::Method("unshift" | "shift")
                    | Std::Slice(SliceOp::DequeInsert { .. })
                    | Std::DequeRemove
                    // An `Option`'s that moves what it holds out, or in, its
                    // old value given back (ADR 0098).
                    | Std::OptionTake
                    | Std::OptionReplace
                    | Std::OptionPlace(OptionPlaceOp::TakeIf)
                    // A map's that drop the values they take out (ADR 0321).
                    | Std::Map(MapOp::Retain | MapOp::Extend | MapOp::Clear | MapOp::OrInsert)
                    // And what drops what it doesn't keep, `insert`'s old value
                    // or `get_or_insert`'s given one.
                    | Std::OptionPlace(
                        OptionPlaceOp::Insert
                            | OptionPlaceOp::GetOrInsert
                            | OptionPlaceOp::GetOrInsertWith
                            | OptionPlaceOp::GetOrInsertDefault
                    )
                    | Std::Slice(
                        SliceOp::SortUnstable { .. } | SliceOp::SelectNth { .. } | SliceOp::SortByCachedKey
                    )
                    | Std::MemTake
                    // What drops what it takes out of a vector, given its
                    // items' drop, as std drops each (ADR 0098).
                    | Std::Comb(Comb::Truncate | Comb::DedupBy | Comb::DedupByKey | Comb::ResizeWith)
                    | Std::Clear
                    | Std::Retain
                    // What it takes out, given back as what's drained, which
                    // drops what's left of it as an `IntoIter` does.
                    | Std::Text(TextOp::Drain)
            )
        {
            // Of a type parameter's only, a generic iterator's chrono folds:
            // one its callers give none of (ADR 0190).
            let held: Vec<Ty<'tcx>> = tys
                .iter()
                .filter_map(|&ty| match *ty.kind() {
                    ty::Ref(_, inner, Mutability::Mut) => Some(inner),
                    ty::Ref(..) => None,
                    _ => Some(ty),
                })
                .filter(|&ty| self.drops(ty) != Drops::Nothing)
                .collect();
            if !held
                .iter()
                .all(|&ty| self.drop_query().drops_but_params(ty) == Drops::Nothing)
            {
                let path = self.tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("`{path}` of a value with a destructor")));
            }
            for ty in held {
                self.require_no_drops(ty);
            }
        }
        Ok(())
    }

    /// Whether what `ty`, an iterator or what makes one, drops once it's
    /// given all its items is nothing: an array's, a `Vec`'s, an `Option`'s,
    /// their iterators', or one closed as it ends, `repeat(x).take(n)`, whose
    /// end drops what it holds (ADR 0422). Not the crate's, whose `Drop` runs.
    fn ends_holding_nothing(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        let recognition = self.recognition();
        matches!(ty.kind(), ty::Array(..))
            || self.is_vec_like(ty)
            || self.option_of(ty).is_some()
            || recognition.array_source(ty) == Some(true)
            || recognition.owns_one_item(ty)
            || recognition.closed_on_drop(ty).is_some()
    }

    /// Whether `known` gives an `Option` whose `Some` is boxed where it looks
    /// like `None` (ADR 0051), and isn't one of those that make one boxed.
    pub(in crate::lower) fn refuses_boxed_option(&self, known: Std, output: Ty<'tcx>) -> bool {
        self.option_of(output).is_some_and(|inner| self.boxed_payload(inner))
            && !matches!(
                known,
                Std::Same
            | Std::OptionMap
            | Std::Method("pop")
            | Std::First
            | Std::SliceLast
            | Std::SliceGet
            | Std::OptionCloned
            | Std::Comb(Comb::Then | Comb::ThenSome)
            // An `Option` of the same type, or the closure's own.
            | Std::Comb(Comb::Filter | Comb::Or | Comb::OrElse | Comb::AndThen)
            | Std::Last
            // What a pointer points to, boxed as it's given (ADR 0407).
            | Std::Pointer(PointerOp::AsRef | PointerOp::AsMut)
            // A map's value it had, or has, boxed by `$insert`, `$remove`, `$someGet`.
            | Std::Map(MapOp::Get | MapOp::Insert | MapOp::Remove)
            // `as_ref()`: the same value, box and all.
            | Std::Pointee
            | Std::ResultOk
            // A downcast's `Some`, boxed as it's made (ADR 0331).
            | Std::Any(AnyOp::DowncastRef | AnyOp::DowncastMut)
            | Std::ArrayMethod("find")
            | Std::Extreme(_)
            | Std::Step(StepOp::Next | StepOp::Peek)
            // The old value, as it's kept: boxed already.
            | Std::OptionTake
            // A `OnceCell`'s, kept as `$some` makes it, and a `LazyCell`'s.
            | Std::Once(OnceOp::Get | OnceOp::Take)
            | Std::Lazy(LazyOp::Get)
            | Std::OptionReplace
            // The inner `Option`, box and all.
            | Std::OptionFlatten
            )
    }

    /// The call of a std function rust-js knows, of its arguments' values
    /// (ADR 0364). `boxed`: whether the `Option` it gives boxes its `Some`
    /// (ADR 0051); `output`, what it gives.
    pub(in crate::lower) fn std_values(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: Vec<Expr>,
        boxed: bool,
        output: Ty<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let Call {
            def_id,
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut values = values.into_iter();
        // A counted `Rc` (ADR 0320): `Rc::new` makes its `{ value, .. }`, and
        // what it points at is its `value`.
        if known == Std::Same && self.counted_same_by(def_id, generic_args).is_some() {
            let (_, pointee) = self.recognition().rc_same(def_id, generic_args).expect("an `Rc`'s");
            self.counted_here(pointee, span)?;
            let value = values.next().expect("rustc checked the arguments");
            return Ok(self
                .counted_same(def_id, generic_args, value)
                .expect("a counted `Rc`'s"));
        }
        if let Some(js) = self.vec_call(known, call, &mut values, boxed, out)? {
            return Ok(js);
        }
        if let Some(js) = self.option_call(known, call, &mut values, boxed, out)? {
            return Ok(js);
        }
        if let Some(js) = self.cell_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.once_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.lazy_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.pin_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.any_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.process_call(known, &mut values) {
            return Ok(js);
        }
        if let Some(js) = self.location_call(known, call, &mut values) {
            return Ok(js);
        }
        if let Some(js) = self.alloc_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.path_call(known, &mut values) {
            return Ok(js);
        }
        if let Some(js) = self.pointer_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.transmute_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.uninit_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.channel_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.numeric_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        if let Some(js) = self.iter_source_call(known, call, &mut values)? {
            return Ok(js);
        }
        if let Some(js) = self.string_call(known, call, &mut values)? {
            return Ok(js);
        }
        if let Some(js) = self.print_call(known, call, &mut values, out)? {
            return Ok(js);
        }
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(match known {
            Std::Swap | Std::Replace | Std::OptionTake | Std::OptionReplace | Std::MemTake => {
                unreachable!("lowered from their places, above")
            }
            // An `Rc` is the JS reference itself: the garbage collector does
            // its counting, so a clone is the same object.
            Std::Same | Std::Format => arg(),
            // Its variant's discriminant, of the enum it's a reference to.
            Std::Discriminant => {
                let of = generic_args.types().next().expect("`discriminant`'s type argument");
                self.discriminant_value(arg(), of, out)
            }
            Std::Leak => self.leaked(arg(), output),
            Std::Pointee => self.through_refs(arg(), tys[0]).0,
            Std::Last
            | Std::Cloned
            | Std::ArrayMethod(_)
            | Std::Enumerate
            | Std::Rev
            | Std::Skip
            | Std::Take
            | Std::Fold
            | Std::Sum
            | Std::CollectString
            | Std::Collect
            | Std::Position
            | Std::Extreme(_)
            | Std::Sort
            | Std::SortByKey => {
                unreachable!("handled above")
            }
            Std::PushStr | Std::AssignOperator(_) | Std::StringEdit(_) | Std::StringWithCapacity => {
                unreachable!("handled above")
            }
            Std::VecMacro | Std::FmtNew | Std::AssertFailed => unreachable!("handled above"),
            Std::Map(_)
            | Std::Range(_)
            | Std::Stream(_)
            | Std::TypeName { .. }
            | Std::Comb(_)
            | Std::IterComb(_)
            | Std::Text(_)
            | Std::Number(_)
            | Std::FromElem
            | Std::Heap(_)
            | Std::ExactLen
            | Std::SizeHint(_)
            | Std::WrappingOp(..)
            | Std::IterByRef
            | Std::NonZeroNew
            | Std::UserWrite
            | Std::UserIoWrite
            | Std::DequeRemove
            | Std::Step(_)
            | Std::ToJson(_)
            | Std::FromJson => {
                unreachable!("handled above")
            }
            Std::Method(_)
            | Std::First
            | Std::SliceLast
            | Std::SliceGet
            | Std::ToVec
            | Std::JoinItems
            | Std::SortBy
            | Std::IsEmpty
            | Std::VecNew
            | Std::Nothing
            | Std::Append
            | Std::WriteText
            | Std::Push
            | Std::Len
            | Std::Index
            | Std::Clear
            | Std::Retain => unreachable!("lowered by vec_call"),
            Std::Then
            | Std::ThenWith
            | Std::IsOk(_)
            | Std::UnwrapOk
            | Std::UnwrapErr
            | Std::ResultOk
            | Std::ResultOr
            | Std::IsSome
            | Std::IsNone
            | Std::Unwrap
            | Std::UnwrapUnchecked
            | Std::UnwrapOr
            | Std::OptionMap
            | Std::OptionIter
            | Std::OptionIterMut
            | Std::OptionCloned
            | Std::OptionFlatten => unreachable!("lowered by option_call"),
            Std::Channel(_) => unreachable!("lowered by channel_call"),
            Std::CellNew
            | Std::CellGet
            | Std::CellSet
            | Std::CellReplace
            | Std::CellTake
            | Std::CellReplaceWith
            | Std::Borrow { .. }
            | Std::TryBorrow { .. }
            | Std::GuardValue { .. }
            | Std::Lock
            | Std::Drop
            | Std::Forget
            | Std::AtomicLoad
            | Std::AtomicStore
            | Std::AtomicSwap
            | Std::AtomicFetch(_)
            | Std::AtomicFetchMax(_)
            | Std::AtomicCompareExchange
            | Std::LocalWith
            | Std::LocalBorrow { .. }
            | Std::CellGetMut
            | Std::CellSwap
            | Std::CellUpdate
            | Std::NotPoisoned => unreachable!("lowered by cell_call"),
            Std::Once(_) => unreachable!("lowered by once_call"),
            Std::Lazy(_) => unreachable!("lowered by lazy_call"),
            Std::Pin(_) => unreachable!("lowered by pin_call"),
            Std::Any(_) => unreachable!("lowered by any_call"),
            Std::Process(_) => unreachable!("lowered by process_call"),
            Std::Location(_) => unreachable!("lowered by location_call"),
            Std::Alloc(_) => unreachable!("lowered by alloc_call"),
            Std::Capacity(_) => unreachable!("lowered by vec_call"),
            Std::Path(_) => unreachable!("lowered by path_call"),
            Std::Pointer(_) => unreachable!("lowered by pointer_call"),
            Std::Uninit(_) => unreachable!("lowered by uninit_call"),
            Std::Transmute => unreachable!("lowered by transmute_call"),
            Std::Cow(_) => unreachable!("lowered by mir_std_call"),
            Std::Rc(_) => unreachable!("lowered by mir_std_call"),
            Std::Slice(_) => unreachable!("lowered by mir_std_call"),
            Std::OptionPlace(_) => unreachable!("lowered by mir_std_call"),
            Std::PtrEq => unreachable!("lowered by mir_std_call"),
            Std::ToBig
            | Std::Cast { .. }
            | Std::Duration(_)
            | Std::SliceToArray { .. }
            | Std::TryFromInt { .. }
            | Std::FromDigit
            | Std::FromU32
            | Std::Cmp
            | Std::MaxOf(_)
            | Std::Operator(_)
            | Std::UnaryOperator(_)
            | Std::Reverse
            | Std::Layout { .. }
            | Std::NeedsDrop => unreachable!("lowered by numeric_call"),
            Std::IterSource(_) => unreachable!("lowered by iter_source_call"),
            Std::Concat
            | Std::StripPrefix
            | Std::StripSuffix
            | Std::StripCircumfix
            | Std::SplitOnce
            | Std::RsplitOnce
            | Std::Chars
            | Std::StringNew
            | Std::Trim { .. }
            | Std::AsciiCase { .. }
            | Std::AsciiEq
            | Std::ToString => unreachable!("lowered by string_call"),
            Std::Panic
            | Std::PanicFmt
            | Std::PanicDisplay
            | Std::BeginPanic
            | Std::Unreachable
            | Std::Print { .. }
            | Std::FmtStr
            | Std::FmtDisplay
            | Std::FmtDebug
            | Std::FmtRadix(_)
            | Std::FmtExp(_)
            | Std::FmtPointer
            | Std::FmtUsize => unreachable!("lowered by print_call"),
        })
    }
}
