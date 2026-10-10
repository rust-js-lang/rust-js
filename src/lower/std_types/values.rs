//! A std function's call, of its arguments' values: what THIR's lowering
//! and MIR's share (ADR 0364), each handed to its type's module.

use crate::js::{Expr, Stmt};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};
use rustc_middle::ty::Ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std function rust-js knows, of its arguments' values: what THIR
    /// and MIR both lower its call to (ADR 0364). `boxed`: whether the
    /// `Option` it gives boxes its `Some` (ADR 0051); `output`, what it gives.
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
            Std::Leak => self.leaked(arg(), output),
            Std::Pointee => self.through_refs(arg(), tys[0]).0,
            Std::Last
            | Std::Cloned
            | Std::Fuse
            | Std::ArrayMethod(_)
            | Std::Enumerate
            | Std::Rev
            | Std::Skip
            | Std::Take
            | Std::Fold
            | Std::Sum
            | Std::CollectString
            | Std::CollectFallible
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
            | Std::IterLen
            | Std::ExactLen
            | Std::SizeHint(_)
            | Std::WrappingOp(..)
            | Std::GenericSizeHint
            | Std::IterByRef
            | Std::NonZeroNew
            | Std::UserWrite
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
            Std::Uninit(_) => unreachable!("lowered by uninit_call"),
            Std::Cow(_) => unreachable!("lowered by cow_call"),
            Std::Rc(_) => unreachable!("lowered by rc_call"),
            Std::Slice(_) => unreachable!("lowered by slice_call"),
            Std::OptionPlace(_) => unreachable!("lowered by option_place"),
            Std::PtrEq => unreachable!("lowered by ptr_eq"),
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
            | Std::SizeOf
            | Std::AlignOf
            | Std::SizeOfVal => unreachable!("lowered by numeric_call"),
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
