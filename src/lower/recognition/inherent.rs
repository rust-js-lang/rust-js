//! Which `Std` an inherent method of a std type is.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// A method of a std type's own, by its type and its name.
    pub(super) fn classify_inherent(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let self_ty = args.types().next();
        let owner = tcx
            .type_of(tcx.inherent_impl_of_assoc(def_id)?)
            .instantiate_identity()
            .skip_normalization();
        let adt = |item: StdItem| self.is_std_type(owner, item);
        let string = self.is_lang_adt(owner, LangItem::String);
        let option = self.is_lang_adt(owner, LangItem::Option);
        let range = self.range_kind(owner);
        let result = self.is_std_type(owner, StdItem::Result);
        let ordering = self.is_lang_adt(owner, LangItem::OrderingEnum);
        let local_key = adt(StdItem::LocalKey);
        // A thread-local of a cell, whose own `take` and `replace` it has.
        let local_cell = local_key && cell_of_key(tcx, owner) != owner;
        let rc = rc_pointee(tcx, owner).is_some();
        let weak = weak_pointee(tcx, owner).is_some();
        let lazy = self.is_std_type(owner, StdItem::LazyCell);
        let cow = self.is_cow(owner);
        let arguments = self.is_lang_adt(owner, LangItem::FormatArguments);
        let argument = self.is_lang_adt(owner, LangItem::FormatArgument);
        let of = MapOwner {
            map: adt(StdItem::HashMap) || adt(StdItem::BTreeMap) || self.is_json_map(owner),
            set: adt(StdItem::HashSet) || adt(StdItem::BTreeSet),
            std_map: adt(StdItem::HashMap) || adt(StdItem::BTreeMap),
            sorted: self.is_sorted(owner),
            placed: matches!(owner.kind(), ty::Adt(adt, _) if is_index_map(tcx, adt.did())),
            hashed: adt(StdItem::HashMap) || adt(StdItem::HashSet),
            entry: adt(StdItem::HashMapEntry) || adt(StdItem::BTreeEntry),
            occupied: self.is_occupied_entry(owner),
            vacant: self.is_vacant_entry(owner),
        };
        let name = tcx.item_name(def_id);
        if (adt(StdItem::Cell) || adt(StdItem::RefCell))
            && let Some(cell) = cell_method(name.as_str(), adt(StdItem::RefCell))
        {
            return Some(cell);
        }
        if let Some(op) = once_method(tcx, def_id) {
            return Some(Std::Once(op));
        }
        // A clock's: of its nanoseconds (ADR 0447).
        if let Some(clock) = clock_type_of(owner) {
            return clock_method(clock, name.as_str());
        }
        // A `Duration`'s: of its nanoseconds (ADR 0188).
        if is_duration_ty(owner) {
            const SECOND: i64 = 1_000_000_000;
            return Some(Std::Duration(match name.as_str() {
                "new" => DurationOp::New,
                "from_secs" => DurationOp::From(SECOND),
                "from_millis" => DurationOp::From(1_000_000),
                "from_micros" => DurationOp::From(1_000),
                "from_nanos" => DurationOp::From(1),
                "as_secs" => DurationOp::As(SECOND),
                "as_millis" => DurationOp::As(1_000_000),
                "as_micros" => DurationOp::As(1_000),
                "as_nanos" => DurationOp::As(1),
                "subsec_nanos" => DurationOp::Subsec(1),
                "subsec_micros" => DurationOp::Subsec(1_000),
                "subsec_millis" => DurationOp::Subsec(1_000_000),
                "is_zero" => DurationOp::IsZero,
                "checked_add" => DurationOp::Checked { add: true },
                "checked_sub" => DurationOp::Checked { add: false },
                _ => return None,
            }));
        }
        // A `LinkedList` is an array as a deque is (ADR 0316).
        let (deque, heap) = (
            adt(StdItem::VecDeque) || adt(StdItem::LinkedList),
            adt(StdItem::BinaryHeap),
        );
        // Theirs first: `push` and `pop` keep a heap's order, and a deque's
        // `remove` is an `Option` (ADR 0068).
        let peekable = self.is_peekable(owner);
        let chars = self.is_str_chars(owner) || self.is_string_drain(owner);
        let slice_iter = adt(StdItem::SliceIter)
            || matches!(owner.kind(), ty::Adt(it, _) if std_path(tcx, it.did()) == "std::vec::IntoIter");
        let own = match name.as_str() {
            "send" if self.channel_end(owner) == Some(ChannelEnd::Sender) => Some(Std::Channel(ChannelOp::Send)),
            "recv" if self.channel_end(owner) == Some(ChannelEnd::Receiver) => Some(Std::Channel(ChannelOp::Recv)),
            "try_recv" if self.channel_end(owner) == Some(ChannelEnd::Receiver) => {
                Some(Std::Channel(ChannelOp::TryRecv))
            }
            name if peekable && let Some(step) = peekable_step(name) => Some(Std::Step(step)),
            "as_str" if chars || self.is_char_indices(owner) => Some(Std::Step(StepOp::AsStr)),
            "offset" if self.is_char_indices(owner) => Some(Std::Step(StepOp::Offset)),
            "as_slice" if slice_iter => Some(Std::Step(StepOp::AsSlice)),
            "push" if heap => Some(Std::Heap(HeapOp::Push)),
            "pop" if heap => Some(Std::Heap(HeapOp::Pop)),
            "peek" if heap => Some(Std::First),
            "into_sorted_vec" if heap => Some(Std::Heap(HeapOp::IntoSorted)),
            "into_vec" if heap => Some(Std::Same),
            // In the order std's heap keeps them (ADR 0333).
            "append" if heap => Some(Std::Heap(HeapOp::Append)),
            "retain" if heap => Some(Std::Heap(HeapOp::Retain)),
            "drain" if heap => Some(Std::Heap(HeapOp::Drain)),
            "peek_mut" if heap => Some(Std::Heap(HeapOp::PeekMut)),
            "as_slice" | "make_contiguous" if heap || deque => Some(Std::Same),
            "remove" if deque => Some(Std::DequeRemove),
            "extract_if" if adt(StdItem::Vec) || adt(StdItem::LinkedList) => Some(Std::Slice(SliceOp::ExtractIf {
                range: adt(StdItem::Vec),
            })),
            // A `VecDeque`'s `insert`, checked with its own message, and the
            // `_mut` ones' `&mut` to what they put in (ADR 0333).
            "insert" | "insert_mut" if adt(StdItem::VecDeque) => Some(Std::Slice(SliceOp::DequeInsert {
                mutable: name.as_str() == "insert_mut",
            })),
            "push_back_mut" if deque => Some(Std::Slice(SliceOp::PushMut { at: false })),
            "push_front_mut" if deque => Some(Std::Slice(SliceOp::PushFrontMut)),
            "push_back" if deque => Some(Std::Push),
            "pop_back" if deque => Some(Std::Method("pop")),
            "push_front" if deque => Some(Std::Method("unshift")),
            "pop_front" if deque => Some(Std::Method("shift")),
            "front" | "front_mut" if deque => Some(Std::First),
            "back" | "back_mut" if deque => Some(Std::SliceLast),
            "as_slices" | "as_mut_slices" if deque => Some(Std::Slice(SliceOp::DequeSlices)),
            // A JS map or string has no capacity: asking for some does nothing
            // (ADRs 0315, 0322), but a `String`'s and a hash table's room is
            // checked (ADR 0774); an array collection's is `capacity_op`'s (0419).
            "reserve" | "reserve_exact" if string => Some(Std::Text(TextOp::Reserve)),
            "reserve" | "reserve_exact" if (of.map || of.set) && !of.hashed => Some(Std::Nothing),
            "shrink_to" | "shrink_to_fit" if of.map || of.set || string => Some(Std::Nothing),
            "into_boxed_slice" if adt(StdItem::Vec) => Some(Std::Same),
            // A `Box<str>` is its string, and a `Box<[T]>` its array.
            "into_boxed_str" if string => Some(Std::Same),
            // A `String`'s bytes, a new array, as a `str`'s are.
            "as_bytes" | "into_bytes" if string => Some(Std::Text(TextOp::Bytes)),
            "into_string" if owner.is_str() => Some(Std::Same),
            "into_vec" if owner.is_slice() => Some(Std::Same),
            "into_flattened" if adt(StdItem::Vec) => Some(Std::Comb(Comb::Concat)),
            "splice" if adt(StdItem::Vec) => Some(Std::Text(TextOp::Splice)),
            "extend_from_within" if adt(StdItem::Vec) => Some(Std::Text(TextOp::ExtendFromWithin)),
            // A `Vec` is its array, and the whole of it the same array: what's
            // written through `as_mut_slice()` is the `Vec`'s. An array's is it too.
            "as_slice" | "as_mut_slice" if adt(StdItem::Vec) || owner.is_array() => Some(Std::Same),
            "drain" if adt(StdItem::Vec) || deque => Some(Std::Text(TextOp::Drain)),
            "iter" | "iter_mut" if deque || heap => Some(Std::Same),
            "new" if deque || heap => Some(Std::VecNew),
            "len" if deque || heap => Some(Std::Len),
            "is_empty" if deque || heap => Some(Std::IsEmpty),
            "clear" if deque || heap => Some(Std::Clear),
            "retain" if deque => Some(Std::Retain),
            _ => None,
        };
        if own.is_some() {
            return own;
        }
        if let Some(op) = methods::text(name.as_str(), owner.is_char(), owner.is_str()) {
            return Some(Std::Text(op));
        }
        if let Some(num) = Num::of(owner)
            && let Some(op) = methods::number(name.as_str(), num)
        {
            return Some(Std::Number(op));
        }
        if let Some(comb) = methods::combinator(
            name.as_str(),
            option,
            result,
            adt(StdItem::Vec) || deque,
            adt(StdItem::Vec) || deque || owner.is_slice(),
        )
        .or_else(|| methods::boolean(name.as_str(), owner.is_bool()))
        {
            // A `VecDeque`'s assertion calls the count `n`.
            let comb = match comb {
                Comb::Rotate { left, .. } if deque => Comb::Rotate { left, count: "n" },
                comb => comb,
            };
            return Some(Std::Comb(comb));
        }
        Some(match tcx.item_name(def_id).as_str() {
            // A map's, a set's or an entry's (ADRs 0059, 0325, 0345).
            name if let Some(op) = map_method(name, of) => Std::Map(op),
            _ if arguments || argument => return format_args_method(name.as_str(), arguments),
            "new" if adt(StdItem::Rc) || adt(StdItem::Arc) => Std::Same,
            // What only a counted one answers (ADR 0320).
            "strong_count" if rc => Std::Rc(RcOp::StrongCount),
            "weak_count" if rc => Std::Rc(RcOp::WeakCount),
            "ptr_eq" if rc || weak => Std::Rc(RcOp::PtrEq),
            "downgrade" if rc => Std::Rc(RcOp::Downgrade),
            "get_mut" if rc => Std::Rc(RcOp::GetMut),
            "make_mut" if rc => Std::Rc(RcOp::MakeMut),
            "try_unwrap" if rc => Std::Rc(RcOp::TryUnwrap),
            "into_inner" if rc => Std::Rc(RcOp::IntoInner),
            "unwrap_or_clone" if rc => Std::Rc(RcOp::UnwrapOrClone),
            "new_cyclic" if rc => Std::Rc(RcOp::NewCyclic),
            "new" if weak => Std::Rc(RcOp::WeakNew),
            "upgrade" if weak => Std::Rc(RcOp::Upgrade),
            "strong_count" if weak => Std::Rc(RcOp::WeakStrongCount),
            "weak_count" if weak => Std::Rc(RcOp::WeakWeakCount),
            // Kept for good, a `&'static` of it: in JS, which frees nothing
            // itself, the value itself.
            "leak" if adt(StdItem::Vec) => Std::Same,
            "leak" if string || owner.is_box() => Std::Leak,
            // A `dyn Any`'s downcasts, by its type's key (ADR 0331).
            "is" if self.is_dyn_any(owner) => Std::Any(AnyOp::Is),
            "downcast_ref" if self.is_dyn_any(owner) => Std::Any(AnyOp::DowncastRef),
            "downcast_mut" if self.is_dyn_any(owner) => Std::Any(AnyOp::DowncastMut),
            "downcast"
                if (owner.is_box() || rc)
                    && owner
                        .walk()
                        .any(|part| part.as_type().is_some_and(|t| self.is_dyn_any(t))) =>
            {
                Std::Any(AnyOp::Downcast)
            }
            "of" if self.is_type_id(owner) => Std::Any(AnyOp::TypeIdOf),
            // A `Pin` is its pointer (ADR 0329).
            "pin" | "into_pin" if owner.is_box() => Std::Same,
            "pop" if self.peek_mut_of(owner).is_some() => Std::Heap(HeapOp::PeekPop),
            // What a `Box` or a `MaybeUninit` holds, or `undefined` before it's
            // written (ADR 0332).
            "new_uninit" if owner.is_box() => Std::Uninit(UninitOp::Uninit),
            "new_zeroed" if owner.is_box() => Std::Uninit(UninitOp::Zeroed),
            "new_uninit_slice" if owner.is_box() => Std::Uninit(UninitOp::Slice { zeroed: false }),
            "new_zeroed_slice" if owner.is_box() => Std::Uninit(UninitOp::Slice { zeroed: true }),
            "write" if owner.is_box() => Std::Uninit(UninitOp::BoxWrite),
            "assume_init" if owner.is_box() => Std::Same,
            "uninit" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::Uninit),
            "zeroed" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::Zeroed),
            "new" | "assume_init" | "assume_init_ref" | "assume_init_read" if self.uninit_of(owner).is_some() => {
                Std::Same
            }
            "write" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::Write),
            "assume_init_mut" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::InitMut),
            "new" | "new_unchecked" | "into_inner" | "into_inner_unchecked" | "get_ref" | "static_ref"
                if self.pinned(owner).is_some() =>
            {
                Std::Same
            }
            "as_mut" | "get_mut" | "get_unchecked_mut" | "static_mut" if self.pinned(owner).is_some() => {
                Std::Pin(PinOp::Mut)
            }
            "as_ref" | "into_ref" if self.pinned(owner).is_some() => Std::Pin(PinOp::Ref),
            "set" if self.pinned(owner).is_some() => Std::Pin(PinOp::Set),
            "map_unchecked" | "map_unchecked_mut" if self.pinned(owner).is_some() => Std::Pin(PinOp::Map),
            "new"
                if adt(StdItem::Cell)
                    || adt(StdItem::RefCell)
                    || adt(StdItem::Atomic)
                    || adt(StdItem::Mutex)
                    || adt(StdItem::RwLock) =>
            {
                Std::CellNew
            }
            // On one thread a lock is never contested by another: always `Ok`
            // (ADR 0025), but locked again while held, a deadlock (ADR 0328).
            "lock" if adt(StdItem::Mutex) => Std::Borrow {
                mutable: true,
                lock: true,
            },
            "read" | "write" if adt(StdItem::RwLock) => Std::Borrow {
                mutable: name.as_str() == "write",
                lock: true,
            },
            "into_inner" | "get_mut" if adt(StdItem::Mutex) || adt(StdItem::RwLock) => Std::Lock,
            "is_poisoned" if adt(StdItem::Mutex) || adt(StdItem::RwLock) => Std::NotPoisoned,
            "clear_poison" if adt(StdItem::Mutex) || adt(StdItem::RwLock) => Std::Nothing,
            "into_inner" | "get_ref" if is_poison_error(owner) => Std::PoisonData,
            "each_ref" | "each_mut" if owner.is_array() => Std::Same,
            "swap_remove_back" if deque => Std::Slice(SliceOp::DequeSwapRemove { front: false }),
            "swap_remove_front" if deque => Std::Slice(SliceOp::DequeSwapRemove { front: true }),
            "retain_mut" if adt(StdItem::Vec) || deque => Std::Slice(SliceOp::RetainMut),
            "pop_front_if" if deque => Std::Slice(SliceOp::PopFrontIf),
            "partition_point" if deque => Std::Slice(SliceOp::PartitionPoint),
            "range" if deque => Std::Text(TextOp::Slice),
            "range_mut" if deque => Std::Slice(SliceOp::DequeRangeMut),
            "escape_default" | "escape_debug" | "escape_unicode" if owner.is_char() || owner.is_str() => {
                Std::Text(TextOp::Escape {
                    kind: match name.as_str() {
                        "escape_default" => "default",
                        "escape_debug" => "debug",
                        _ => "unicode",
                    },
                    str: owner.is_str(),
                })
            }
            "encode_utf8" if owner.is_char() => Std::Text(TextOp::EncodeUtf8),
            "encode_utf16" if owner.is_char() => Std::Text(TextOp::CharEncodeUtf16),
            "decode_utf16" if owner.is_char() => Std::Text(TextOp::DecodeUtf16),
            "unpaired_surrogate" if is_decode_utf16_error(tcx, owner) => Std::Same,
            "make_ascii_uppercase" | "make_ascii_lowercase" if owner.is_char() || owner.is_str() => {
                Std::StringEdit(StringEdit::AsciiCase {
                    upper: name.as_str() == "make_ascii_uppercase",
                })
            }
            "borrow" | "borrow_mut" if adt(StdItem::RefCell) => Std::Borrow {
                mutable: name.as_str() == "borrow_mut",
                lock: false,
            },
            "try_lock" if adt(StdItem::Mutex) => Std::TryBorrow {
                mutable: true,
                lock: true,
            },
            "try_read" | "try_write" if adt(StdItem::RwLock) => Std::TryBorrow {
                mutable: name.as_str() == "try_write",
                lock: true,
            },
            "try_borrow" | "try_borrow_mut" if adt(StdItem::RefCell) => Std::TryBorrow {
                lock: false,
                mutable: name.as_str() == "try_borrow_mut",
            },
            "load" | "into_inner" if adt(StdItem::Atomic) => Std::AtomicLoad,
            "store" if adt(StdItem::Atomic) => Std::AtomicStore,
            "swap" if adt(StdItem::Atomic) => Std::AtomicSwap,
            "fetch_add" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::Add),
            "fetch_sub" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::Sub),
            "fetch_and" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::BitAnd),
            "fetch_or" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::BitOr),
            "fetch_xor" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::BitXor),
            "fetch_max" if adt(StdItem::Atomic) => Std::AtomicFetchMax(true),
            "fetch_min" if adt(StdItem::Atomic) => Std::AtomicFetchMax(false),
            "compare_exchange" | "compare_exchange_weak" if adt(StdItem::Atomic) => Std::AtomicCompareExchange,
            "into_owned" if cow => Std::Cow(CowOp::IntoOwned),
            "to_mut" if cow => Std::Cow(CowOp::ToMut),
            "new" if lazy => Std::Lazy(LazyOp::New),
            "force" if lazy => Std::Lazy(LazyOp::Force),
            "force_mut" if lazy => Std::Lazy(LazyOp::ForceMut),
            "get" if lazy => Std::Lazy(LazyOp::Get),
            "get_mut" if lazy => Std::Lazy(LazyOp::GetMut),
            "new" if adt(StdItem::Vec) => Std::VecNew,
            "push" if adt(StdItem::Vec) => Std::Push,
            "push_mut" if adt(StdItem::Vec) => Std::Slice(SliceOp::PushMut { at: false }),
            "insert_mut" if adt(StdItem::Vec) => Std::Slice(SliceOp::PushMut { at: true }),
            // JS's `pop()` gives `undefined` when empty: `None` (ADR 0030).
            "pop" if adt(StdItem::Vec) => Std::Method("pop"),
            "len" if adt(StdItem::Vec) || owner.is_slice() => Std::Len,
            // A string counts its UTF-8 bytes, as Rust's does (ADR 0138).
            "len" if owner.is_str() || string => Std::Text(TextOp::ByteLen),
            "char_indices" if owner.is_str() => Std::Text(TextOp::CharIndices),
            "clear" if adt(StdItem::Vec) => Std::Clear,
            "retain" if adt(StdItem::Vec) => Std::Retain,
            "iter" | "iter_mut" if owner.is_slice() => Std::Same,
            "new" if string => Std::StringNew,
            "with_capacity" if string => Std::StringWithCapacity,
            "pop" if string => Std::StringEdit(StringEdit::Pop),
            "remove" if string => Std::StringEdit(StringEdit::Remove),
            "truncate" if string => Std::StringEdit(StringEdit::Truncate),
            "insert" | "insert_str" if string => Std::StringEdit(StringEdit::Insert),
            "retain" if string => Std::StringEdit(StringEdit::Retain),
            "clear" if string => Std::StringEdit(StringEdit::Clear),
            "split_off" if string => Std::StringEdit(StringEdit::SplitOff),
            "drain" if string => Std::StringEdit(StringEdit::Drain),
            "replace_range" if string => Std::StringEdit(StringEdit::ReplaceRange),
            "extend_from_within" if string => Std::StringEdit(StringEdit::ExtendFromWithin),
            "trim_ascii" if owner.is_str() => Std::Text(TextOp::TrimAscii { start: true, end: true }),
            "trim_ascii_start" if owner.is_str() => Std::Text(TextOp::TrimAscii {
                start: true,
                end: false,
            }),
            "trim_ascii_end" if owner.is_str() => Std::Text(TextOp::TrimAscii {
                start: false,
                end: true,
            }),
            "split_at_checked" if owner.is_str() => Std::Text(TextOp::StrSplitAtChecked),
            "encode_utf16" if owner.is_str() => Std::Text(TextOp::EncodeUtf16),
            "floor_char_boundary" if owner.is_str() => Std::Text(TextOp::CharBoundaryNear { ceil: false }),
            "ceil_char_boundary" if owner.is_str() => Std::Text(TextOp::CharBoundaryNear { ceil: true }),
            "from_utf16" if string => Std::Text(TextOp::FromUtf16 { lossy: false }),
            "from_utf16_lossy" if string => Std::Text(TextOp::FromUtf16 { lossy: true }),
            "as_str" | "as_mut_str" if string => Std::Same,
            "trim" if owner.is_str() => Std::Trim { start: true, end: true },
            // A closure, a function or a set of `char`s as the pattern (ADRs 0063, 0157).
            name if owner.is_str()
                && self_ty.is_some_and(|p| self.is_char_predicate(p))
                && let Some(op) = methods::char_pattern(name) =>
            {
                Std::Text(op)
            }
            "split_ascii_whitespace" if owner.is_str() => Std::Text(TextOp::SplitAsciiWhitespace),
            "get" if owner.is_str() => Std::Text(TextOp::StrGet),
            "get_mut" if owner.is_str() => Std::Text(TextOp::StrGetMut),
            "get_unchecked" if owner.is_str() => Std::Text(TextOp::StrSlice),
            "get_unchecked_mut" if owner.is_str() => Std::Text(TextOp::StrPart { bounds: false }),
            "slice_unchecked" if owner.is_str() => Std::Text(TextOp::StrSliceBounds),
            "slice_mut_unchecked" if owner.is_str() => Std::Text(TextOp::StrPart { bounds: true }),
            "split_at_mut" | "split_at_mut_checked" if owner.is_str() => Std::Text(TextOp::StrSplitAtMut {
                checked: name.as_str() == "split_at_mut_checked",
            }),
            "get" if owner.is_slice() && args.types().nth(1).is_some_and(|r| self.range_kind(r).is_some()) => {
                Std::Text(TextOp::SliceGet)
            }
            "split_at" | "split_at_checked" if owner.is_slice() => Std::Text(TextOp::SliceSplitAt {
                checked: name.as_str() == "split_at_checked",
            }),
            "split_first" | "split_last" if owner.is_slice() => Std::Text(TextOp::SliceSplitFirst {
                last: name.as_str() == "split_last",
            }),
            "is_char_boundary" if owner.is_str() => Std::Text(TextOp::IsCharBoundary),
            "len_utf8" if owner.is_char() => Std::Text(TextOp::CharLen { utf16: false }),
            "len_utf16" if owner.is_char() => Std::Text(TextOp::CharLen { utf16: true }),
            // Methods taking a pattern: only a string or a `char` one.
            "starts_with" | "ends_with" | "contains" | "replace" | "replacen" | "split" | "strip_prefix"
            | "strip_suffix" | "split_once" | "rsplit_once" | "find" | "rfind"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p)) =>
            {
                return None;
            }
            // Splitting and searching by a `&str` or a `char` (ADR 0150); trimming by
            // a closure or a function too.
            "splitn" | "rsplitn" | "rsplit" | "split_terminator" | "match_indices" | "matches" | "split_inclusive"
            | "rsplit_terminator" | "rmatch_indices" | "rmatches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p)) =>
            {
                return None;
            }
            "splitn" if owner.is_str() => Std::Text(TextOp::SplitN),
            "rsplitn" if owner.is_str() => Std::Text(TextOp::Rsplit(true)),
            "rsplit" if owner.is_str() => Std::Text(TextOp::Rsplit(false)),
            "split_terminator" if owner.is_str() => Std::Text(TextOp::SplitTerminator),
            "split_inclusive" if owner.is_str() => Std::Text(TextOp::SplitInclusive),
            "rsplit_terminator" if owner.is_str() => Std::Text(TextOp::RsplitTerminator),
            "rmatch_indices" if owner.is_str() => Std::Text(TextOp::RmatchIndices),
            "rmatches" if owner.is_str() => Std::Text(TextOp::Rmatches),
            "split_at" if owner.is_str() => Std::Text(TextOp::SplitAt),
            "match_indices" if owner.is_str() => Std::Text(TextOp::MatchIndices),
            "matches" if owner.is_str() => Std::Text(TextOp::Matches),
            "trim_matches" | "trim_start_matches" | "trim_end_matches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p) || self.is_char_predicate(p)) =>
            {
                return None;
            }
            "trim_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches { start: true, end: true }),
            // `trim_left` and `trim_right`, deprecated, are `trim_start` and
            // `trim_end`, and theirs of a pattern (ADR 0322).
            "trim_left_matches" | "trim_right_matches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p) || self.is_char_predicate(p)) =>
            {
                return None;
            }
            "trim_left_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: true,
                end: false,
            }),
            "trim_right_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: false,
                end: true,
            }),
            "trim_left" if owner.is_str() => Std::Trim {
                start: true,
                end: false,
            },
            "trim_right" if owner.is_str() => Std::Trim {
                start: false,
                end: true,
            },
            "trim_start_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: true,
                end: false,
            }),
            "trim_end_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: false,
                end: true,
            }),
            "find" if owner.is_str() => Std::Text(TextOp::Find(false)),
            "rfind" if owner.is_str() => Std::Text(TextOp::Find(true)),
            "starts_with" if owner.is_str() => Std::Method("startsWith"),
            "ends_with" if owner.is_str() => Std::Method("endsWith"),
            "contains" if owner.is_str() => Std::Method("includes"),
            "replace" if owner.is_str() => Std::Method("replaceAll"),
            // `replacen(p, r, 1)`: JS's `replace`, of the first (ADR 0034).
            "replacen" if owner.is_str() => Std::Method("replace"),
            "split" if owner.is_str() => Std::Method("split"),
            "strip_prefix" if owner.is_str() => Std::StripPrefix,
            "strip_suffix" if owner.is_str() => Std::StripSuffix,
            "strip_circumfix" if owner.is_str() && args.types().all(|p| self.is_string_like(p)) => Std::StripCircumfix,
            "split_once" if owner.is_str() => Std::SplitOnce,
            "rsplit_once" if owner.is_str() => Std::RsplitOnce,
            "to_uppercase" if owner.is_str() => Std::Method("toUpperCase"),
            "to_lowercase" if owner.is_str() => Std::Method("toLowerCase"),
            "trim_start" if owner.is_str() => Std::Trim {
                start: true,
                end: false,
            },
            "trim_end" if owner.is_str() => Std::Trim {
                start: false,
                end: true,
            },
            "repeat" if owner.is_str() => Std::Method("repeat"),
            // Of text, JS's `join`; of lists, their items with the separator's
            // between (ADR 0322). `connect`, deprecated, is `join`.
            "join" | "connect"
                if owner.is_slice() && self_ty.is_some_and(|item| self.is_string_like(item.peel_refs())) =>
            {
                Std::Method("join")
            }
            "join" | "connect" if owner.is_slice() => Std::JoinItems,
            "push_str" | "push" if string => Std::PushStr,
            "is_empty" if adt(StdItem::Vec) || owner.is_slice() || owner.is_str() || string => Std::IsEmpty,
            "lock" if self.stream(owner).is_some() => Std::Stream(StreamOp::Nothing),
            "contains" if range.is_some() => Std::Range(RangeOp::Contains),
            "start" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::Bound("start")),
            "end" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::Bound("end")),
            "into_inner" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::IntoInner),
            "is_empty" if matches!(range, Some(RangeKind::Exclusive | RangeKind::Inclusive)) => {
                Std::Range(RangeOp::IsEmpty)
            }
            "as_ref" | "as_mut" if option => Std::Pointee,
            "get_or_insert" if option => Std::OptionPlace(OptionPlaceOp::GetOrInsert),
            "get_or_insert_with" if option => Std::OptionPlace(OptionPlaceOp::GetOrInsertWith),
            "get_or_insert_default" if option => Std::OptionPlace(OptionPlaceOp::GetOrInsertDefault),
            "insert" if option => Std::OptionPlace(OptionPlaceOp::Insert),
            "take_if" if option => Std::OptionPlace(OptionPlaceOp::TakeIf),
            // A `String`'s `&str` is the string, a `Vec`'s slice the array, a
            // box's or a reference's what it points at (ADRs 0211, 0326): of an
            // `Option` of one, it's the option; of a `Result`, the `Result`.
            // Another's `Deref` is refused.
            "as_deref" | "as_deref_mut"
                if (option || result)
                    && self_ty.is_some_and(|t| {
                        self.is_lang_adt(t, LangItem::String)
                            || self.is_std_type(t, StdItem::Vec)
                            || t.is_box()
                            || t.is_ref()
                    }) =>
            {
                if option {
                    Std::Pointee
                } else {
                    Std::Same
                }
            }
            "as_deref" if option || result => Std::Comb(Comb::AsDeref),
            // A reference is the value (ADR 0023): what's in the `Result` is.
            "as_ref" if result => Std::Same,
            "to_ascii_lowercase" if owner.is_str() => Std::AsciiCase { upper: false },
            "to_ascii_uppercase" if owner.is_str() => Std::AsciiCase { upper: true },
            "eq_ignore_ascii_case" if owner.is_str() || owner.is_char() => Std::AsciiEq,
            "append" if adt(StdItem::Vec) || adt(StdItem::VecDeque) || adt(StdItem::LinkedList) => Std::Append,
            "is_some" if option => Std::IsSome,
            "unwrap_unchecked" if option => Std::UnwrapUnchecked,
            "take" if option => Std::OptionTake,
            "replace" if option => Std::OptionReplace,
            "flatten" if option => Std::OptionFlatten,
            "iter" if option => Std::OptionIter,
            "iter_mut" if option => Std::OptionIterMut,
            "copied" | "cloned" if option => Std::OptionCloned,
            "is_none" if option => Std::IsNone,
            "unwrap_or" if option => Std::UnwrapOr,
            "map" if option => Std::OptionMap,
            // A thread-local (ADR 0037) is its `Cell` or `RefCell`: `{ value }`.
            "with" if local_key => Std::LocalWith,
            "get" if local_key => Std::CellGet,
            "set" if local_key => Std::CellSet,
            "take" if local_cell => Std::CellTake,
            "replace" if local_cell => Std::CellReplace,
            "with_borrow" | "with_borrow_mut" if local_key => Std::LocalBorrow {
                mutable: name.as_str() == "with_borrow_mut",
            },
            "then" if ordering => Std::Then,
            "then_with" if ordering => Std::ThenWith,
            "reverse" if ordering => Std::Reverse,
            "chars" if owner.is_str() => Std::Chars,
            "to_vec" if owner.is_slice() => Std::ToVec,
            "fill" if owner.is_slice() => Std::Slice(SliceOp::Fill),
            "fill_with" if owner.is_slice() => Std::Slice(SliceOp::FillWith),
            "copy_from_slice" if owner.is_slice() => Std::Slice(SliceOp::CopyFromSlice),
            "clone_from_slice" if owner.is_slice() => Std::Slice(SliceOp::CloneFromSlice),
            "swap_with_slice" if owner.is_slice() => Std::Slice(SliceOp::SwapWithSlice),
            "copy_within" if owner.is_slice() => Std::Slice(SliceOp::CopyWithin),
            "is_sorted" if owner.is_slice() => Std::Slice(SliceOp::IsSorted),
            "is_sorted_by" if owner.is_slice() => Std::Slice(SliceOp::IsSortedBy),
            "is_sorted_by_key" if owner.is_slice() => Std::Slice(SliceOp::IsSortedByKey),
            "partition_point" if owner.is_slice() => Std::Slice(SliceOp::PartitionPoint),
            "chunks_exact" | "chunks_exact_mut" if owner.is_slice() => Std::Slice(SliceOp::ChunksExact {
                mutable: name.as_str().ends_with("_mut"),
            }),
            "rchunks" | "rchunks_mut" | "rchunks_exact" | "rchunks_exact_mut" if owner.is_slice() => {
                Std::Slice(SliceOp::Rchunks {
                    exact: name.as_str().contains("exact"),
                    mutable: name.as_str().ends_with("_mut"),
                })
            }
            "chunks_mut" if owner.is_slice() => Std::Slice(SliceOp::ChunksMut),
            "remainder" | "into_remainder" if self.is_exact_chunks(owner) => Std::Slice(SliceOp::Remainder),
            "first_chunk" | "first_chunk_mut" | "last_chunk" | "last_chunk_mut" if owner.is_slice() => {
                Std::Slice(SliceOp::Chunk {
                    last: name.as_str().starts_with("last"),
                    mutable: name.as_str().ends_with("_mut"),
                })
            }
            "split_first_chunk" | "split_first_chunk_mut" | "split_last_chunk" | "split_last_chunk_mut"
                if owner.is_slice() =>
            {
                Std::Slice(SliceOp::SplitChunk {
                    last: name.as_str().starts_with("split_last"),
                    mutable: name.as_str().ends_with("_mut"),
                })
            }
            "split"
            | "splitn"
            | "rsplit"
            | "rsplitn"
            | "split_inclusive"
            | "split_mut"
            | "splitn_mut"
            | "rsplit_mut"
            | "rsplitn_mut"
            | "split_inclusive_mut"
                if owner.is_slice() =>
            {
                let base = name.as_str().trim_end_matches("_mut");
                Std::Slice(SliceOp::SplitBy {
                    limited: base.ends_with('n'),
                    inclusive: base == "split_inclusive",
                    back: base.starts_with('r'),
                    mutable: base != name.as_str(),
                })
            }
            "chunk_by" | "chunk_by_mut" if owner.is_slice() => Std::Slice(SliceOp::ChunkBy {
                mutable: name.as_str().ends_with("_mut"),
            }),
            "split_at_mut" | "split_at_mut_unchecked" | "split_at_mut_checked" if owner.is_slice() => {
                Std::Slice(SliceOp::SplitAtMut {
                    checked: name.as_str() == "split_at_mut_checked",
                })
            }
            "split_at_unchecked" if owner.is_slice() => Std::Text(TextOp::SliceSplitAt { checked: false }),
            "split_off"
            | "split_off_mut"
            | "split_off_first"
            | "split_off_first_mut"
            | "split_off_last"
            | "split_off_last_mut"
                if owner.is_slice() =>
            {
                let base = name.as_str().trim_end_matches("_mut");
                Std::Slice(SliceOp::SplitOff {
                    end: (base != "split_off").then_some(base == "split_off_last"),
                    mutable: base != name.as_str(),
                })
            }
            // A slice of `MaybeUninit`s is the array of what they hold (ADR 0332).
            "assume_init_ref" | "assume_init_mut" if owner.is_slice() => Std::Same,
            "write_copy_of_slice" | "write_clone_of_slice" if owner.is_slice() => Std::Slice(SliceOp::UninitWrite {
                clone: name.as_str() == "write_clone_of_slice",
            }),
            "assume_init_drop" if owner.is_slice() => Std::Slice(SliceOp::UninitDrop),
            // A byte slice's text: escaped, or its runs of UTF-8 (ADR 0341).
            "escape_ascii" if owner.is_slice() => Std::Text(TextOp::EscapeAscii),
            "utf8_chunks" if owner.is_slice() => Std::Text(TextOp::Utf8Chunks),
            "get_disjoint_mut" | "get_disjoint_unchecked_mut" if owner.is_slice() => {
                Std::Slice(SliceOp::GetDisjointMut {
                    unchecked: name.as_str() == "get_disjoint_unchecked_mut",
                })
            }
            // A slice as arrays of `N`, and arrays as one slice (ADR 0337).
            "as_array" | "as_mut_array" if owner.is_slice() => Std::Slice(SliceOp::AsArray),
            "array_windows" if owner.is_slice() => Std::Slice(SliceOp::ArrayWindows),
            "as_chunks"
            | "as_rchunks"
            | "as_chunks_mut"
            | "as_rchunks_mut"
            | "as_chunks_unchecked"
            | "as_chunks_unchecked_mut"
                if owner.is_slice() =>
            {
                Std::Slice(SliceOp::AsChunks {
                    back: name.as_str().starts_with("as_r"),
                    mutable: name.as_str().ends_with("_mut"),
                    unchecked: name.as_str().contains("unchecked"),
                })
            }
            "as_flattened" | "as_flattened_mut" if owner.is_slice() => Std::Slice(SliceOp::Flattened {
                mutable: name.as_str().ends_with("_mut"),
            }),
            "split_first_mut" | "split_last_mut" if owner.is_slice() => Std::Slice(SliceOp::SplitEndMut {
                last: name.as_str() == "split_last_mut",
            }),
            "get_mut" | "get_unchecked_mut"
                if owner.is_slice() && args.types().nth(1).is_some_and(|r| self.range_kind(r).is_some()) =>
            {
                Std::Slice(SliceOp::View {
                    checked: name.as_str() == "get_mut",
                })
            }
            "sort_by_cached_key" if owner.is_slice() => Std::Slice(SliceOp::SortByCachedKey),
            "repeat" if owner.is_slice() => Std::Slice(SliceOp::Repeat),
            "to_ascii_uppercase" | "to_ascii_lowercase" | "make_ascii_uppercase" | "make_ascii_lowercase"
                if owner.is_slice() =>
            {
                Std::Slice(SliceOp::AsciiCase {
                    upper: name.as_str().ends_with("uppercase"),
                    in_place: name.as_str().starts_with("make"),
                })
            }
            "is_ascii" if owner.is_slice() => Std::Slice(SliceOp::IsAscii),
            "trim_ascii" if owner.is_slice() => Std::Slice(SliceOp::TrimAscii { start: true, end: true }),
            "trim_ascii_start" if owner.is_slice() => Std::Slice(SliceOp::TrimAscii {
                start: true,
                end: false,
            }),
            "trim_ascii_end" if owner.is_slice() => Std::Slice(SliceOp::TrimAscii {
                start: false,
                end: true,
            }),
            // An unstable sort is JS's stable one only where items that compare
            // equal can't be told apart; else std's own (ADR 0342).
            "sort" if owner.is_slice() => Std::Sort,
            "sort_unstable" if owner.is_slice() && self_ty.is_some_and(|t| self.ties_unseen(t)) => Std::Sort,
            "sort_by" if owner.is_slice() => Std::SortBy,
            "sort_by_key" if owner.is_slice() => Std::SortByKey,
            n if owner.is_slice()
                && let Some(op) = unstable_sort(n) =>
            {
                Std::Slice(op)
            }
            "reverse" if owner.is_slice() => Std::Method("reverse"),
            // An array's `map` is JS's: a new array of what `f` makes of each.
            "map" if owner.is_array() => Std::Method("map"),
            // `v[0]` and `v.at(-1)` are `undefined` when `v` is empty: `None`.
            // A `_mut` one's item, of numbers or strings, is a handle on it
            // (ADR 0152).
            "first" | "first_mut" if owner.is_slice() => Std::First,
            "get" | "get_mut" if (owner.is_slice() && args.types().nth(1).is_some_and(|i| i.is_usize())) || deque => {
                Std::SliceGet
            }
            // Unchecked, the item, as JS reads one (ADR 0294).
            "get_unchecked" | "get_unchecked_mut"
                if owner.is_slice() && args.types().nth(1).is_some_and(|i| i.is_usize()) =>
            {
                Std::SliceGet
            }
            "last" | "last_mut" if owner.is_slice() => Std::SliceLast,
            // `includes` compares strings and numbers by value, as `==` does,
            // but objects by identity: only for those.
            // A `&mut` to one is a cell, an object (ADR 0099): not by identity.
            // Of items compared by their own `==` where it isn't JS's `===`
            // (ADR 0336).
            "starts_with" | "ends_with" if owner.is_slice() => Std::Text(TextOp::SliceStartsWith {
                end: name.as_str() == "ends_with",
            }),
            "strip_prefix" | "strip_suffix" | "strip_circumfix" if owner.is_slice() => Std::Slice(SliceOp::Strip {
                suffix: name.as_str() == "strip_suffix",
                circumfix: name.as_str() == "strip_circumfix",
            }),
            // Only `[u8]` has it.
            "eq_ignore_ascii_case" if owner.is_slice() => Std::Text(TextOp::BytesAsciiEq),
            "is_ok" if result => Std::IsOk(true),
            "is_err" if result => Std::IsOk(false),
            "ok" if result => Std::ResultOk,
            // An unchecked one panics where std's would be undefined behavior.
            "unwrap" | "expect" | "unwrap_unchecked" if result => Std::UnwrapOk,
            "unwrap_err" | "expect_err" | "unwrap_err_unchecked" if result => Std::UnwrapErr,
            "unwrap_or" if result => Std::ResultOr,
            _ => return None,
        })
    }
}
