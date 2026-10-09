//! What dropping a type runs (ADR 0098): nothing JS can see, a user `Drop`,
//! its own or a part's, or what rust-js can't run yet, and why. A type is
//! walked once. Asked through `DropQuery`, of types and the function's
//! facts alone: it sees no JS, nor emission's state.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::lower::analysis::Counted;
use crate::lower::recognition::{Recognition, StdItem, rc_pointee, weak_pointee};
use crate::lower::traits::{EvidenceQuery, may_have_destructors};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::{FieldPat, LocalVarId, Pat, PatKind};
use rustc_middle::ty::{self, Ty};
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
}

/// What a type drops depends on, of the function being lowered, beside
/// its types: the type parameters a caller gives a drop for, and those whose
/// drops rust-js can't make, and why; and what's been found of each type.
#[derive(Default)]
pub(in crate::lower) struct TypeDrops<'tcx> {
    pub(super) cache: RefCell<HashMap<Ty<'tcx>, Drops<'tcx>>>,
    pub(super) given: HashSet<u32>,
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
    pub(in crate::lower) state: &'a TypeDrops<'tcx>,
}

impl<'a, 'tcx> DropQuery<'a, 'tcx> {
    /// Is `drop`, a type's `Drop::drop`, one rust-js runs: the crate's own, or
    /// a library's it exports (ADR 0100)?
    pub(in crate::lower) fn runs_drop(&self, drop: DefId) -> bool {
        drop.is_local() || self.recognition.foreign.item(drop).is_some()
    }

    /// `traits::may_have_destructors`, of this crate.
    fn may_have_destructors(&self) -> bool {
        self.counted.any() || may_have_destructors(self.recognition.tcx, self.recognition.foreign)
    }

    /// An associated type's trait and item, `<Z as Zone>::Offset`'s
    /// `Z: Zone` and `Offset`, whose drop its impl's dictionary has where
    /// its type has one (ADR 0178). Not an `async fn`'s, which has no name.
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

    /// Is an associated type's drop given, in a dictionary this function has?
    fn has_item_drop(&self, ty: Ty<'tcx>) -> bool {
        self.item_drop_of(ty)
            .is_some_and(|(owner, _)| self.evidence.has_evidence(owner))
    }

    /// What dropping a `ty` runs.
    pub(in crate::lower) fn drops(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.drops_in(
            ty,
            &mut Walk {
                seen: Vec::new(),
                reached: usize::MAX,
                params_none: false,
            },
        )
    }

    /// What `ty` drops of its own, its type parameters, and what only their
    /// caller knows, dropping nothing (ADR 0190).
    pub(in crate::lower) fn drops_but_params(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.drops_in(
            ty,
            &mut Walk {
                seen: Vec::new(),
                reached: usize::MAX,
                params_none: true,
            },
        )
    }

    pub(in crate::lower) fn has_drops(&self, ty: Ty<'tcx>) -> bool {
        self.drops(ty) == Drops::Runs
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

    fn takes_whole(&self, closure: DefId) -> bool {
        self.held(closure).is_some()
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
            // `Single(t) | Ambiguous(t, _)` (ADR 0191): every alternative's,
            // as their flags are cleared whichever matched, so what one moves
            // that another doesn't must be of a variant the other excludes.
            PatKind::Or { pats } => {
                let mut each = Vec::new();
                for p in pats {
                    let mut moved = Vec::new();
                    if !self.paths_in(p, at, &mut moved) {
                        return false;
                    }
                    each.push(moved);
                }
                let apart = each.iter().enumerate().all(|(i, moved)| {
                    moved.iter().all(|path| {
                        pats.iter()
                            .enumerate()
                            .all(|(j, other)| i == j || each[j].contains(path) || excludes(other, &path[at.len()..]))
                    })
                });
                for path in each.into_iter().flatten() {
                    if !found.contains(&path) {
                        found.push(path);
                    }
                }
                apart
            }
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
        if !walk.params_none
            && let Some(&known) = self.state.cache.borrow().get(&ty)
        {
            return known;
        }
        let depth = walk.seen.len();
        let outer = std::mem::replace(&mut walk.reached, usize::MAX);
        walk.seen.push(ty);
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
        let found = match ty.kind() {
            ty::Tuple(items) => all(self, &mut items.iter(), walk),
            ty::Array(item, _) | ty::Slice(item) => self.drops_in(*item, walk),
            // What it holds, which it took whole: its drop drops that. Part of
            // one, `t.0`, would need its own flags.
            ty::Closure(def_id, args) => match all(self, &mut args.as_closure().upvar_tys().iter(), walk) {
                Drops::Runs if self.takes_whole(*def_id) => Drops::Runs,
                Drops::Runs => Drops::Unsupported(ty, "a closure that holds part of a value with a destructor"),
                found => found,
            },
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
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { .. },
                    ..
                },
            ) if self.recognition.is_unknown(ty) => match (self.has_item_drop(ty), self.may_have_destructors()) {
                (true, may) if may || self.library => Drops::Runs,
                (false, true) => {
                    Drops::Unsupported(ty, "a value of an associated type, where a type may have a destructor")
                }
                _ => Drops::Nothing,
            },
            // A channel's end: one sender fewer, or no receiver (ADR 0142). The
            // queue would drop what's still in it, its own way.
            ty::Adt(_, args) if self.recognition.channel_end(ty).is_some() => {
                match self.drops_in(args.type_at(0), walk) {
                    Drops::Nothing => Drops::Runs,
                    _ => Drops::Unsupported(ty, "a channel of a value with a destructor"),
                }
            }
            // A counted `Rc`: one count fewer, and the last drops what it points
            // at. A `Weak`: one weak count fewer (ADR 0320).
            ty::Adt(..)
                if let Some(pointee) = rc_pointee(tcx, ty)
                    && self.counted.counts(tcx, pointee) =>
            {
                match self.drops_in(pointee, walk) {
                    Drops::Unsupported(t, what) => Drops::Unsupported(t, what),
                    _ => Drops::Runs,
                }
            }
            ty::Adt(..) if weak_pointee(tcx, ty).is_some() => Drops::Runs,
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
                    // `Rc` when its last clone goes, a map its entries. A `Cell`
                    // drops the old value when it's set. A `RefCell` is its fields',
                    // its value's, and a write through `borrow_mut()` a place's (ADR 0320).
                    _ if own.is_some()
                        || [StdItem::Cell, StdItem::OnceCell, StdItem::LazyCell]
                            .into_iter()
                            .any(std) =>
                    {
                        match all(self, &mut args.types(), walk) {
                            Drops::Nothing => Drops::Nothing,
                            _ => Drops::Unsupported(ty, "a std type holding a value with a destructor"),
                        }
                    }
                    _ => parts(walk),
                }
            }
            // A trait object of the crate's trait, or a library's: whatever it
            // holds, whose dictionary has its drop, where a type may have one.
            ty::Dynamic(predicates, ..)
                if predicates
                    .principal_def_id()
                    .is_some_and(|id| self.recognition.is_rust_trait(id)) =>
            {
                match self.may_have_destructors() {
                    true => Drops::Runs,
                    false => Drops::Nothing,
                }
            }
            _ => Drops::Nothing,
        };
        walk.seen.pop();
        if walk.reached >= depth && !walk.params_none {
            self.state.cache.borrow_mut().insert(ty, found);
        }
        walk.reached = walk.reached.min(outer);
        found
    }
}

/// Whether what matches `pat` can't hold a part at `path`: a variant on the
/// way to it is another.
fn excludes(pat: &Pat<'_>, path: &[(Option<u32>, usize)]) -> bool {
    let Some(&(variant, field)) = path.first() else {
        return false;
    };
    let inside = |subpatterns: &[FieldPat<'_>]| {
        subpatterns
            .iter()
            .any(|f| f.field.as_usize() == field && excludes(&f.pattern, &path[1..]))
    };
    match &pat.kind {
        PatKind::Variant {
            variant_index,
            subpatterns,
            ..
        } => variant != Some(variant_index.as_u32()) || inside(subpatterns),
        PatKind::Leaf { subpatterns } => inside(subpatterns),
        PatKind::Binding {
            subpattern: Some(sub), ..
        } => excludes(sub, path),
        PatKind::Or { pats } => pats.iter().all(|p| excludes(p, path)),
        _ => false,
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
