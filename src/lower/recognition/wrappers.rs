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
            || is_assert_unwind_safe(self.tcx, adt.did())
            // A `NeverShortCircuit`, what std's `fold` folds into through
            // its `try_fold`, what it holds (ADR 0913).
            || try_kind(self.tcx, ty) == Some(TryKind::NeverShortCircuit)
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
    /// Whether std's `AsRef<to>` of a `from` gives the same JS value: text
    /// as a `str`, a sequence as a slice (ADR 0162), what a `Box` or an `Rc`
    /// holds, and text or a path as a `Path`, which is its text (ADRs 0331,
    /// 0505, 0173). The one answer, of a call of `as_ref` and of the
    /// dictionary generic code is given.
    pub(in crate::lower) fn as_ref_is_itself(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let held = from.is_box() || rc_pointee(self.tcx, from.peel_refs()).is_some() || self.is_path_like(to);
        self.views_as_itself(from, to) || held && self.borrows_as_itself(from, to)
    }

    /// What std's `AsRef<to>` of a `from` is, where rust-js knows: the same
    /// JS value, a `Box`'s, an `Rc`'s, or text's as a `Path` (ADRs 0331,
    /// 0505, 0173); or a path's `Component`'s text (ADR 0716).
    pub(in crate::lower) fn as_ref_of(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> Option<Std> {
        if self.as_ref_is_itself(from, to) {
            return Some(Std::Same);
        }
        self.as_ref_is_component_text(from, to)
            .then_some(Std::Path(crate::lower::std_types::path::PathOp::ComponentText))
    }

    /// Whether std's `AsRef<to>` of a `from` is a path's `Component`'s, as a
    /// `Path` or an `OsStr`: its text, `$componentText` (ADR 0716).
    pub(in crate::lower) fn as_ref_is_component_text(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        self.is_path_component(from) && self.is_path_like(to)
    }

    /// Whether std's `AsMut<to>` of a `from` is the `&mut` it's given: text
    /// as a `str`, a sequence as a slice (ADR 0454), and what a `Box` holds,
    /// which the `Box` is in JS (ADR 0023). The one answer, of a call of
    /// `as_mut` and of the dictionary generic code is given.
    pub(in crate::lower) fn as_mut_is_itself(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let boxed = from.peel_refs();
        self.views_as_itself(from, to) || (boxed.is_box() && boxed.boxed_ty() == Some(to))
    }

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
                && self.as_mut_is_itself(ty, args.type_at(1)))
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
/// where it's `Rc::new`'s of it: of the value, or a `Box`'s, which is its
/// value (ADR 0023), a bound function's `Box<dyn Fn(..)>` (ADR 0907,
/// amended), of a `str` its text, a `&str`, a `String` or a `Box<str>`, of
/// a `[T]` a `Vec<T>`'s or a `Box<[T]>`'s items, moved.
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
        _ if from.boxed_ty().is_some_and(|inner| erased(inner) == erased(pointee)) => true,
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

/// Which of a `OnceCell`'s or a `OnceLock`'s own methods `def_id` is (ADR
/// 0317): what lowering calls, and what the body's ranges ask of a `set`,
/// whether its cell is still empty there (ADR 0847).
pub(in crate::lower) fn once_method(tcx: TyCtxt<'_>, def_id: DefId) -> Option<OnceOp> {
    let owner = (tcx.type_of(tcx.inherent_impl_of_assoc(def_id)?))
        .instantiate_identity()
        .skip_normalization();
    if !matches!(owner.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::OnceCell)) {
        return None;
    }
    Some(match tcx.item_name(def_id).as_str() {
        "new" => OnceOp::New,
        "get" | "into_inner" => OnceOp::Get,
        "get_mut" => OnceOp::GetMut,
        "set" => OnceOp::Set,
        "get_or_init" => OnceOp::GetOrInit,
        "take" => OnceOp::Take,
        _ => return None,
    })
}

/// `Rc::new`, `Arc::new` or `Box::new`, std's, which move what they're
/// given into a new allocation and run nothing else (ADR 0847).
pub(in crate::lower) fn is_pointer_new(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.item_name(def_id) == sym::new
        && tcx.inherent_impl_of_assoc(def_id).is_some_and(|imp| {
            let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
            owner.is_box() || rc_pointee(tcx, owner).is_some()
        })
}

/// An `Rc`'s or an `Arc`'s `clone`, std's, which counts one more and runs
/// nothing else (ADR 0847).
pub(in crate::lower) fn is_rc_clone<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    tcx.trait_of_assoc(def_id)
        .is_some_and(|t| tcx.is_lang_item(t, LangItem::Clone))
        && args.types().next().is_some_and(|of| rc_pointee(tcx, of).is_some())
}

/// An `Rc`'s or an `Arc`'s `Deref`, std's, which runs nothing (ADR 0847).
pub(in crate::lower) fn is_rc_deref<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("deref_method"), def_id)
        && args.types().next().is_some_and(|of| rc_pointee(tcx, of).is_some())
}

/// A `String`'s `Deref`, std's, its text as a `&str`, which reads it and
/// runs nothing: `s.len()` of a `String`'s `deref`.
pub(in crate::lower) fn is_string_deref<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("deref_method"), def_id)
        && args.types().next().is_some_and(|of| match of.kind() {
            ty::Adt(adt, _) => tcx.is_lang_item(adt.did(), LangItem::String),
            _ => false,
        })
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
/// `Arc`'s counts, or a `Weak`'s, or upgrades one (ADR 0320): what only a
/// counted one can answer. `downgrade` asks none (ADR 0843). `ptr_eq` of a
/// struct with fields, a JS object whose `===` is its identity, needs none
/// (ADR 0582); not of a transparent one, which is its field, of a number
/// whose `===` is its value (ADR 0582, amended). Of the crate's, or a
/// library's, whose struct is the object its consumers share (ADR 0844);
/// not std's, a `String`'s text, nor a binding's, of markers alone, whose
/// value may be any JS value.
pub(crate) fn reads_rc_count<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, pointee: Option<Ty<'tcx>>) -> bool {
    // A function's identity is the function's where each `Rc` of it is
    // made with it, which the crate's `Rc`s answer (ADR 0959).
    if compares_fn_identity(tcx, id, pointee) {
        return false;
    }
    let object = matches!(pointee.map(|pointee| *pointee.kind()), Some(ty::Adt(adt, _))
        if adt.is_struct()
            && (adt.did().is_local() || !super::std_items::is_std_item(tcx, adt.did()) && holds_data(tcx, adt))
            && !adt.non_enum_variant().fields.is_empty()
            && transparent_field(tcx, adt).is_none())
        // A `RefCell`, its own object, `{ value }`, or the object it holds,
        // which it alone owns (ADR 0863).
        || matches!(pointee.map(|pointee| *pointee.kind()), Some(ty::Adt(adt, _))
            if super::is_std_def(tcx, adt.did(), StdItem::RefCell));
    // An object's identity is the object's: its `ptr_eq` and `as_ptr`
    // (ADR 0562, amended) ask no count.
    if object && matches!(tcx.item_name(id).as_str(), "ptr_eq" | "as_ptr") {
        return false;
    }
    let Some(imp) = tcx.inherent_impl_of_assoc(id) else {
        return false;
    };
    let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
    // A `Weak`'s `new` and `ptr_eq` of an object ask nothing of a count: one
    // to nothing is `null`, one to a value the value (ADR 0843).
    weak_pointee(tcx, owner).is_some() && tcx.item_name(id).as_str() != "new"
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
                    | "new_cyclic"
                    | "ptr_eq"
                    | "as_ptr"
            )
}

/// Is `id`, of an `Rc` of `pointee`, `Rc::ptr_eq` or `Arc::ptr_eq` of a
/// `dyn Fn`'s, which is the JS function itself (ADR 0022)? Its `===` is the
/// two `Rc`s' identity only where each `Rc` that may be one is made of a
/// closure made with it, a JS function made there, which `rc_counts` asks
/// of the crate's (ADR 0959).
pub(crate) fn compares_fn_identity<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, pointee: Option<Ty<'tcx>>) -> bool {
    matches!(pointee.map(|pointee| *pointee.kind()), Some(ty::Dynamic(predicates, ..)) if is_fn_dyn(tcx, predicates))
        && tcx.item_name(id).as_str() == "ptr_eq"
        && is_rc_method(tcx, id)
}

/// Is `id` `Rc::new` or `Arc::new`, an `Rc` of what it's given?
/// One of a `Cell`'s own methods, as what holds a function in it asks:
/// what it writes, and what it reads (#427).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CellMethod {
    /// `Cell::new(value)`.
    New,
    /// `cell.get()`.
    Get,
    /// `cell.set(value)` or `cell.replace(value)`: it writes `value`.
    Writes,
    /// Any other, which may write what isn't its argument, `swap`.
    Other,
}

/// Which of a `Cell`'s own methods `id` is, if it's one.
pub(crate) fn cell_own_method(tcx: TyCtxt<'_>, id: DefId) -> Option<CellMethod> {
    let imp = tcx.inherent_impl_of_assoc(id)?;
    let ty::Adt(adt, _) = tcx.type_of(imp).instantiate_identity().skip_normalization().kind() else {
        return None;
    };
    if !is_std_def(tcx, adt.did(), StdItem::Cell) {
        return None;
    }
    Some(match tcx.item_name(id).as_str() {
        "new" => CellMethod::New,
        "get" => CellMethod::Get,
        "set" | "replace" => CellMethod::Writes,
        _ => CellMethod::Other,
    })
}

pub(crate) fn is_rc_new(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "new" && is_rc_method(tcx, id)
}

/// Is `id` a function of an `Rc`'s or an `Arc`'s own impl?
fn is_rc_method(tcx: TyCtxt<'_>, id: DefId) -> bool {
    (tcx.inherent_impl_of_assoc(id))
        .is_some_and(|imp| rc_pointee(tcx, tcx.type_of(imp).instantiate_identity().skip_normalization()).is_some())
}

/// Is `id` a thread-local's `with_borrow` or `with_borrow_mut`, which runs
/// its closure while its `RefCell` is borrowed (ADR 0328)?
pub(in crate::lower) fn is_local_borrow(tcx: TyCtxt<'_>, id: DefId) -> bool {
    local_key_access(tcx, id) == Some(false) && tcx.item_name(id).as_str() != "get"
}

/// Is `id` a thread-local's `get`, `set`, `take`, `replace`, `with_borrow`
/// or `with_borrow_mut`, which only reads or sets what it holds (ADRs 0270,
/// 0595)? `Some(true)` of `set`, `take` and `replace`, which set it.
pub(in crate::lower) fn local_key_access(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    let imp = tcx.inherent_impl_of_assoc(id)?;
    let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
    if !matches!(owner.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::LocalKey)) {
        return None;
    }
    match tcx.item_name(id).as_str() {
        "set" | "take" | "replace" => Some(true),
        "get" | "with_borrow" | "with_borrow_mut" => Some(false),
        _ => None,
    }
}

/// Does `id`, a thread-local's method, set what it holds or lend its
/// cell: `set`, `take`, `replace`, `with_borrow_mut` or `with` (ADR 0270,
/// amended)?
pub(in crate::lower) fn local_key_sets(tcx: TyCtxt<'_>, id: DefId) -> bool {
    local_key_access(tcx, id) == Some(true)
        || is_local_with(tcx, id)
        || (local_key_access(tcx, id).is_some() && tcx.item_name(id).as_str() == "with_borrow_mut")
}

/// Is `id` a thread-local's `with`, which lends its closure the cell
/// itself (ADR 0037)?
pub(in crate::lower) fn is_local_with(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.inherent_impl_of_assoc(id).is_some_and(|imp| {
        let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
        matches!(owner.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::LocalKey))
    }) && tcx.item_name(id).as_str() == "with"
}

/// The cell `ty` is, as a `Cell`'s or a `RefCell`'s own method has it:
/// of a thread-local's key of one, the cell it holds, whose `take` and
/// `replace` its own are, a `Cell`'s a read and a write, a `RefCell`'s a
/// mutable borrow (ADR 0595, amended); else `ty` itself.
pub(in crate::lower) fn cell_of_key<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    match ty.kind() {
        ty::Adt(key, args) if is_std_def(tcx, key.did(), StdItem::LocalKey) => match args.types().next() {
            Some(held)
                if matches!(held.kind(), ty::Adt(cell, _)
                    if is_std_def(tcx, cell.did(), StdItem::Cell) || is_std_def(tcx, cell.did(), StdItem::RefCell)) =>
            {
                held
            }
            _ => ty,
        },
        _ => ty,
    }
}

/// `std::panic::AssertUnwindSafe`, the value it holds (ADR 0660).
pub(in crate::lower) fn is_assert_unwind_safe(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id) == "std::panic::AssertUnwindSafe"
}

/// Of a `#[repr(transparent)]` struct of the crate's or a dependency's, the
/// index of the field it is in JS (ADR 0584): Rust lays it out as that
/// field, its others of no size, a `PhantomData` or a `()`.
/// Whether a struct has a field that isn't a marker, as a binding's are.
fn holds_data(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> bool {
    adt.non_enum_variant()
        .fields
        .iter()
        .any(|field| holds_value(tcx, field))
}

/// Whether a field holds a value: isn't a marker, `()` or a `PhantomData`.
fn holds_value(tcx: TyCtxt<'_>, field: &ty::FieldDef) -> bool {
    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
    !ty.is_unit() && !matches!(ty.kind(), ty::Adt(held, _) if tcx.is_lang_item(held.did(), LangItem::PhantomData))
}

pub(in crate::lower) fn transparent_field(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> Option<usize> {
    if !adt.is_struct() || !adt.repr().transparent() || super::std_items::is_std_item(tcx, adt.did()) {
        return None;
    }
    adt.non_enum_variant()
        .fields
        .iter()
        .position(|field| holds_value(tcx, field))
}

/// Is `ty` an `Rc`, an `Arc` or a `Box` of a function, a closure or a
/// `dyn Fn`: a function held as a value (`js::Known::Valued`)?
pub(crate) fn holds_a_function<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let held = match ty.kind() {
        ty::Adt(adt, _) if adt.is_box() => ty.boxed_ty(),
        _ => rc_pointee(tcx, ty),
    };
    held.is_some_and(|held| match held.kind() {
        ty::Closure(..) => true,
        ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),
        _ => false,
    })
}

/// What a call of std's does in a closure's handle on itself, an `Rc` of a
/// `RefCell` of the `Weak` its own `Rc` was made of (ADR 0843, amended):
/// each step of making the cell, setting it and reading it back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum HandleStep {
    /// `Rc::new` or `Arc::new`.
    RcNew,
    /// `RefCell::new`.
    CellNew,
    /// An `Rc`'s `Clone::clone`.
    RcClone,
    /// An `Rc`'s `Deref::deref`.
    RcDeref,
    /// `Rc::downgrade`.
    Downgrade,
    /// `RefCell::borrow_mut`.
    BorrowMut,
    /// A `RefMut`'s `DerefMut::deref_mut`.
    GuardDerefMut,
    /// `RefCell::borrow`.
    Borrow,
    /// A `Ref`'s `Deref::deref`.
    GuardDeref,
    /// `Option::as_ref`.
    AsRef,
    /// `Option::and_then`.
    AndThen,
    /// `Option::unwrap`.
    Unwrap,
    /// `Weak::upgrade`, given as a function.
    Upgrade,
}

/// Which `HandleStep` a call of `id`, of `args`, is, if any.
pub(crate) fn handle_step<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<HandleStep> {
    let self_ty = || args.types().next();
    let adt_of = |ty: Option<Ty<'tcx>>| match ty.map(|ty| *ty.kind()) {
        Some(ty::Adt(adt, _)) => Some(adt.did()),
        _ => None,
    };
    if tcx.is_lang_item(id, LangItem::CloneFn) {
        return self_ty()
            .and_then(|ty| rc_pointee(tcx, ty))
            .map(|_| HandleStep::RcClone);
    }
    if tcx.is_diagnostic_item(Symbol::intern("deref_method"), id) {
        let of = self_ty()?;
        if rc_pointee(tcx, of).is_some() {
            return Some(HandleStep::RcDeref);
        }
        return adt_of(Some(of))
            .filter(|&adt| is_std_def(tcx, adt, StdItem::RefCellRef))
            .map(|_| HandleStep::GuardDeref);
    }
    if tcx.is_diagnostic_item(Symbol::intern("deref_mut_method"), id) {
        return adt_of(self_ty())
            .filter(|&adt| is_std_def(tcx, adt, StdItem::RefCellRefMut))
            .map(|_| HandleStep::GuardDerefMut);
    }
    if is_rc_new(tcx, id) {
        return Some(HandleStep::RcNew);
    }
    let imp = tcx.inherent_impl_of_assoc(id)?;
    let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
    let name = tcx.item_name(id);
    match name.as_str() {
        "downgrade" if rc_pointee(tcx, owner).is_some() => Some(HandleStep::Downgrade),
        "upgrade" if weak_pointee(tcx, owner).is_some() => Some(HandleStep::Upgrade),
        "new" | "borrow" | "borrow_mut"
            if adt_of(Some(owner)).is_some_and(|adt| is_std_def(tcx, adt, StdItem::RefCell)) =>
        {
            Some(match name.as_str() {
                "new" => HandleStep::CellNew,
                "borrow" => HandleStep::Borrow,
                _ => HandleStep::BorrowMut,
            })
        }
        "as_ref" | "and_then" | "unwrap"
            if adt_of(Some(owner)).is_some_and(|adt| tcx.is_lang_item(adt, LangItem::Option)) =>
        {
            Some(match name.as_str() {
                "as_ref" => HandleStep::AsRef,
                "and_then" => HandleStep::AndThen,
                _ => HandleStep::Unwrap,
            })
        }
        _ => None,
    }
}
