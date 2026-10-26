//! Traits, impls and bounds: whose impl a call reaches, what a bound says,
//! and the operators and conversions a trait is.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Can no two `ty` items that `Ord` calls equal be told apart? Of
    /// numbers, `bool`s, `char`s and text, and tuples and arrays of them; not
    /// through a `&mut`, a cell, an object (ADR 0099).
    pub(super) fn ties_unseen(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::Tuple(items) => items.iter().all(|item| self.ties_unseen(item)),
            ty::Array(item, _) => self.ties_unseen(*item),
            _ => {
                !ty.walk()
                    .any(|part| matches!(part.as_type().map(|p| *p.kind()), Some(ty::Ref(_, _, Mutability::Mut))))
                    && (self.is_string_like(ty) || Num::of(ty).is_some() || ty.is_bool() || ty.is_char())
            }
        }
    }

    pub(in crate::lower) fn args_of(&self, trait_id: DefId, ty: Ty<'tcx>) -> ty::GenericArgsRef<'tcx> {
        let ty = self.tcx.erase_and_anonymize_regions(ty);
        self.tcx.mk_args_from_iter(std::iter::repeat_n(
            ty::GenericArg::from(ty),
            self.tcx.generics_of(trait_id).count(),
        ))
    }

    pub(in crate::lower) fn has_user_impl(&self, trait_id: DefId, ty: Ty<'tcx>) -> bool {
        self.is_user_impl(ty::TraitRef::new_from_args(
            self.tcx,
            trait_id,
            self.args_of(trait_id, ty),
        ))
    }

    pub(in crate::lower) fn is_user_impl(&self, tr: ty::TraitRef<'tcx>) -> bool {
        self.user_impl(tr).is_some()
    }

    /// The impl of the crate's, or a dependency's, that `tr` selects.
    pub(in crate::lower) fn user_impl(&self, tr: ty::TraitRef<'tcx>) -> Option<DefId> {
        let tr = self.tcx.erase_and_anonymize_regions(tr);
        match self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)) {
            Ok(ImplSource::UserDefined(imp))
                if self.trait_impls.contains(&imp.impl_def_id) || self.foreign.has_impl(imp.impl_def_id) =>
            {
                Some(imp.impl_def_id)
            }
            _ => None,
        }
    }

    /// A type only a caller knows: a type parameter, or an associated type of
    /// one, `<S as Source>::Item`, that isn't a type here (ADR 0106). Its
    /// dictionaries are the ones a function is given. One that is a type here,
    /// `<Count as Source>::Item` in an impl's signature, is that type.
    pub(in crate::lower) fn is_unknown(&self, ty: Ty<'tcx>) -> bool {
        self.is_unknown_in(ty, self.typing_env)
    }

    /// `is_unknown`, of a type of another function's, whose parameters are
    /// its own: asked in its `typing_env`, as the caller's would find its
    /// parameters at other indexes, arrayvec's `CAP`.
    pub(in crate::lower) fn is_unknown_in(&self, ty: Ty<'tcx>, typing_env: ty::TypingEnv<'tcx>) -> bool {
        match ty.kind() {
            ty::Param(_) => true,
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { .. },
                    ..
                },
            ) => self
                .tcx
                .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
                .map_or(true, |known| {
                    matches!(
                        known.kind(),
                        ty::Param(_)
                            | ty::Alias(
                                _,
                                ty::AliasTy {
                                    kind: ty::Projection { .. },
                                    ..
                                }
                            )
                    )
                }),
            _ => false,
        }
    }

    /// A trait rust-js compiled: the crate's own, or a library's (ADR 0100).
    /// Any other is std's, whose dictionaries rust-js makes as it knows them.
    pub(in crate::lower) fn is_rust_trait(&self, id: DefId) -> bool {
        id.is_local() || self.foreign.in_library(id)
    }

    /// A trait whose `dyn` is a pair, its value and its dictionary (ADR
    /// 0049): one rust-js compiled, or std's `Display`, `ToString`, `Error`,
    /// `io::Write`, `fmt::Write` or `Any` (ADRs 0141, 0331, 0446).
    pub(in crate::lower) fn is_pair_trait(&self, id: DefId) -> bool {
        self.is_rust_trait(id) || self.is_std_pair_trait(id)
    }

    /// Whether a dictionary of `id`'s holds its supertraits': a pair trait's,
    /// as a `dyn` calls them, `DerefMut`'s, its `Deref` (ADR 0394), and
    /// `BorrowMut`'s, its `Borrow` (ADR 0483).
    pub(in crate::lower) fn holds_supertraits(&self, id: DefId) -> bool {
        self.is_pair_trait(id)
            || self.tcx.is_lang_item(id, LangItem::DerefMut)
            || is_std_def(self.tcx, id, StdItem::BorrowMut)
            // `Try`'s its `FromResidual` (ADR 0913).
            || self.tcx.is_lang_item(id, LangItem::Try)
    }

    /// Of those, std's.
    pub(in crate::lower) fn is_std_pair_trait(&self, id: DefId) -> bool {
        [
            StdItem::Display,
            StdItem::ToString,
            StdItem::Error,
            StdItem::IoWrite,
            StdItem::FmtWrite,
        ]
        .into_iter()
        .any(|item| is_std_def(self.tcx, id, item))
            || any_trait(self.tcx) == Some(id)
            // A `dyn Hasher`, its dictionary a hasher's (ADR 0463).
            || is_hasher(self.tcx, id)
    }

    /// A `dyn` of `predicates` that's a pair: of a pair trait, or of no
    /// trait but auto traits, `dyn Send`, whose dictionary is its drop alone.
    /// Each keeps its value's drop, its dictionary's `$drop`, as Rust's
    /// vtable does (ADR 0098).
    pub(in crate::lower) fn is_pair_dyn(&self, predicates: &ty::List<ty::PolyExistentialPredicate<'tcx>>) -> bool {
        predicates.principal_def_id().is_none_or(|id| self.is_pair_trait(id))
    }

    pub(in crate::lower) fn reveal(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        if !rustc_middle::ty::TypeVisitableExt::has_opaque_types(&ty) {
            return ty;
        }
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    /// The `Clone` impl of the crate's that a `ty`'s is, if it has a
    /// `clone_from` of its own, which `clone_from_slice` calls (ADR 0324).
    pub(in crate::lower) fn own_clone_from(&self, ty: Ty<'tcx>) -> bool {
        let clone = self.tcx.require_lang_item(LangItem::Clone, rustc_span::DUMMY_SP);
        let tr = ty::TraitRef::new(self.tcx, clone, [self.tcx.erase_and_anonymize_regions(ty)]);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if !self.is_std(imp.impl_def_id) && self.has_clone_from(imp.impl_def_id))
    }

    /// Whether a `clone_from` of the crate's may run in a `clone_from` of a
    /// `ty` but its own: one of a type inside it, or of what a type parameter
    /// stands for, where any `Clone` impl of the crate's has one.
    pub(in crate::lower) fn reaches_clone_from(&self, ty: Ty<'tcx>) -> bool {
        let clone = self.tcx.require_lang_item(LangItem::Clone, rustc_span::DUMMY_SP);
        let any = self
            .trait_impls
            .iter()
            .any(|&imp| self.tcx.impl_opt_trait_id(imp) == Some(clone) && self.has_clone_from(imp));
        any && ty.walk().skip(1).any(|part| {
            part.as_type()
                .is_some_and(|t| known_by_bounds(t) || matches!(t.kind(), ty::Dynamic(..)) || self.own_clone_from(t))
        }) || any && (known_by_bounds(ty) || matches!(ty.kind(), ty::Dynamic(..)))
    }

    /// Does the impl `imp` write its own `clone_from`?
    pub(in crate::lower) fn has_clone_from(&self, imp: DefId) -> bool {
        self.tcx
            .associated_items(imp)
            .in_definition_order()
            .any(|item| item.name().as_str() == "clone_from")
    }

    pub(in crate::lower) fn is_derived_impl(&self, trait_id: DefId, ty: Ty<'tcx>) -> bool {
        let tr = ty::TraitRef::new_from_args(self.tcx, trait_id, self.args_of(trait_id, ty));
        let tr = self.tcx.erase_and_anonymize_regions(tr);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if self.trait_impls.contains(&imp.impl_def_id)
                && known_derive(self.tcx, imp.impl_def_id))
    }

    /// Is `ty`'s impl of `trait_id` a `#[derive]`d one, whether or not
    /// rust-js lowers it: a derived `PartialEq` it never does.
    pub(in crate::lower) fn derives(&self, trait_id: DefId, ty: Ty<'tcx>) -> bool {
        let tr = ty::TraitRef::new_from_args(self.tcx, trait_id, self.args_of(trait_id, ty));
        let tr = self.tcx.erase_and_anonymize_regions(tr);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if known_derive(self.tcx, imp.impl_def_id))
    }
}

/// Whether `ty` is known here only by its bounds: a type parameter, or an
/// associated type of one, `T::Buffer`, whose dictionaries are given (ADR
/// 0533). The one place that answers it.
pub(crate) fn known_by_bounds(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Param(_) | ty::Alias(..))
}

/// How an explicitly fallible JavaScript binding delivers its result.
pub(in crate::lower) enum Catching {
    Direct,
    Result,
    PromiseResult,
}

#[derive(Clone, Copy)]
pub(in crate::lower) enum OrderingCall {
    Compare,
    Lt,
    Le,
    Gt,
    Ge,
    Max,
    Min,
    /// `x.clamp(min, max)`, of `Ord`.
    Clamp,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// How a binding says it may throw: by the result it's declared with, a
    /// `Result` or a `Promise<Result<..>>`, or what its trait says one is,
    /// `T::Stringified`; not by a type parameter a caller gives one:
    /// `js::promise::<Result<..>>` of an `async fn` is its promise (ADR 0588).
    pub(in crate::lower) fn catching(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Catching {
        let sig = self.tcx.fn_sig(id);
        let declared = sig.instantiate_identity().skip_normalization().skip_binder().output();
        let given = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Param(_));
        if given(declared)
            || matches!(declared.kind(), ty::Adt(_, inner) if self.is_js_object(declared)
                && inner.types().next().is_some_and(given))
        {
            return Catching::Direct;
        }
        let output = sig
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder()
            .output();
        let output = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
            .unwrap_or(output);
        if self.is_std_type(output, StdItem::Result) {
            Catching::Result
        } else if matches!(output.kind(), ty::Adt(adt, args) if self.is_js_object(output)
            && self.tcx.item_name(adt.did()).as_str() == "Promise"
            && args.types().next().is_some_and(|t| self.is_std_type(t, StdItem::Result)))
        {
            Catching::PromiseResult
        } else {
            Catching::Direct
        }
    }

    pub(in crate::lower) fn ordering_call(&self, id: DefId, tr: ty::TraitRef<'tcx>) -> Option<(OrderingCall, bool)> {
        let partial = self.tcx.is_lang_item(tr.def_id, LangItem::PartialOrd);
        if !partial && !self.tcx.is_diagnostic_item(sym::Ord, tr.def_id) {
            return None;
        }
        let name = self.tcx.item_name(id);
        // A number's `max` and `cmp` are its own methods'; its `<` is JS's, as
        // a copied default's `self < Self::zero()` of an `f64` calls it.
        if Num::of(tr.self_ty().peel_refs()).is_some()
            && !matches!(name.as_str(), "partial_cmp" | "lt" | "le" | "gt" | "ge")
        {
            return None;
        }
        let call = match name.as_str() {
            "lt" => OrderingCall::Lt,
            "le" => OrderingCall::Le,
            "gt" => OrderingCall::Gt,
            "ge" => OrderingCall::Ge,
            "cmp" | "partial_cmp" => OrderingCall::Compare,
            "max" => OrderingCall::Max,
            "min" => OrderingCall::Min,
            "clamp" if !partial => OrderingCall::Clamp,
            _ => return None,
        };
        Some((call, partial))
    }
}

/// Is `id` a trait whose impls are passed as dictionaries (ADR 0049)? One of
/// the crate's own, or a library's (ADR 0100), which its consumers must pass
/// as it takes them, and the std traits rust-js calls.
pub(in crate::lower) fn operational(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    id: DefId,
) -> bool {
    id.is_local()
        || foreign.in_library(id)
        || tcx.is_lang_item(id, LangItem::Copy)
        || tcx.is_lang_item(id, LangItem::Clone)
        || tcx.is_lang_item(id, LangItem::PartialEq)
        || tcx.is_lang_item(id, LangItem::PartialOrd)
        || tcx.is_diagnostic_item(sym::Ord, id)
        || tcx.is_diagnostic_item(Symbol::intern("Display"), id)
        || tcx.is_diagnostic_item(Symbol::intern("Debug"), id)
        || tcx.is_diagnostic_item(Symbol::intern("Default"), id)
        // Std's of a `Display`: `{ to_string }`, what it shows.
        || tcx.is_diagnostic_item(Symbol::intern("ToString"), id)
        // A `dyn Error`'s (ADR 0141), its `Display` and `Debug` its supertraits'.
        || tcx.is_diagnostic_item(Symbol::intern("Error"), id)
        // A `dyn Any`'s, its type's key (ADR 0331).
        || tcx.is_diagnostic_item(Symbol::intern("Any"), id)
        // Its evidence is the writer or reader itself (ADR 0081).
        || serde_trait(tcx, id).is_some()
        // `R: Try` of std's generic code, `try_fold`'s (ADR 0913).
        || is_try_trait(tcx, id)
        // `parse` of a `T: FromStr`: its dictionary's `from_str` (ADR 0161).
        || is_from_str(tcx, id)
        // `s.as_ref()` of an `S: AsRef<str>`: its dictionary's `as_ref`, and
        // `s.as_mut()` of an `S: AsMut<str>` its `as_mut` (ADR 0162).
        || is_std_def(tcx, id, StdItem::AsRef)
        || is_std_def(tcx, id, StdItem::AsMut)
        // `key.borrow()` of a `K: Borrow<Q>`, as a map's lookup is: its
        // dictionary's `borrow` (ADR 0167); and `v.borrow_mut()` of a `V:
        // BorrowMut<Q>`, its `borrow_mut`, a supertrait's too (ADR 0483).
        || is_std_def(tcx, id, StdItem::Borrow)
        || is_std_def(tcx, id, StdItem::BorrowMut)
        // `{:x}`, `{:e}` and `{:p}` of the crate's types (ADR 0165), and of a
        // `T: LowerHex` or `T: Pointer` (ADR 0174).
        || is_other_fmt_trait(tcx, id)
        // `a + b` of a `T: Add`, and `x.into()` of a `T: Into<U>` (ADR 0108).
        || value_operator(tcx, id).is_some()
        || tcx.is_diagnostic_item(sym::Into, id)
        // `W::from(u)` of a `W: From<U>`: its dictionary's `from` (ADR 0477).
        || tcx.is_diagnostic_item(sym::From, id)
        // A writer of the crate's own, given its text a `write!` at a time
        // (ADR 0166), and generic code that writes to any writer, a `W:
        // fmt::Write`: its dictionary's `write_str`, `write_char` and
        // `write_fmt` (ADR 0180).
        || tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), id)
        // Generic code, or a `dyn`, that writes bytes to any writer, a `W:
        // io::Write`: its dictionary's `write`, `write_all`, `write_fmt` and
        // `flush` (ADR 0392).
        || is_std_def(tcx, id, StdItem::IoWrite)
        // `range.start_bound()` of an `R: RangeBounds<usize>`, arrayvec's
        // `drain`: its dictionary's, a range's bounds as `Bound`s.
        || is_std_def(tcx, id, StdItem::RangeBounds)
        // `v[i]` of an `I: SliceIndex<[T]>`: its dictionary's (ADR 0418).
        || is_std_def(tcx, id, StdItem::SliceIndex)
        // `&*p` of a `P: Deref`: its dictionary's `deref` (ADR 0394), and
        // `&mut *p` of a `P: DerefMut` its `deref_mut`.
        || tcx.is_lang_item(id, LangItem::Deref)
        || tcx.is_lang_item(id, LangItem::DerefMut)
        // `a += b` of a `T: AddAssign`: its dictionary's `add_assign` (ADR 0396).
        || assign_operator(tcx, id).is_some()
        // `c.extend(items)` of a `C: Extend<A>`, and `collect` into a `B:
        // FromIterator<A>`: its dictionary's `extend` or `from_iter` (ADR 0445).
        || is_extend(tcx, id)
        || tcx.is_diagnostic_item(sym::FromIterator, id)
        // `for x in c` of a `for<'a> &'a mut C: IntoIterator`: its
        // dictionary's `into_iter` (ADR 0457).
        || tcx.is_diagnostic_item(sym::IntoIterator, id)
        // `value.hash(state)` of a `T: Hash`, and a hasher's writes of an `H:
        // Hasher`: their dictionaries' `hash`, and `write`, `write_u32`, ...
        // and `finish` (ADR 0463).
        || is_hashing(tcx, id)
        // `items.sum()` into an `S: Sum<A>`, and `product()` into a `P:
        // Product<A>`: its dictionary's `sum` or `product` (ADR 0476), the
        // crate's collection's too (ADR 0160).
        || is_sum_or_product(tcx, id)
}

/// `Hash` or `Hasher`: hashing's traits, whose impls a map never calls
/// (ADR 0463).
pub(crate) fn is_hashing(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_def(tcx, id, StdItem::Hash) || is_hasher(tcx, id)
}

/// `std::hash::Hasher`, which has no diagnostic item (ADR 0463).
pub(crate) fn is_hasher(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id) == "std::hash::Hasher"
}

/// An operator that makes a value, `a + b` or `-a`: what a number's is, as
/// the operator of a MIR binary or unary operation.
pub(in crate::lower) fn value_operator(tcx: TyCtxt<'_>, id: DefId) -> Option<Result<BinOp, UnOp>> {
    [
        (LangItem::Add, Ok(BinOp::Add)),
        (LangItem::Sub, Ok(BinOp::Sub)),
        (LangItem::Mul, Ok(BinOp::Mul)),
        (LangItem::Div, Ok(BinOp::Div)),
        (LangItem::Rem, Ok(BinOp::Rem)),
        (LangItem::BitAnd, Ok(BinOp::BitAnd)),
        (LangItem::BitOr, Ok(BinOp::BitOr)),
        (LangItem::BitXor, Ok(BinOp::BitXor)),
        (LangItem::Shl, Ok(BinOp::Shl)),
        (LangItem::Shr, Ok(BinOp::Shr)),
        (LangItem::Neg, Err(UnOp::Neg)),
        (LangItem::Not, Err(UnOp::Not)),
    ]
    .into_iter()
    .find_map(|(item, op)| tcx.is_lang_item(id, item).then_some(op))
}

pub(in crate::lower) fn implementable(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    id: DefId,
) -> bool {
    operational(tcx, foreign, id)
        // A hand-written `Deserialize`'s visitor, its methods the reader's
        // visitor's (ADR 0459).
        || serde_visitor(tcx, id)
        || tcx.is_diagnostic_item(sym::TryFrom, id)
        // What `.await` calls first, `into_future` (ADR 0571).
        || tcx.is_diagnostic_item(Symbol::intern("IntoFuture"), id)
        // Never lowered (ADRs 0168, 0425).
        || unlowered_trait(tcx, id)
        // An iterator of the crate's from both ends, and of a known length:
        // what `rev()`, `next_back()` and `len()` call (ADR 0164).
        || is_iterator_extension(tcx, id)
        || tcx.is_diagnostic_item(sym::Eq, id)
        || tcx.is_diagnostic_item(sym::Iterator, id)
        || is_operator(tcx, id)
        // A trait of no items: its impl says what the type may be used for,
        // and runs nothing. An auto trait, as `Send`, `Sync`, `Unpin` or
        // `UnwindSafe`; `FusedIterator`, a promise that `next` stays `None`,
        // which `fuse()` is without (ADR 0055); a binding crate's marker of
        // what its bindings take, react's `Key` of a list's key. Or of types
        // only, which rustc works out: react's `Lifetimes`, a props type's
        // `'static` form (ADR 0265).
        || (tcx.associated_item_def_ids(id).iter()).all(|&item| matches!(tcx.def_kind(item), DefKind::AssocTy))
        // Run where rustc drops a value (ADR 0098).
        || tcx.is_lang_item(id, LangItem::Drop)
}

/// A std trait whose impls, the crate's own, rust-js allows and never
/// lowers: what would call one is refused where it would. `Future`, as
/// `.await` is JS's of a promise (ADR 0029); and `io::Read`, `BufRead` and
/// `Seek`, which JS has no stream of (ADR 0425). The one list.
pub(crate) fn unlowered_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_lang_item(id, LangItem::Future)
        || matches!(
            std_path(tcx, id).as_str(),
            "std::io::Read" | "std::io::BufRead" | "std::io::Seek"
        )
}

/// An impl of one of `unlowered_trait`'s, derived or the crate's own.
pub(crate) fn is_unlowered_impl(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
        && unlowered_trait(
            tcx,
            tcx.impl_trait_ref(id)
                .instantiate_identity()
                .skip_normalization()
                .def_id,
        )
}

/// What a conversion converts, into what, of its trait's method's `args`:
/// `From`'s and `TryFrom`'s from their argument into their type, `Into`'s
/// and `TryInto`'s from their type into their argument. The one answer of
/// which side is which.
pub(crate) fn conversion_sides<'tcx>(
    tcx: TyCtxt<'tcx>,
    trait_: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    let (this, other) = (args.types().next()?, args.types().nth(1)?);
    let is = |name: Symbol| tcx.is_diagnostic_item(name, trait_);
    match () {
        _ if is(sym::From) || is(sym::TryFrom) => Some((other, this)),
        _ if is(sym::Into) || is(sym::TryInto) => Some((this, other)),
        _ => None,
    }
}

/// What a conversion makes, a `From` or an `Into`: the trait's method's
/// target, of its `args`, or the impl's own type of a `from` a call resolves
/// to. An untagged enum's is the value itself (ADR 0214).
pub(crate) fn conversion_target<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    if let Some(imp) = tcx.trait_impl_of_assoc(def_id) {
        let trait_ref = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
        return tcx
            .is_diagnostic_item(sym::From, trait_ref.def_id)
            .then(|| trait_ref.self_ty());
    }
    let trait_ = tcx.trait_of_assoc(def_id)?;
    if tcx.is_diagnostic_item(sym::From, trait_) {
        return Some(args.type_at(0));
    }
    tcx.is_diagnostic_item(sym::Into, trait_).then(|| args.type_at(1))
}

/// An operator that assigns, `a += b`: the operation it does.
pub(in crate::lower) fn assign_operator(tcx: TyCtxt<'_>, id: DefId) -> Option<BinOp> {
    [
        (LangItem::AddAssign, BinOp::Add),
        (LangItem::SubAssign, BinOp::Sub),
        (LangItem::MulAssign, BinOp::Mul),
        (LangItem::DivAssign, BinOp::Div),
        (LangItem::RemAssign, BinOp::Rem),
        (LangItem::BitAndAssign, BinOp::BitAnd),
        (LangItem::BitOrAssign, BinOp::BitOr),
        (LangItem::BitXorAssign, BinOp::BitXor),
        (LangItem::ShlAssign, BinOp::Shl),
        (LangItem::ShrAssign, BinOp::Shr),
    ]
    .into_iter()
    .find_map(|(item, op)| tcx.is_lang_item(id, item).then_some(op))
}

/// An operator's trait, which the crate may implement: a value's, `a + b`,
/// an assignment's, `a += b`, each in its one list, and indexing (`Deref`
/// and `DerefMut` are `operational`, ADR 0394).
pub(in crate::lower) fn is_operator(tcx: TyCtxt<'_>, id: DefId) -> bool {
    value_operator(tcx, id).is_some()
        || assign_operator(tcx, id).is_some()
        || [LangItem::Index, LangItem::IndexMut]
            .into_iter()
            .any(|item| tcx.is_lang_item(id, item))
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(in crate::lower) fn bounded_by(&self, ty: ty::Ty<'tcx>, name: Symbol) -> bool {
        let ty = ty.peel_refs();
        let Some(trait_id) = self.tcx.get_diagnostic_item(name) else {
            return false;
        };
        // A type parameter, or an associated type of one, `<S as Source>::Iter`,
        // that only a caller knows (ADR 0106).
        let ty = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty);
        self.declared_bounds(ty, self.typing_env.param_env).contains(&trait_id)
    }

    /// The traits `ty`, a type only a caller knows, is bound by, their
    /// supertraits too: a type parameter's where-clauses', and an associated
    /// type's, and its trait's declaration's, `type Iter: Iterator` (ADR
    /// 0106). And a type parameter's that the where-clauses make an
    /// associated type, `U` of `I: Iterator<Item: IntoIterator<IntoIter =
    /// U>>`, that type's declaration's, as std's `FlattenCompat` is bound
    /// (ADR 0924). None of a type that's known.
    pub(in crate::lower) fn declared_bounds(&self, ty: ty::Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> Vec<DefId> {
        let declared = match ty.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { def_id },
                    ..
                },
            ) => Some(self.tcx.item_bounds(*def_id).skip_binder()),
            _ => None,
        };
        let unknown = declared.is_some() || matches!(ty.kind(), ty::Param(_));
        let clauses = param_env.caller_bounds().iter().filter(|clause| {
            clause
                .as_trait_clause()
                .is_some_and(|bound| bound.self_ty().skip_binder() == ty)
        });
        let equated = (param_env.caller_bounds().iter())
            .filter_map(|clause| clause.as_projection_clause())
            .filter(|equal| matches!(ty.kind(), ty::Param(_)) && equal.term().skip_binder().as_type() == Some(ty))
            .flat_map(|equal| self.tcx.item_bounds(equal.item_def_id()).skip_binder());
        let clauses = clauses.chain(declared.into_iter().flatten()).chain(equated);
        clauses
            .filter(|_| unknown)
            .filter_map(|clause| clause.as_trait_clause().map(|bound| bound.def_id()))
            .collect()
    }
}

pub(in crate::lower) enum TraitCall {
    Clone,
    Default,
    Equality { negate: bool },
    Ordering,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(in crate::lower) fn trait_call(&self, method: DefId, trait_id: DefId) -> Option<TraitCall> {
        if self.tcx.is_lang_item(method, LangItem::CloneFn) {
            Some(TraitCall::Clone)
        } else if self.tcx.is_diagnostic_item(Symbol::intern("Default"), trait_id) {
            Some(TraitCall::Default)
        } else if self.tcx.is_lang_item(trait_id, LangItem::PartialEq) {
            Some(TraitCall::Equality {
                negate: self.tcx.item_name(method).as_str() == "ne",
            })
        } else if self.tcx.is_diagnostic_item(sym::Ord, trait_id)
            || self.tcx.is_lang_item(trait_id, LangItem::PartialOrd)
        {
            Some(TraitCall::Ordering)
        } else {
            None
        }
    }

    /// Resolve blanket Into/TryInto through From/TryFrom. The caller decides
    /// whether the resulting implementation is available for emission.
    /// A std method that only calls its bound's, `iter.sum::<S>()` that's
    /// `<S as Sum<Item>>::sum(iter)`: `product()`, `collect()` that's
    /// `FromIterator::from_iter`, and `parse()` that's `FromStr::from_str`.
    /// The bound's method, its arguments, and the impl's that it resolves to
    /// (ADR 0160). Its own type parameter, the iterator, is the receiver's.
    pub(in crate::lower) fn resolve_delegated(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>, DefId)> {
        let tcx = self.tcx;
        if tcx.crate_name(def_id.krate) != sym::core {
            return None;
        }
        let wanted = match tcx.item_name(def_id).as_str() {
            "sum" => "sum",
            "product" => "product",
            "collect" => "from_iter",
            "parse" => "from_str",
            _ => return None,
        };
        tcx.clauses_of(def_id).clauses.iter().find_map(|&(clause, _)| {
            let bound = tcx
                .instantiate_bound_regions_with_erased(clause.as_trait_clause()?)
                .trait_ref;
            let method = tcx
                .associated_items(bound.def_id)
                .in_definition_order()
                .find(|item| item.is_fn() && item.name().as_str() == wanted)?
                .def_id;
            let bound_args = ty::EarlyBinder::bind(tcx, bound.args)
                .instantiate(tcx, args)
                .skip_normalization();
            let own = tcx.generics_of(method).own_params.len();
            let method_args = tcx.mk_args_from_iter(bound_args.iter().chain(args.types().take(own).map(Into::into)));
            let method_args = tcx
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(method_args))
                .ok()?;
            let instance =
                crate::lower::items::resolve_instance_in(tcx, self.typing_env, method, method_args).ok()??;
            Some((method, method_args, instance.def_id()))
        })
    }

    pub(in crate::lower) fn resolve_into(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>, DefId)> {
        let into = self.tcx.trait_of_assoc(def_id)?;
        let from = if self.tcx.is_diagnostic_item(sym::Into, into) {
            self.tcx.get_diagnostic_item(sym::From)?
        } else if self.tcx.is_diagnostic_item(sym::TryInto, into) {
            self.tcx.get_diagnostic_item(sym::TryFrom)?
        } else {
            return self.resolve_delegated(def_id, args);
        };
        let method = self
            .tcx
            .associated_items(from)
            .in_definition_order()
            .find(|item| item.is_fn())?
            .def_id;
        let args = self.tcx.mk_args(&[args[1], args[0]]);
        let args = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(args))
            .ok()?;
        // Of a type only its bounds know, `U: From<T>`, no impl yet: the
        // bound's method, its dictionary's.
        let implementation = match crate::lower::items::resolve_instance_in(self.tcx, self.typing_env, method, args) {
            Ok(Some(instance)) => instance.def_id(),
            _ if known_by_bounds(args.type_at(0)) => method,
            _ => return None,
        };
        Some((method, args, implementation))
    }
    pub(in crate::lower) fn conversion(&self, adt: DefId, convert: StdItem, serialize: bool) -> Option<Ty<'tcx>> {
        struct Finder<'tcx> {
            tcx: TyCtxt<'tcx>,
            types: &'tcx ty::TypeckResults<'tcx>,
            convert: DefId,
            found: Option<Ty<'tcx>>,
        }
        impl<'tcx> intravisit::Visitor<'tcx> for Finder<'tcx> {
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                if let hir::ExprKind::Path(ref path) = expr.kind
                    && let hir::def::Res::Def(_, id) = self.types.qpath_res(path, expr.hir_id)
                    && self.tcx.trait_of_assoc(id) == Some(self.convert)
                {
                    self.found = Some(self.types.node_args(expr.hir_id).type_at(1));
                }
                intravisit::walk_expr(self, expr);
            }
        }
        let convert = self.tcx.get_diagnostic_item(convert.name())?;
        for owner in self.tcx.hir_body_owners() {
            let mut parent = self.tcx.opt_parent(owner.to_def_id());
            while let Some(id) = parent
                && serde_impl(self.tcx, id).is_none()
            {
                parent = self.tcx.opt_parent(id);
            }
            let Some(imp) = parent else { continue };
            let self_ty = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
            if serde_impl(self.tcx, imp) != Some(serialize)
                || !matches!(self_ty.kind(), ty::Adt(a, _) if a.did() == adt)
            {
                continue;
            }
            let body = self.tcx.hir_body_owned_by(owner);
            let mut finder = Finder {
                tcx: self.tcx,
                types: self.tcx.typeck_body(body.id()),
                convert,
                found: None,
            };
            intravisit::Visitor::visit_body(&mut finder, body);
            if finder.found.is_some() {
                return finder.found;
            }
        }
        None
    }
}

/// Is a struct's value its fields? The crate's own are, and std's
/// `PhantomData` and `Reverse(x)`, `[x]`; any other of std's has fields of
/// its own that aren't the JS value rust-js makes of it.
pub(in crate::lower) fn struct_is_its_fields(tcx: TyCtxt<'_>, id: DefId) -> bool {
    ![sym::std, sym::core, sym::alloc].contains(&tcx.crate_name(id.krate))
        || tcx.is_lang_item(id, LangItem::PhantomData)
        || tcx.item_name(id).as_str() == "Reverse"
        // `Wrapping(x)`, `[x]` (ADR 0175).
        || tcx.is_diagnostic_item(Symbol::intern("Wrapping"), id)
}

pub(in crate::lower) fn ordering_value(tcx: TyCtxt<'_>, enum_def: DefId, variant: Symbol) -> Option<i128> {
    if !tcx.is_lang_item(enum_def, LangItem::OrderingEnum) {
        return None;
    }
    Some(match variant.as_str() {
        "Less" => -1,
        "Equal" => 0,
        _ => 1,
    })
}

/// Whether the closure `closure` is one its maker gives back, the type its
/// `impl Fn` hides, which carries its drop (ADR 0484): `ClosureFacts::carries_its_drop`'s
/// to answer.
pub(crate) fn given_back(tcx: TyCtxt<'_>, closure: DefId) -> bool {
    let maker = tcx.typeck_root_def_id(closure);
    if !matches!(tcx.def_kind(maker), DefKind::Fn | DefKind::AssocFn) {
        return false;
    }
    let output = tcx.fn_sig(maker).instantiate_identity().skip_binder().output();
    output
        .walk()
        .filter_map(|part| part.as_type())
        .any(|part| match part.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { def_id },
                    ..
                },
            ) => (tcx.type_of(*def_id).instantiate_identity())
                .skip_normalization()
                .walk()
                .any(|t| matches!(t.as_type().map(|t| *t.kind()), Some(ty::Closure(id, _)) if id == closure)),
            _ => false,
        })
}

/// How a function of type `ty`, in `owner`, is called: a closure's kind, or
/// a type parameter's or an associated type's, by the strongest `Fn` bound
/// its `owner` gives it, `F: FnMut`'s; `None` of what isn't one. One called
/// by `FnOnce` alone drops what it holds as its body ends, and one called by
/// `&` or `&mut` is dropped by what holds it (ADR 0470). An
/// `AssertUnwindSafe` is called as what it holds: its `call_once` calls
/// `(self.0)()` and drops it as that returns (ADR 0660).
pub(crate) fn function_kind<'tcx>(tcx: TyCtxt<'tcx>, owner: DefId, ty: Ty<'tcx>) -> Option<ty::ClosureKind> {
    match ty.kind() {
        ty::Closure(_, args) => Some(args.as_closure().kind()),
        ty::Adt(adt, args) if super::wrappers::is_assert_unwind_safe(tcx, adt.did()) => {
            function_kind(tcx, owner, args.type_at(0))
        }
        ty::Param(_) | ty::Alias(..) => (tcx.param_env(owner).caller_bounds().iter())
            .filter_map(|clause| clause.as_trait_clause())
            .filter(|bound| bound.self_ty().skip_binder() == ty)
            .filter_map(|bound| tcx.fn_trait_kind_from_def_id(bound.def_id()))
            .reduce(|a, b| if a.extends(b) { a } else { b }),
        _ => None,
    }
}

/// Does `owner` bound its type parameter `index` by `Any` (ADR 0331)?
pub(crate) fn bound_by_any(tcx: TyCtxt<'_>, owner: DefId, index: u32) -> bool {
    (tcx.clauses_of(owner).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .map(|bound| bound.skip_binder())
        .any(|bound| {
            Some(bound.def_id()) == any_trait(tcx) && matches!(bound.self_ty().kind(), ty::Param(p) if p.index == index)
        })
}

/// `cmp::max`, `Some(true)`, or `cmp::min`, `Some(false)`: `core`'s, which
/// `std` re-exports.
pub(crate) fn cmp_extreme(tcx: TyCtxt<'_>, def_id: DefId) -> Option<bool> {
    if tcx.crate_name(def_id.krate) != sym::core {
        return None;
    }
    match std_path(tcx, def_id).as_str() {
        "std::cmp::max" => Some(true),
        "std::cmp::min" => Some(false),
        _ => None,
    }
}

/// Is `id` std's `item`?
/// What an `Rc` or `Arc` points at (ADR 0320).
/// Whether a `dyn` of `predicates` is a function's, `dyn Fn(&str)`: the JS
/// function itself, which carries its drop, `$dropping`'s (ADR 0521).
pub(crate) fn is_fn_dyn(tcx: TyCtxt<'_>, predicates: &ty::List<ty::PolyExistentialPredicate<'_>>) -> bool {
    predicates.principal_def_id().is_some_and(|id| {
        tcx.fn_trait_kind_from_def_id(id).is_some() || tcx.async_fn_trait_kind_from_def_id(id).is_some()
    })
}

/// `Clone::clone_from`, which a `Clone` dictionary has where its impl
/// writes its own (ADR 0324).
pub(crate) fn is_clone_from(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "clone_from"
        && tcx
            .trait_of_assoc(id)
            .is_some_and(|tr| tcx.is_lang_item(tr, LangItem::Clone))
}

/// A provided method of a std trait that rust-js's dictionary of it has,
/// which a `dyn` or generic code calls through it: `Error::source` (ADR
/// 0141), `fmt::Write`'s `write_char` and `write_fmt` (ADR 0180), and
/// `io::Write`'s `write_all` and `write_fmt` (ADR 0392).
pub(crate) fn in_std_dictionary(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_method(tcx, id, StdItem::Error, "source")
        || error_default(tcx, id).is_some()
        || is_writer_default(tcx, id)
        || is_std_method(tcx, id, StdItem::IoWrite, "write_all")
        || is_std_method(tcx, id, StdItem::IoWrite, "write_fmt")
        // A crate's `Hasher`'s provided methods, `$hasher`'s, and `Hash`'s
        // `hash_slice`, `$hashSlice`'s (ADR 0463).
        || tcx.trait_of_assoc(id).is_some_and(|tr| is_hasher(tcx, tr))
        || is_std_method(tcx, id, StdItem::Hash, "hash_slice")
}
