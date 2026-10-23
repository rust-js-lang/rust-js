//! Which `Std` a function, a trait method, an index or a conversion is,
//! and the method tables they share.

use super::*;

/// A `Vec`'s capacity's (ADR 0419).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum CapacityOp {
    /// `Vec::with_capacity(n)`.
    WithCapacity,
    /// `v.capacity()`.
    Of,
    /// `v.reserve(n)`, `reserve_exact(n)`.
    Reserve,
    /// `v.shrink_to(n)`, or `shrink_to_fit()`.
    ShrinkTo { fit: bool },
}

/// std's collections that are a JS array, whose capacity is the array's
/// (ADRs 0068, 0419): the paths of their methods, of `T` and an allocator.
const ARRAY_COLLECTIONS: [&str; 3] = [
    "std::vec::Vec::<T, A>::",
    "std::collections::VecDeque::<T, A>::",
    "std::collections::BinaryHeap::<T, A>::",
];

/// Which of a `Vec`'s capacity's functions `def_id` is, or a `VecDeque`'s
/// or a `BinaryHeap`'s, one of `ARRAY_COLLECTIONS`.
pub(crate) fn capacity_op(tcx: TyCtxt<'_>, def_id: DefId) -> Option<CapacityOp> {
    let path = std_path(tcx, def_id);
    if matches!(
        path.as_str(),
        "std::vec::Vec::<T>::with_capacity"
            | "std::collections::VecDeque::<T>::with_capacity"
            | "std::collections::BinaryHeap::<T>::with_capacity"
    ) {
        return Some(CapacityOp::WithCapacity);
    }
    Some(match array_collection_method(&path)? {
        "capacity" => CapacityOp::Of,
        "reserve" | "reserve_exact" => CapacityOp::Reserve,
        "shrink_to_fit" => CapacityOp::ShrinkTo { fit: true },
        "shrink_to" => CapacityOp::ShrinkTo { fit: false },
        _ => return None,
    })
}

/// Whether `def_id`, of one of `ARRAY_COLLECTIONS`, may shorten it, which
/// keeps its capacity (ADR 0419): the array it's given first, or its
/// second, `append`'s.
pub(crate) fn shortens_vec(tcx: TyCtxt<'_>, def_id: DefId) -> Option<usize> {
    match array_collection_method(&std_path(tcx, def_id))? {
        "append" => Some(1),
        "clear" | "truncate" | "set_len" | "pop" | "pop_if" | "pop_front" | "pop_back" | "pop_front_if"
        | "pop_back_if" | "remove" | "swap_remove" | "drain" | "retain" | "retain_mut" | "dedup" | "dedup_by"
        | "dedup_by_key" | "split_off" | "resize" | "resize_with" | "extract_if" | "splice" => Some(0),
        _ => None,
    }
}

/// The method of one of `ARRAY_COLLECTIONS` that `path` names.
fn array_collection_method(path: &str) -> Option<&str> {
    ARRAY_COLLECTIONS.iter().find_map(|prefix| path.strip_prefix(prefix))
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
    pub(in crate::lower) fn classify(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
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
        let diagnostic = |name: &str| tcx.is_diagnostic_item(Symbol::intern(name), def_id);
        let self_ty = args.types().next();
        if let Some((fact, of_val)) = layout_query(tcx, def_id) {
            return Some(Some(Std::Layout { fact, of_val }));
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
            std_path(tcx, def_id).as_str(),
            "std::mem::discriminant" | "std::intrinsics::discriminant_value"
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
        if std_path(tcx, def_id) == "std::thread::yield_now" {
            return Some(Some(Std::Nothing));
        }
        // `hint::assert_unchecked(c)`, a promise that `c` holds, which a
        // program that's right keeps: nothing, its `c` made as it's given
        // (ADR 0399).
        if std_path(tcx, def_id) == "std::hint::assert_unchecked" {
            return Some(Some(Std::Nothing));
        }
        // Plain data's bytes as another type's, and all zero (ADR 0408).
        match std_path(tcx, def_id).as_str() {
            "std::intrinsics::transmute" | "std::intrinsics::transmute_unchecked" => {
                return Some(Some(Std::Transmute));
            }
            "std::mem::zeroed" => return Some(Some(Std::Uninit(UninitOp::Zeroed))),
            "std::array::from_fn" => return Some(Some(Std::Slice(SliceOp::FromFn))),
            "std::slice::from_ref" | "std::array::from_ref" => return Some(Some(Std::Slice(SliceOp::FromRef))),
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
        if let Some(op) = crate::lower::pointers::pointer_op(&std_path(tcx, def_id)) {
            return Some(Some(Std::Pointer(op)));
        }
        // A `Vec`'s capacity's, kept where it's asked (ADR 0419).
        if let Some(op) = capacity_op(tcx, def_id) {
            return Some(Some(Std::Capacity(op)));
        }
        // The allocator's, an array of slots (ADR 0417).
        if let Some(op) = crate::lower::std_types::alloc::alloc_op(&std_path(tcx, def_id)) {
            return Some(Some(Std::Alloc(op)));
        }
        // memchr's searches, of the crate rustc builds (ADR 0529).
        if let Some(op) = self.memchr_op(def_id) {
            return Some(Some(Std::Memchr(op)));
        }
        // The process a program runs in, Node's (ADR 0391).
        if let Some(op) = node_only(tcx, def_id) {
            return Some(Some(Std::Process(op)));
        }
        // A C-variadic function's next argument, the first of what's left of
        // its array (ADR 0428).
        if std_path(tcx, def_id) == "std::ffi::VaList::<'f>::next_arg" {
            return Some(Some(Std::Method("shift")));
        }
        // Where a call is, `Location::caller()`, and its parts (ADR 0427).
        if let Some(op) = location_of(tcx, def_id) {
            return Some(Some(Std::Location(op)));
        }
        // A backtrace, disabled unless forced (ADR 0568).
        if let Some(op) = crate::lower::std_types::backtrace::backtrace_op(&std_path(tcx, def_id)) {
            return Some(Some(Std::Backtrace(op)));
        }
        // A future already done, a promise (ADR 0029).
        if let Some(op) = crate::lower::std_types::future::future_op(&std_path(tcx, def_id)) {
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
        if let Some(op) = self.hash_op(def_id, args) {
            return Some(Some(Std::Hash(op)));
        }
        // Bytes to text, as Rust validates UTF-8 (ADR 0172).
        let utf8 = match std_path(tcx, def_id).as_str() {
            "std::str::from_utf8" | "std::str::<impl str>::from_utf8" => Some(TextOp::FromUtf8 { owned: false }),
            "std::string::String::from_utf8" => Some(TextOp::FromUtf8 { owned: true }),
            "std::str::from_utf8_unchecked"
            | "std::str::<impl str>::from_utf8_unchecked"
            | "std::string::String::from_utf8_unchecked" => Some(TextOp::Utf8Unchecked),
            "std::string::String::from_utf8_lossy" => Some(TextOp::Utf8Lossy),
            "std::str::from_utf8_mut" | "std::str::<impl str>::from_utf8_mut" => {
                Some(TextOp::FromUtf8Mut { unchecked: false })
            }
            "std::str::from_utf8_unchecked_mut" | "std::str::<impl str>::from_utf8_unchecked_mut" => {
                Some(TextOp::FromUtf8Mut { unchecked: true })
            }
            "std::str::Utf8Error::valid_up_to" => Some(TextOp::Utf8Part("validUpTo")),
            // A `Utf8Chunk`'s parts, `{ valid, invalid }` (ADR 0341).
            "std::str::Utf8Chunk::<'a>::valid" => Some(TextOp::Utf8Part("valid")),
            "std::str::Utf8Chunk::<'a>::invalid" => Some(TextOp::Utf8Part("invalid")),
            "std::num::ParseIntError::kind" => Some(TextOp::ParseErrorKind),
            // A byte's escape, its bytes (ADR 0558).
            "std::ascii::escape_default" | "std::num::<impl u8>::escape_ascii" => Some(TextOp::EscapeByte),
            _ => None,
        };
        if let Some(op) = utf8 {
            return Some(Some(Std::Text(op)));
        }
        // A path is its text (ADR 0173): what reads it as text is itself.
        match std_path(tcx, def_id).as_str() {
            "std::path::Path::new"
            | "std::path::Path::display"
            | "std::path::Path::to_str"
            | "std::path::Path::as_os_str"
            | "std::ffi::OsStr::to_str"
            | "std::path::PathBuf::as_path" => return Some(Some(Std::Same)),
            "std::path::PathBuf::new" => return Some(Some(Std::StringNew)),
            // Its methods, over its components (ADR 0405).
            "std::path::Path::join" => return Some(Some(Std::Path(PathOp::Join))),
            "std::path::Path::parent" => return Some(Some(Std::Path(PathOp::Parent))),
            "std::path::Path::file_name" => return Some(Some(Std::Path(PathOp::FileName))),
            "std::path::Path::file_stem" => return Some(Some(Std::Path(PathOp::FileStem))),
            "std::path::Path::extension" => return Some(Some(Std::Path(PathOp::Extension))),
            "std::path::Path::with_extension" => return Some(Some(Std::Path(PathOp::WithExtension))),
            "std::path::Path::with_file_name" => return Some(Some(Std::Path(PathOp::WithFileName))),
            "std::path::Path::starts_with" => return Some(Some(Std::Path(PathOp::StartsWith))),
            "std::path::Path::ends_with" => return Some(Some(Std::Path(PathOp::EndsWith))),
            "std::path::Path::strip_prefix" => return Some(Some(Std::Path(PathOp::StripPrefix))),
            "std::path::Path::is_absolute" | "std::path::Path::has_root" => {
                return Some(Some(Std::Path(PathOp::IsAbsolute)));
            }
            "std::path::Path::is_relative" => return Some(Some(Std::Path(PathOp::IsRelative))),
            "std::path::Path::iter" => return Some(Some(Std::Path(PathOp::Iter))),
            "std::path::Path::ancestors" => return Some(Some(Std::Path(PathOp::Ancestors))),
            "std::path::Path::to_path_buf" => return Some(Some(Std::Same)),
            "std::path::PathBuf::push" => return Some(Some(Std::StringEdit(StringEdit::PathPush))),
            "std::path::PathBuf::pop" => return Some(Some(Std::StringEdit(StringEdit::PathPop))),
            "std::path::PathBuf::set_extension" => return Some(Some(Std::StringEdit(StringEdit::SetExtension))),
            "std::path::PathBuf::set_file_name" => return Some(Some(Std::StringEdit(StringEdit::SetFileName))),
            // A `ManuallyDrop` is what it holds (ADR 0406), never dropped.
            "std::mem::ManuallyDrop::<T>::new" | "std::mem::ManuallyDrop::<T>::into_inner" => {
                return Some(Some(Std::Same));
            }
            "std::mem::ManuallyDrop::<T>::drop" => return Some(Some(Std::DropPointee)),
            // A `NonZero` is its number (ADR 0177).
            "std::num::NonZero::<T>::get" | "std::num::NonZero::<T>::new_unchecked" => return Some(Some(Std::Same)),
            "std::num::NonZero::<T>::new" => return Some(Some(Std::NonZeroNew)),
            _ => {}
        }
        let utf8 = match std_path(tcx, def_id).as_str() {
            "std::str::Utf8Error::error_len" => Some(TextOp::Utf8Part("errorLen")),
            "std::string::FromUtf8Error::utf8_error" => Some(TextOp::Utf8Part("error")),
            "std::string::FromUtf8Error::into_bytes" | "std::string::FromUtf8Error::as_bytes" => {
                Some(TextOp::Utf8Part("bytes"))
            }
            _ => None,
        };
        if let Some(op) = utf8 {
            return Some(Some(Std::Text(op)));
        }
        if std_path(tcx, def_id) == "std::sync::mpsc::channel" {
            return Some(Some(Std::Channel(ChannelOp::New)));
        }
        if diagnostic("mem_swap") {
            return Some(Some(Std::Swap));
        }
        if diagnostic("ptr_eq") {
            return Some(Some(Std::PtrEq));
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
        if tcx.crate_name(def_id.krate) == sym::core && std_path(tcx, def_id) == "std::mem::take" {
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
        if tcx.crate_name(def_id.krate) == sym::std {
            match std_path(tcx, def_id).as_str() {
                "std::io::stdout" | "std::io::stderr" => return Some(Some(Std::Stream(StreamOp::Open))),
                _ => {}
            }
        }
        if tcx.crate_name(def_id.krate) == sym::core {
            match std_path(tcx, def_id).as_str() {
                "std::any::type_name" => return Some(Some(Std::TypeName { of_val: false })),
                "std::any::type_name_of_val" => return Some(Some(Std::TypeName { of_val: true })),
                _ => {}
            }
        }
        // std's iterator sources (ADR 0128), by path: most have no
        // diagnostic item.
        if tcx.crate_name(def_id.krate) == sym::core && tcx.def_kind(def_id) == DefKind::Fn {
            let source = match std_path(tcx, def_id).as_str() {
                "std::iter::once" => Some(IterSource::Once),
                "std::iter::empty" => Some(IterSource::Empty),
                "std::iter::repeat" => Some(IterSource::Repeat),
                "std::iter::repeat_n" => Some(IterSource::RepeatN),
                "std::iter::repeat_with" => Some(IterSource::RepeatWith),
                "std::iter::successors" => Some(IterSource::Successors),
                "std::iter::from_fn" => Some(IterSource::FromFn),
                "std::iter::zip" => Some(IterSource::Zip),
                _ => None,
            };
            if let Some(source) = source {
                return Some(Some(Std::IterSource(source)));
            }
        }
        // `hint::black_box(x)` is `x`: it only hides `x` from an optimizer.
        // libtest's `test::black_box` is the same.
        let krate = tcx.crate_name(def_id.krate);
        if tcx.item_name(def_id).as_str() == "black_box"
            && ((krate == sym::core && std_path(tcx, def_id).contains("hint")) || krate.as_str() == "test")
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
            tcx.crate_name(def_id.krate) == sym::core
                && tcx.item_name(def_id).as_str() == name
                && std_path(tcx, def_id).contains("char")
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
        if tcx.crate_name(def_id.krate).as_str() == "serde_json" {
            match tcx.item_name(def_id).as_str() {
                "to_string" => return Some(Some(Std::ToJson(false))),
                "to_string_pretty" => return Some(Some(Std::ToJson(true))),
                "from_str" => return Some(Some(Std::FromJson)),
                _ => {}
            }
        }
        if diagnostic("vec_from_elem") {
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
        let krate = tcx.crate_name(def_id.krate);
        let name = tcx.item_name(def_id);
        if krate == sym::core
            && let Some(imp) = tcx.inherent_impl_of_assoc(def_id)
            && Num::of(tcx.type_of(imp).instantiate_identity().skip_normalization()).is_some_and(Num::float)
        {
            match name.as_str() {
                "max" => return Some(Some(Std::MaxOf(true))),
                "min" => return Some(Some(Std::MaxOf(false))),
                _ => {}
            }
        }
        if krate == sym::alloc && name.as_str() == "format" && std_path(tcx, def_id).ends_with("fmt::format") {
            return Some(Some(Std::Format));
        }
        if krate == sym::core && name.as_str() == "must_use" {
            return Some(Some(Std::Same));
        }
        if tcx.is_lang_item(def_id, LangItem::Panic) {
            return Some(Some(Std::Panic));
        }
        // `panic!("{}", x)`, which std writes as `panic_display(&x)` (ADR 0189).
        if tcx.is_lang_item(def_id, LangItem::PanicDisplay) {
            return Some(Some(Std::PanicDisplay { unreachable: false }));
        }
        if krate == sym::core && name.as_str() == "unreachable_display" && std_path(tcx, def_id).contains("panicking") {
            return Some(Some(Std::PanicDisplay { unreachable: true }));
        }
        if krate == sym::std {
            match std_path(tcx, def_id).as_str() {
                "std::io::_print" => return Some(Some(Std::Print { error: false })),
                "std::io::_eprint" => return Some(Some(Std::Print { error: true })),
                "std::rt::begin_panic" => return Some(Some(Std::BeginPanic)),
                "std::panic::catch_unwind" => return Some(Some(Std::Unwind(UnwindOp::Catch))),
                "std::panic::resume_unwind" => return Some(Some(Std::Unwind(UnwindOp::Resume))),
                _ => {}
            }
        }
        if tcx.is_lang_item(def_id, LangItem::PanicFmt) {
            return Some(Some(Std::PanicFmt));
        }
        // `hint::unreachable_unchecked()`, which a program that's right never
        // reaches: what MIR's unreachable code is.
        if krate == sym::core && std_path(tcx, def_id) == "std::hint::unreachable_unchecked" {
            return Some(Some(Std::Unreachable));
        }
        if tcx.crate_name(def_id.krate) == sym::core && tcx.item_name(def_id).as_str() == "assert_failed" {
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
            return Some(Std::Map(MapOp::Index));
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
        // `b.as_ref()` of std's `AsRef` where it's the same JS value, a `Box`'s,
        // an `Rc`'s, or text's as a `Path` (ADRs 0331, 0505, 0173).
        if is_std_def(tcx, trait_, StdItem::AsRef)
            && tcx.item_name(def_id).as_str() == "as_ref"
            && (ty.is_box() || rc_pointee(tcx, ty.peel_refs()).is_some() || self.is_path_like(args.type_at(1)))
            && self.borrows_as_itself(ty, args.type_at(1))
        {
            return Some(Std::Same);
        }
        // `std::num::Wrapping`'s: its number's, wrapped as release Rust wraps
        // them (ADR 0175).
        if self.wrapping_of(ty.peel_refs()).is_some() {
            if let Some(op) = value_operator(tcx, trait_) {
                return Some(Std::WrappingOp(op, false));
            }
            if let Some(op) = assign_operator(tcx, trait_) {
                return Some(Std::WrappingOp(Ok(op), true));
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
        if Num::of(ty.peel_refs()).is_some() {
            if let Some(Ok(op)) = value_operator(tcx, trait_) {
                return Some(Std::Operator(op));
            }
            // `total += x` with a `&u32` `x`: the same assignment as with a `u32`.
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
        if std_path(tcx, trait_) == "std::iter::ExactSizeIterator"
            && tcx.item_name(def_id).as_str() == "len"
            && self.is_user_iterator(ty)
        {
            return Some(Std::ExactLen);
        }
        // Of what only a caller knows too, `I` or `H::Item`: an array or a JS
        // iterator (ADR 0061), never a collection of the crate's (ADR 0160).
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && (ty.peel_refs().is_array()
                || ty.peel_refs().is_slice()
                || self.is_vec_like(ty.peel_refs())
                || self.is_given_iterable(ty.peel_refs()))
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
        // `cmp`, `max` and `min` of what JS's `<` orders the same way.
        if tcx.is_diagnostic_item(sym::Ord, trait_) {
            let peeled = ty.peel_refs();
            let comparable = Num::of(peeled).is_some() || peeled.is_bool() || self.is_string_like(peeled);
            return match tcx.item_name(def_id).as_str() {
                "cmp" if comparable => Some(Std::Cmp),
                "max" if Num::of(peeled).is_some() => Some(Std::MaxOf(true)),
                "min" if Num::of(peeled).is_some() => Some(Std::MaxOf(false)),
                "clamp" if Num::of(peeled).is_some() => Some(Std::Number(NumOp::Clamp)),
                _ => None,
            };
        }
        // `write!(s, ..)` into a `String`: `s += ..`, as `push_str` is; writing
        // to a string can't fail, so its `fmt::Result` is nothing (ADR 0148).
        if self.is_lang_adt(ty.peel_refs(), LangItem::String)
            && tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), trait_)
        {
            return match tcx.item_name(def_id).as_str() {
                "write_fmt" | "write_str" | "write_char" => Some(Std::PushStr),
                _ => None,
            };
        }
        // A writer of the crate's own: what `write!` and `write_char` give its
        // `write_str` (ADR 0166). One it writes itself is called as it is.
        if tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), trait_)
            && matches!(tcx.item_name(def_id).as_str(), "write_fmt" | "write_char")
            && self.has_user_impl(trait_, ty.peel_refs())
        {
            return Some(Std::UserWrite);
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
        let to_owned = tcx.is_diagnostic_item(Symbol::intern("ToOwned"), trait_);
        // A slice's `to_owned()` is its `to_vec()`.
        (to_owned && ty.is_str())
            .then_some(Std::Same)
            .or((to_owned && ty.is_slice()).then_some(Std::ToVec))
    }

    /// A range's own iterator method: a range is an iterator already, and
    /// its `len` and `next_back` are its bounds' (ADR 0129).
    fn range_iterator_method(&self, def_id: DefId, trait_: DefId, ty: Ty<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let kind = self.range_kind(ty)?;
        let stepped = matches!(kind, RangeKind::Exclusive | RangeKind::Inclusive);
        Some(Std::Range(match tcx.item_name(def_id).as_str() {
            "into_iter" if tcx.is_diagnostic_item(sym::IntoIterator, trait_) => return Some(Std::Same),
            "len" if stepped && std_path(tcx, trait_) == "std::iter::ExactSizeIterator" => RangeOp::Len,
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
        let owned = matches!(from.kind(), ty::Adt(adt, _) if self.tcx.is_diagnostic_item(sym::Vec, adt.did()));
        let slice = matches!(from.kind(), ty::Ref(_, slice, Mutability::Not) if slice.is_slice());
        let array = match to.kind() {
            ty::Ref(_, array, Mutability::Not) => slice && array.is_array(),
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
            None if self.is_array_iter(ty.peel_refs()) => Some(Std::SizeHint(true)),
            _ => None,
        }
    }

    /// A write, `io::Write`'s, to a standard stream, `print!`'s (ADR 0132),
    /// or to a `Vec<u8>`, its bytes added, as `extend_from_slice` adds
    /// them (ADR 0392), where the writer's type is known.
    fn classify_io_write(&self, def_id: DefId, trait_: DefId, ty: Ty<'tcx>) -> Option<Std> {
        if std_path(self.tcx, trait_) != "std::io::Write" {
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

    /// A `From`'s or an `Into`'s conversion, of std's, if rust-js knows it:
    /// one list of what it converts into what, `conversion_sides`', the same
    /// of either.
    fn classify_conversion(&self, trait_: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
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
            return Some(Std::Text(TextOp::Bytes));
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
                _ => None,
            };
        }
        let tr = tcx.trait_of_assoc(def_id)?;
        let mut types = args.types();
        let (first, second) = (types.next()?, types.next()?);
        let (to, from) = match () {
            _ if is_std_def(tcx, tr, StdItem::From) => (first, second),
            _ if is_std_def(tcx, tr, StdItem::Into) => (second, first),
            _ => return None,
        };
        (self.is_io_error(to) && self.is_io_error_kind(from)).then_some(IoErrorOp::FromKind)
    }

    /// Which of hashing's functions `def_id` is (ADR 0463): a std hasher's
    /// `new` and `new_with_keys`, its own `Hasher` methods, and a std type's
    /// `Hash`.
    fn hash_op(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<HashOp> {
        let tcx = self.tcx;
        match std_path(tcx, def_id).as_str() {
            "std::hash::DefaultHasher::new" => return Some(HashOp::New(StdHasher::Default)),
            "std::hash::SipHasher::new" => return Some(HashOp::New(StdHasher::Sip)),
            "std::hash::SipHasher::new_with_keys" => return Some(HashOp::NewWithKeys),
            _ => {}
        }
        let tr = tcx.trait_of_assoc(def_id)?;
        let self_ty = args.types().next()?;
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
                tcx.codegen_select_candidate(
                    self.typing_env
                        .as_query_input(tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, tr, [self_ty]))),
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
/// or an `Argument`'s. `as_str()` is `Some` of the text the arguments are
/// already (ADR 0034): its contract promises only the literal case, and
/// allows `Some` of any.
pub(super) fn format_args_method(name: &str, arguments: bool) -> Option<Std> {
    Some(match (name, arguments) {
        ("from_str" | "from_str_nonconst", true) => Std::FmtStr,
        ("new", true) => Std::FmtNew,
        ("as_str", true) => Std::Same,
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
/// a `RefCell`'s borrows, which are their kinds'.
pub(super) fn cell_method(name: &str, refcell: bool) -> Option<Std> {
    Some(match name {
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

/// A `TryFrom` or a `TryInto` of `from` into `to` std makes, `into` of
/// `try_into()`: between integers, which may not fit; a `u32` into a
/// `char`, or a `char` into a `u8`.
fn try_conversion<'tcx>(from: Ty<'tcx>, to: Ty<'tcx>, into: bool) -> Option<Std> {
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
