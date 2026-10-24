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
        match std_path(self.tcx, adt.did()).as_str() {
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

    /// `std::backtrace::Backtrace`, `undefined` or JS's stack (ADR 0568).
    pub(in crate::lower) fn is_backtrace(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(b, _) if std_path(self.tcx, b.did()) == "std::backtrace::Backtrace")
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
        match std_path(self.tcx, adt.did()).as_str() {
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
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::VecDeque) => Hashed::Items(args.type_at(0)),
            ty::Adt(_, args) if self.option_of(ty).is_some() => Hashed::Option(args.type_at(0)),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Result) && !self.is_write_result(ty) => {
                Hashed::Result(args.type_at(0), args.type_at(1))
            }
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::String) => Hashed::Text,
            ty::Adt(..) if self.is_std_type(ty, StdItem::PhantomData) => Hashed::Tuple(Vec::new()),
            _ => return None,
        })
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

    /// `std::str::Chars`, what a string's `chars()` gives.
    pub(in crate::lower) fn is_str_chars(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::core
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
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::core
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
        match std_path(self.tcx, adt.did()).as_str() {
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
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == rustc_span::sym::core
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
        match std_path(self.tcx, adt.did()).as_str() {
            "std::str::Utf8Error" => Some(false),
            "std::string::FromUtf8Error" => Some(true),
            _ => None,
        }
    }

    /// A `Path` or a `PathBuf`, whose `==` and ordering are its components'
    /// (ADR 0401), where an `OsStr`'s are its text's.
    pub(in crate::lower) fn is_path(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _)
            if matches!(std_path(self.tcx, adt.did()).as_str(), "std::path::Path" | "std::path::PathBuf"))
    }

    /// A `Path`, a `PathBuf`, what `display()` shows of one, or an `OsStr` or
    /// `OsString`: its text, a JS string (ADR 0173). Not a string otherwise: a
    /// path's `==` compares its components, `a/b` and `a//b` equal.
    pub(in crate::lower) fn is_path_like(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()).as_str(),
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
            if matches!(std_path(self.tcx, adt.did()).as_str(), "std::ffi::OsStr" | "std::ffi::OsString"))
    }

    /// A slice's `chunks_exact(n)` or `rchunks_exact(n)`, whose `remainder()`
    /// is what's left (ADR 0324).
    pub(in crate::lower) fn is_exact_chunks(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()).as_str(),
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
            std_path(self.tcx, adt.did()).as_str(),
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
    /// erases its lifetimes, so one without stays the type it was.
    pub(in crate::lower) fn field_ty(&self, field: &ty::FieldDef, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
        let ty = field.ty(self.tcx, args).skip_normalization();
        if !rustc_middle::ty::TypeVisitableExt::has_aliases(&ty) {
            return ty;
        }
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
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
    crate::lower::std_types::location::location_op(&std_path(tcx, id))
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
pub(super) fn is_poison_error(ty: Ty<'_>) -> bool {
    ty::tls::with_opt(|tcx| {
        tcx.is_some_and(|tcx| {
            matches!(ty.kind(), ty::Adt(adt, _)
                if tcx.crate_name(adt.did().krate) == sym::std && std_path(tcx, adt.did()) == "std::sync::PoisonError")
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
        if tcx.crate_name(id.krate) != sym::std {
            return None;
        }
        match std_path(tcx, id).as_str() {
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
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && tcx.item_name(adt.did()).as_str() == "TryFromSliceError")
}

/// `String::from_utf16`'s error, of a lone surrogate, the one kind it makes
/// (ADR 0323).
pub(crate) fn is_from_utf16_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::alloc
        && tcx.item_name(adt.did()).as_str() == "FromUtf16Error")
}

/// `char::decode_utf16`'s error, its unpaired surrogate's code (ADR 0327).
pub(crate) fn is_decode_utf16_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && tcx.item_name(adt.did()).as_str() == "DecodeUtf16Error")
}

/// A `char`'s or a `str`'s `escape_default()`, `escape_debug()` or
/// `escape_unicode()`, and a byte slice's `escape_ascii()`, its text (ADR
/// 0327).
pub(crate) fn is_text_escape(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && (matches!(tcx.item_name(adt.did()).as_str(), "EscapeDefault" | "EscapeDebug" | "EscapeUnicode")
            && matches!(std_path(tcx, adt.did()).as_str(), p if p.starts_with("std::char::") || p.starts_with("std::str::"))
            // A byte slice's `escape_ascii()` (ADR 0341).
            || std_path(tcx, adt.did()) == "std::slice::EscapeAscii"))
}

/// A byte's `escape_ascii()`, `ascii::escape_default(b)`: a `$iter` of its
/// escape's bytes (ADR 0558).
pub(crate) fn is_byte_escape(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::ascii::EscapeDefault")
}

pub(crate) fn is_case_mapping(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && matches!(std_path(tcx, adt.did()).as_str(), "std::char::ToUppercase" | "std::char::ToLowercase"))
}

/// A `NonZero<T>` type (ADR 0177).
pub(crate) fn is_non_zero_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if is_non_zero(adt.did()))
}

/// Whether `ty` is a React context's `Provider` or `Consumer`, which
/// `react::provider` gives of a context, and React never changes.
pub(crate) fn is_context_member(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if tcx.crate_name(adt.did().krate).as_str() == "react"
            && matches!(tcx.item_name(adt.did()).as_str(), "Provider" | "Consumer"))
}

/// What `size_of`, `align_of` and `type_name` tell of a type (ADR 0145).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum TypeFact {
    Size,
    Align,
    Name,
    /// Its `TypeId`'s key (ADR 0331).
    Id,
    /// `mem::needs_drop::<T>()`: whether dropping one runs code, as rustc
    /// says (ADR 0145).
    NeedsDrop,
    /// The class of a `HashMap` of it, and of a `HashSet`, keyed as its
    /// type says: `Map`, or `$KeyMap` of one found by value (ADR 0466).
    Map,
    Set,
    /// The class of a `BTreeMap` of it, and of a `BTreeSet`: a hashed one's,
    /// or of one whose `Ord` is its own, `$ordMap(cmp)` (ADR 0404).
    TreeMap,
    TreeSet,
    /// What std's unstable sort picks for its items by their layout, its
    /// small sort or partition (ADRs 0342, 0471).
    Sort,
    /// Whether it's a `FusedIterator`, whose `Fuse` keeps it as it ends
    /// (ADRs 0473, 0543).
    Fused,
}

impl TypeFact {
    /// Every fact: the one list of them.
    pub(crate) const ALL: [TypeFact; 11] = [
        TypeFact::Size,
        TypeFact::Align,
        TypeFact::Name,
        TypeFact::Id,
        TypeFact::NeedsDrop,
        TypeFact::Map,
        TypeFact::Set,
        TypeFact::TreeMap,
        TypeFact::TreeSet,
        TypeFact::Sort,
        TypeFact::Fused,
    ];

    /// Its word in the name of a type parameter's, `TSize`.
    /// The fact a word names, a library's manifest's (ADR 0145).
    pub(crate) fn of_word(word: &str) -> Option<TypeFact> {
        TypeFact::ALL.into_iter().find(|fact| fact.word() == word)
    }

    pub(crate) fn word(self) -> &'static str {
        match self {
            TypeFact::Size => "Size",
            TypeFact::Align => "Align",
            TypeFact::Name => "Name",
            TypeFact::Id => "Id",
            TypeFact::NeedsDrop => "NeedsDrop",
            TypeFact::Map => "Map",
            TypeFact::Set => "Set",
            TypeFact::TreeMap => "TreeMap",
            TypeFact::TreeSet => "TreeSet",
            TypeFact::Sort => "Sort",
            TypeFact::Fused => "Fused",
        }
    }

    /// The fact of a map's class, or a set's, hashed or a B-tree.
    pub(crate) fn of_map(set: bool, sorted: bool) -> TypeFact {
        [[TypeFact::Map, TypeFact::Set], [TypeFact::TreeMap, TypeFact::TreeSet]][usize::from(sorted)][usize::from(set)]
    }

    /// Of a map's class, or a set's: whether a set's, and whether a B-tree's.
    pub(crate) fn map_kind(self) -> Option<(bool, bool)> {
        [(false, false), (true, false), (false, true), (true, true)]
            .into_iter()
            .find(|&(set, sorted)| TypeFact::of_map(set, sorted) == self)
    }
}

/// A map or a set, hashed or a B-tree, of a type parameter's keys, or a
/// std associated type's of one, `ty` itself: its key, and its class's
/// fact, `Map`, `Set`, `TreeMap` or `TreeSet`, which a caller gives
/// (ADRs 0466, 0404).
pub(crate) fn map_of_param<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, TypeFact)> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let is = |item| is_std_def(tcx, adt.did(), item);
    let fact = match () {
        _ if is(StdItem::HashMap) => TypeFact::Map,
        _ if is(StdItem::HashSet) => TypeFact::Set,
        _ if is(StdItem::BTreeMap) => TypeFact::TreeMap,
        _ if is(StdItem::BTreeSet) => TypeFact::TreeSet,
        _ => return None,
    };
    let key = args.types().next()?;
    known_by_bounds(key).then_some((key, fact))
}

/// The layout `def_id` asks of its type argument, its size or alignment, and
/// whether of a value's: `size_of`, `align_of`, `size_of_val` and
/// `align_of_val`. None of any other function.
pub(super) fn layout_query(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(TypeFact, bool)> {
    let diagnostic = |name: &str| tcx.is_diagnostic_item(Symbol::intern(name), def_id);
    Some(match () {
        _ if diagnostic("mem_size_of") => (TypeFact::Size, false),
        _ if diagnostic("mem_size_of_val") => (TypeFact::Size, true),
        _ if diagnostic("mem_align_of") => (TypeFact::Align, false),
        // It has no diagnostic name.
        _ if std_path(tcx, def_id) == "std::mem::align_of_val" => (TypeFact::Align, true),
        _ => return None,
    })
}

/// The facts `def_id` asks of its type argument: `size_of` and
/// `size_of_val` its size, `align_of` its alignment, `Layout::new` both,
/// `type_name` and `type_name_of_val` its name. None of any other function.
pub(crate) fn type_facts(tcx: TyCtxt<'_>, def_id: DefId) -> &'static [TypeFact] {
    if let Some((fact, _)) = layout_query(tcx, def_id) {
        return match fact {
            TypeFact::Align => &[TypeFact::Align],
            _ => &[TypeFact::Size],
        };
    }
    match std_path(tcx, def_id).as_str() {
        "std::any::type_name" | "std::any::type_name_of_val" => return &[TypeFact::Name],
        "std::any::TypeId::of" => return &[TypeFact::Id],
        "std::mem::needs_drop" => return &[TypeFact::NeedsDrop],
        // Its size and its alignment (ADR 0417).
        "std::alloc::Layout::new" => return &[TypeFact::Size, TypeFact::Align],
        // An address asks the pointee's size, an item's stride; a dangling
        // pointer its alignment (ADR 0562).
        "std::ptr::with_exposed_provenance"
        | "std::ptr::with_exposed_provenance_mut"
        | "std::ptr::without_provenance"
        | "std::ptr::without_provenance_mut" => return &[TypeFact::Size],
        "std::ptr::dangling" | "std::ptr::dangling_mut" | "std::ptr::NonNull::<T>::dangling" => {
            return &[TypeFact::Size, TypeFact::Align];
        }
        _ => {}
    }
    if let Some(facts) = crate::lower::pointers::address_facts(tcx, def_id) {
        return facts;
    }
    // `x.type_id()`, and a downcast's type, its last type argument (ADR 0331).
    let any = tcx.get_diagnostic_item(Symbol::intern("Any"));
    let name = tcx.item_name(def_id);
    if tcx.trait_of_assoc(def_id) == any && name.as_str() == "type_id" {
        return &[TypeFact::Id];
    }
    let Some(owner) = tcx.inherent_impl_of_assoc(def_id) else {
        return &[];
    };
    let owner = tcx.type_of(owner).instantiate_identity().skip_normalization();
    let of_any = owner.walk().any(|part| {
        part.as_type().is_some_and(|t| {
            matches!(t.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some() && traits.principal_def_id() == any)
        })
    });
    match of_any && matches!(name.as_str(), "is" | "downcast_ref" | "downcast_mut" | "downcast") {
        true => &[TypeFact::Id],
        false => &[],
    }
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

/// Is `variant` a `Result`'s `Err`?
pub(crate) fn is_err_variant(tcx: TyCtxt<'_>, variant: DefId) -> bool {
    tcx.is_lang_item(variant, LangItem::ResultErr)
}

/// What a function asks of a `Vec`'s or a slice's length: its `len()`,
/// its `is_empty()`, or that each index it's given is below it, `v[i]` of
/// a `Vec`, `swap(i, j)`, `swap_remove(i)`; or what it does to it, a
/// `Vec`'s `push` one more, its `pop` one fewer where it has one. What
/// knows an index in bounds reads these (ADRs 0292, 0576).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum SliceLength {
    Len,
    IsEmpty,
    Index,
    Swap,
    SwapRemove,
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
        let indexed = args.types().next().is_some_and(|ty| vec(ty.peel_refs()));
        return ((tcx.is_lang_item(trait_, LangItem::Index) || tcx.is_lang_item(trait_, LangItem::IndexMut))
            && indexed
            && args.types().nth(1).is_some_and(|i| i.is_usize()))
        .then_some(SliceLength::Index);
    }
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    if !vec(owner) && !owner.is_slice() {
        return None;
    }
    match tcx.item_name(def_id).as_str() {
        "len" => Some(SliceLength::Len),
        "is_empty" => Some(SliceLength::IsEmpty),
        "swap" => Some(SliceLength::Swap),
        "swap_remove" => Some(SliceLength::SwapRemove),
        "push" if vec(owner) => Some(SliceLength::Push),
        "pop" if vec(owner) => Some(SliceLength::Pop),
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

/// Whether a call of `def_id` that gives a `made` makes a string's
/// `encode_utf16()`, std's, which reads the string and changes nothing.
pub(in crate::lower) fn makes_utf16_units<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, made: Ty<'tcx>) -> bool {
    tcx.crate_name(def_id.krate) == sym::core && is_str_utf16(tcx, made)
}

/// `std::str::EncodeUtf16`, what a string's `encode_utf16()` gives.
fn is_str_utf16<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && tcx.item_name(adt.did()).as_str() == "EncodeUtf16")
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

/// Whether `did` is `js::StructuredClone`, what a structured clone copies
/// as it is, whose `()` is `undefined` (ADR 0225).
pub(in crate::lower) fn is_structured_clone(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js" && tcx.item_name(did).as_str() == "StructuredClone"
}

/// Whether `did` is `js::SymbolFor`, the registry's symbol of a key, its JS
/// `Symbol.for(key)` (ADR 0634): the builtins crate's. The one answer, which
/// a constant's value asks too.
pub(in crate::lower) fn is_symbol_for_def(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js" && tcx.item_name(did).as_str() == "SymbolFor"
}

/// Whether `did` is `js::SymbolFor::from_value`, a value of any shape read
/// as the registry's symbol (ADR 0634).
pub(in crate::lower) fn is_symbol_for_read(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && tcx.item_name(did).as_str() == "from_value"
        && tcx.opt_parent(did).is_some_and(|imp| {
            tcx.def_kind(imp) == DefKind::Impl { of_trait: false }
                && matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(adt, _) if is_symbol_for_def(tcx, adt.did()))
        })
}

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
