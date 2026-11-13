//! What a std type is: its identity, its facts, and what it holds.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(in crate::lower) fn is_string_like(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        ty.is_str() || ty.is_char() || self.is_lang_adt(ty, LangItem::String)
    }

    /// `Layout`, `{ size, align }` (ADR 0417), `Some(true)`; or
    /// `LayoutError`, which says nothing, `Some(false)`.
    pub(in crate::lower) fn layout(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        match std_path(self.tcx, adt.did()) {
            "std::alloc::Layout" => Some(true),
            "std::alloc::LayoutError" => Some(false),
            _ => None,
        }
    }

    /// `panic::Location`: `{ file, line, column }` (ADR 0427).
    pub(in crate::lower) fn is_location(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(location, _) if std_path(self.tcx, location.did()) == "std::panic::Location")
    }

    /// `io::Error`, a `$IoError` (ADR 0462).
    pub(in crate::lower) fn is_io_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(error, _) if std_path(self.tcx, error.did()) == "std::io::Error")
    }

    /// What an `io::Error` may hold and drops as it's dropped, as its
    /// `into_inner()` gives it, std's own signature's
    /// `Option<Box<dyn Error + Send + Sync>>` (ADR 0462, amended).
    pub(in crate::lower) fn io_error_held(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty::Adt(error, _) = ty.kind() else { return None };
        // Core's type, whose methods alloc's and std's impls write.
        let incoherent = self
            .tcx
            .incoherent_impls(rustc_middle::ty::fast_reject::SimplifiedType::Adt(error.did()));
        let into_inner = (self.tcx.inherent_impls(error.did()).iter().chain(incoherent))
            .flat_map(|&imp| self.tcx.associated_items(imp).in_definition_order())
            .find(|item| item.is_fn() && item.name().as_str() == "into_inner")?;
        let sig = self.tcx.fn_sig(into_inner.def_id).instantiate_identity().skip_binder();
        Some(
            self.tcx
                .instantiate_bound_regions_with_erased(ty::Binder::dummy(sig.output())),
        )
    }

    /// `std::backtrace::Backtrace`, `undefined` or JS's stack (ADR 0568).
    pub(in crate::lower) fn is_backtrace(&self, ty: Ty<'tcx>) -> bool {
        is_backtrace_type(self.tcx, ty)
    }

    /// `io::ErrorKind`, its variant's name (ADR 0462).
    pub(in crate::lower) fn is_io_error_kind(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(kind, _) if std_path(self.tcx, kind.did()) == "std::io::ErrorKind")
    }

    /// std's `Hasher`, found among the traits, as it has no diagnostic item.
    pub(in crate::lower) fn hasher_trait(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_hasher(self.tcx, id))
    }

    /// Which of std's hashers `ty` is, a `$SipHasher` (ADR 0463).
    pub(in crate::lower) fn std_hasher(&self, ty: Ty<'tcx>) -> Option<StdHasher> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        match std_path(self.tcx, adt.did()) {
            "std::hash::DefaultHasher" => Some(StdHasher::Default),
            "std::hash::SipHasher" => Some(StdHasher::Sip),
            _ => None,
        }
    }

    /// What std's `Hash` of a `ty` writes, where it's std's (ADR 0463):
    /// `None` of a crate's type, a type parameter, or one rust-js doesn't
    /// hash.
    pub(in crate::lower) fn hashed_as(&self, ty: Ty<'tcx>) -> Option<Hashed<'tcx>> {
        Some(match *ty.kind() {
            ty::Int(_) | ty::Uint(_) => Hashed::Number(ty),
            // A `NonZero` is its number (ADR 0177), and an `Ordering` its
            // `i8`, `-1`, `0` or `1`, as its derived `Hash` writes it.
            ty::Adt(_, args) if is_non_zero_ty(ty) => Hashed::Number(args.type_at(0)),
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::OrderingEnum) => Hashed::Number(self.tcx.types.i8),
            // A `Duration`, an `Instant` or a `SystemTime`, its nanoseconds:
            // its seconds and the nanoseconds under one, as std's (#1447).
            ty::Adt(..) if is_duration_ty(ty) || clock_type_of(ty) == Some(ClockType::Instant) => {
                Hashed::Nanoseconds { before_epoch: false }
            }
            ty::Adt(..) if clock_type_of(ty) == Some(ClockType::System) => Hashed::Nanoseconds { before_epoch: true },
            ty::Adt(..) if self.is_os_text(ty) => Hashed::OsText,
            ty::Adt(..) if self.compares_by_components(ty) => Hashed::PathText,
            ty::Adt(..) if self.layout(ty) == Some(true) => Hashed::Layout,
            // A `TypeId`, its type's name (ADR 0331): that text, `write_str`,
            // where std writes an id of its own, its value unspecified and
            // changing between compilers; equal ones hash equal (#1447).
            ty::Adt(..) if self.is_type_id(ty) => Hashed::Text,
            // A `mem::Discriminant<T>`, its number (ADR 0393), as `T`'s
            // discriminant type, which std's hashes (#1447).
            ty::Adt(_, args) if self.is_discriminant(ty) => Hashed::Number(args.type_at(0).discriminant_ty(self.tcx)),
            // One of std's with no fields whose `Hash` is derived,
            // `PhantomPinned`: nothing (#1447).
            ty::Adt(adt, _)
                if adt.is_struct()
                    && adt.non_enum_variant().fields.is_empty()
                    && is_std_crate(self.tcx, adt.did())
                    && self.derives(std_item(self.tcx, StdItem::Hash), ty) =>
            {
                Hashed::Tuple(Vec::new())
            }
            ty::Bool => Hashed::Bool,
            ty::Char => Hashed::Char,
            ty::Str => Hashed::Text,
            ty::Ref(_, pointee, _) => Hashed::Pointer(pointee),
            // A raw pointer hashes its address (ADR 0562).
            _ if let Some((pointee, _)) = crate::lower::pointers::raw_pointee(self.tcx, ty) => Hashed::Address(pointee),
            ty::Tuple(parts) => Hashed::Tuple(parts.to_vec()),
            ty::Slice(item) | ty::Array(item, _) => Hashed::Slice(item),
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => Hashed::Pointer(args.type_at(0)),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec) => Hashed::Slice(args.type_at(0)),
            // A C string is its bytes with the nul (ADR 1119), and its
            // derived `Hash` is `[c_char]`'s, a `CString`'s `Box<[u8]>`'s:
            // its length, then its bytes in one write, the same bytes
            // whether `c_char` is signed or not.
            ty::Adt(..) if is_c_string_like(self.tcx, ty) => Hashed::Slice(self.tcx.types.u8),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::VecDeque) => Hashed::Items(args.type_at(0)),
            ty::Adt(_, args) if self.option_of(ty).is_some() => Hashed::Option(args.type_at(0)),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Result) && !self.is_write_result(ty) => {
                Hashed::Result(args.type_at(0), args.type_at(1))
            }
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::String) => Hashed::Text,
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::PhantomData) => Hashed::Tuple(Vec::new()),
            ty::Adt(_, args) if let Some(kind) = self.range_kind(ty) => Hashed::Range(kind, args.types().next()),
            // One of std's that's in JS the value it holds, `ManuallyDrop`
            // and the `MaybeDangling` in it: what it holds, as std's `Hash`
            // of a `ManuallyDrop` hashes, `self.value.as_ref().hash(state)`
            // (#1289).
            ty::Adt(..) if let Some(held) = self.std_wrapped(ty) => Hashed::Held(held),
            // A `Pin`, its pointer in JS (ADR 0329), whose `Hash` is its
            // target's, `Ptr::Target::hash` (`pinned_target`): of std's
            // pointer, that pointer's, which hashes as its target (#1289); of
            // the crate's own, its `deref()`'s target (#1435).
            ty::Adt(..) if let Some(target) = self.pinned_target(ty) => match target {
                super::PinTarget::Pointer(pointer) => Hashed::Held(pointer),
                super::PinTarget::Deref(pointer, target) => Hashed::Target(pointer, target),
            },
            // A `Cow`, whose `Hash` is what it derefs to's (#1289).
            ty::Adt(..) if let Some((borrowed, _)) = self.cow_parts(ty) => Hashed::Borrowed(borrowed),
            // One of std's enums whose `Hash` is derived, `Bound`, `Poll`,
            // `ControlFlow`: its discriminant, then its variant's fields, read
            // as a pattern reads them (#1289).
            ty::Adt(adt, args)
                if adt.is_enum()
                    && is_std_crate(self.tcx, adt.did())
                    && self.derives(std_item(self.tcx, StdItem::Hash), ty) =>
            {
                Hashed::Variants(adt, args)
            }
            // One of std's that's its fields in JS, `Reverse(x)` and
            // `Wrapping(x)`, `[x]`, whose `Hash` is derived: each field's, in
            // order, as a tuple's, which runs a crate's own `Hash` of one as
            // std's does (#1289).
            ty::Adt(adt, args)
                if adt.is_struct()
                    && is_std_crate(self.tcx, adt.did())
                    && super::traits::struct_is_its_fields(self.tcx, adt.did())
                    && self.derives(std_item(self.tcx, StdItem::Hash), ty) =>
            {
                let fields = adt.non_enum_variant().fields.iter();
                Hashed::Tuple(fields.map(|field| self.field_ty(field, args)).collect())
            }
            _ => return None,
        })
    }

    /// `<pointer as Deref>::Target`, of a pointer that derefs, the crate's
    /// own smart pointer's.
    pub(in crate::lower) fn deref_target_of(&self, pointer: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let target = self.tcx.lang_items().deref_target()?;
        let projection = Ty::new_projection(self.tcx, ty::IsRigid::No, target, [pointer]);
        (self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection)))
        .ok()
    }

    /// Which of std's ranges `ty` is, if it's one (ADR 0129).
    pub(in crate::lower) fn range_kind(&self, ty: Ty<'tcx>) -> Option<RangeKind> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        [
            (LangItem::Range, RangeKind::Exclusive),
            (LangItem::RangeInclusiveStruct, RangeKind::Inclusive),
            (LangItem::RangeFrom, RangeKind::From),
            (LangItem::RangeTo, RangeKind::To),
            (LangItem::RangeToInclusive, RangeKind::ToInclusive),
            (LangItem::RangeFull, RangeKind::Full),
        ]
        .into_iter()
        .find(|&(item, _)| self.tcx.is_lang_item(adt.did(), item))
        .map(|(_, kind)| kind)
    }

    /// Is `ty` std's `item`?
    pub(in crate::lower) fn is_std_type(&self, ty: Ty<'tcx>, item: StdItem) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if is_std_def(self.tcx, adt.did(), item))
    }

    pub(in crate::lower) fn is_lang_adt(&self, ty: Ty<'tcx>, item: LangItem) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.is_lang_item(adt.did(), item))
    }

    /// A `Box<[T; N]>` or a `Box<[T]>`: its array, in JS (ADR 0023).
    pub(in crate::lower) fn is_boxed_items(&self, ty: Ty<'tcx>) -> bool {
        ty.boxed_ty().is_some_and(|inner| inner.is_array() || inner.is_slice())
    }

    /// A `Vec`, a `VecDeque`, a `BinaryHeap` or a `LinkedList`: a JS array.
    pub(in crate::lower) fn is_vec_like(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if is_vec_like_def(self.tcx, adt.did()))
    }

    /// Whether std's `FromIterator` of `ty` grows a `Vec`, asking what it
    /// steps its `size_hint` at its first item and each time it's full:
    /// a `Vec`'s, a `VecDeque`'s and a `Box<[T]>`'s, and of what std makes
    /// of one, a `BinaryHeap`'s, `BinaryHeap::from(vec)`, and a `BTreeSet`'s
    /// or `BTreeMap`'s, a sorted `bulk_build` (#797). Not a `LinkedList`'s,
    /// which extends a new one.
    pub(in crate::lower) fn grows_as_vec(&self, ty: Ty<'tcx>) -> bool {
        GROWS_AS_VEC.iter().any(|&item| self.is_std_type(ty, item))
            || ty.boxed_ty().is_some_and(|inner| inner.is_slice())
    }

    /// `std::str::Chars`, what a string's `chars()` gives.
    pub(in crate::lower) fn is_str_chars(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
            && self.tcx.item_name(adt.did()).as_str() == "Chars")
    }

    /// A string's `drain(range)`, the `char`s it took out, which steps as
    /// `chars()`' do, an array of them (#118).
    pub(in crate::lower) fn is_string_drain(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::string::Drain")
    }

    /// A string's `char_indices()`, an array of `[byte, char]` (#118).
    pub(in crate::lower) fn is_char_indices(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::str::CharIndices")
    }

    /// A string's `bytes()`, an array of its UTF-8 bytes (ADR 0034).
    pub(in crate::lower) fn is_str_bytes(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
            && self.tcx.item_name(adt.did()).as_str() == "Bytes")
    }

    /// `js::JsError`, what a JS function threw: the builtins crate's.
    pub(in crate::lower) fn is_js_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate).as_str() == "js"
            && self.tcx.item_name(adt.did()).as_str() == "JsError")
    }

    /// `js::SymbolFor`, the registry's symbol of a key (ADR 0634).
    pub(in crate::lower) fn is_symbol_for(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if is_symbol_for_def(self.tcx, adt.did()))
    }

    pub(in crate::lower) fn is_js_object(&self, ty: Ty<'tcx>) -> bool {
        let ty::Adt(adt, args) = ty.kind() else { return false };
        if !adt.is_struct() {
            return false;
        }
        // `JsObject` itself, what `Kind::Object` holds, TypeScript's
        // `object` (ADR 0677).
        if self.is_js_object_itself(ty) {
            return true;
        }
        // `PhantomData<JsObject>`, then only more markers, for a generic one
        // like `Promise<T>`.
        let mut fields = adt.non_enum_variant().fields.iter().map(|f| self.field_ty(f, args));
        let first = fields.next();
        first.is_some_and(|field| {
            matches!(field.kind(), ty::Adt(marker, marked) if marker.is_phantom_data()
            && marked.types().next().is_some_and(|t| matches!(t.kind(), ty::Foreign(_)) || self.is_js_object_itself(t)))
        }) && fields.all(|field| matches!(field.kind(), ty::Adt(marker, _) if marker.is_phantom_data()))
    }

    /// The js crate's `JsObject`, `#[rust_js::js_object]`, a struct no one makes,
    /// where it was an extern type, a nightly feature (ADR 0111): a struct of a
    /// `PhantomData` of it is a JS object, as one of an extern type still is.
    /// A program never has a `JsObject` itself, only a reference to one.
    fn is_js_object_itself(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx
            .get_attrs_by_path(adt.did(), &[Symbol::intern("rust_js"), Symbol::intern("js_object")])
            .next()
            .is_some())
    }

    /// Whether std extends a collection of type `ty` as each item comes,
    /// a hash map's or a set's, or a `LinkedList`'s, never collecting it as
    /// a `Vec` first: so what's left of a collect a panic stops is dropped
    /// before what it collected, and nothing is collected in place. The one
    /// answer of the lowering's `rest_dropped_first` and the analysis's
    /// `in_place_asked` (#1443).
    pub(in crate::lower) fn extends_as_each_comes(&self, ty: Ty<'tcx>) -> bool {
        (self.is_map(ty) && !self.is_sorted(ty)) || self.is_std_type(ty, StdItem::LinkedList)
    }

    pub(in crate::lower) fn is_map(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        [StdItem::HashMap, StdItem::HashSet, StdItem::BTreeMap, StdItem::BTreeSet]
            .into_iter()
            .any(|item| self.is_std_type(ty, item))
            || self.is_json_map(ty)
    }

    /// A pattern that's a predicate of a `char`: a closure, a function, or a
    /// set of `char`s, an array or a slice of them (ADR 0157).
    pub(in crate::lower) fn is_char_predicate(&self, pattern: Ty<'tcx>) -> bool {
        match pattern.peel_refs().kind() {
            ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..) => true,
            ty::Array(item, _) | ty::Slice(item) => item.is_char(),
            _ => false,
        }
    }

    /// A pattern that's a predicate running the program's code, a closure,
    /// a function not std's, or one known only as a pointer: which `char`s
    /// it's asked of, and when, the program can see, so its searches are
    /// std's, asked as they're stepped (#1501). A `char`, a `&str`, a set of
    /// `char`s and std's own predicates run none.
    pub(in crate::lower) fn pattern_runs_code(&self, pattern: Ty<'tcx>) -> bool {
        match pattern.peel_refs().kind() {
            ty::Closure(..) | ty::FnPtr(..) => true,
            ty::FnDef(def_id, _) => !self.is_std(*def_id),
            _ => false,
        }
    }

    pub(in crate::lower) fn is_set(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        self.is_std_type(ty, StdItem::HashSet) || self.is_std_type(ty, StdItem::BTreeSet)
    }

    /// A B-tree, `BTreeMap` or `BTreeSet`, serde_json's `Map` too: one
    /// that goes over its keys in their order.
    pub(in crate::lower) fn is_sorted(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        self.is_std_type(ty, StdItem::BTreeMap) || self.is_std_type(ty, StdItem::BTreeSet) || self.is_json_map(ty)
    }

    /// A channel's end, `mpsc::Sender` or `mpsc::Receiver` (ADR 0142).
    pub(in crate::lower) fn channel_end(&self, ty: Ty<'tcx>) -> Option<ChannelEnd> {
        let ty::Adt(adt, _) = ty.kind() else {
            return None;
        };
        match std_path(self.tcx, adt.did()) {
            "std::sync::mpsc::Sender" => Some(ChannelEnd::Sender),
            "std::sync::mpsc::Receiver" => Some(ChannelEnd::Receiver),
            _ => None,
        }
    }

    /// A channel's error, which std defines with its multi-producer channels.
    pub(in crate::lower) fn channel_error(&self, ty: Ty<'tcx>) -> Option<ChannelError> {
        let ty::Adt(adt, _) = ty.kind() else {
            return None;
        };
        let path = std_path(self.tcx, adt.did());
        if !path.starts_with("std::sync::mpsc::") && !path.starts_with("std::sync::mpmc::") {
            return None;
        }
        match self.tcx.item_name(adt.did()).as_str() {
            "RecvError" => Some(ChannelError::Recv),
            "TryRecvError" => Some(ChannelError::TryRecv),
            "SendError" => Some(ChannelError::Send),
            _ => None,
        }
    }

    pub(in crate::lower) fn is_parse_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
            && ["ParseIntError", "ParseFloatError", "ParseBoolError", "ParseCharError", "TryFromIntError", "CharTryFromError", "TryFromCharError", "BorrowError", "BorrowMutError"]
                .contains(&self.tcx.item_name(adt.did()).as_str()))
    }

    /// A lock's `TryLockError`: `WouldBlock`, or `Poisoned` of a `PoisonError`.
    pub(in crate::lower) fn is_try_lock_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::sync::TryLockError")
    }

    /// A `Utf8Error`, `false`, or a `FromUtf8Error`, `true`: the objects the
    /// runtime makes of them (ADR 0172).
    pub(in crate::lower) fn utf8_error(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Adt(adt, _) = ty.kind() else {
            return None;
        };
        match std_path(self.tcx, adt.did()) {
            "std::str::Utf8Error" => Some(false),
            "std::string::FromUtf8Error" => Some(true),
            _ => None,
        }
    }

    /// A `Path` or a `PathBuf`, whose `==` and ordering are its components'
    /// (ADR 0401), where an `OsStr`'s are its text's.
    /// Whether a `ty` value compares, orders and hashes by its components,
    /// not its text, `a/b` and `a//b` equal: a `Path`'s and a `PathBuf`'s
    /// `==`, `cmp` and `Hash` (ADR 0401, #1467, #1447). The one rule `==`,
    /// order, a derived or structural comparison of what holds one, and a
    /// table's keys ask.
    pub(in crate::lower) fn compares_by_components(&self, ty: Ty<'tcx>) -> bool {
        self.is_path(ty)
    }

    pub(in crate::lower) fn is_path(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _)
            if matches!(std_path(self.tcx, adt.did()), "std::path::Path" | "std::path::PathBuf"))
    }

    /// A `Path`, a `PathBuf`, what `display()` shows of one, or an `OsStr` or
    /// `OsString`: its text, a JS string (ADR 0173). Not a string otherwise: a
    /// path's `==` compares its components, `a/b` and `a//b` equal.
    pub(in crate::lower) fn is_path_like(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()),
            "std::path::Path" | "std::path::PathBuf" | "std::path::Display" | "std::ffi::OsStr" | "std::ffi::OsString"
        ))
    }

    /// A path's `Component`, std's enum as any enum is (ADR 0033).
    pub(in crate::lower) fn is_path_component(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::path::Component")
    }

    /// An `OsStr` or an `OsString`, the text of a path's component: a JS
    /// string, equal and ordered by its bytes, as text is by code point.
    pub(in crate::lower) fn is_os_text(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _)
            if matches!(std_path(self.tcx, adt.did()), "std::ffi::OsStr" | "std::ffi::OsString"))
    }

    /// A slice's `chunks_exact(n)` or `rchunks_exact(n)`, whose `remainder()`
    /// is what's left (ADR 0324).
    pub(in crate::lower) fn is_exact_chunks(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()),
            "std::slice::ChunksExact"
                | "std::slice::RChunksExact"
                | "std::slice::ChunksExactMut"
                | "std::slice::RChunksExactMut"
        ))
    }

    /// A `Cow`, `{ TAG, _0 }` (ADR 0033): what it borrowed, or owns.
    pub(in crate::lower) fn is_cow(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::borrow::Cow")
    }

    /// What a `Cow` borrows and what it owns, where they're one JS value,
    /// as std's `ToOwned` makes them (ADR 0319): `T` and `T`, `str` and
    /// `String`, `[T]` and `Vec<T>`, `Path` and `PathBuf`. `None` for another.
    pub(in crate::lower) fn cow_parts(&self, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let ty::Adt(adt, args) = ty.kind() else { return None };
        if !self.is_cow(ty) {
            return None;
        }
        let borrowed = args.types().next()?;
        let owned_variant = adt.variants().iter().nth(1)?;
        let owned = self.field_ty(owned_variant.fields.iter().next()?, args);
        let same = owned == borrowed
            || borrowed.is_str() && self.is_lang_adt(owned, LangItem::String)
            // `Path` and `PathBuf`, `OsStr` and `OsString`: text (ADR 0173).
            || self.is_path_like(borrowed) && self.is_path_like(owned)
            || matches!(borrowed.kind(), ty::Slice(item) if self.is_std_type(owned, StdItem::Vec)
                && matches!(owned.kind(), ty::Adt(_, vec) if vec.type_at(0) == *item));
        same.then_some((borrowed, owned))
    }

    /// `cmp::max`'s name, `"max"`, or `cmp::min`'s, `"min"`: of what isn't a
    /// number, `Ord`'s method of that name's call (ADR 0136).
    /// A `Result`.
    pub(in crate::lower) fn is_result(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Result)
    }

    /// `env::args()`'s, an array of the process's arguments, which knows
    /// its length (ADR 0391).
    pub(in crate::lower) fn is_process_items(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::env::Args")
    }

    /// A path's iterators, `Path::iter()`'s, `components()`'s and
    /// `ancestors()`'s: an array of what they give (ADR 0405).
    pub(in crate::lower) fn is_path_items(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()),
            "std::path::Iter" | "std::path::Components" | "std::path::Ancestors"
        ))
    }

    /// `env::VarError`, what `env::var` gives of a variable it hasn't:
    /// `"NotPresent"`, as an enum's unit variant is (ADR 0391).
    pub(in crate::lower) fn is_var_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::env::VarError")
    }

    /// `path::StripPrefixError`, what `strip_prefix` gives of a path that
    /// doesn't start with what's asked: `undefined`, as it holds nothing
    /// (ADR 0405).
    pub(in crate::lower) fn is_strip_prefix_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::path::StripPrefixError")
    }

    /// A field's type, of an ADT given `args`: a projection in it, `K::Value`,
    /// is the type it stands for, `Option<u32>`, where that's known, as in
    /// rustc's own types of places. Only a type with one is normalized, which
    /// erases its lifetimes, so one without stays the type it was; and only
    /// where the env gives each const parameter in it its type: of a type in
    /// another body's terms, `<Buf<SIZE> as Measured>::Metadata` asked where
    /// `SIZE` isn't, rustc crashes, so it stays as written, as a projection
    /// that can't be normalized does (#1618).
    pub(in crate::lower) fn field_ty(&self, field: &ty::FieldDef, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
        let ty = field.ty(self.tcx, args).skip_normalization();
        if !rustc_middle::ty::TypeVisitableExt::has_aliases(&ty) || !self.consts_typed(ty) {
            return ty;
        }
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    /// Whether the env gives each const parameter `ty` names its type, a
    /// `ConstArgHasType` of it among its bounds, as normalizing asks
    /// (#1618).
    fn consts_typed(&self, ty: Ty<'tcx>) -> bool {
        let bounds = self.typing_env.param_env.caller_bounds();
        ty.walk().all(|part| match part.as_const().map(|c| c.kind()) {
            Some(ty::ConstKind::Param(param)) => bounds.clone().any(|clause| {
                matches!(clause.kind().skip_binder(), ty::ClauseKind::ConstArgHasType(c, _)
                    if matches!(c.kind(), ty::ConstKind::Param(p) if p == param))
            }),
            _ => true,
        })
    }

    /// `TypeId`, which rust-js holds as its type's key, a string (ADR 0331).
    pub(in crate::lower) fn is_type_id(&self, ty: Ty<'tcx>) -> bool {
        self.is_lang_adt(ty, LangItem::TypeId)
    }

    /// `mem::Discriminant<T>`, what `mem::discriminant` gives: its variant's
    /// discriminant, a number (ADR 0393).
    pub(in crate::lower) fn is_discriminant(&self, ty: Ty<'tcx>) -> bool {
        is_discriminant_type(self.tcx, ty)
    }

    /// A `dyn Any`, `+ Send` or not (ADR 0331).
    pub(in crate::lower) fn is_dyn_any(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some_and(|id| self.tcx.is_diagnostic_item(Symbol::intern("Any"), id)))
    }

    /// A `dyn Error`, `+ Send + Sync` or not, whose downcasts ask its
    /// dictionary's `type_id`, as a `dyn Any`'s do (ADR 0331, amended).
    pub(in crate::lower) fn is_dyn_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some_and(|id| is_std_def(self.tcx, id, StdItem::Error)))
    }

    /// A `dyn` std's downcasts are of: a `dyn Any`'s or a `dyn Error`'s.
    pub(in crate::lower) fn downcasts(&self, ty: Ty<'tcx>) -> bool {
        self.is_dyn_any(ty) || self.is_dyn_error(ty)
    }

    /// A `dyn Any + Send`, an unwind's payload's: a `dyn Any` that holds no
    /// `&JsError` of its own, as one isn't `Send` (ADR 0710).
    pub(in crate::lower) fn is_dyn_any_send(&self, ty: Ty<'tcx>) -> bool {
        self.is_dyn_any(ty)
            && matches!(ty.kind(), ty::Dynamic(traits, ..)
                if traits.auto_traits().any(|id| self.tcx.is_diagnostic_item(Symbol::intern("Send"), id)))
    }

    /// A `PeekMut<T>`'s `T`, a guard of a heap's top (ADR 0333).
    pub(in crate::lower) fn peek_mut_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if std_path(self.tcx, adt.did()) == "std::collections::binary_heap::PeekMut" => {
                args.types().next()
            }
            _ => None,
        }
    }

    /// A `MaybeUninit<T>`'s `T`, which it is in JS, or `undefined` (ADR 0332).
    pub(in crate::lower) fn uninit_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if is_std_def(self.tcx, adt.did(), StdItem::MaybeUninit) => args.types().next(),
            _ => None,
        }
    }
}

/// A JS object type's `Deref` to the class it extends, `TypeError`'s to
/// `Error`: never lowered, as a call of it is the object itself (ADR 0024),
/// and its body, a cast of a pointer, means nothing in JS.
pub(crate) fn is_js_object_deref(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
        && [LangItem::Deref, LangItem::DerefMut].into_iter().any(|item| {
            tcx.is_lang_item(
                tcx.impl_trait_ref(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .def_id,
                item,
            )
        })
        && matches!(tcx.type_of(id).instantiate_identity().skip_normalization().kind(),
        ty::Adt(adt, _) if adt.is_struct()
            && adt.non_enum_variant().fields.iter().next().is_some_and(|first| {
                matches!(tcx.type_of(first.did).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(marker, marked) if marker.is_phantom_data()
                        && marked.types().next().is_some_and(|t| match t.kind() {
                            ty::Foreign(_) => true,
                            ty::Adt(object, _) => tcx
                                .get_attrs_by_path(object.did(), &[Symbol::intern("rust_js"), Symbol::intern("js_object")])
                                .next()
                                .is_some(),
                            _ => false,
                        }))
            }))
}

/// Where a `Cell` or a `RefCell` keeps what it holds, its `UnsafeCell`
/// field `value`: std's layout, of which a constant one is made.
pub(crate) fn cell_value_field(adt: ty::AdtDef<'_>) -> Option<usize> {
    adt.non_enum_variant().fields.iter().position(|f| f.name == sym::value)
}

/// An impl of `Deref`'s `Target`, the type it derefs to.
pub(crate) fn deref_target(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    let target = tcx
        .associated_items(imp)
        .filter_by_name_unhygienic(sym::Target)
        .next()?;
    Some(target.def_id)
}

/// A C-variadic function's `VaList`, the arguments it hasn't read yet: an
/// array of them (ADR 0428).
pub(crate) fn is_va_list(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(list, _) if std_path(tcx, list.did()) == "std::ffi::VaList")
}

/// Which of `panic::Location`'s functions `id` is, if it's one (ADR 0427).
pub(crate) fn location_of(tcx: TyCtxt<'_>, id: DefId) -> Option<crate::lower::std_types::location::LocationOp> {
    crate::lower::std_types::location::location_op(std_path(tcx, id))
}

/// `mem::Discriminant<T>`, its variant's discriminant, a number (ADR 0393).
pub(crate) fn is_discriminant_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::mem::Discriminant")
}

/// A type of addressable memory, or with one in it (ADR 0390): a raw
/// pointer, a union, whose fields share their bytes, or std's `NonNull`;
/// in what a crate's own type holds too, but not in std's, whose private
/// fields a `Vec`'s pointer is among.
pub(crate) fn is_memory_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    fn walk<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        ty.walk().filter_map(|arg| arg.as_type()).any(|part| match part.kind() {
            _ if crate::lower::pointers::raw_pointee(tcx, part).is_some() => true,
            ty::Adt(adt, _) if adt.did().is_local() && part != ty => walk(tcx, part, seen),
            ty::Adt(adt, args) if adt.did().is_local() => adt
                .all_fields()
                .any(|field| walk(tcx, field.ty(tcx, args).skip_normalization(), seen)),
            _ => false,
        })
    }
    walk(tcx, ty, &mut Vec::new())
}

/// `std::num::NonZero`, which is its number (ADR 0177). Asked where no
/// `TyCtxt` is passed, by `Num::of`, of rustc's own for the thread.
pub(crate) fn is_non_zero(id: DefId) -> bool {
    ty::tls::with_opt(|tcx| tcx.is_some_and(|tcx| tcx.is_diagnostic_item(Symbol::intern("NonZero"), id)))
}

/// std's `Duration`, which is its nanoseconds (ADR 0188).
pub(crate) fn is_duration(id: DefId) -> bool {
    ty::tls::with_opt(|tcx| tcx.is_some_and(|tcx| tcx.is_diagnostic_item(Symbol::intern("Duration"), id)))
}

/// A `Duration` type (ADR 0188).
pub(crate) fn is_duration_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if is_duration(adt.did()))
}

/// Whether `ty`'s arithmetic is its number's, `a += b` its JS number's
/// `+`, checked as Rust's is: not a `Duration`'s (ADR 0188) or a clock's
/// (ADR 0447), numbers whose `+` std writes, which is compiled.
pub(crate) fn has_number_arithmetic(ty: Ty<'_>) -> bool {
    Num::of(ty).is_some() && !is_duration_ty(ty) && clock_type_of(ty).is_none()
}

/// std's `PoisonError`, a lock's `Err`, which has no diagnostic item.
pub(crate) fn is_poison_error(ty: Ty<'_>) -> bool {
    ty::tls::with_opt(|tcx| {
        tcx.is_some_and(|tcx| {
            matches!(ty.kind(), ty::Adt(adt, _)
                if of_std_crate(tcx, adt.did(), StdCrate::Std) && std_path(tcx, adt.did()) == "std::sync::PoisonError")
        })
    })
}

/// What std's clock has of `id` (ADR 0447): `SystemTime`, `Instant`, or
/// `SystemTimeError`.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ClockType {
    System,
    Instant,
    Error,
}

pub(crate) fn clock_type(id: DefId) -> Option<ClockType> {
    ty::tls::with_opt(|tcx| {
        let tcx = tcx?;
        if tcx.is_diagnostic_item(Symbol::intern("Instant"), id) {
            return Some(ClockType::Instant);
        }
        if !of_std_crate(tcx, id, StdCrate::Std) {
            return None;
        }
        match std_path(tcx, id) {
            "std::time::SystemTime" => Some(ClockType::System),
            "std::time::SystemTimeError" => Some(ClockType::Error),
            _ => None,
        }
    })
}

/// A clock's type, `SystemTime`, `Instant` or `SystemTimeError` (ADR 0447).
pub(crate) fn clock_type_of(ty: Ty<'_>) -> Option<ClockType> {
    match ty.kind() {
        ty::Adt(adt, _) => clock_type(adt.did()),
        _ => None,
    }
}

/// Whether no value of `variant` is ever made: `Component::Prefix`, of a
/// `PrefixComponent`, a Windows path's prefix, which a Unix path, the only
/// one rust-js has, never has (ADR 0405). What shows or orders its enum's
/// values has nothing to do of it.
pub(in crate::lower) fn never_made(tcx: TyCtxt<'_>, variant: &ty::VariantDef) -> bool {
    variant.fields.iter().any(|field| {
        matches!(tcx.type_of(field.did).skip_binder().kind(),
            ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::path::PrefixComponent")
    })
}

/// core's `TryFromSliceError` (ADR 0189).
pub(crate) fn is_try_from_slice_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(tcx, adt.did(), StdCrate::Core)
        && tcx.item_name(adt.did()).as_str() == "TryFromSliceError")
}

/// `String::from_utf16`'s error, of a lone surrogate, the one kind it makes
/// (ADR 0323).
pub(crate) fn is_from_utf16_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(tcx, adt.did(), StdCrate::Alloc)
        && tcx.item_name(adt.did()).as_str() == "FromUtf16Error")
}

/// Whether `ty` is `char::decode_utf16`'s iterator, `DecodeUtf16<I>`:
/// `$decodeUtf16`'s `$Steps`, whose hint is std's of its source's (#1151).
pub(crate) fn is_decode_utf16(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::char::DecodeUtf16")
}

/// Whether `ty` is one of std's iterators whose JS is always a `$Steps`
/// that asks its source's hint, whatever that is, an array's `$iter`
/// among them: `decode_utf16`'s (#1151) and `cycle()`'s, `$cycleOf`'s
/// (#1179); and `skip_while`'s, `take_while`'s, `map_while`'s and
/// `scan`'s, `$lazySkipWhile`'s and the others, `(0, upper)` of its
/// source's, as std's are, `take_while`'s `(0, Some(0))` once it's
/// stopped (#1169).
pub(crate) fn asks_its_sources_hint(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    is_decode_utf16(tcx, ty)
        || matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if matches!(
            std_path(tcx, adt.did()),
            "std::iter::Cycle" | "std::iter::SkipWhile" | "std::iter::TakeWhile" | "std::iter::MapWhile" | "std::iter::Scan"
        ))
}

/// `char::decode_utf16`'s error, its unpaired surrogate's code (ADR 0327).
pub(crate) fn is_decode_utf16_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(tcx, adt.did(), StdCrate::Core)
        && tcx.item_name(adt.did()).as_str() == "DecodeUtf16Error")
}

/// A `char`'s or a `str`'s `escape_default()`, `escape_debug()` or
/// `escape_unicode()`: the array of its `char`s, as a case mapping is (ADRs
/// 0327, 0971).
pub(crate) fn is_text_escape(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(tcx, adt.did(), StdCrate::Core)
        && matches!(tcx.item_name(adt.did()).as_str(), "EscapeDefault" | "EscapeDebug" | "EscapeUnicode")
        && matches!(std_path(tcx, adt.did()), p if p.starts_with("std::char::") || p.starts_with("std::str::")))
}

/// A byte's `escape_ascii()`, `ascii::escape_default(b)`, and a byte slice's
/// `escape_ascii()`: the array of its escape's bytes (ADRs 0558, 0971).
pub(crate) fn is_byte_escape(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if matches!(std_path(tcx, adt.did()), "std::ascii::EscapeDefault" | "std::slice::EscapeAscii"))
}

pub(crate) fn is_case_mapping(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(tcx, adt.did(), StdCrate::Core)
        && matches!(std_path(tcx, adt.did()), "std::char::ToUppercase" | "std::char::ToLowercase"))
}

/// A `NonZero<T>` type (ADR 0177).
/// Std's `Try` of a type std's own generic code is given, as its dictionary
/// is in JS (ADR 0913): a `ControlFlow`'s, whose `Continue` is its output
/// and `Break` its residual; a `NeverShortCircuit`'s, which is its value, a
/// `#[repr(transparent)]` struct (ADR 0584), and never breaks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TryKind {
    ControlFlow,
    NeverShortCircuit,
}

/// `ty`'s `TryKind`, of one std's generic code is given.
pub(crate) fn try_kind(tcx: TyCtxt<'_>, ty: Ty<'_>) -> Option<TryKind> {
    let ty::Adt(adt, _) = ty.kind() else { return None };
    match () {
        _ if is_std_def(tcx, adt.did(), StdItem::ControlFlow) => Some(TryKind::ControlFlow),
        _ if is_std_crate(tcx, adt.did()) && tcx.item_name(adt.did()).as_str() == "NeverShortCircuit" => {
            Some(TryKind::NeverShortCircuit)
        }
        _ => None,
    }
}

/// Whether `id` is std's `Try`, or its supertrait `FromResidual`, whose
/// methods `?` and std's `try_fold` call (ADR 0913).
pub(crate) fn is_try_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_lang_item(id, LangItem::Try)
        || tcx
            .lang_items()
            .get(LangItem::TryTraitFromResidual)
            .and_then(|from_residual| tcx.trait_of_assoc(from_residual))
            == Some(id)
}

/// Whether a `?` of `ty` is written as it is (ADR 0052): an `Option` or a
/// `Result`, whose `Try::branch` the lowering keeps as the value tried and
/// whose `FromResidual::from_residual` it returns, never std's bodies of
/// them; another `Try`'s, std's generic code's, is its dictionary's (ADR
/// 0913).
pub(crate) fn tried_as_written<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if tcx.is_lang_item(adt.did(), LangItem::Option) || is_std_def(tcx, adt.did(), StdItem::Result))
}

/// Whether `id` of `args` is a `?` the lowering writes itself: `Try::branch`
/// of what's tried as written, or `FromResidual::from_residual` of its
/// residual, `FromResidual<R>`'s `R`, the type its call is given.
pub(crate) fn is_question_mark_written<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    let given = match () {
        _ if tcx.is_lang_item(id, LangItem::TryTraitBranch) => args.types().next(),
        _ if tcx.is_lang_item(id, LangItem::TryTraitFromResidual) => args.types().nth(1),
        _ => None,
    };
    given.is_some_and(|ty| tried_as_written(tcx, ty))
}

pub(crate) fn is_non_zero_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if is_non_zero(adt.did()))
}

/// `Any`, std's trait of a type's `TypeId` (ADR 0331).
pub(crate) fn any_trait(tcx: TyCtxt<'_>) -> Option<DefId> {
    tcx.get_diagnostic_item(Symbol::intern("Any"))
}

/// What a reference, a `Box` or an `Rc` points at, and any other type itself.
pub(crate) fn pointer_target<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    match ty.kind() {
        ty::Ref(_, inner, _) => *inner,
        ty::Adt(_, args) if ty.is_box() || rc_pointee(tcx, ty).is_some() => args.type_at(0),
        _ => ty,
    }
}

/// A `dyn Any`, `+ Send` or not, through a reference, a `Box` or an
/// `Rc` (ADR 0331).
pub(crate) fn points_at_dyn_any<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(pointer_target(tcx, ty).kind(), ty::Dynamic(traits, ..) if traits.principal_def_id().is_some() && traits.principal_def_id() == any_trait(tcx))
}

/// Whether `ty` holds a `dyn Any`, `+ Send` or not, among its type's
/// parts: a `Box`'s, a reference's, an `Option`'s, a tuple's; not a
/// struct's field, which its type doesn't name.
pub(crate) fn holds_dyn_any<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.walk().any(|part| {
        part.as_type().is_some_and(|part| {
            matches!(part.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some() && traits.principal_def_id() == any_trait(tcx))
        })
    })
}

/// Is `variant` a `Result`'s `Err`?
pub(crate) fn is_err_variant(tcx: TyCtxt<'_>, variant: DefId) -> bool {
    tcx.is_lang_item(variant, LangItem::ResultErr)
}

/// What a function asks of a `Vec`'s or a slice's length: its `len()`,
/// its `is_empty()`, or that each index it's given is below it, `v[i]` of
/// a `Vec`, `swap(i, j)`, `swap_remove(i)`; or what it does to it, a
/// `Vec`'s `push` one more, its `pop` one fewer where it has one; or that
/// the range it's given ends at most at it, `&v[a..b]` of a `Vec`, a slice
/// or an array, and a `Vec`'s `drain(a..b)`, which takes them out. What
/// knows an index in bounds reads these (ADRs 0292, 0576, 0806, 0881).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum SliceLength {
    Len,
    IsEmpty,
    Index,
    Range,
    Drain,
    Swap,
    SwapRemove,
    /// A `Vec`'s `remove(i)`, of an index below its length.
    Remove,
    /// A `Vec`'s `insert(i, v)`, of an index at most its length.
    Insert,
    Push,
    Pop,
}

pub(in crate::lower) fn slice_length<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<SliceLength> {
    // A `Vec` or a `VecDeque`, an array in JS either way.
    let vec = |ty: Ty<'tcx>| {
        matches!(ty.kind(), ty::Adt(adt, _)
            if tcx.is_diagnostic_item(sym::Vec, adt.did()) || is_std_def(tcx, adt.did(), StdItem::VecDeque))
    };
    if let Some(trait_) = tcx.trait_of_assoc(def_id) {
        if !tcx.is_lang_item(trait_, LangItem::Index) && !tcx.is_lang_item(trait_, LangItem::IndexMut) {
            return None;
        }
        let indexed = args.types().next().map(|ty| ty.peel_refs());
        let index = args.types().nth(1);
        // A range of `usize`s, `a..b`, `..b`, `a..` or `..`.
        let range = index.is_some_and(|i| {
            matches!(i.kind(), ty::Adt(adt, range_args)
                if [LangItem::Range, LangItem::RangeTo, LangItem::RangeFrom, LangItem::RangeFull]
                    .into_iter()
                    .any(|item| tcx.is_lang_item(adt.did(), item))
                    && range_args.types().all(|t| t.is_usize()))
        });
        return match indexed {
            Some(ty) if vec(ty) && index.is_some_and(|i| i.is_usize()) => Some(SliceLength::Index),
            Some(ty) if (vec(ty) || ty.is_slice() || ty.is_array()) && range => Some(SliceLength::Range),
            _ => None,
        };
    }
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    if !vec(owner) && !owner.is_slice() {
        return None;
    }
    let is_vec = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_diagnostic_item(sym::Vec, adt.did()));
    match tcx.item_name(def_id).as_str() {
        "len" => Some(SliceLength::Len),
        "is_empty" => Some(SliceLength::IsEmpty),
        "swap" => Some(SliceLength::Swap),
        "swap_remove" => Some(SliceLength::SwapRemove),
        "push" if vec(owner) => Some(SliceLength::Push),
        // A `Vec`'s, which panics past its length, not a `VecDeque`'s.
        "remove" if is_vec(owner) => Some(SliceLength::Remove),
        "insert" if is_vec(owner) => Some(SliceLength::Insert),
        "drain" if vec(owner) => Some(SliceLength::Drain),
        "pop" if vec(owner) => Some(SliceLength::Pop),
        _ => None,
    }
}

/// What a function asks of a std map's key, a `HashMap`'s or a
/// `BTreeMap`'s: whether the map holds it, `contains_key(k)`, `Has`; or its
/// value, which panics where it doesn't, `m[k]`'s `Index::index`, `Index`.
/// What knows a key held reads these (rd558).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum MapKey {
    Has,
    Index,
}

pub(in crate::lower) fn map_key<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<MapKey> {
    let map = |ty: Ty<'tcx>| {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _)
            if is_std_def(tcx, adt.did(), StdItem::HashMap) || is_std_def(tcx, adt.did(), StdItem::BTreeMap))
    };
    if let Some(trait_) = tcx.trait_of_assoc(def_id) {
        let indexed = args.types().next().is_some_and(map);
        return (tcx.is_lang_item(trait_, LangItem::Index) && indexed).then_some(MapKey::Index);
    }
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    (map(owner) && tcx.item_name(def_id).as_str() == "contains_key").then_some(MapKey::Has)
}

/// What a call of `def_id` gives of an item where it's there: a slice's
/// `get(i)` of a `usize`, a `Vec`'s by its slice, the item at `i`, `Get`;
/// or `Option`'s `copied()` or `cloned()`, `Some` where what it's given
/// is, `Passed` (ADR 0881, amended); or its `filter(p)`, `Some` where what
/// it's given is and `p` holds of it, `Filtered` (rd376).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum PresentItem {
    Get,
    Passed,
    Filtered,
}

/// Whether `def_id` is an `Option`'s or a `Result`'s own method, an
/// inherent one, `as_deref`, `filter`, `map_or_else`, which gives back the
/// option it had (#784).
pub(in crate::lower) fn is_option_or_result_method(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.inherent_impl_of_assoc(def_id).is_some_and(|imp| {
        matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(), ty::Adt(adt, _)
            if [sym::Option, sym::Result].into_iter().any(|name| tcx.is_diagnostic_item(name, adt.did())))
    })
}

pub(in crate::lower) fn present_item<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<PresentItem> {
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    match tcx.item_name(def_id).as_str() {
        "get" if owner.is_slice() && args.types().nth(1).is_some_and(|i| i.is_usize()) => Some(PresentItem::Get),
        "copied" | "cloned" if matches!(owner.kind(), ty::Adt(adt, _) if tcx.is_diagnostic_item(sym::Option, adt.did())) => {
            Some(PresentItem::Passed)
        }
        "filter" if matches!(owner.kind(), ty::Adt(adt, _) if tcx.is_diagnostic_item(sym::Option, adt.did())) => {
            Some(PresentItem::Filtered)
        }
        _ => None,
    }
}

/// Whether `def_id`, of an iterator of type `iter_ty`, counts a string's
/// UTF-16 units: `Iterator::count` of `str::EncodeUtf16`, what
/// `s.encode_utf16().count()` calls, which is its JS `length` (ADR 0632).
/// The one answer, of what it's written as and of what the integer ranges
/// know of it.
pub(in crate::lower) fn counts_utf16_units<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, iter_ty: Ty<'tcx>) -> bool {
    tcx.trait_of_assoc(def_id)
        .is_some_and(|trait_| tcx.is_diagnostic_item(sym::Iterator, trait_))
        && tcx.item_name(def_id).as_str() == "count"
        && is_str_utf16(tcx, iter_ty)
}

/// Whether `def_id`, of an iterator of type `iter_ty`, compares a string's
/// UTF-16 units with another's: `Iterator::cmp` of `str::EncodeUtf16`, what
/// `a.encode_utf16().cmp(b.encode_utf16())` calls, JS's own order of
/// strings, its `sort()`'s (ADR 0945).
pub(in crate::lower) fn compares_utf16_units<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, iter_ty: Ty<'tcx>) -> bool {
    tcx.trait_of_assoc(def_id)
        .is_some_and(|trait_| tcx.is_diagnostic_item(sym::Iterator, trait_))
        && tcx.item_name(def_id).as_str() == "cmp"
        && is_str_utf16(tcx, iter_ty)
}

/// Whether a call of `def_id` that gives a `made` makes a string's
/// `encode_utf16()`, std's, which reads the string and changes nothing.
pub(in crate::lower) fn makes_utf16_units<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, made: Ty<'tcx>) -> bool {
    of_std_crate(tcx, def_id, StdCrate::Core) && is_str_utf16(tcx, made)
}

/// `std::str::EncodeUtf16`, what a string's `encode_utf16()` gives.
fn is_str_utf16<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(tcx, adt.did(), StdCrate::Core)
        && tcx.item_name(adt.did()).as_str() == "EncodeUtf16")
}

/// What a `RefCell`'s call makes of what it holds, as long as it is: a
/// guard on it, `borrow()` or `borrow_mut()`, or a guard's `deref` or
/// `deref_mut`, the value itself (ADR 0881); and an `Rc`'s or an `Arc`'s
/// `deref` of one, std's, which runs nothing, the cell, the same one as long
/// as the `Rc` is: `listeners.borrow()` of a shared cell of a `Vec`.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum CellView {
    Guard { mutable: bool },
    Value,
    Cell,
}

pub(in crate::lower) fn cell_view<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<CellView> {
    let of = |ty: Ty<'tcx>, item| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), item));
    if is_rc_deref(tcx, def_id, args) {
        let cell = (args.types().next()).and_then(|rc| rc_pointee(tcx, rc.peel_refs()));
        let of_cell = cell.is_some_and(|cell| of(cell, StdItem::RefCell));
        return of_cell.then_some(CellView::Cell);
    }
    if let Some(trait_) = tcx.trait_of_assoc(def_id) {
        let deref = tcx.is_lang_item(trait_, LangItem::Deref) || tcx.is_lang_item(trait_, LangItem::DerefMut);
        let guard = args
            .types()
            .next()
            .is_some_and(|ty| of(ty, StdItem::RefCellRef) || of(ty, StdItem::RefCellRefMut));
        return (deref && guard).then_some(CellView::Value);
    }
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    match tcx.item_name(def_id).as_str() {
        "borrow" if of(owner, StdItem::RefCell) => Some(CellView::Guard { mutable: false }),
        "borrow_mut" if of(owner, StdItem::RefCell) => Some(CellView::Guard { mutable: true }),
        _ => None,
    }
}

/// Whether `ty` is a `RefCell`'s guard, `Ref` or `RefMut`, whose drop
/// gives the borrow back and drops nothing of what it's of.
pub(in crate::lower) fn is_cell_guard<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if is_std_def(tcx, adt.did(), StdItem::RefCellRef) || is_std_def(tcx, adt.did(), StdItem::RefCellRefMut))
}

/// What a `Vec`'s `Deref`, `DerefMut` or `IndexMut` makes of it, as long
/// as it is: its slice, `deref` and `deref_mut`, or an item, `index_mut`
/// (ADR 0576).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum VecView {
    Slice,
    Item,
}

pub(in crate::lower) fn vec_view<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<VecView> {
    let trait_ = tcx.trait_of_assoc(def_id)?;
    let of_vec = args.types().next().is_some_and(
        |ty| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if tcx.is_diagnostic_item(sym::Vec, adt.did())),
    );
    match () {
        _ if !of_vec => None,
        _ if tcx.is_lang_item(trait_, LangItem::Deref) || tcx.is_lang_item(trait_, LangItem::DerefMut) => {
            Some(VecView::Slice)
        }
        _ if tcx.is_lang_item(trait_, LangItem::IndexMut) => Some(VecView::Item),
        _ => None,
    }
}

/// A bound of a range, `RangeBounds`', of what the range is.
#[derive(Clone, Copy)]
pub(crate) enum RangeBound {
    /// Its field's value, `Included` or `Excluded` of it: `start` of `2..5`.
    Field(&'static str, &'static str),
    /// None, `Unbounded`: the end of `2..`.
    Unbounded,
    /// A pair of `Bound`s' item: `(Bound::Excluded(1), Bound::Unbounded)`'s.
    Item(usize),
}

/// The start and end bounds of a range of `ty`, a range of std's or a pair
/// of `Bound`s, as its `RangeBounds` gives them; `None` of anything else.
pub(crate) fn range_bounds(tcx: TyCtxt<'_>, ty: Ty<'_>) -> Option<(RangeBound, RangeBound)> {
    use RangeBound::{Field, Item, Unbounded};
    let lang = |item: LangItem| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), item));
    Some(match () {
        _ if lang(LangItem::Range) => (Field("Included", "start"), Field("Excluded", "end")),
        _ if lang(LangItem::RangeFrom) => (Field("Included", "start"), Unbounded),
        _ if lang(LangItem::RangeTo) => (Unbounded, Field("Excluded", "end")),
        _ if lang(LangItem::RangeInclusiveStruct) => (Field("Included", "start"), Field("Included", "end")),
        _ if lang(LangItem::RangeToInclusive) => (Unbounded, Field("Included", "end")),
        _ if lang(LangItem::RangeFull) => (Unbounded, Unbounded),
        _ if matches!(ty.kind(), ty::Tuple(items) if items.len() == 2) => (Item(0), Item(1)),
        _ => return None,
    })
}

/// Whether `def_id` is a std collection a JS array is, `VEC_LIKE`'s: the
/// one list, of what's an array and what a change makes one apart.
pub(in crate::lower) fn is_vec_like_def(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    VEC_LIKE.iter().any(|&item| is_std_def(tcx, def_id, item))
}

/// The std collections a JS array is.
const VEC_LIKE: &[StdItem] = &[
    StdItem::Vec,
    StdItem::VecDeque,
    StdItem::BinaryHeap,
    StdItem::LinkedList,
];

/// The std collections whose `FromIterator` grows a `Vec`, `grows_as_vec`'s.
const GROWS_AS_VEC: &[StdItem] = &[
    StdItem::Vec,
    StdItem::VecDeque,
    StdItem::BinaryHeap,
    StdItem::BTreeSet,
    StdItem::BTreeMap,
];

/// What a closure, an `async` closure or an `async` block's future holds,
/// by type, what it took, and whose it is: the one answer of each place
/// that drops, reads or lays out what one of them took. `None` of another
/// type.
pub(crate) fn held_types<'tcx>(ty: Ty<'tcx>) -> Option<(DefId, &'tcx ty::List<Ty<'tcx>>)> {
    Some(match *ty.kind() {
        ty::Closure(def_id, args) => (def_id, args.as_closure().upvar_tys()),
        ty::CoroutineClosure(def_id, args) => (def_id, args.as_coroutine_closure().upvar_tys()),
        ty::Coroutine(def_id, args) => (def_id, args.as_coroutine().upvar_tys()),
        _ => return None,
    })
}

/// Whether `def_id` is an `async` block's coroutine, `async move { .. }`:
/// what it took is its own, as a closure's is, where an `async fn`'s holds
/// its arguments, which its body drops.
pub(crate) fn is_async_block(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(
        tcx.coroutine_kind(def_id),
        Some(rustc_hir::CoroutineKind::Desugared(
            rustc_hir::CoroutineDesugaring::Async,
            rustc_hir::CoroutineSource::Block
        ))
    )
}

/// `str::repeat`, std's, a string's `repeat(n)`, JS's own (ADR 0774): what
/// `classify` names `Std::Method("repeat")`, and a body's query asks of a
/// call's function.
pub(in crate::lower) fn is_str_repeat(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    is_std_crate(tcx, def_id)
        && tcx.item_name(def_id).as_str() == "repeat"
        && (tcx.inherent_impl_of_assoc(def_id))
            .is_some_and(|imp| tcx.type_of(imp).instantiate_identity().skip_normalization().is_str())
}

/// `Recognition::is_backtrace`, of `tcx` alone.
pub(in crate::lower) fn is_backtrace_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(b, _) if std_path(tcx, b.did()) == "std::backtrace::Backtrace")
}

/// What a `for` loop over a range of integers calls of it, `for i in a..b`'s
/// desugaring: its `into_iter()`, the range itself, and each `next()`,
/// which gives its start and steps it, never past its end. By the lang
/// items the desugaring calls, of a `Range` of an integer type (ADR 1137).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum RangeStep {
    IntoIter,
    Next,
}

pub(in crate::lower) fn range_step<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<RangeStep> {
    let step = if tcx.is_lang_item(def_id, LangItem::IntoIterIntoIter) {
        RangeStep::IntoIter
    } else if tcx.is_lang_item(def_id, LangItem::IteratorNext) {
        RangeStep::Next
    } else {
        return None;
    };
    let of_integers = args.types().next().is_some_and(|ty| {
        matches!(ty.kind(), ty::Adt(adt, item) if tcx.is_lang_item(adt.did(), LangItem::Range)
            && item.types().next().is_some_and(|item| item.is_integral()))
    });
    of_integers.then_some(step)
}
