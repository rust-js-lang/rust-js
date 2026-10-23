//! Rc and Weak, cells and guards, pins and futures, and what a borrow of
//! one is.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// `std::num::Wrapping<T>`'s `T`: a number in a `[x]` (ADR 0175).
    pub(in crate::lower) fn wrapping_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Wrapping) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// A `OnceLock` or `LazyLock`, std's, not a `OnceCell` or `LazyCell`,
    /// `core`'s: their reentrant inits differ (ADRs 0317, 0318).
    pub(in crate::lower) fn is_lock_cell(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::std)
            && (self.is_std_type(ty, StdItem::OnceCell) || self.is_std_type(ty, StdItem::LazyCell))
    }

    /// `Rc::new` or an `Rc`'s `Deref`, each `Std::Same` of an `Rc` that's
    /// its value (ADR 0023): whether it's `new`, and what it points at, for
    /// a counted one's (ADR 0320).
    pub(in crate::lower) fn rc_same(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<(bool, Ty<'tcx>)> {
        let tcx = self.tcx;
        if tcx.is_diagnostic_item(Symbol::intern("deref_method"), def_id) {
            return rc_pointee(tcx, args.type_at(0)).map(|pointee| (false, pointee));
        }
        // `rc.borrow()` and `rc.as_ref()` to what it points at, as `*rc` is
        // (ADRs 0503, 0505).
        if let Some(trait_) = tcx.trait_of_assoc(def_id)
            && (is_std_def(tcx, trait_, StdItem::Borrow) || is_std_def(tcx, trait_, StdItem::AsRef))
            && let Some(pointee) = rc_pointee(tcx, args.type_at(0))
            && args.types().nth(1) == Some(pointee)
        {
            return Some((false, pointee));
        }
        // `Rc::from(x)` and `x.into()`, of what `rc_made_of` says.
        if let Some(trait_) = tcx.trait_of_assoc(def_id)
            && let Some((from, to)) = conversion_sides(tcx, trait_, args)
            && tcx.is_diagnostic_item(sym::From, trait_) | tcx.is_diagnostic_item(sym::Into, trait_)
        {
            return rc_made_of(tcx, from, to).map(|pointee| (true, pointee));
        }
        let imp = tcx.inherent_impl_of_assoc(def_id)?;
        let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
        (rc_pointee(tcx, owner).is_some() && tcx.item_name(def_id).as_str() == "new").then(|| (true, args.type_at(0)))
    }

    /// What a wrapper of std's that's what it holds holds: `ManuallyDrop`'s
    /// and `MaybeDangling`'s, whose `==` and clone are its value's, as it is
    /// in JS (`transparent`).
    pub(in crate::lower) fn std_wrapped(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        (self.is_lang_adt(ty, LangItem::ManuallyDrop) || self.is_lang_adt(ty, LangItem::MaybeDangling))
            .then(|| self.transparent(ty))
            .flatten()
    }

    /// A type that's the value it holds in JS, whose field is it: a
    /// `#[repr(transparent)]` struct of the crate's or a dependency's (ADR
    /// 0584), a `MaybeUninit` (ADR 0332), and what std makes one of,
    /// `ManuallyDrop` and `MaybeDangling`. The type of that field, what it
    /// is in JS.
    pub(in crate::lower) fn transparent(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty::Adt(adt, args) = *ty.kind() else {
            return None;
        };
        // A `#[repr(transparent)]` struct is its field (ADR 0584).
        if let Some(at) = transparent_field(self.tcx, adt) {
            return Some(self.field_ty(&adt.non_enum_variant().fields.raw[at], args));
        }
        let wraps = self.uninit_of(ty).is_some()
            || self.is_lang_adt(ty, LangItem::ManuallyDrop)
            || self.is_lang_adt(ty, LangItem::MaybeDangling)
            // What `pin!` pins, held where its `&mut` is taken (ADR 0329).
            || self.tcx.is_diagnostic_item(rustc_span::sym::PinMacroHelper, adt.did())
            // A `NonZero`, its number (ADR 0177), in std's `NonZeroU8Inner`.
            || is_non_zero(adt.did())
            // An `AssertUnwindSafe`, what it holds, which rustc checks
            // where it's needed (ADR 0660).
            || std_path(self.tcx, adt.did()) == "std::panic::AssertUnwindSafe"
            || std_path(self.tcx, adt.did()).starts_with("std::num::niche_types::");
        // A `MaybeUninit` is a union of it and `()`.
        let mut held = adt.all_fields().map(|field| self.field_ty(field, args));
        wraps.then(|| held.find(|t| !t.is_unit())).flatten()
    }

    /// What `ty` is in JS through each type that's the value it holds:
    /// `dyn Display` of a `ManuallyDrop<dyn Display>`; `ty` if it's none.
    pub(in crate::lower) fn unwrapped(&self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        while let Some(held) = self.transparent(ty) {
            ty = held;
        }
        ty
    }

    /// A pinned reference, a `&mut` to which is the pin, which only what it
    /// points at is changed through (ADR 0329).
    pub(in crate::lower) fn mut_is_the_pin(&self, ty: Ty<'tcx>) -> bool {
        self.pinned(ty).is_some_and(|pointer| pointer.is_ref())
    }

    /// A `Pin<P>`'s `P`, the pointer it is in JS (ADR 0329).
    pub(in crate::lower) fn pinned(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Pin) => args.types().next(),
            _ => None,
        }
    }

    /// A `dyn Future`: a JS promise (ADR 0029).
    pub(in crate::lower) fn is_dyn_future(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Future)))
    }

    /// A future whose `Future` is the crate's own, or a reference, `Box` or
    /// `Pin` of one: an impl rust-js never lowers (ADR 0425), so no JS
    /// promise is a value of it, and what would await one is refused.
    pub(in crate::lower) fn unlowered_future(&self, ty: Ty<'tcx>) -> bool {
        self.own_impl_of(LangItem::FuturePoll, self.future_pointee(ty))
    }

    /// Whether `ty`'s impl of the std trait method `method` is the crate's
    /// own, or a library's rust-js compiled.
    fn own_impl_of(&self, method: LangItem, ty: Ty<'tcx>) -> bool {
        let Some(method) = self.tcx.lang_items().get(method) else {
            return false;
        };
        let args = self.tcx.mk_args(&[ty.into()]);
        matches!(crate::lower::items::resolve_instance_in(self.tcx, self.typing_env, method, args),
            Ok(Some(instance)) if self.tcx.trait_impl_of_assoc(instance.def_id()).is_some_and(|imp| self.is_rust_trait(imp)))
    }

    /// A type whose `IntoFuture` is the crate's own (ADR 0571), awaited as
    /// what its `into_future` makes, not as the promise it isn't.
    pub(in crate::lower) fn own_into_future(&self, ty: Ty<'tcx>) -> bool {
        self.own_impl_of(LangItem::IntoFutureIntoFuture, ty)
    }

    /// A type with an `own_into_future` that calling `def_id` of `args`
    /// asks an `IntoFuture` of, among its bounds, other than `into_future`
    /// itself: generic code, which awaits what it's given as a promise.
    pub(in crate::lower) fn own_into_future_given(
        &self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Option<Ty<'tcx>> {
        if self.tcx.is_lang_item(def_id, LangItem::IntoFutureIntoFuture) {
            return None;
        }
        self.tcx
            .clauses_of(def_id)
            .instantiate(self.tcx, args)
            .into_iter()
            .find_map(|(clause, _)| match clause.skip_normalization().kind().skip_binder() {
                ty::ClauseKind::Trait(bound)
                    if self
                        .tcx
                        .is_diagnostic_item(Symbol::intern("IntoFuture"), bound.def_id())
                        && self.own_into_future(bound.self_ty()) =>
                {
                    Some(bound.self_ty())
                }
                _ => None,
            })
    }

    /// What a future, or a reference, `Box` or `Pin` of one, is of.
    fn future_pointee(&self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        while let Some(inner) = self.pinned(ty).or_else(|| ty.builtin_deref(true)) {
            ty = inner;
        }
        ty
    }

    /// The `unlowered_future` that unsizing `from` to `to` makes a `dyn
    /// Future` of, as `Box::pin(f)` as a `Pin<Box<dyn Future>>` does.
    pub(in crate::lower) fn unsized_unlowered_future(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> Option<Ty<'tcx>> {
        (self.is_dyn_future(self.future_pointee(to)) && self.unlowered_future(from)).then(|| self.future_pointee(from))
    }

    /// The `unlowered_future` that calling `def_id` of `args` asks a
    /// `Future` or an `IntoFuture` of, among its bounds: `.await`'s
    /// `into_future(x)`, or a generic function's `F: Future`.
    pub(in crate::lower) fn awaited_unlowered_future(
        &self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Option<Ty<'tcx>> {
        self.tcx
            .clauses_of(def_id)
            .instantiate(self.tcx, args)
            .into_iter()
            .find_map(|(clause, _)| match clause.skip_normalization().kind().skip_binder() {
                ty::ClauseKind::Trait(bound)
                    if self.tcx.is_lang_item(bound.def_id(), LangItem::Future)
                        || self
                            .tcx
                            .is_diagnostic_item(Symbol::intern("IntoFuture"), bound.def_id()) =>
                {
                    let ty = bound.self_ty();
                    self.unlowered_future(ty).then(|| self.future_pointee(ty))
                }
                _ => None,
            })
    }

    pub(in crate::lower) fn is_std_wrapper(&self, ty: Ty<'tcx>) -> bool {
        ty.is_box()
            || self.pinned(ty).is_some()
            || self.uninit_of(ty).is_some()
            || self.is_lang_adt(ty, LangItem::String)
            // A `Formatter` is its text, a string (ADR 0054): a `&mut` to
            // one a cell, as a `String`'s is (ADR 0461).
            || self.is_formatter_type(ty)
            || self.is_rc(ty)
            || self.is_guard(ty)
            || [
                StdItem::Cell,
                StdItem::RefCell,
                StdItem::Atomic,
                StdItem::Mutex,
                StdItem::RwLock,
            ]
            .into_iter()
            .any(|item| self.is_std_type(ty, item))
            || self.is_std_type(ty, StdItem::OnceCell)
            || self.is_std_type(ty, StdItem::LazyCell)
            || weak_pointee(self.tcx, ty).is_some()
            || self.is_vec_like(ty)
    }

    /// An `Rc`, or an `Arc`, which on one thread is one: the value it
    /// points at, shared (ADR 0023).
    pub(in crate::lower) fn is_rc(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Rc) || self.is_std_type(ty, StdItem::Arc)
    }

    /// Is std's `AsRef<to>` or `AsMut<to>` of a `from`, through as many
    /// references as it's behind, the same JS value (ADR 0162): text as a
    /// `str`, and a `Vec`, an array or a slice as a slice.
    pub(in crate::lower) fn views_as_itself(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let from = from.peel_refs();
        let sequence = from.is_array() || from.is_slice() || self.is_vec_like(from);
        (self.is_string_like(from) && to.is_str()) || (sequence && to.is_slice())
    }

    /// Whether `method` of std's `trait_`, of a `ty` as a `to`, is the value
    /// itself: `x.borrow()` of std's `Borrow`, as its dictionary's is (ADR
    /// 0167); and `v.as_mut()` of std's `AsMut` of a sequence as a slice or
    /// text as a `str`, of a type known here, the value its `&mut` is, as
    /// its dictionary's is of the cell generic code has (ADRs 0454, 0550). A
    /// type parameter's is its dictionary's.
    pub(super) fn views_itself(
        &self,
        trait_: DefId,
        method: DefId,
        ty: Ty<'tcx>,
        args: ty::GenericArgsRef<'tcx>,
    ) -> bool {
        let tcx = self.tcx;
        (is_std_def(tcx, trait_, StdItem::Borrow)
            && tcx.item_name(method).as_str() == "borrow"
            && self.borrows_as_itself(ty, args.type_at(1)))
            || (is_std_def(tcx, trait_, StdItem::AsMut)
                && !known_by_bounds(ty.peel_refs())
                && self.views_as_itself(ty, args.type_at(1)))
    }

    /// Is std's `Borrow<to>` of a `from` the same JS value (ADR 0167): a
    /// value as itself, through a reference, a `Box` or an `Rc` (ADR 0023),
    /// text as a `str`, and a `Vec` or an array as a slice.
    pub(in crate::lower) fn borrows_as_itself(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let pointee = |mut ty: Ty<'tcx>| loop {
            ty = match ty.kind() {
                ty::Ref(_, inner, _) => *inner,
                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) || self.pinned(ty).is_some() => args.type_at(0),
                _ => break ty,
            };
        };
        let (from, to) = (pointee(from), pointee(to));
        from == to
            || (self.is_string_like(from) && to.is_str())
            // A path is its text (ADR 0173).
            || ((self.is_string_like(from) || self.is_path_like(from)) && self.is_path_like(to))
            || ((from.is_array() || self.is_std_type(from, StdItem::Vec)) && to.is_slice())
    }

    /// A type whose `Borrow` is the crate's that a std function takes as
    /// what it borrows as (ADR 0167): a map's `get(q)` of its keys, its
    /// `K: Borrow<Q>`, or `[S]::join` of its items, through `[S]: Join`'s
    /// impl's `S: Borrow<str>`. std's JS compares and joins the value itself.
    pub(in crate::lower) fn borrowed_by_user(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Ty<'tcx>> {
        let borrow = std_item(self.tcx, StdItem::Borrow);
        let bounds = |id: DefId, args: ty::GenericArgsRef<'tcx>| {
            let clauses = self.tcx.clauses_of(id).instantiate(self.tcx, args).clauses;
            clauses
                .into_iter()
                .filter_map(|clause| clause.skip_normalization().as_trait_clause())
                .map(|clause| {
                    self.tcx
                        .erase_and_anonymize_regions(self.tcx.instantiate_bound_regions_with_erased(clause).trait_ref)
                })
                .collect::<Vec<_>>()
        };
        let users = |tr: ty::TraitRef<'tcx>| (tr.def_id == borrow && self.is_user_impl(tr)).then(|| tr.self_ty());
        bounds(def_id, args).into_iter().find_map(|tr| {
            users(tr).or_else(|| {
                // Of std's impl, whose JS is std's: the crate's is given its
                // `Borrow`'s dictionary.
                match self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)) {
                    Ok(ImplSource::UserDefined(imp))
                        if !self.trait_impls.contains(&imp.impl_def_id) && !self.foreign.has_impl(imp.impl_def_id) =>
                    {
                        bounds(imp.impl_def_id, imp.args).into_iter().find_map(users)
                    }
                    _ => None,
                }
            })
        })
    }

    /// A guard of a `RefCell` or a lock: what it guards (ADR 0025).
    pub(in crate::lower) fn is_guard(&self, ty: Ty<'tcx>) -> bool {
        [
            StdItem::RefCellRef,
            StdItem::RefCellRefMut,
            StdItem::MutexGuard,
            StdItem::RwLockReadGuard,
            StdItem::RwLockWriteGuard,
        ]
        .into_iter()
        .any(|item| self.is_std_type(ty, item))
    }

    /// A thread-local `RefCell`'s function, or a guard's own, `Ref::map`
    /// say (ADR 0328).
    pub(in crate::lower) fn asks_borrows(&self, id: DefId) -> bool {
        let Some(imp) = self.tcx.impl_of_assoc(id) else {
            return false;
        };
        let owner = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
        match owner.kind() {
            _ if self.is_guard(owner) => !self.tcx.impl_is_of_trait(imp),
            ty::Adt(_, args) if self.is_std_type(owner, StdItem::LocalKey) => {
                args.types().next().is_some_and(|held| self.is_borrowed_cell(held))
            }
            _ => false,
        }
    }

    /// Whether `T::default()` of `ty` is std's, which runs nothing of the
    /// program's: `Option`'s, a number's, `Vec`'s; not a derive's.
    pub(in crate::lower) fn std_default(&self, ty: Ty<'tcx>) -> bool {
        let Some(default) = (self.tcx.get_diagnostic_item(sym::Default))
            .and_then(|t| self.tcx.associated_item_def_ids(t).first().copied())
        else {
            return false;
        };
        let args = self.tcx.mk_args(&[ty.into()]);
        matches!(
            crate::lower::items::resolve_instance_in(self.tcx, self.typing_env, default, args),
            Ok(Some(instance)) if self.is_std(instance.def_id())
        )
    }

    /// A `RefCell`, a `Mutex` or an `RwLock`: what a guard borrows (ADR 0328).
    pub(in crate::lower) fn is_borrowed_cell(&self, ty: Ty<'tcx>) -> bool {
        [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock]
            .into_iter()
            .any(|item| self.is_std_type(ty, item))
    }
}

/// What an `Rc` or an `Arc`, `to`, a conversion makes of `from` points at,
/// where it's `Rc::new`'s of it: of the value, of a `str` its text, a
/// `&str`, a `String` or a `Box<str>`, of a `[T]` a `Vec<T>`'s or a
/// `Box<[T]>`'s items, moved.
pub(crate) fn rc_made_of<'tcx>(tcx: TyCtxt<'tcx>, from: Ty<'tcx>, to: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let pointee = rc_pointee(tcx, to)?;
    let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
    let text = |ty: Ty<'tcx>| {
        ty.peel_refs().is_str()
            || matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String))
            || ty.boxed_ty().is_some_and(|inner| inner.is_str())
    };
    let items = |item: Ty<'tcx>| match from.kind() {
        ty::Adt(adt, args) if is_std_def(tcx, adt.did(), StdItem::Vec) => erased(args.type_at(0)) == erased(item),
        _ => from
            .boxed_ty()
            .is_some_and(|inner| matches!(inner.kind(), ty::Slice(i) if erased(*i) == erased(item))),
    };
    let made = match pointee.kind() {
        _ if erased(pointee) == erased(from) => true,
        ty::Str => text(from) && !matches!(from.kind(), ty::Ref(_, _, Mutability::Mut)),
        ty::Slice(item) => items(*item),
        _ => false,
    };
    made.then_some(pointee)
}

pub(crate) fn rc_pointee<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if tcx.is_diagnostic_item(sym::Rc, adt.did()) || tcx.is_diagnostic_item(sym::Arc, adt.did()) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// What an `rc::Weak` or `sync::Weak` points at (ADR 0320).
pub(crate) fn weak_pointee<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if ["RcWeak", "ArcWeak"]
                .into_iter()
                .any(|name| tcx.is_diagnostic_item(Symbol::intern(name), adt.did())) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// Is `id`, of an `Rc` of `pointee`, a function that reads an `Rc`'s or
/// `Arc`'s counts, or makes or reads a `Weak` (ADR 0320): what only a
/// counted one can answer. `ptr_eq` of a struct with fields, a JS object
/// whose `===` is its identity, needs none (ADR 0582).
pub(crate) fn reads_rc_count<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, pointee: Option<Ty<'tcx>>) -> bool {
    let object = matches!(pointee.map(|pointee| *pointee.kind()), Some(ty::Adt(adt, _))
        if adt.is_struct() && adt.did().is_local()
            && !adt.non_enum_variant().fields.is_empty());
    if object && tcx.item_name(id).as_str() == "ptr_eq" {
        return false;
    }
    let Some(imp) = tcx.inherent_impl_of_assoc(id) else {
        return false;
    };
    let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
    weak_pointee(tcx, owner).is_some()
        || rc_pointee(tcx, owner).is_some()
            && matches!(
                tcx.item_name(id).as_str(),
                "strong_count"
                    | "weak_count"
                    | "get_mut"
                    | "make_mut"
                    | "try_unwrap"
                    | "into_inner"
                    | "unwrap_or_clone"
                    | "downgrade"
                    | "new_cyclic"
                    | "ptr_eq"
            )
}

/// Is `id` a thread-local's `with_borrow` or `with_borrow_mut`, which runs
/// its closure while its `RefCell` is borrowed (ADR 0328)?
pub(in crate::lower) fn is_local_borrow(tcx: TyCtxt<'_>, id: DefId) -> bool {
    local_key_access(tcx, id) == Some(false) && tcx.item_name(id).as_str() != "get"
}

/// Is `id` a thread-local's `get`, `set`, `with_borrow` or
/// `with_borrow_mut`, which only reads or sets what it holds (ADR 0270)?
/// `Some(true)` of `set`, which sets it.
pub(in crate::lower) fn local_key_access(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    let imp = tcx.inherent_impl_of_assoc(id)?;
    let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
    if !matches!(owner.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::LocalKey)) {
        return None;
    }
    match tcx.item_name(id).as_str() {
        "set" => Some(true),
        "get" | "with_borrow" | "with_borrow_mut" => Some(false),
        _ => None,
    }
}

/// Whether `key`, a `LocalKey`, holds a `Cell`: one whose `take` and
/// `replace` are its cell's, a read and a write, not a borrow (ADR 0595).
pub(in crate::lower) fn local_key_of_cell(tcx: TyCtxt<'_>, key: Ty<'_>) -> bool {
    matches!(key.kind(), ty::Adt(_, args)
        if args.types().next().is_some_and(|held| matches!(held.kind(), ty::Adt(cell, _) if is_std_def(tcx, cell.did(), StdItem::Cell))))
}

/// Of a `#[repr(transparent)]` struct of the crate's or a dependency's, the
/// index of the field it is in JS (ADR 0584): Rust lays it out as that
/// field, its others of no size, a `PhantomData` or a `()`.
pub(in crate::lower) fn transparent_field(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> Option<usize> {
    if !adt.is_struct() || !adt.repr().transparent() || super::std_items::is_std_item(tcx, adt.did()) {
        return None;
    }
    adt.non_enum_variant().fields.iter().position(|field| {
        let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
        !ty.is_unit() && !matches!(ty.kind(), ty::Adt(held, _) if tcx.is_lang_item(held.did(), LangItem::PhantomData))
    })
}
