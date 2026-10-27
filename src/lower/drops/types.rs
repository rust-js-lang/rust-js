//! What dropping a type runs (ADR 0098): nothing JS can see, a user `Drop`,
//! its own or a part's, or what rust-js can't run yet, and why. A type is
//! walked once. Asked through `DropQuery`, of types and the function's
//! facts alone: it sees no JS, nor emission's state.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::lower::analysis::{ClosureFacts, Counted};
use crate::lower::recognition::{Recognition, StdItem, any_trait, rc_pointee, weak_pointee};
use crate::lower::traits::{EvidenceQuery, may_have_destructors};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::{LocalVarId, Pat, PatKind};
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::def_id::DefId;

use super::Path;

/// What dropping a type runs.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum Drops<'tcx> {
    /// Nothing JS can see: no user `Drop` anywhere in it.
    Nothing,
    /// A user `Drop`, its own or a part's.
    Runs,
    /// One where rust-js can't run it yet, and what to say.
    Unsupported(Ty<'tcx>, &'static str),
}

/// The types a `drops` walk is inside, outermost first, and the outermost
/// of them that a type inside it was found inside of.
struct Walk<'tcx> {
    seen: Vec<Ty<'tcx>>,
    reached: usize,
    /// Its type parameters, and what only their caller knows, drop nothing
    /// (ADR 0190): what's found is what it drops of its own.
    params_none: bool,
    /// Guards drop nothing: what's found is what else it drops (ADR 0328).
    guards_none: bool,
    /// Where it's asked whether the outermost type is inside itself, each
    /// type walked so far: one walked again reaches no more than it did,
    /// and what's found of each is no answer to cache, a cached one no
    /// answer of what it reaches.
    visited: Option<HashSet<Ty<'tcx>>>,
}

impl Walk<'_> {
    fn new() -> Self {
        Walk {
            seen: Vec::new(),
            reached: usize::MAX,
            params_none: false,
            guards_none: false,
            visited: None,
        }
    }

    /// Whether what it finds of a type is that type's answer anywhere.
    fn caches(&self) -> bool {
        !self.params_none && !self.guards_none && self.visited.is_none()
    }
}

/// What a type drops depends on, of the function being lowered, beside
/// its types: the type parameters a caller gives a drop for, and those whose
/// drops rust-js can't make, and why; and what's been found of each type.
#[derive(Default)]
pub(in crate::lower) struct TypeDrops<'tcx> {
    pub(super) cache: RefCell<HashMap<Ty<'tcx>, Drops<'tcx>>>,
    pub(super) given: HashSet<u32>,
    /// The std traits' associated types a caller gives a drop for,
    /// `<I as IntoIterator>::IntoIter`, and each one's index (ADR 0421).
    pub(super) items: HashMap<ty::AliasTy<'tcx>, u32>,
    /// Each one a caller would give a drop for, given one or not: one none
    /// gives one drops nothing, as no caller's has one (ADR 0489).
    pub(super) slots: HashSet<ty::AliasTy<'tcx>>,
    /// A copied default's impl's arguments for its trait's parameters, and
    /// the impl's typing environment, whose body names a slot in the trait's
    /// terms, `Self::Item` of the impl's `T::Item` (ADR 0421); and the
    /// default's own parameters, which those terms name alone.
    pub(super) self_args: Option<(ty::GenericArgsRef<'tcx>, ty::TypingEnv<'tcx>, ty::GenericArgsRef<'tcx>)>,
    pub(super) unsupported: HashMap<u32, (Ty<'tcx>, &'static str)>,
}

/// What a type drops, and what a pattern moves, asked of types, the
/// crate's facts and the function's (ADRs 0098, 0178): which of its type
/// parameters have drops, and the bounds of the dictionaries it's given.
/// Nothing it emits, nor emission's state.
pub(in crate::lower) struct DropQuery<'a, 'tcx> {
    pub(in crate::lower) recognition: Recognition<'a, 'tcx>,
    pub(in crate::lower) evidence: EvidenceQuery<'a, 'tcx>,
    /// A library's: its consumers may have destructors (ADR 0163).
    pub(in crate::lower) library: bool,
    /// The `Rc`s counted, which drop (ADR 0320).
    pub(in crate::lower) counted: &'a Counted<'tcx>,
    /// Whether a function holding a value with a destructor is made a `dyn
    /// Fn`, which drops it then (ADR 0521).
    pub(in crate::lower) closure_facts: &'a ClosureFacts,
    pub(in crate::lower) state: &'a TypeDrops<'tcx>,
    /// Where it's asked as a body's lowered, the crate's `RefCell`s, and
    /// what its bodies may borrow: a guard of one no borrow of which can
    /// meet another is never counted, so drops nothing (ADR 0863). Not
    /// before, as which are is found by what bodies drop.
    pub(in crate::lower) ref_cells: Option<(&'a super::RefCells<'tcx>, &'a crate::lower::analysis::CellBorrows<'tcx>)>,
}

impl<'a, 'tcx> DropQuery<'a, 'tcx> {
    /// Is `drop`, a type's `Drop::drop`, one rust-js runs: the crate's own, a
    /// library's it exports (ADR 0100), or one the crate compiles itself,
    /// which its library's JS doesn't export (ADR 0914, amended)?
    pub(in crate::lower) fn runs_drop(&self, drop: DefId) -> bool {
        let foreign = self.recognition.foreign;
        drop.is_local() || foreign.item(drop).is_some() || foreign.is_instance(drop) || foreign.compiles(drop)
    }

    /// Does making a `from` a `to` lose a drop: what owns it, `target`, a
    /// `dyn` of it that keeps none, or of what rust-js can't drop?
    pub(in crate::lower) fn dyn_drop_lost(&self, target: Ty<'tcx>, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let ty::Dynamic(predicates, ..) = to.kind() else {
            return false;
        };
        !target.is_ref()
            && match self.drops(from) {
                Drops::Nothing => false,
                Drops::Runs => {
                    !self.recognition.is_pair_dyn(predicates)
                        && !crate::lower::recognition::is_fn_dyn(self.recognition.tcx, predicates)
                }
                Drops::Unsupported(..) => true,
            }
    }

    /// `traits::may_have_destructors`, of this crate: a library's always
    /// may, as its consumers' types may have one (ADR 0163); and one that
    /// counts an `Rc`, whose drop runs (ADR 0098).
    fn may_have_destructors(&self) -> bool {
        self.counted.any() || may_have_destructors(self.recognition.tcx, (self.recognition.foreign, self.library))
    }

    /// An associated type's trait and item, `<Z as Zone>::Offset`'s
    /// `Z: Zone` and `Offset`, whose drop its impl's dictionary has where
    /// its type has one (ADR 0178), a std trait's too, `Add`'s `Output` (ADR
    /// 0497). Not an `async fn`'s, which has no name. An iterator's, of no
    /// dictionary, is a slot's, found first (ADRs 0421, 0489).
    pub(in crate::lower) fn item_drop_of(&self, ty: Ty<'tcx>) -> Option<(ty::TraitRef<'tcx>, DefId)> {
        let tcx = self.recognition.tcx;
        let ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) = *ty.kind()
        else {
            return None;
        };
        (!tcx.is_impl_trait_in_trait(def_id)).then(|| (alias.trait_ref(tcx), def_id))
    }

    /// The index of the drop given for `alias`, a std associated type of a
    /// type parameter (ADR 0421), named as it's keyed or, in a copied
    /// default, in its trait's terms: the one place asked.
    pub(in crate::lower) fn slot_of(&self, alias: ty::AliasTy<'tcx>) -> Option<u32> {
        let items = &self.state.items;
        let index = items.get(&alias).or_else(|| items.get(&self.in_impl_terms(alias)?));
        index.copied()
    }

    /// Whether `alias` is one a caller would give a drop for, given one or
    /// not (ADR 0489), named as `slot_of` names it.
    fn is_slot(&self, alias: ty::AliasTy<'tcx>) -> bool {
        self.state.slots.contains(&alias)
            || self
                .in_impl_terms(alias)
                .is_some_and(|alias| self.state.slots.contains(&alias))
    }

    /// `alias`, of a copied default's trait's terms, in its impl's,
    /// normalized there: `<Holder<I> as Iterator>::Item` of `Self::Item` is
    /// `I::Item`.
    fn in_impl_terms(&self, alias: ty::AliasTy<'tcx>) -> Option<ty::AliasTy<'tcx>> {
        let tcx = self.recognition.tcx;
        let ty::AliasTy {
            kind: ty::Projection { def_id },
            args,
            ..
        } = alias
        else {
            return None;
        };
        let (self_args, impl_env, own) = self.state.self_args?;
        // One in the trait's terms names the default's own parameters alone:
        // not `<I as Iterator>::Item`, in the impl's already.
        let in_trait_terms = (args.iter().flat_map(|arg| arg.walk())).all(|part| match part.kind() {
            ty::GenericArgKind::Type(t) => !matches!(t.kind(), ty::Param(_)) || own.contains(&part),
            ty::GenericArgKind::Const(c) => !matches!(c.kind(), ty::ConstKind::Param(_)) || own.contains(&part),
            ty::GenericArgKind::Lifetime(_) => true,
        });
        if !in_trait_terms {
            return None;
        }
        let ty = Ty::new_projection(tcx, ty::IsRigid::No, def_id, args);
        let ty = ty::EarlyBinder::bind(tcx, ty)
            .instantiate(tcx, self_args)
            .skip_normalization();
        let ty = (tcx.try_normalize_erasing_regions(impl_env, ty::Unnormalized::new_wip(ty))).unwrap_or(ty);
        match *ty.kind() {
            ty::Alias(_, alias) => Some(alias),
            _ => None,
        }
    }

    /// Is an associated type's drop given, in a dictionary this function has?
    fn has_item_drop(&self, ty: Ty<'tcx>) -> bool {
        self.item_drop_of(ty)
            .is_some_and(|(owner, _)| self.evidence.has_evidence(owner))
    }

    /// What dropping a `ty` runs.
    pub(in crate::lower) fn drops(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.drops_in(ty, &mut Walk::new())
    }

    /// Whether dropping a `ty` drops a `ty` again, inside what it holds: a
    /// list's node in a `Box` of itself, a trie's in a map of its `Rc`s.
    /// Its drop is then a function that calls itself.
    pub(in crate::lower) fn drops_itself(&self, ty: Ty<'tcx>) -> bool {
        let mut walk = Walk {
            visited: Some(HashSet::new()),
            ..Walk::new()
        };
        self.drops_in(ty, &mut walk);
        walk.reached == 0
    }

    /// What `ty` drops of its own, its type parameters, and what only their
    /// caller knows, dropping nothing (ADR 0190).
    pub(in crate::lower) fn drops_but_params(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.drops_in(
            ty,
            &mut Walk {
                params_none: true,
                ..Walk::new()
            },
        )
    }

    /// Whether dropping a `ty` runs more than a guard's, which only counts
    /// its cell borrowed once fewer (ADR 0328).
    pub(in crate::lower) fn drops_but_guards(&self, ty: Ty<'tcx>) -> bool {
        self.drops_in(
            ty,
            &mut Walk {
                guards_none: true,
                ..Walk::new()
            },
        ) != Drops::Nothing
    }

    pub(in crate::lower) fn has_drops(&self, ty: Ty<'tcx>) -> bool {
        self.drops(ty) == Drops::Runs
    }

    /// Whether `ty` is a `Ref` or a `RefMut` of a `RefCell` no borrow of
    /// which can meet another, which is never counted (ADR 0863).
    fn uncounted_guard(&self, ty: Ty<'tcx>) -> bool {
        let Some((ref_cells, borrows)) = self.ref_cells else {
            return false;
        };
        let tcx = self.recognition.tcx;
        let ty::Adt(_, args) = ty.kind() else { return false };
        let of_ref_cell = [StdItem::RefCellRef, StdItem::RefCellRefMut]
            .into_iter()
            .any(|item| self.recognition.is_std_type(ty, item));
        let Some(held) = args.types().next().filter(|_| of_ref_cell) else {
            return false;
        };
        let cell = Ty::new_adt(
            tcx,
            tcx.adt_def(crate::lower::recognition::std_item(tcx, StdItem::RefCell)),
            tcx.mk_args(&[held.into()]),
        );
        ref_cells.quiet(&self.recognition, borrows, (cell, false), &|t| {
            super::drop_runs(self.recognition, (self.library, self.closure_facts), self.counted, t)
        })
    }

    /// What a closure takes by value that has a destructor: each variable,
    /// and its type, in the order its drop drops them. None if it takes
    /// part of one, or is another crate's.
    pub(in crate::lower) fn held(&self, closure: DefId) -> Option<Vec<(LocalVarId, Ty<'tcx>)>> {
        let local = closure.as_local()?;
        let mut held = Vec::new();
        for captured in self.recognition.tcx.closure_captures(local) {
            let ty = captured.place.ty();
            if captured.is_by_ref() || self.drops(ty) == Drops::Nothing {
                continue;
            }
            if !captured.place.projections.is_empty() {
                return None;
            }
            held.push((LocalVarId(captured.get_root_variable()), ty));
        }
        Some(held)
    }

    /// Whether a closure's drop drops what it holds whole: one that takes
    /// no part of a value, or another crate's its maker gives back, which
    /// carries its drop (ADRs 0484, 0523).
    fn takes_whole(&self, closure: DefId) -> bool {
        self.held(closure).is_some()
            || !closure.is_local() && crate::lower::recognition::given_back(self.recognition.tcx, closure)
    }

    /// The parts a pattern moves out of what it's matched against, by value:
    /// those it binds that have a destructor. None if it moves one a way this
    /// doesn't follow yet, as through a `Box` or into `x @ ..`.
    pub(in crate::lower) fn pattern_paths(&self, pat: &Pat<'tcx>) -> Option<Vec<Path>> {
        let mut found = Vec::new();
        self.paths_in(pat, &mut Path::new(), &mut found).then_some(found)
    }

    fn paths_in(&self, pat: &Pat<'tcx>, at: &mut Path, found: &mut Vec<Path>) -> bool {
        let moves = |p: &Pat<'tcx>| {
            let mut any = false;
            p.walk_always(|p| {
                if let PatKind::Binding {
                    mode: BindingMode(ByRef::No, _),
                    ty,
                    ..
                } = p.kind
                {
                    any |= self.has_drops(ty);
                }
            });
            any
        };
        match &pat.kind {
            PatKind::Wild => true,
            PatKind::Binding {
                mode: BindingMode(ByRef::No, _),
                subpattern: None,
                ty,
                ..
            } => {
                if self.has_drops(*ty) {
                    found.push(at.clone());
                }
                true
            }
            PatKind::Binding {
                mode: BindingMode(ByRef::Yes(..), _),
                subpattern: None,
                ..
            } => true,
            PatKind::Leaf { subpatterns } => subpatterns.iter().all(|f| {
                at.push((None, f.field.as_usize()));
                let ok = self.paths_in(&f.pattern, at, found);
                at.pop();
                ok
            }),
            PatKind::Variant {
                variant_index,
                subpatterns,
                ..
            } => subpatterns.iter().all(|f| {
                at.push((Some(variant_index.as_u32()), f.field.as_usize()));
                let ok = self.paths_in(&f.pattern, at, found);
                at.pop();
                ok
            }),
            // `Single(t) | Ambiguous(t, _)` (ADR 0191): each alternative followed.
            PatKind::Or { pats } => pats.iter().all(|p| self.paths_in(p, at, &mut Vec::new())),
            _ => !moves(pat),
        }
    }

    /// Each type is walked once and cached, so a type whose parts double at
    /// each level, `S2<S2<T>>` in `S3<T>`, isn't walked once for each path to
    /// it. What's found while taking a type further out as running nothing,
    /// being inside itself, is only as sure as that type's walk, so it's
    /// cached only once that one's done.
    fn drops_in(&self, ty: Ty<'tcx>, walk: &mut Walk<'tcx>) -> Drops<'tcx> {
        let tcx = self.recognition.tcx;
        let ty = self.recognition.reveal(ty);
        // What drops nothing at all, a number or a `&T`, and what's being
        // walked further out: a type inside itself runs no more than it does.
        if !ty.needs_drop(tcx, self.recognition.typing_env) {
            return Drops::Nothing;
        }
        if let Some(at) = walk.seen.iter().position(|&t| t == ty) {
            walk.reached = walk.reached.min(at);
            return Drops::Nothing;
        }
        if let Some(visited) = &mut walk.visited
            && !visited.insert(ty)
        {
            return Drops::Nothing;
        }
        if walk.caches()
            && let Some(&known) = self.state.cache.borrow().get(&ty)
        {
            return known;
        }
        let depth = walk.seen.len();
        let outer = std::mem::replace(&mut walk.reached, usize::MAX);
        walk.seen.push(ty);
        let found = self.parts_drop(ty, walk);
        walk.seen.pop();
        if walk.reached >= depth && walk.caches() {
            self.state.cache.borrow_mut().insert(ty, found);
        }
        walk.reached = walk.reached.min(outer);
        found
    }

    /// What `ty`'s drop runs, by what it is and what its parts run, `ty`
    /// inside `walk` already: `drops_in` alone ends its walk.
    fn parts_drop(&self, ty: Ty<'tcx>, walk: &mut Walk<'tcx>) -> Drops<'tcx> {
        let tcx = self.recognition.tcx;
        let std = |item: StdItem| self.recognition.is_std_type(ty, item);
        let all = |cx: &Self, tys: &mut dyn Iterator<Item = Ty<'tcx>>, walk: &mut Walk<'tcx>| {
            let mut found = Drops::Nothing;
            for t in tys {
                match cx.drops_in(t, walk) {
                    Drops::Nothing => {}
                    Drops::Runs => found = Drops::Runs,
                    unsupported => return unsupported,
                }
            }
            found
        };
        match ty.kind() {
            ty::Tuple(items) => all(self, &mut items.iter(), walk),
            ty::Array(item, _) | ty::Slice(item) => self.drops_in(*item, walk),
            // What it holds, which it took whole: its drop drops that. Part of
            // one, `t.0`, would need its own flags. A closure's, an `async`
            // closure's, and an `async` block's future's, which drops what it
            // took as its body ends (#173); an `async fn`'s, whose body drops
            // its arguments, holds nothing else.
            ty::Closure(..) | ty::CoroutineClosure(..) | ty::Coroutine(..)
                if let Some((def_id, held)) = crate::lower::recognition::held_types(ty)
                    && (!matches!(ty.kind(), ty::Coroutine(..))
                        || crate::lower::recognition::is_async_block(self.recognition.tcx, def_id)) =>
            {
                match all(self, &mut held.iter(), walk) {
                    Drops::Runs if self.takes_whole(def_id) => Drops::Runs,
                    Drops::Runs => Drops::Unsupported(ty, "a closure that holds part of a value with a destructor"),
                    found => found,
                }
            }
            ty::Adt(_, args) if ty.is_box() || self.recognition.is_vec_like(ty) => self.drops_in(args.type_at(0), walk),
            ty::Param(_) if walk.params_none => Drops::Nothing,
            // A type parameter a caller gives a drop function for.
            ty::Param(param) if self.state.given.contains(&param.index) => Drops::Runs,
            ty::Param(param) if let Some(&(t, what)) = self.state.unsupported.get(&param.index) => {
                Drops::Unsupported(t, what)
            }
            // An associated type only a caller knows, `<S as Source>::Item` (ADR
            // 0106): its impl's dictionary has its drop, where it has one (ADR
            // 0178), as a library's consumers' may. One of no dictionary, of a
            // std trait's, has nothing to drop only where nothing does.
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { .. },
                    ..
                },
            ) if walk.params_none && self.recognition.is_unknown(ty) => Drops::Nothing,
            // A std trait's, `<I as IntoIterator>::IntoIter`, whose drop a
            // caller gives (ADR 0421).
            ty::Alias(_, alias) if self.slot_of(*alias).is_some() => Drops::Runs,
            // One a caller would give a drop for, that none does: none of
            // theirs has one, as of a type parameter none's given for (ADR
            // 0489).
            ty::Alias(_, alias) if self.is_slot(*alias) => Drops::Nothing,
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { .. },
                    ..
                },
            ) if self.recognition.is_unknown(ty) => match (self.has_item_drop(ty), self.may_have_destructors()) {
                (true, true) => Drops::Runs,
                (false, true) => {
                    Drops::Unsupported(ty, "a value of an associated type, where a type may have a destructor")
                }
                _ => Drops::Nothing,
            },
            // A channel's end: one sender fewer, or no receiver, which drops
            // what's still in its queue (ADR 0142).
            ty::Adt(_, args) if self.recognition.channel_end(ty).is_some() => {
                match self.drops_in(args.type_at(0), walk) {
                    Drops::Unsupported(t, what) => Drops::Unsupported(t, what),
                    _ => Drops::Runs,
                }
            }
            // A set's items, and a map's keys and values (ADRs 0321, 0467).
            ty::Adt(_, args) if self.recognition.is_set(ty) => self.drops_in(args.type_at(0), walk),
            ty::Adt(_, args) if self.recognition.is_map(ty) => {
                match (
                    self.drops_in(args.type_at(0), walk),
                    self.drops_in(args.type_at(1), walk),
                ) {
                    (Drops::Unsupported(t, what), _) | (_, Drops::Unsupported(t, what)) => Drops::Unsupported(t, what),
                    (Drops::Nothing, Drops::Nothing) => Drops::Nothing,
                    _ => Drops::Runs,
                }
            }
            // A counted `Rc`: one count fewer, and the last drops what it points
            // at. A `Weak`: one weak count fewer (ADR 0320); an uncounted
            // one, the value itself, nothing (ADR 0843).
            ty::Adt(..)
                if let Some(pointee) = rc_pointee(tcx, ty)
                    && self.counted.counts(tcx, pointee) =>
            {
                match self.drops_in(pointee, walk) {
                    Drops::Unsupported(t, what) => Drops::Unsupported(t, what),
                    _ => Drops::Runs,
                }
            }
            ty::Adt(..)
                if let Some(pointee) = weak_pointee(tcx, ty)
                    && self.counted.counts(tcx, pointee) =>
            {
                Drops::Runs
            }
            ty::Adt(..) if weak_pointee(tcx, ty).is_some() => Drops::Nothing,
            // A guard: its cell borrowed once fewer (ADR 0328). What it guards
            // is its cell's to drop.
            ty::Adt(..) if self.recognition.is_guard(ty) && (walk.guards_none || self.uncounted_guard(ty)) => {
                Drops::Nothing
            }
            ty::Adt(..) if self.recognition.is_guard(ty) => Drops::Runs,
            // An iterator that owns an array's items, `vec::IntoIter`: what it
            // hasn't given yet, each dropped in order, as a loop that leaves
            // early drops them (ADR 0098). MIR drops it as one value.
            ty::Adt(..) if let Some(item) = self.recognition.owned_items(ty) => self.drops_in(item, walk),
            // std's struct compiled for this build, a `Scan`'s with std's
            // structs alone, or a private one it holds, `FlattenCompat`:
            // what its fields drop, which its own `close`, compiled, drops in
            // their order (ADR 0934).
            ty::Adt(adt, args)
                if crate::lower::recognition::is_compiled_std_iterator(tcx, ty)
                    || crate::lower::recognition::has_compiled_bodies(tcx, adt.did()) =>
            {
                let mut fields = adt.all_fields().map(|f| self.recognition.field_ty(f, args));
                all(self, &mut fields, walk)
            }
            // An adapter that holds a function, `map(f)`'s: its source closed,
            // then its function dropped (ADR 0485). A source it can't close
            // that has something to drop, as before.
            ty::Adt(_, args) if let Some(function) = self.recognition.held_function(ty) => {
                let source = match self.recognition.closed_on_drop(ty) {
                    Some(held) => self.drops_in(held, walk),
                    None => match self.drops_in(args.type_at(0), walk) {
                        Drops::Nothing => Drops::Nothing,
                        _ => Drops::Unsupported(ty, "a std type holding a value with a destructor"),
                    },
                };
                match (source, self.drops_in(function, walk)) {
                    (Drops::Unsupported(t, what), _) | (_, Drops::Unsupported(t, what)) => Drops::Unsupported(t, what),
                    (Drops::Nothing, Drops::Nothing) => Drops::Nothing,
                    _ => Drops::Runs,
                }
            }
            // One closed to drop what it holds, `repeat`'s value (ADR 0422).
            ty::Adt(..) if let Some(held) = self.recognition.closed_on_drop(ty) => self.drops_in(held, walk),
            // An `extract_if()`'s: its test, what it hasn't reached left in
            // its collection (ADR 0506).
            ty::Adt(..) if let Some(test) = self.recognition.extract_if_test(ty) => self.drops_in(test, walk),
            // A heap's `PeekMut`: its top sifted down, if it changed (ADR 0333).
            ty::Adt(..) if self.recognition.peek_mut_of(ty).is_some() => Drops::Runs,
            // Never dropped, or dropped by hand.
            ty::Adt(..) if self.recognition.is_lang_adt(ty, LangItem::ManuallyDrop) || std(StdItem::MaybeUninit) => {
                Drops::Nothing
            }
            ty::Adt(adt, args) => {
                let own = tcx.adt_destructor(adt.did());
                let parts = |walk: &mut Walk<'tcx>| {
                    let mut fields = adt.all_fields().map(|f| self.recognition.field_ty(f, args));
                    all(self, &mut fields, walk)
                };
                match own {
                    Some(d) if self.runs_drop(d.did) => match parts(walk) {
                        Drops::Unsupported(t, what) => Drops::Unsupported(t, what),
                        _ => Drops::Runs,
                    },
                    // Another crate's, that rust-js didn't compile, or that doesn't
                    // export it: what it runs is out of sight (ADR 0100).
                    Some(_) if !self.recognition.in_sysroot(adt.did()) => {
                        Drops::Unsupported(ty, "a destructor of another crate's that its manifest doesn't export")
                    }
                    // A std type that drops what it holds its own way: an
                    // `Rc` when its last clone goes, a map its entries. A
                    // `RefCell` is its fields', its value's, and a write through
                    // `borrow_mut()` a place's (ADR 0320), as a `Mutex`'s and an
                    // `RwLock`'s are (ADR 0144), and a `Cell`'s, whose `set`
                    // drops the old, and a `OnceCell`'s, its `Option` (ADR
                    // 0519). And an iterator adapter, a JS
                    // iterator, of what has one; a generic one's is its
                    // iterator's, whatever that is.
                    _ if own.is_some()
                        || (self.recognition.is_iter_adapter(ty) && !ty.has_non_region_param())
                        || std(StdItem::LazyCell) =>
                    {
                        match all(self, &mut args.types(), walk) {
                            Drops::Nothing => Drops::Nothing,
                            _ => Drops::Unsupported(ty, "a std type holding a value with a destructor"),
                        }
                    }
                    _ => parts(walk),
                }
            }
            // A trait object that's a pair: whatever it holds, whose
            // dictionary has its drop, where a type may have one.
            ty::Dynamic(predicates, ..) if self.recognition.is_pair_dyn(predicates) => {
                match self.may_have_destructors() {
                    true => self.implementors_drop(predicates, walk),
                    false => Drops::Nothing,
                }
            }
            // A function's, the drop it carries, where a function holding a
            // value with a destructor is made one (ADR 0521).
            ty::Dynamic(predicates, ..) if crate::lower::recognition::is_fn_dyn(self.recognition.tcx, predicates) => {
                match self.closure_facts.fn_dyns_drop {
                    true => Drops::Runs,
                    false => Drops::Nothing,
                }
            }
            _ => Drops::Nothing,
        }
    }
}

impl<'a, 'tcx> DropQuery<'a, 'tcx> {
    /// What a `dyn` of the crate's own trait drops: what one of its
    /// implementors does, each of them known, where nothing else can be
    /// one, a `dyn Listener` of a `Logger` nothing. Of another crate's
    /// trait, a library's, whose consumers may implement it, or one a
    /// generic impl implements, whatever it holds may (ADR 0320).
    fn implementors_drop(
        &self,
        predicates: &'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>,
        walk: &mut Walk<'tcx>,
    ) -> Drops<'tcx> {
        let tcx = self.recognition.tcx;
        // A `dyn Any`'s, where the crate makes one of what drops (ADR 0691).
        if predicates.principal_def_id().is_some() && predicates.principal_def_id() == any_trait(tcx) {
            return match self.closure_facts.any_dyns_drop {
                true => Drops::Runs,
                false => Drops::Nothing,
            };
        }
        let Some(principal) = predicates
            .principal_def_id()
            .filter(|id| id.is_local() && !self.library)
        else {
            return Drops::Runs;
        };
        for implementation in tcx.all_impls(principal) {
            let implementor = tcx.type_of(implementation).instantiate_identity().skip_normalization();
            if implementor.has_non_region_param() || !matches!(self.drops_in(implementor, walk), Drops::Nothing) {
                return Drops::Runs;
            }
        }
        Drops::Nothing
    }
}

/// What's unsupported, and of what type, unless it's a closure's, which
/// is only where it's written.
pub(super) fn describe(ty: Ty<'_>, what: &str) -> String {
    match ty.kind() {
        ty::Closure(..) => what.to_string(),
        _ => format!("{what}, `{ty}`,"),
    }
}
