//! Which `Std` a function, a trait method, an index or a conversion is,
//! and the method tables they share.

use super::conversions::try_conversion;
use super::*;

/// A `Vec`'s capacity's (ADR 0419).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum CapacityOp {
    /// `Vec::with_capacity(n)`.
    WithCapacity,
    /// `v.capacity()`.
    Of,
    /// `v.reserve(n)`, std's amortized growth, or `reserve_exact(n)`,
    /// `exact`, as many as it needs (#849).
    Reserve { exact: bool },
    /// `v.shrink_to(n)`, or `shrink_to_fit()`.
    ShrinkTo { fit: bool },
}

/// Which of a `Vec`'s capacity's functions `def_id` is, or a `VecDeque`'s
/// or a `BinaryHeap`'s.
pub(crate) fn capacity_op(tcx: TyCtxt<'_>, def_id: DefId) -> Option<CapacityOp> {
    let item = std_path_item(tcx, def_id);
    if matches!(
        item,
        Some(StdPath::VecWithCapacity | StdPath::VecDequeWithCapacity | StdPath::BinaryHeapWithCapacity)
    ) {
        return Some(CapacityOp::WithCapacity);
    }
    Some(match array_collection_method(tcx, def_id)?.as_str() {
        "capacity" => CapacityOp::Of,
        "reserve" => CapacityOp::Reserve { exact: false },
        "reserve_exact" => CapacityOp::Reserve { exact: true },
        "shrink_to_fit" => CapacityOp::ShrinkTo { fit: true },
        "shrink_to" => CapacityOp::ShrinkTo { fit: false },
        _ => return None,
    })
}

/// Whether `def_id`, a `Vec`'s, a `VecDeque`'s or a `BinaryHeap`'s, may shorten it, which
/// keeps its capacity (ADR 0419): the array it's given first, or its
/// second, `append`'s.
pub(crate) fn shortens_vec(tcx: TyCtxt<'_>, def_id: DefId) -> Option<usize> {
    match array_collection_method(tcx, def_id)?.as_str() {
        "append" => Some(1),
        "clear" | "truncate" | "set_len" | "pop" | "pop_if" | "pop_front" | "pop_back" | "pop_front_if"
        | "pop_back_if" | "remove" | "swap_remove" | "drain" | "retain" | "retain_mut" | "dedup" | "dedup_by"
        | "dedup_by_key" | "split_off" | "resize" | "resize_with" | "extract_if" | "splice" => Some(0),
        _ => None,
    }
}

/// Whether `def_id`, a `Vec`'s, a `VecDeque`'s or a `BinaryHeap`'s, may add to it, which
/// std does by reserving as many as it adds, `grow_amortized` (#1180): the
/// array it's given first.
pub(crate) fn grows_vec(tcx: TyCtxt<'_>, def_id: DefId) -> Option<usize> {
    match array_collection_method(tcx, def_id)?.as_str() {
        "push" | "push_back" | "push_front" | "insert" | "append" | "extend_from_slice" | "extend_from_within"
        | "resize" | "resize_with" => Some(0),
        _ => None,
    }
}

/// The method of a `Vec`, a `VecDeque` or a `BinaryHeap` that `def_id` is, by
/// its name: std's collections that are a JS array, whose capacity is the
/// array's (ADRs 0068, 0419).
fn array_collection_method(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Symbol> {
    match std_inherent_method(tcx, def_id) {
        Some((MethodOf::Type(StdPath::Vec | StdPath::CollectionsVecDeque | StdPath::CollectionsBinaryHeap), name)) => {
            Some(name)
        }
        _ => None,
    }
}

/// The algebra a set's operator `op` is: `&a - &b`'s `difference`.
fn set_operator(op: BinOp) -> Option<&'static str> {
    Some(match op {
        BinOp::BitOr => "union",
        BinOp::BitAnd => "intersection",
        BinOp::Sub => "difference",
        BinOp::BitXor => "symmetric",
        _ => return None,
    })
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// What std function `def_id` of `args` is, which rust-js lowers by
    /// its own model of it: every recogniser's answer goes through here.
    pub(in crate::lower) fn classify(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        unrecognized::left(self.tcx, def_id, self.classified(def_id, args))
    }

    fn classified(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        if let Some(decided) = self.classify_fn(def_id, args) {
            return decided;
        }
        if let Some(trait_) = self.tcx.trait_of_assoc(def_id) {
            return self.classify_trait_method(def_id, trait_, args);
        }
        self.classify_inherent(def_id, args)
    }

    /// A free function's, or one rust-js knows by its identity, as `mem::swap`
    /// and `Box::new`: `Some` once decided, of what it is or that it's none
    /// of std's, and `None` to look further, as a method.
    fn classify_fn(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Option<Std>> {
        let tcx = self.tcx;
        // Its std item and its diagnostic name, each asked once: each test
        // below is of them, some forty a call a crate classifies (#882).
        let item = std_path_item(tcx, def_id);
        let diagnostic_name = tcx.get_diagnostic_name(def_id);
        let diagnostic = |name: &str| diagnostic_name.is_some_and(|found| found.as_str() == name);
        let self_ty = args.types().next();
        if let Some((fact, of_val)) = layout_query(tcx, def_id) {
            return Some(Some(Std::Layout { fact, of_val }));
        }
        // `fmt::write(w, args)`, std's own `write_fmt` of a `dyn fmt::Write`:
        // what it writes given to the writer's `write_str` (ADR 0446, #1697).
        if super::std_items::is_fmt_write_fn(tcx, def_id) {
            return Some(Some(Std::UserWrite));
        }
        // A write to a `Formatter` held where it isn't its writer's, a `&mut`
        // to it kept in a struct: its text's, a `String`'s, through the
        // handle it is (ADR 0461).
        if self.formatter_write(def_id, args) {
            return Some(Some(Std::PushStr));
        }
        if let Some(operation) = self.formatter_fmt_to(def_id) {
            return Some(Some(Std::FmtTo(operation)));
        }
        if type_facts(tcx, def_id) == [TypeFact::NeedsDrop] {
            return Some(Some(Std::NeedsDrop));
        }
        // An enum's variant's discriminant (ADR 0393).
        // And as a derived `Hash` hashes it (ADR 0463).
        if matches!(
            item,
            Some(StdPath::MemDiscriminantFn | StdPath::IntrinsicsDiscriminantValueFn)
        ) {
            return Some(Some(Std::Discriminant));
        }
        if diagnostic("mem_drop") {
            return Some(Some(Std::Drop));
        }
        if diagnostic("mem_forget") {
            return Some(Some(Std::Forget));
        }
        // The one thread there is yields to none (ADR 0142).
        // And a spin loop's hint, which no other thread waits on (#1285).
        if item == Some(StdPath::ThreadYieldNowFn) || item == Some(StdPath::HintSpinLoopFn) {
            return Some(Some(Std::Nothing));
        }
        // Whether this thread unwinds, a drop in a panic's cleanup asks
        // (#1285).
        if item == Some(StdPath::ThreadPanickingFn) {
            return Some(Some(Std::Panicking));
        }
        // `hint::assert_unchecked(c)`, a promise that `c` holds, which a
        // program that's right keeps: nothing, its `c` made as it's given
        // (ADR 0399).
        if item == Some(StdPath::HintAssertUncheckedFn) {
            return Some(Some(Std::Nothing));
        }
        // A hint to the optimizer rustc's MIR inlines into std's bodies,
        // `unlikely(b)`'s `cold_path()`, read as off (ADR 0364): nothing, or
        // the condition it's given (ADR 0899).
        if tcx.intrinsic(def_id).is_some() {
            match tcx.item_name(def_id).as_str() {
                "cold_path" | "assume" => return Some(Some(Std::Nothing)),
                "likely" | "unlikely" => return Some(Some(Std::Same)),
                _ => {}
            }
        }
        // An arithmetic intrinsic a number's method is, which rustc's MIR
        // inlines into std's bodies: the method's operation, of its type,
        // `saturating_add(lo, n)` of `FlattenCompat`'s `size_hint` (ADR
        // 0899).
        // So is one called as itself, custom MIR's and `core_intrinsics`':
        // `bswap(x)`, `x.swap_bytes()`; `fmaf64(a, b, c)`, `a.mul_add(b, c)`,
        // rounded once (ADR 0430); and the bit counts and turns (#1384).
        // 1.100's `Ord::min` and `max` of an integer, which std's bodies
        // call, as `a.min(b)` is.
        if tcx.intrinsic(def_id).is_some()
            && let Some(max) = match tcx.item_name(def_id).as_str() {
                "integer_max" => Some(true),
                "integer_min" => Some(false),
                _ => None,
            }
        {
            return Some(Some(Std::MaxOf(max)));
        }
        if tcx.intrinsic(def_id).is_some()
            && let Some((method, num)) = number_intrinsic(tcx.item_name(def_id).as_str(), args)
            && let Some(op) = super::methods::number(method, num)
        {
            return Some(Some(Std::Number(op)));
        }
        // Plain data's bytes as another type's, and all zero (ADR 0408).
        match item {
            Some(StdPath::IntrinsicsTransmuteFn | StdPath::IntrinsicsTransmuteUncheckedFn) => {
                return Some(Some(Std::Transmute));
            }
            Some(StdPath::MemZeroedFn) => return Some(Some(Std::Uninit(UninitOp::Zeroed))),
            Some(StdPath::ArrayFromFnFn) => return Some(Some(Std::Slice(SliceOp::FromFn))),
            Some(StdPath::SliceFromRefFn | StdPath::ArrayFromRefFn) => return Some(Some(Std::Slice(SliceOp::FromRef))),
            Some(StdPath::SliceFromMutFn | StdPath::ArrayFromMutFn) => return Some(Some(Std::Slice(SliceOp::FromMut))),
            _ => {}
        }
        // A `NonNull` of a reference, `NonNull::from(&mut x)`: a pointer to
        // what it refers to (ADR 0407).
        if diagnostic("from_fn")
            && self_ty
                .is_some_and(|t| matches!(t.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::NonNull)))
            && args.types().nth(1).is_some_and(|t| t.is_ref())
        {
            return Some(Some(Std::Pointer(crate::lower::pointers::PointerOp::FromRef)));
        }
        // A raw pointer's, a handle on its place (ADR 0407).
        if let Some(op) = crate::lower::pointers::pointer_op(tcx, def_id) {
            return Some(Some(Std::Pointer(op)));
        }
        // A `Vec`'s capacity's, kept where it's asked (ADR 0419).
        if let Some(op) = capacity_op(tcx, def_id) {
            return Some(Some(Std::Capacity(op)));
        }
        // The allocator's, an array of slots (ADR 0417).
        if let Some(op) = crate::lower::std_types::alloc::alloc_op(item) {
            return Some(Some(Std::Alloc(op)));
        }
        // memchr's searches, of the crate rustc builds (ADR 0529).
        if let Some(op) = self.memchr_op(def_id) {
            return Some(Some(Std::Memchr(op)));
        }
        // The process a program runs in, and its files, Node's (ADR 0391).
        match node_only(tcx, def_id) {
            Some(NodeOnly::Process(op)) => return Some(Some(Std::Process(op))),
            Some(NodeOnly::Fs(op)) => return Some(Some(Std::Fs(op))),
            None => {}
        }
        // An `OpenOptions`'s, its flags, which open nothing till `open`
        // (#1156); and a `File`'s, a `Metadata`'s, a `FileType`'s and a
        // `DirEntry`'s (#1397).
        if let Some(op) = crate::lower::std_types::fs::fs_method_op(item) {
            return Some(Some(Std::Fs(op)));
        }
        // A C-variadic function's next argument, the first of what's left of
        // its array (ADR 0428).
        if item == Some(StdPath::VaListNextArg) {
            return Some(Some(Std::Method("shift")));
        }
        // Where a call is, `Location::caller()`, and its parts (ADR 0427).
        if let Some(op) = location_of(tcx, def_id) {
            return Some(Some(Std::Location(op)));
        }
        // A backtrace, disabled unless forced (ADR 0568).
        if let Some(op) = crate::lower::std_types::backtrace::backtrace_op(tcx, def_id) {
            return Some(Some(Std::Backtrace(op)));
        }
        // A C string's, its bytes with its nul (#1157).
        if let Some(known) = c_str_op(tcx, def_id) {
            return Some(Some(known));
        }
        // A future already done, a promise (ADR 0029).
        if let Some(op) = crate::lower::std_types::future::future_op(item) {
            return Some(Some(Std::Future(op)));
        }
        // `pin!`'s `Pin::new_unchecked(&mut { value })`, the pointer, as
        // `Pin::new_unchecked` is (ADR 0329).
        if is_pin_macro_fn(tcx, def_id) {
            return Some(Some(Std::Same));
        }
        if let Some(op) = self.io_error_op(def_id, args) {
            return Some(Some(Std::IoError(op)));
        }
        if let Some(op) = self.io_op(def_id, args) {
            return Some(Some(Std::Io(op)));
        }
        if let Some(op) = self.hash_op(def_id, args) {
            return Some(Some(Std::Hash(op)));
        }
        // Bytes to text, as Rust validates UTF-8 (ADR 0172).
        let utf8 = match item {
            Some(StdPath::StrFromUtf8Fn | StdPath::StrFromUtf8) => Some(TextOp::FromUtf8 { owned: false }),
            Some(StdPath::StringFromUtf8) => Some(TextOp::FromUtf8 { owned: true }),
            Some(
                StdPath::StrFromUtf8UncheckedFn | StdPath::StrFromUtf8Unchecked | StdPath::StringFromUtf8Unchecked,
            ) => Some(TextOp::Utf8Unchecked),
            Some(StdPath::StringFromUtf8Lossy) => Some(TextOp::Utf8Lossy),
            Some(StdPath::StrFromUtf8MutFn | StdPath::StrFromUtf8Mut) => Some(TextOp::FromUtf8Mut { unchecked: false }),
            Some(StdPath::StrFromUtf8UncheckedMutFn | StdPath::StrFromUtf8UncheckedMut) => {
                Some(TextOp::FromUtf8Mut { unchecked: true })
            }
            Some(StdPath::Utf8ErrorValidUpTo) => Some(TextOp::Utf8Part("validUpTo")),
            // A `Utf8Chunk`'s parts, `{ valid, invalid }` (ADR 0341).
            Some(StdPath::Utf8ChunkValid) => Some(TextOp::Utf8Part("valid")),
            Some(StdPath::Utf8ChunkInvalid) => Some(TextOp::Utf8Part("invalid")),
            Some(StdPath::ParseIntErrorKind) => Some(TextOp::ParseErrorKind),
            // A byte's escape, its bytes (ADR 0558).
            Some(StdPath::AsciiEscapeDefaultFn | StdPath::U8EscapeAscii) => Some(TextOp::EscapeByte),
            _ => None,
        };
        if let Some(op) = utf8 {
            return Some(Some(Std::Text(op)));
        }
        // A path is its text (ADR 0173): what reads it as text is itself.
        match item {
            Some(
                StdPath::PathDisplayMethod
                | StdPath::PathToStr
                | StdPath::PathAsOsStr
                | StdPath::OsStrToStr
                | StdPath::PathBufAsPath,
            ) => return Some(Some(Std::Same)),
            Some(StdPath::PathBufNew) => return Some(Some(Std::StringNew)),
            Some(StdPath::PathNew | StdPath::OsStrNew) => return Some(Some(Std::Path(PathOp::New))),
            Some(StdPath::OsStringIntoString) => return Some(Some(Std::Path(PathOp::IntoString))),
            // Its methods, over its components (ADR 0405).
            Some(StdPath::PathJoin) => return Some(Some(Std::Path(PathOp::Join))),
            Some(StdPath::PathParent) => return Some(Some(Std::Path(PathOp::Parent))),
            Some(StdPath::PathFileName) => return Some(Some(Std::Path(PathOp::FileName))),
            Some(StdPath::PathFileStem) => return Some(Some(Std::Path(PathOp::FileStem))),
            Some(StdPath::PathExtension) => return Some(Some(Std::Path(PathOp::Extension))),
            Some(StdPath::PathWithExtension) => return Some(Some(Std::Path(PathOp::WithExtension))),
            Some(StdPath::PathWithFileName) => return Some(Some(Std::Path(PathOp::WithFileName))),
            Some(StdPath::PathStartsWith) => return Some(Some(Std::Path(PathOp::StartsWith))),
            Some(StdPath::PathEndsWith) => return Some(Some(Std::Path(PathOp::EndsWith))),
            Some(StdPath::PathStripPrefix) => return Some(Some(Std::Path(PathOp::StripPrefix))),
            Some(StdPath::PathIsAbsolute | StdPath::PathHasRoot) => {
                return Some(Some(Std::Path(PathOp::IsAbsolute)));
            }
            Some(StdPath::PathIsRelative) => return Some(Some(Std::Path(PathOp::IsRelative))),
            Some(StdPath::PathIterMethod) => return Some(Some(Std::Path(PathOp::Iter))),
            Some(StdPath::PathAncestorsMethod) => return Some(Some(Std::Path(PathOp::Ancestors))),
            Some(StdPath::PathComponentsMethod) => return Some(Some(Std::Path(PathOp::Components))),
            Some(StdPath::ComponentAsOsStr) => return Some(Some(Std::Path(PathOp::ComponentText))),
            Some(StdPath::PathToPathBuf) => return Some(Some(Std::Same)),
            Some(StdPath::PathBufPush) => return Some(Some(Std::StringEdit(StringEdit::PathPush))),
            Some(StdPath::PathBufPop) => return Some(Some(Std::StringEdit(StringEdit::PathPop))),
            Some(StdPath::PathBufSetExtension) => return Some(Some(Std::StringEdit(StringEdit::SetExtension))),
            Some(StdPath::PathBufSetFileName) => return Some(Some(Std::StringEdit(StringEdit::SetFileName))),
            // A `ManuallyDrop` is what it holds (ADR 0406), never dropped.
            Some(StdPath::ManuallyDropNew | StdPath::ManuallyDropIntoInner) => {
                return Some(Some(Std::Same));
            }
            Some(StdPath::ManuallyDropDrop) => return Some(Some(Std::DropPointee)),
            // A `NonZero` is its number (ADR 0177).
            Some(StdPath::NonZeroGet | StdPath::NonZeroNewUnchecked) => return Some(Some(Std::Same)),
            Some(StdPath::NonZeroNew) => return Some(Some(Std::NonZeroNew)),
            _ => {}
        }
        let utf8 = match item {
            Some(StdPath::Utf8ErrorErrorLen) => Some(TextOp::Utf8Part("errorLen")),
            Some(StdPath::FromUtf8ErrorUtf8Error) => Some(TextOp::Utf8Part("error")),
            Some(StdPath::FromUtf8ErrorIntoBytes | StdPath::FromUtf8ErrorAsBytes) => Some(TextOp::Utf8Part("bytes")),
            _ => None,
        };
        if let Some(op) = utf8 {
            return Some(Some(Std::Text(op)));
        }
        if item == Some(StdPath::MpscChannelFn) {
            return Some(Some(Std::Channel(ChannelOp::New)));
        }
        if diagnostic("mem_swap") {
            return Some(Some(Std::Swap));
        }
        // `ptr::fn_addr_eq(f, g)`, what rustc's lint of `f == g` of two
        // function pointers asks for: the same function, `===`, as `==` of
        // them is (ADR 0395, #987).
        if diagnostic("ptr_eq") || item == Some(StdPath::PtrFnAddrEqFn) {
            return Some(Some(Std::PtrEq));
        }
        // `ptr::hash(p, state)`, its pointer's address, as `*const T`'s
        // `hash` writes it (ADRs 0562, 0864).
        if item == Some(StdPath::PtrHashFn) {
            return Some(Some(Std::Hash(HashOp::Address)));
        }
        if diagnostic("mem_replace") {
            return Some(Some(Std::Replace));
        }
        // `Clone::clone_from`, std's own: `*dest = source.clone()`. A crate's
        // impl of its own is its own.
        if tcx
            .trait_of_assoc(def_id)
            .is_some_and(|t| tcx.is_lang_item(t, LangItem::Clone))
            && tcx.item_name(def_id).as_str() == "clone_from"
        {
            return Some(Some(Std::CloneFrom));
        }
        if of_std_crate(tcx, def_id, StdCrate::Core) && item == Some(StdPath::MemTakeFn) {
            return Some(Some(Std::MemTake));
        }
        // `cmp::max(a, b)` of numbers is `a.max(b)`'s (ADR 0136); of anything
        // else, the call is lowered as `Ord::max`'s.
        if let Some(max) = cmp_extreme(tcx, def_id)
            && self_ty.and_then(Num::of).is_some_and(|num| !num.float())
        {
            return Some(Some(Std::MaxOf(max)));
        }
        if tcx.is_lang_item(def_id, LangItem::RangeInclusiveNew) {
            return Some(Some(Std::Range(RangeOp::New)));
        }
        if of_std_crate(tcx, def_id, StdCrate::Std) && matches!(item, Some(StdPath::IoStdoutFn | StdPath::IoStderrFn)) {
            return Some(Some(Std::Stream(StreamOp::Open)));
        }
        if of_std_crate(tcx, def_id, StdCrate::Core) {
            match item {
                Some(StdPath::AnyTypeNameFn) => return Some(Some(Std::TypeName { of_val: false })),
                Some(StdPath::AnyTypeNameOfValFn) => return Some(Some(Std::TypeName { of_val: true })),
                _ => {}
            }
        }
        // std's iterator sources (ADR 0128), by path: most have no
        // diagnostic item.
        if of_std_crate(tcx, def_id, StdCrate::Core) && tcx.def_kind(def_id) == DefKind::Fn {
            let source = match item {
                Some(StdPath::IterOnceFn) => Some(IterSource::Once),
                Some(StdPath::IterEmptyFn) => Some(IterSource::Empty),
                Some(StdPath::IterRepeatFn) => Some(IterSource::Repeat),
                Some(StdPath::IterRepeatNFn) => Some(IterSource::RepeatN),
                Some(StdPath::IterRepeatWithFn) => Some(IterSource::RepeatWith),
                Some(StdPath::IterOnceWithFn) => Some(IterSource::OnceWith),
                Some(StdPath::IterSuccessorsFn) => Some(IterSource::Successors),
                Some(StdPath::IterFromFnFn) => Some(IterSource::FromFn),
                Some(StdPath::IterZipFn) => Some(IterSource::Zip),
                _ => None,
            };
            if let Some(source) = source {
                return Some(Some(Std::IterSource(source)));
            }
        }
        // `hint::black_box(x)` is `x`: it only hides `x` from an optimizer.
        // libtest's `test::black_box` is the same.
        let krate = std_crate(tcx, def_id.krate);
        if tcx.item_name(def_id).as_str() == "black_box"
            && ((krate == Some(StdCrate::Core) && std_path_under(tcx, def_id, StdPath::HintMod))
                || is_libtest(tcx, def_id))
        {
            return Some(Some(Std::Same));
        }
        // `convert::identity(x)` is `x`, as its name says.
        if diagnostic("box_new") || diagnostic("convert_identity") {
            return Some(Some(Std::Same));
        }
        if diagnostic("box_assume_init_into_vec_unsafe") {
            return Some(Some(Std::VecMacro));
        }
        // `char::from_digit`, `std::char::from_digit` and `from_u32`.
        let char_fn = |name: &str| {
            of_std_crate(tcx, def_id, StdCrate::Core)
                && tcx.item_name(def_id).as_str() == name
                && std_path_under(tcx, def_id, StdPath::CharMod)
        };
        if char_fn("from_digit") {
            return Some(Some(Std::FromDigit));
        }
        if char_fn("from_u32") {
            return Some(Some(Std::FromU32));
        }
        // Of a code point its caller checked.
        if char_fn("from_u32_unchecked") {
            return Some(Some(Std::Text(TextOp::CharFromCode)));
        }
        // serde_json's free functions, by their names: not a method of the
        // same name, `Deserializer::from_str` (#1541).
        if tcx.crate_name(def_id.krate).as_str() == "serde_json" && tcx.def_kind(def_id) == rustc_hir::def::DefKind::Fn
        {
            match tcx.item_name(def_id).as_str() {
                "to_string" => return Some(Some(Std::ToJson(false))),
                "to_string_pretty" => return Some(Some(Std::ToJson(true))),
                "from_str" => return Some(Some(Std::FromJson)),
                _ => {}
            }
        }
        if is_vec_from_elem(tcx, def_id) {
            return Some(Some(Std::FromElem));
        }
        if diagnostic("to_string_method") {
            return Some(Some(Std::ToString));
        }
        if diagnostic("option_unwrap") || diagnostic("option_expect") {
            return Some(Some(Std::Unwrap));
        }
        // `format!(..)` is `must_use(format(format_args!(..)))`, and the
        // arguments are a string already (ADR 0034).
        let krate = std_crate(tcx, def_id.krate);
        let name = tcx.item_name(def_id);
        if krate == Some(StdCrate::Core)
            && let Some(imp) = tcx.inherent_impl_of_assoc(def_id)
            && Num::of(tcx.type_of(imp).instantiate_identity().skip_normalization()).is_some_and(Num::float)
        {
            match name.as_str() {
                "max" => return Some(Some(Std::MaxOf(true))),
                "min" => return Some(Some(Std::MaxOf(false))),
                _ => {}
            }
        }
        if krate == Some(StdCrate::Alloc) && name.as_str() == "format" && item == Some(StdPath::FmtFormatFn) {
            return Some(Some(Std::Format));
        }
        if krate == Some(StdCrate::Core) && name.as_str() == "must_use" {
            return Some(Some(Std::Same));
        }
        if tcx.is_lang_item(def_id, LangItem::Panic) {
            return Some(Some(Std::Panic));
        }
        // `panic!("{}", x)`, which std writes as `panic_display(&x)` (ADR 0189).
        if tcx.is_lang_item(def_id, LangItem::PanicDisplay) {
            return Some(Some(Std::PanicDisplay { unreachable: false }));
        }
        if krate == Some(StdCrate::Core)
            && name.as_str() == "unreachable_display"
            && std_path_under(tcx, def_id, StdPath::PanickingMod)
        {
            return Some(Some(Std::PanicDisplay { unreachable: true }));
        }
        if let Some(error) = print_of(tcx, def_id) {
            return Some(Some(Std::Print { error }));
        }
        if krate == Some(StdCrate::Std) && item == Some(StdPath::RtBeginPanicFn) {
            return Some(Some(Std::BeginPanic));
        }
        if let Some(op) = unwind_op(tcx, def_id) {
            return Some(Some(Std::Unwind(op)));
        }
        if tcx.is_lang_item(def_id, LangItem::PanicFmt) {
            return Some(Some(Std::PanicFmt));
        }
        if of_std_crate(tcx, def_id, StdCrate::Core) && tcx.item_name(def_id).as_str() == "assert_failed" {
            return Some(Some(Std::AssertFailed));
        }
        if diagnostic("deref_method") || diagnostic("deref_mut_method") {
            // A reference to what's inside is the same JS value (ADR 0024 for JS objects).
            let Some(ty) = self_ty else { return Some(None) };
            // What a `Cow` borrows or owns (ADRs 0172, 0319).
            if diagnostic("deref_method") && self.is_cow(ty) {
                return Some(Some(Std::Cow(CowOp::Deref)));
            }
            // A `LazyCell`'s value, made the first time (ADR 0318).
            if self.is_std_type(ty, StdItem::LazyCell) {
                let op = if diagnostic("deref_method") {
                    LazyOp::Force
                } else {
                    LazyOp::ForceMut
                };
                return Some(Some(Std::Lazy(op)));
            }
            // A `Box`'s, and a shared reference's, the value it is (ADR
            // 0023), which generic code's `deref` calls (ADR 0394).
            let boxed =
                diagnostic("deref_method") && (ty.is_box() || matches!(ty.kind(), ty::Ref(_, _, Mutability::Not)));
            let same = self.is_string_like(ty)
                || self.is_path_like(ty)
                // A `ManuallyDrop`'s, the value it is (ADR 0406).
                || self.is_lang_adt(ty, LangItem::ManuallyDrop)
                || self.is_js_object(ty)
                || self.is_rc(ty)
                || self.is_vec_like(ty)
                // A `CString`'s, its bytes with the nul, its `CStr`'s (#1157).
                || is_c_string(tcx, ty)
                || boxed;
            // A `PeekMut`'s heap's top (ADR 0333).
            if self.peek_mut_of(ty).is_some() {
                return Some(Some(Std::Heap(HeapOp::PeekTop {
                    mutable: diagnostic("deref_mut_method"),
                })));
            }
            // What a `Pin`'s pointer points at (ADR 0329).
            if self.pinned(ty).is_some() {
                return Some(Some(Std::Pin(match diagnostic("deref_mut_method") {
                    true => PinOp::Mut,
                    false => PinOp::Ref,
                })));
            }
            // What a guard guards, its cell's `value` (ADR 0328).
            if self.is_guard(ty) {
                return Some(Some(Std::GuardValue {
                    mutable: diagnostic("deref_mut_method"),
                }));
            }
            return Some(same.then_some(Std::Same));
        }
        None
    }

    /// `v[i]`, `&v[a..b]` or `&mut v[a..b]`, of a map, a slice, an array, a
    /// `Vec` or text: `Index::index` or `IndexMut::index_mut` of `ty`.
    fn classify_index(&self, trait_: DefId, ty: Ty<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let items =
            |ty: Ty<'tcx>| ty.peel_refs().is_slice() || ty.peel_refs().is_array() || self.is_vec_like(ty.peel_refs());
        // `m[k]` of a map: its value, or a panic, as `get(k).expect(..)`.
        if tcx.is_lang_item(trait_, LangItem::Index) && self.is_map(ty) && !self.is_set(ty) {
            return Some(Std::Map(MapOp::Index { held: false }));
        }
        // `&v[a..b]` of a slice, an array or a `Vec` (ADR 0063).
        if tcx.is_lang_item(trait_, LangItem::Index)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && items(ty)
        {
            return Some(Std::Text(TextOp::Slice));
        }
        // `&mut v[a..b]` of a slice, an array or a `Vec`: a view of those
        // items (ADR 0335).
        if tcx.is_lang_item(trait_, LangItem::IndexMut)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && items(ty)
        {
            return Some(Std::Slice(SliceOp::View { checked: false }));
        }
        // `&s[a..b]` of a string: by its UTF-8 bytes (ADR 0138).
        if tcx.is_lang_item(trait_, LangItem::Index)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && self.is_string_like(ty)
        {
            return Some(Std::Text(TextOp::StrSlice));
        }
        // `&mut s[a..b]` of a string: that part of it (ADR 0334).
        if tcx.is_lang_item(trait_, LangItem::IndexMut)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && self.is_string_like(ty)
        {
            return Some(Std::Text(TextOp::StrPart { bounds: false }));
        }
        // `v[i]` of a `Vec` is a slice's, checked the same way.
        if (tcx.is_lang_item(trait_, LangItem::Index) || tcx.is_lang_item(trait_, LangItem::IndexMut))
            && self.is_vec_like(ty.peel_refs())
            && args.types().nth(1).is_some_and(|i| i.is_usize())
        {
            return Some(Std::Index);
        }
        // Of a type parameter's index, `I: SliceIndex<[T]>`, smallvec's
        // `Index<I>`: its dictionary's (ADR 0418).
        let mutable = tcx.is_lang_item(trait_, LangItem::IndexMut);
        if (tcx.is_lang_item(trait_, LangItem::Index) || mutable)
            && args.types().nth(1).is_some_and(|index| self.is_unknown(index))
            && items(ty)
        {
            return Some(Std::Slice(SliceOp::GenericIndex { mutable }));
        }
        None
    }

    /// A trait's method: an operator's, a comparison's, an iterator's, a
    /// conversion's.
    fn classify_trait_method(&self, def_id: DefId, trait_: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let self_ty = args.types().next();
        let ty = self_ty?;
        // `x.type_id()`, its type's key (ADR 0331).
        if tcx.is_diagnostic_item(Symbol::intern("Any"), trait_) && tcx.item_name(def_id).as_str() == "type_id" {
            return Some(Std::Any(AnyOp::TypeIdOfValue));
        }
        // `x.borrow()` and `v.as_mut()`, a view of a value as itself.
        if self.views_itself(trait_, def_id, ty, args) {
            return Some(Std::Same);
        }
        // `b.as_ref()` of std's `AsRef`, as its dictionary's (`as_ref_of`).
        if is_std_def(tcx, trait_, StdItem::AsRef)
            && tcx.item_name(def_id).as_str() == "as_ref"
            && let Some(known) = self.as_ref_of(ty, args.type_at(1))
        {
            return Some(known);
        }
        // `std::num::Wrapping`'s and `Saturating`'s: its number's methods, as
        // std's impls write them (ADR 0175, #1434).
        if self.int_newtype(ty.peel_refs()).is_some() {
            if let Some(op) = value_operator(tcx, trait_) {
                return Some(Std::IntNewtypeOp(op, false));
            }
            if let Some(op) = assign_operator(tcx, trait_) {
                return Some(Std::IntNewtypeOp(Ok(op), true));
            }
        }
        if Num::of(ty.peel_refs()).is_some() || ty.peel_refs().is_bool() {
            let operators = [(LangItem::Neg, UnOp::Neg), (LangItem::Not, UnOp::Not)];
            if let Some(&(_, op)) = operators.iter().find(|(item, _)| tcx.is_lang_item(trait_, *item)) {
                return Some(Std::UnaryOperator(op));
            }
        }
        // A clock's `+` and `-`, its only operators (ADR 0447).
        if let Some(clock) = clock_type_of(ty.peel_refs()) {
            return self.clock_operator(trait_, args, clock);
        }
        // A `Duration`'s `+` and `-`, which panic past it as std's do; its
        // other operators aren't its number's (ADR 0188).
        if is_duration_ty(ty.peel_refs()) {
            if tcx.is_lang_item(trait_, LangItem::Add) {
                return Some(Std::Duration(DurationOp::Add));
            }
            if tcx.is_lang_item(trait_, LangItem::Sub) {
                return Some(Std::Duration(DurationOp::Sub));
            }
            if value_operator(tcx, trait_).is_some() || assign_operator(tcx, trait_).is_some() {
                return None;
            }
        }
        // `&a - &b` of sets, `|`, `&` and `^`: a set of their algebra's items.
        if let Some(Ok(op)) = value_operator(tcx, trait_)
            && self.is_set(ty.peel_refs())
        {
            return set_operator(op).map(|name| Std::Map(MapOp::SetOperator(name)));
        }
        if Num::of(ty.peel_refs()).is_some() || ty.peel_refs().is_bool() {
            if let Some(Ok(op)) = value_operator(tcx, trait_) {
                return Some(Std::Operator(op));
            }
            // `total += x` with a `&u32` `x`, `a ^ b` of `&bool`s: the same as of values.
            if let Some(op) = assign_operator(tcx, trait_) {
                return Some(Std::AssignOperator(op));
            }
            if tcx.is_lang_item(trait_, LangItem::PartialOrd) {
                return Some(Std::Operator(match tcx.item_name(def_id).as_str() {
                    "lt" => BinOp::Lt,
                    "le" => BinOp::Le,
                    "gt" => BinOp::Gt,
                    "ge" => BinOp::Ge,
                    _ => return None,
                }));
            }
        }
        if let Some(index) = self.classify_index(trait_, ty, args) {
            return Some(index);
        }
        if tcx.is_lang_item(trait_, LangItem::Add) {
            return self.is_lang_adt(ty, LangItem::String).then_some(Std::Concat);
        }
        // `s += t` is `s.push_str(t)`.
        if tcx.is_lang_item(trait_, LangItem::AddAssign) && self.is_lang_adt(ty.peel_refs(), LangItem::String) {
            return Some(Std::PushStr);
        }
        // A map's or a set's `into_iter()`: its entries, as an array (ADR 0059).
        // A `for` over one takes the `Map` itself.
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_) && self.is_map(ty) {
            return Some(Std::Map(MapOp::Iter(Part::Entries)));
        }
        // An iterator is a JS array (ADR 0036), and a `split` one of strings
        // (ADR 0034). Its adapters are the array's methods.
        // `size_hint()`, where it's known (`classify_size_hint`).
        if tcx.is_diagnostic_item(sym::Iterator, trait_)
            && tcx.item_name(def_id).as_str() == "size_hint"
            && let Some(known) = self.classify_size_hint(ty)
        {
            return Some(known);
        }
        if tcx.is_diagnostic_item(sym::Iterator, trait_) {
            let collects_string = || {
                args.types()
                    .nth(1)
                    .is_some_and(|b| self.is_lang_adt(b, LangItem::String))
            };
            return Some(match tcx.item_name(def_id).as_str() {
                // A `Range`'s moves its `start` (ADR 0129).
                "next" if matches!(self.range_kind(ty.peel_refs()), Some(RangeKind::Exclusive)) => {
                    Std::Range(RangeOp::Next)
                }
                // One of the crate's own is its impl's `next` (ADR 0055).
                "next" if !self.is_user_iterator(ty) => Std::Step(StepOp::Next),
                // The iterator itself, which knows where it is (ADR 0071).
                "by_ref" if !self.is_user_iterator(ty) => Std::IterByRef,
                name if let Some(step) = iterator_step(name) => Std::Step(step),
                "map" => Std::ArrayMethod("map"),
                "filter" => Std::ArrayMethod("filter"),
                "any" => Std::ArrayMethod("some"),
                "all" => Std::ArrayMethod("every"),
                "find" => Std::ArrayMethod("find"),
                "for_each" => Std::ArrayMethod("forEach"),
                "enumerate" => Std::Enumerate,
                "rev" => Std::Rev,
                "skip" => Std::Skip,
                "take" => Std::Take,
                "fold" => Std::Fold,
                "sum" => Std::Sum,
                "position" => Std::Position,
                "max" => Std::Extreme(true),
                "min" => Std::Extreme(false),
                "last" => Std::Last,
                "count" => Std::Len,
                name if let Some(comb) = methods::iterator(name) => Std::IterComb(comb),
                "copied" | "cloned" => Std::Cloned,
                "collect" if collects_string() => Std::CollectString,
                "collect" if args.types().nth(1).is_some_and(|b| self.is_map(b)) => {
                    let set = args.types().nth(1).is_some_and(|b| self.is_set(b));
                    Std::Map(MapOp::From { set })
                }
                "collect" => Std::Collect,
                _ => return None,
            });
        }
        // An iterator's `len()` is its `count()`, without taking its items.
        if is_std_path(tcx, trait_, StdPath::IterExactSizeIterator)
            && tcx.item_name(def_id).as_str() == "len"
            && self.is_user_iterator(ty)
        {
            return Some(Std::ExactLen);
        }
        // Of what only a caller knows too, `I` or `H::Item`: an array or a JS
        // iterator (ADR 0061), never a collection of the crate's (ADR 0160).
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && (ty.peel_refs().is_array() || ty.peel_refs().is_slice() || self.is_vec_like(ty.peel_refs()))
        {
            return Some(Std::Same);
        }
        // 1.99's of a `Box<[T; N]>` or a `Box<[T]>`, which is its array:
        // through a `&mut`, a handle on the box (ADRs 0099, 0388).
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && self.is_boxed_items(ty.peel_refs())
        {
            return Some(Std::Pointee);
        }
        if let Some(op) = self.range_iterator_method(def_id, trait_, ty) {
            return Some(op);
        }
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && let Some(op) = self.variants_stepped(ty)
        {
            return Some(op);
        }
        // `cmp`, `max` and `min` of what JS's `<` orders the same way, never through a `&mut` (#2188).
        if tcx.is_diagnostic_item(sym::Ord, trait_) {
            let peeled = crate::lower::mut_refs::past_shared_refs(ty)?;
            let comparable = Num::of(peeled).is_some() || peeled.is_bool() || self.is_string_like(peeled);
            return match tcx.item_name(def_id).as_str() {
                "cmp" if comparable => Some(Std::Cmp),
                "max" if Num::of(peeled).is_some() => Some(Std::MaxOf(true)),
                "min" if Num::of(peeled).is_some() => Some(Std::MaxOf(false)),
                "clamp" if Num::of(peeled).is_some() => Some(Std::Number(NumOp::Clamp)),
                _ => None,
            };
        }
        if let Some(write) = self.classify_fmt_write(def_id, trait_, ty) {
            return write;
        }
        // Writing to a standard stream or a `Vec<u8>` (ADRs 0132, 0392).
        if let Some(write) = self.classify_io_write(def_id, trait_, ty) {
            return Some(write);
        }
        if is_extend(tcx, trait_)
            && let Some(extend) = self.std_extend(ty.peel_refs())
        {
            return Some(extend);
        }
        // `String::from_iter(items)` is `items.collect()` into a `String`.
        if tcx.is_diagnostic_item(sym::FromIterator, trait_) && self.is_lang_adt(ty, LangItem::String) {
            return Some(Std::CollectString);
        }
        // A `From` or an `Into`: std's own conversions.
        if let Some(conversion) = self.classify_conversion(trait_, args) {
            return Some(conversion);
        }
        let into = tcx.is_diagnostic_item(sym::TryInto, trait_);
        let sides =
            conversion_sides(tcx, trait_, args).filter(|_| into || tcx.is_diagnostic_item(sym::TryFrom, trait_));
        let (from_ty, to_ty) = (sides.map(|(from, _)| from), sides.map(|(_, to)| to));
        if let Some(tried) = sides.and_then(|(from, to)| try_conversion(from.peel_refs(), to, into)) {
            return Some(tried);
        }
        // A slice into an array of its length, shared or copied (ADR 0189);
        // a `Vec`, moved into one (ADR 0557).
        if let (Some(from), Some(to)) = (from_ty, to_ty)
            && let Some(owned) = self.sequence_to_array(from, to)
        {
            return Some(Std::SliceToArray { into, owned });
        }
        tcx.is_diagnostic_item(Symbol::intern("ToOwned"), trait_)
            .then(|| to_owned_of(ty))
    }

    /// A range's own iterator method: a range is an iterator already, and
    /// its `len` and `next_back` are its bounds' (ADR 0129).
    fn range_iterator_method(&self, def_id: DefId, trait_: DefId, ty: Ty<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let kind = self.range_kind(ty)?;
        let stepped = matches!(kind, RangeKind::Exclusive | RangeKind::Inclusive);
        Some(Std::Range(match tcx.item_name(def_id).as_str() {
            "into_iter" if tcx.is_diagnostic_item(sym::IntoIterator, trait_) => return Some(Std::Same),
            "len" if stepped && is_std_path(tcx, trait_, StdPath::IterExactSizeIterator) => RangeOp::Len,
            "next_back"
                if kind == RangeKind::Exclusive
                    && tcx.get_diagnostic_item(Symbol::intern("DoubleEndedIterator")) == Some(trait_) =>
            {
                RangeOp::NextBack
            }
            _ => return None,
        }))
    }

    /// `into_iter` of an `Option` or a `Result`, or of a reference to one:
    /// its `iter()`, an array of its value or none, or, of a `&mut` one,
    /// its `iter_mut()`, whose items are places.
    fn variants_stepped(&self, ty: Ty<'tcx>) -> Option<Std> {
        let by_mut = matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut));
        if self.is_lang_adt(ty.peel_refs(), LangItem::Option) {
            return Some(if by_mut { Std::OptionIterMut } else { Std::OptionIter });
        }
        if self.is_std_type(ty.peel_refs(), StdItem::Result) {
            return Some(Std::Comb(if by_mut { Comb::ResultIterMut } else { Comb::ResultIter }));
        }
        None
    }

    /// Whether `from` converts into `to` as an array of its length: a slice,
    /// `Some(false)`, into an array or a reference to one (ADR 0189); a
    /// `Vec`, `Some(true)`, moved into an array (ADR 0557).
    fn sequence_to_array(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> Option<bool> {
        // A `Box<[T]>` into a `Box<[T; N]>`, the array a box is in JS,
        // moved as a `Vec` is (#1284).
        let boxed = from.boxed_ty().is_some_and(|slice| slice.is_slice());
        let owned = boxed || matches!(from.kind(), ty::Adt(adt, _) if self.tcx.is_diagnostic_item(sym::Vec, adt.did()));
        let slice = matches!(from.kind(), ty::Ref(_, slice, Mutability::Not) if slice.is_slice());
        let array = match to.kind() {
            ty::Ref(_, array, Mutability::Not) => slice && array.is_array(),
            _ if boxed => to.boxed_ty().is_some_and(|array| array.is_array()),
            _ => to.is_array(),
        };
        ((slice || owned) && array).then_some(owned)
    }

    /// `size_hint()` of an iterator of the crate's that keeps std's, of std's
    /// that knows its length (ADR 0170), or of a range, its bounds' (ADR
    /// 0129).
    fn classify_size_hint(&self, ty: Ty<'tcx>) -> Option<Std> {
        if self.is_user_iterator(ty) {
            return Some(Std::SizeHint(false));
        }
        match self.range_kind(ty.peel_refs()) {
            Some(RangeKind::Exclusive | RangeKind::Inclusive) => Some(Std::Range(RangeOp::SizeHint)),
            None if self.is_std_iterator(ty.peel_refs()) => Some(Std::SizeHint(true)),
            _ => None,
        }
    }

    /// A write, `io::Write`'s, to a standard stream, `print!`'s (ADR 0132),
    /// or to a `Vec<u8>`, its bytes added, as `extend_from_slice` adds
    /// them (ADR 0392), where the writer's type is known.
    fn classify_io_write(&self, def_id: DefId, trait_: DefId, ty: Ty<'tcx>) -> Option<Std> {
        if !is_std_path(self.tcx, trait_, StdPath::IoWrite) {
            return None;
        }
        // A writer of bytes of the crate's own that keeps std's `write_all`
        // and `write_fmt`: its dictionary's (ADR 0392).
        if io_write_default(self.tcx, def_id).is_some()
            && matches!(ty.peel_refs().kind(), ty::Adt(..))
            && self.has_user_impl(trait_, ty.peel_refs())
        {
            return Some(Std::UserIoWrite);
        }
        let name = self.tcx.item_name(def_id);
        if let Some(error) = self.stream(ty.peel_refs()) {
            return match name.as_str() {
                "write_fmt" => Some(Std::Stream(StreamOp::Write { error })),
                "write_all" => Some(Std::Stream(StreamOp::WriteBytes { error })),
                "write" => Some(Std::Stream(StreamOp::Written { error })),
                "flush" => Some(Std::Stream(StreamOp::Nothing)),
                _ => None,
            };
        }
        // A `&mut [u8]`'s, `$sliceWriter`'s, its slice written and re-pointed
        // past what's written, as std's is (#1282): `[u8]` behind its
        // references, as only `&mut [u8]` writes to one.
        if super::fmt::is_byte_slice(ty.peel_refs()) {
            return matches!(name.as_str(), "write" | "write_all" | "write_fmt" | "flush")
                .then_some(Std::Io(IoOp::Dictionary));
        }
        if self.io_writer(ty) != Some("$vecWriter") {
            return None;
        }
        match name.as_str() {
            "write_fmt" => Some(Std::WriteText),
            "write_all" => Some(Std::Comb(Comb::ExtendFromSlice)),
            "flush" => Some(Std::Nothing),
            _ => None,
        }
    }

    /// Which of `io::Error`'s functions `def_id` is: its `new`, `other` and
    /// `kind`, and `From<ErrorKind>`'s, or `Into`'s of one.
    fn io_error_op(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<IoErrorOp> {
        let tcx = self.tcx;
        if let Some(imp) = tcx.inherent_impl_of_assoc(def_id)
            && self.is_io_error(tcx.type_of(imp).instantiate_identity().skip_normalization())
        {
            return match tcx.item_name(def_id).as_str() {
                "new" => Some(IoErrorOp::New),
                "other" => Some(IoErrorOp::Other),
                "kind" => Some(IoErrorOp::Kind),
                "is_interrupted" => Some(IoErrorOp::IsInterrupted),
                "raw_os_error" => Some(IoErrorOp::RawOsError),
                "from_raw_os_error" => Some(IoErrorOp::FromRawOsError),
                _ => None,
            };
        }
        let (to, from) = super::conversion(tcx, def_id, args)?;
        (self.is_io_error(to) && self.is_io_error_kind(from)).then_some(IoErrorOp::FromKind)
    }

    /// Which of hashing's functions `def_id` is (ADR 0463): a std hasher's
    /// `new` and `new_with_keys`, its own `Hasher` methods, and a std type's
    /// `Hash`.
    fn hash_op(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<HashOp> {
        let tcx = self.tcx;
        match std_path_item(tcx, def_id) {
            Some(StdPath::DefaultHasherNew) => return Some(HashOp::New(StdHasher::Default)),
            Some(StdPath::SipHasherNew) => return Some(HashOp::New(StdHasher::Sip)),
            Some(StdPath::SipHasherNewWithKeys) => return Some(HashOp::NewWithKeys),
            Some(StdPath::RandomStateNew) => return Some(HashOp::RandomState),
            _ => {}
        }
        let tr = tcx.trait_of_assoc(def_id)?;
        let self_ty = args.types().next()?;
        // `BuildHasherDefault<H>`'s, of std's `BuildHasher` (#1155), and
        // `RandomState`'s (#1420).
        if is_std_path(tcx, tr, StdPath::HashBuildHasher)
            && matches!(self_ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::BuildHasherDefault)
                || is_std_def(tcx, adt.did(), StdItem::RandomState))
        {
            return match tcx.item_name(def_id).as_str() {
                "build_hasher" => Some(HashOp::BuildHasher),
                "hash_one" => Some(HashOp::HashOne),
                _ => None,
            };
        }
        // A std hasher's own (`HashOp::Own`). Of a `&mut` to one, std's
        // `impl Hasher for &mut H`, which forwards to what it points at,
        // `h.finish()` of an `h: &mut DefaultHasher` found by auto-ref, is
        // passed through to the hasher's own before it's asked here
        // (`passed_through`, #1709, #1724), as #1488's was read through
        // each `&mut` here, now the same JS without it (ADR 0463, amended).
        if is_hasher(tcx, tr) && self.std_hasher(self_ty).is_some() {
            return Some(HashOp::Own);
        }
        // A crate's hasher's provided method that its impl doesn't write,
        // `count.write_i32(n)`: its dictionary's, std's over its own.
        if is_hasher(tcx, tr) && self.left_to_std(def_id, self_ty) {
            return Some(HashOp::Provided);
        }
        if !is_std_def(tcx, tr, StdItem::Hash) {
            return None;
        }
        let std = self.hashed_as(self_ty).is_some();
        match tcx.item_name(def_id).as_str() {
            "hash" if std => Some(HashOp::Hash),
            // Of a crate's type too, whose impl leaves it to std: its
            // dictionary's, `$hashSlice`'s.
            "hash_slice" if std || self.left_to_std(def_id, self_ty) => Some(HashOp::HashSlice),
            _ => None,
        }
    }

    /// Whether `self_ty`'s impl of the trait of provided method `def_id`
    /// leaves it to std's: a crate's type's whose impl doesn't write it.
    fn left_to_std(&self, def_id: DefId, self_ty: Ty<'tcx>) -> bool {
        let tcx = self.tcx;
        let Some(tr) = tcx.trait_of_assoc(def_id) else {
            return false;
        };
        matches!(self_ty.kind(), ty::Adt(..))
            && matches!(
                crate::lower::traits::select(
                    tcx,
                    self.typing_env,
                    tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, tr, [self_ty])),
                ),
                Ok(ImplSource::UserDefined(imp))
                    if !tcx.impl_item_implementor_ids(imp.impl_def_id).contains_key(&def_id)
            )
    }

    /// A clock's operator, of its trait `trait_` and its arguments (ADR
    /// 0447): `t + d` and `t - d` of a `Duration`, and an `Instant`'s
    /// `a - b` of another. `t += d` is std's `*t = *t + d`, as a
    /// `Duration`'s is (ADR 0188); its others aren't its number's.
    fn clock_operator(&self, trait_: DefId, args: ty::GenericArgsRef<'tcx>, clock: ClockType) -> Option<Std> {
        let tcx = self.tcx;
        let instant = clock == ClockType::Instant;
        let rhs = args.types().nth(1)?;
        let shift = |sub: bool| {
            Some(Std::Clock {
                op: ClockOp::Shift { sub },
                instant,
            })
        };
        match (is_duration_ty(rhs), clock_type_of(rhs)) {
            (true, _) if tcx.is_lang_item(trait_, LangItem::Add) => shift(false),
            (true, _) if tcx.is_lang_item(trait_, LangItem::Sub) => shift(true),
            (false, Some(ClockType::Instant)) if instant && tcx.is_lang_item(trait_, LangItem::Sub) => {
                Some(Std::Clock {
                    op: ClockOp::Since { checked: false },
                    instant,
                })
            }
            _ => None,
        }
    }
}

/// A clock's method `name` (ADR 0447), of its nanoseconds.
pub(super) fn clock_method(clock: ClockType, name: &str) -> Option<Std> {
    let op = match (clock, name) {
        (ClockType::Error, "duration") => ClockOp::ErrorDuration,
        (ClockType::Error, _) => return None,
        (_, "now") => ClockOp::Now,
        (_, "duration_since" | "saturating_duration_since") => ClockOp::Since { checked: false },
        (_, "checked_duration_since") => ClockOp::Since { checked: true },
        (_, "elapsed") => ClockOp::Elapsed,
        (_, "checked_add") => ClockOp::Shift { sub: false },
        (_, "checked_sub") => ClockOp::Shift { sub: true },
        _ => return None,
    };
    Some(Std::Clock {
        op,
        instant: clock == ClockType::Instant,
    })
}

/// A slice's unstable sort, or its `select_nth_unstable`, by its method's
/// name, of std's algorithm (ADR 0342).
pub(super) fn unstable_sort(name: &str) -> Option<SliceOp> {
    let with = match name {
        "sort_unstable" | "select_nth_unstable" => SortWith::Order,
        "sort_unstable_by" | "select_nth_unstable_by" => SortWith::By,
        "sort_unstable_by_key" | "select_nth_unstable_by_key" => SortWith::Key,
        _ => return None,
    };
    Some(match name.starts_with("select") {
        true => SliceOp::SelectNth { with },
        false => SliceOp::SortUnstable { with },
    })
}

/// What `format_args!` makes, by its method: an `Arguments`' (`arguments`),
/// or an `Argument`'s. `as_str()` is `Some` of a literal's text and `None`
/// of one made with arguments, as native Rust's are (ADR 0034, amended;
/// #1091).
pub(super) fn format_args_method(name: &str, arguments: bool) -> Option<Std> {
    Some(match (name, arguments) {
        ("from_str" | "from_str_nonconst", true) => Std::FmtStr,
        ("new", true) => Std::FmtNew,
        ("as_str", true) => Std::FmtAsStr,
        ("new_display", false) => Std::FmtDisplay,
        ("new_debug", false) => Std::FmtDebug,
        ("new_lower_hex", false) => Std::FmtRadix(Radix::LowerHex),
        ("new_upper_hex", false) => Std::FmtRadix(Radix::UpperHex),
        ("new_binary", false) => Std::FmtRadix(Radix::Binary),
        ("new_octal", false) => Std::FmtRadix(Radix::Octal),
        ("new_lower_exp", false) => Std::FmtExp(false),
        ("new_upper_exp", false) => Std::FmtExp(true),
        ("new_pointer", false) => Std::FmtPointer,
        ("from_usize", false) => Std::FmtUsize,
        _ => return None,
    })
}

/// A `Cell`'s or a `RefCell`'s method that reads, writes or lends what it
/// holds, of its name (ADRs 0025, 0074, 0153): `None` of another, `new` and
/// a `RefCell`'s borrows, which are their kinds'. Or an `UnsafeCell`'s,
/// `{ value }` as a `Cell` is: its `new`, its `get()`, the cell itself, a
/// pointer's handle on its value, its `get_mut()` and `into_inner()` (#1420).
pub(super) fn cell_method(name: &str, refcell: bool, unsafe_cell: bool) -> Option<Std> {
    Some(match name {
        "new" if unsafe_cell => Std::CellNew,
        "get" if unsafe_cell => Std::CellPointer,
        "get_mut" | "into_inner" if unsafe_cell => return cell_method(name, false, false),
        _ if unsafe_cell => return None,
        "get" | "set" | "update" | "from_mut" | "as_slice_of_cells" | "as_array_of_cells" if refcell => return None,
        "get" | "into_inner" => Std::CellGet,
        "set" => Std::CellSet,
        "update" => Std::CellUpdate,
        // A `&mut` as a `&Cell`, and a cell of each item.
        "from_mut" => Std::CellFromMut,
        "as_slice_of_cells" | "as_array_of_cells" => Std::CellItems,
        "get_mut" => Std::CellGetMut,
        "swap" => Std::CellSwap,
        "replace" => Std::CellReplace,
        "take" => Std::CellTake,
        "replace_with" if refcell => Std::CellReplaceWith,
        _ => return None,
    })
}

/// Which `StepOp` a `Peekable`'s own method is, each a rewrite over std's
/// struct, stepped as the JS iterator it is (ADRs 0513, 0561).
pub(super) fn peekable_step(name: &str) -> Option<StepOp> {
    Some(match name {
        "peek" => StepOp::Peek,
        "peek_mut" => StepOp::PeekMut,
        "next_if" => StepOp::NextIf,
        "next_if_eq" => StepOp::NextIfEq,
        _ => return None,
    })
}

/// Which `StepOp` an `Iterator` method is, one that steps a `$iter` (ADR
/// 0071): `cycle()`, of one over an array, whose clone is what it has left
/// (ADR 0181), or of another, cloned (ADR 0491).
fn iterator_step(name: &str) -> Option<StepOp> {
    match name {
        "cycle" => Some(StepOp::Cycle),
        _ => None,
    }
}

/// `to_owned()` of a `ty`: a `str`'s is itself, and any other's, a
/// slice's too, std's blanket impl for a `T: Clone`'s, its `clone()`, whose
/// clone of a `[T]` copies it as its `to_vec()` does (#1109).
fn to_owned_of(ty: Ty<'_>) -> Std {
    match () {
        _ if ty.is_str() => Std::Same,
        _ => Std::ToOwnedClone,
    }
}

/// Whether `def_id` is what `println!` calls, std's `_print`, `Some(false)`,
/// or `eprintln!`'s `_eprint`, `Some(true)`, its error.
pub(in crate::lower) fn print_of(tcx: TyCtxt<'_>, def_id: DefId) -> Option<bool> {
    if !of_std_crate(tcx, def_id, StdCrate::Std) {
        return None;
    }
    match std_path_item(tcx, def_id) {
        Some(StdPath::IoPrintFn) => Some(false),
        Some(StdPath::IoEprintFn) => Some(true),
        _ => None,
    }
}

/// Which of unwinding's functions `def_id` is, `catch_unwind` or
/// `resume_unwind` (ADR 0660), or the js crate's `catch_stack_overflow`
/// (ADR 1140).
pub(in crate::lower) fn unwind_op(tcx: TyCtxt<'_>, def_id: DefId) -> Option<UnwindOp> {
    if super::js_items::is_catch_stack_overflow(tcx, def_id) {
        return Some(UnwindOp::CatchStackOverflow);
    }
    if !of_std_crate(tcx, def_id, StdCrate::Std) {
        return None;
    }
    match std_path_item(tcx, def_id) {
        Some(StdPath::PanicCatchUnwindFn) => Some(UnwindOp::Catch),
        Some(StdPath::PanicResumeUnwindFn) => Some(UnwindOp::Resume),
        _ => None,
    }
}

/// Is `def_id` `hint::unreachable_unchecked()`, which a program that's
/// right never reaches: what MIR's unreachable code is (ADR 0399, amended),
/// which `Mir::new` makes each call of it, so no lowering sees the call.
pub(in crate::lower) fn is_unreachable_hint(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    of_std_crate(tcx, def_id, StdCrate::Core) && is_std_path(tcx, def_id, StdPath::HintUnreachableUncheckedFn)
}

/// Is `def_id` `vec!`'s of an item and a count, `vec![0.0; n]`, a `Vec` of
/// `n` items?
/// Whether a call of `def_id` checks std's room for items of its first
/// type argument, `T`, by their bytes (ADR 0774): a `Vec`'s, a `VecDeque`'s
/// or a `BinaryHeap`'s `with_capacity`, `reserve` and `reserve_exact`,
/// `vec![x; n]`, a `Vec`'s `resize` and `resize_with`, and a slice's
/// `repeat`, whose caller gives that size where it's a type parameter's
/// (`TypeFact::Room`, #1432).
pub(crate) fn checks_room_of_first(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(
        capacity_op(tcx, def_id),
        Some(CapacityOp::WithCapacity | CapacityOp::Reserve { .. })
    ) || is_vec_from_elem(tcx, def_id)
        || matches!(
            array_collection_method(tcx, def_id).as_ref().map(Symbol::as_str),
            Some("resize" | "resize_with")
        )
        || is_std_crate(tcx, def_id)
            && tcx.item_name(def_id).as_str() == "repeat"
            && tcx
                .impl_of_assoc(def_id)
                .is_some_and(|imp| tcx.type_of(imp).instantiate_identity().skip_normalization().is_slice())
}

pub(in crate::lower) fn is_vec_from_elem(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("vec_from_elem"), def_id)
}

/// The number's method an arithmetic intrinsic is, of the type it's of:
/// its generic argument's, or of `fmaf32` and `fmaf64`, their name's.
/// Of a count, `ctpop`, `ctlz` and `cttz`, the method's `u32`, as the
/// intrinsic's is (#1384).
fn number_intrinsic<'tcx>(name: &str, args: ty::GenericArgsRef<'tcx>) -> Option<(&'static str, Num)> {
    let method = match name {
        "saturating_add" | "saturating_sub" | "wrapping_add" | "wrapping_sub" | "wrapping_mul" => {
            return Some((
                [
                    "saturating_add",
                    "saturating_sub",
                    "wrapping_add",
                    "wrapping_sub",
                    "wrapping_mul",
                ]
                .into_iter()
                .find(|m| *m == name)?,
                args.types().next().and_then(Num::of)?,
            ));
        }
        "fmaf32" => return Some(("mul_add", Num::F32)),
        "fmaf64" => return Some(("mul_add", Num::F64)),
        "bswap" => "swap_bytes",
        "bitreverse" => "reverse_bits",
        "ctpop" => "count_ones",
        "ctlz" => "leading_zeros",
        "cttz" => "trailing_zeros",
        "rotate_left" => "rotate_left",
        "rotate_right" => "rotate_right",
        _ => return None,
    };
    Some((method, args.types().next().and_then(Num::of)?))
}
