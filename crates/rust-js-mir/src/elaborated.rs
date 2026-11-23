//! rustc's own drop elaboration of the crate's bodies, beside rust-js's
//! (`drops.rs`), compared drop by drop in every compile (#2112's change D):
//! rust-js lowers the built MIR and elaborates its drops itself, of the
//! parts JS drops; rustc's `ElaborateDrops` is crate-private and runs after
//! passes that renumber the blocks, so each `Drop` is found again in rustc's
//! body by where it's written, its span, scope and whether it unwinds, and
//! what of the dropped place each side drops there, never, always or as a
//! flag says, is compared. A `Drop` that can't be found again, or is found
//! twice, is counted apart from a disagreement: `unmatched`. A disagreement
//! on the normal path is refused (change D's (c), ADR 0364): the JS would
//! drop otherwise than Rust does; and so is an unmatched drop there (D2):
//! what can't be compared can't be said to drop as Rust does, so the
//! comparison fails closed. `--drops-shadow` writes each verdict.

use super::drops::{Elaboration, Style};
use crate::MirCx;
use rust_js_lower::analysis::Mir;
use rust_js_lower::drops::Drops;
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{
    self, BasicBlock, Local, Operand, Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::consts::ConstExt as _;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_mir_dataflow::move_paths::{LookupResult, MoveData, MovePathIndex};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// When a drop drops a part of what it drops: never, always, or as a flag
/// says.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum When {
    Never,
    Flagged,
    Always,
}

/// A place by what it reaches, not its types: its local and each step,
/// casts none, which rustc's passes add.
type PathKey = (Local, Vec<String>);

fn path_key(place: Place<'_>) -> PathKey {
    path_key_through(place, &IndexMap::default())
}

/// The place's key, each local `derefs` holds a copy of a place in, as
/// rustc's `Derefer` makes one before a deref, `_t = deref_copy (_1.0);
/// drop((*_t))`, read as that place, `(*(_1.0))`.
fn path_key_through(place: Place<'_>, derefs: &IndexMap<Local, PathKey>) -> PathKey {
    let (local, mut steps) = match derefs.get(&place.local) {
        Some((local, steps)) => (*local, steps.clone()),
        None => (place.local, Vec::new()),
    };
    steps.extend((place.projection.iter()).filter_map(|step| match step {
        ProjectionElem::Field(field, _) => Some(format!(".{}", field.as_u32())),
        ProjectionElem::Downcast(_, variant) => Some(format!("@{}", variant.as_u32())),
        ProjectionElem::Deref => Some("*".to_string()),
        ProjectionElem::Index(_) => Some("[i]".to_string()),
        ProjectionElem::ConstantIndex { offset, from_end, .. } => {
            Some(format!("[{offset}{}]", if from_end { "$" } else { "" }))
        }
        ProjectionElem::Subslice { from, to, from_end } => {
            Some(format!("[{from}..{to}{}]", if from_end { "$" } else { "" }))
        }
        ProjectionElem::OpaqueCast(_) | ProjectionElem::UnwrapUnsafeBinder(_) | ProjectionElem::PhantomDeref => None,
    }));
    (local, steps)
}

/// Whether `a` is `b` or holds it.
fn holds(a: &PathKey, b: &PathKey) -> bool {
    a.0 == b.0 && b.1.starts_with(&a.1)
}

/// Where a `Drop` is written: its span, its scope, and whether it unwinds.
type SiteKey = (Span, mir::SourceScope, bool);

/// A `Drop` of rustc's elaborated body, of what JS drops.
struct Theirs {
    site: SiteKey,
    place: PathKey,
    when: When,
    /// The variants of what it holds a test of whose discriminant goes
    /// elsewhere, `switchInt(discriminant(_28)) -> [0: .., otherwise:
    /// drop]` of a ladder's: of those, it drops nothing.
    except: Vec<(PathKey, u32)>,
}

impl Theirs {
    /// Whether it drops what's at `key`.
    fn covers(&self, key: &PathKey) -> bool {
        holds(&self.place, key)
            && !(self.except.iter()).any(|(at, variant)| {
                let mut down = at.1.clone();
                down.push(format!("@{variant}"));
                at.0 == key.0 && key.1.starts_with(&down)
            })
    }
}

/// What a block that tests a discriminant tests, the variant of each value,
/// and how many variants it has.
type Tested = (PathKey, Vec<(u128, u32)>, u32);

/// A closure's `self`, `_1`.
const SELF: Local = Local::from_u32(1);

/// The blocks `body` runs to from its start, but a coroutine's drop path:
/// what a `Yield`'s `drop` edge drops, where the future is dropped while
/// it's suspended. JS's `await` has no such path, as a promise runs on to
/// its end (ADR 0029), so neither rust-js's body nor rustc's is compared
/// there (#2177).
fn running(body: &mir::Body<'_>) -> rustc_index::bit_set::DenseBitSet<BasicBlock> {
    let mut seen = rustc_index::bit_set::DenseBitSet::new_empty(body.basic_blocks.len());
    let mut next = vec![mir::START_BLOCK];
    while let Some(block) = next.pop() {
        if !seen.insert(block) {
            continue;
        }
        match &body.basic_blocks[block].terminator().kind {
            TerminatorKind::Yield { resume, .. } => next.push(*resume),
            kind => next.extend(kind.successors()),
        }
    }
    seen
}

/// The locals of `body` that hold what an `.await` awaits, `__awaitee`. An
/// `async` block's or closure's future, awaited, is its promise's from then
/// on, whose body drops what it holds as it ends or throws, so JS drops
/// nothing of it after; rustc's drop of it there is of a future that has
/// returned, which drops nothing either. Neither body's drop of one is
/// compared (#2177).
fn awaited(body: &mir::Body<'_>) -> IndexSet<Local> {
    (body.var_debug_info.iter())
        .filter(|info| rust_js_lower::recognition::is_awaitee(info))
        .filter_map(|info| match info.value {
            mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() => Some(place.local),
            _ => None,
        })
        .collect()
}

/// Each `Drop` rustc's elaboration left in `body` of what JS drops: whether
/// a drop flag guards it, a test of a `bool` local only constants are
/// written to, above the tests of discriminants that lead to it, and the
/// variants those send elsewhere.
fn theirs<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, js_drops: &dyn Fn(Ty<'tcx>) -> bool) -> Vec<Theirs> {
    let mut written: IndexMap<Local, bool> = IndexMap::default();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind
                && let Some(local) = assign.0.as_local()
            {
                let constant = matches!(&assign.1, Rvalue::Use(Operand::Constant(_), _));
                let entry = written.entry(local).or_insert(true);
                *entry &= constant;
            }
        }
    }
    // Each local assigned once, a copy of a reference a place holds before
    // a deref, as `Derefer` makes one, `_t = copy ((*_1).0)`, by the place
    // it copies. Its `LocalInfo::DerefTemp` is cleared in this body; a
    // user's own such copy points where the place does all the same.
    let mut copies: IndexMap<Local, (usize, Option<Place<'tcx>>)> = IndexMap::default();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind
                && let Some(local) = assign.0.as_local()
            {
                let entry = copies.entry(local).or_insert((0, None));
                entry.0 += 1;
                let ty = body.local_decls[local].ty;
                let pointer = ty.is_ref() || ty.is_raw_ptr() || ty.is_box();
                if let Rvalue::Use(Operand::Copy(of), _) | Rvalue::CopyForDeref(of) = &assign.1
                    && pointer
                    && !of.projection.is_empty()
                {
                    entry.1 = Some(*of);
                }
                // A box's deref as `ElaborateBoxDerefs` writes it, `_p = copy
                // (_b.0) as *const T (BoxDerefTransmute)`: `_p` is the box
                // `_b`'s pointer, so `(*_p)` is `(*_b)`, at this statement.
                if let Rvalue::Cast(mir::CastKind::BoxDerefTransmute, Operand::Copy(of), _) = &assign.1
                    && let [ProjectionElem::Field(field, _)] = of.projection.as_slice()
                    && field.as_u32() == 0
                {
                    entry.1 = Some(Place::from(of.local));
                }
                // And its contents' drop as `open_drop_for_box_contents`
                // writes it, of what's left of a box something moved out of,
                // `_p = copy ((_b.0: Unique<T>).0: NonNull<T>) as *const T
                // (Transmute)`, of a box anywhere, a field's `(_5.1)` too:
                // `(*_p)` is `(*_b)` (#2177).
                if let Rvalue::Cast(mir::CastKind::Transmute, Operand::Copy(of), _) = &assign.1
                    && let [
                        base @ ..,
                        ProjectionElem::Field(unique, _),
                        ProjectionElem::Field(pointer, _),
                    ] = of.projection.as_slice()
                    && unique.as_u32() == 0
                    && pointer.as_u32() == 0
                {
                    let boxed = Place {
                        local: of.local,
                        projection: tcx.mk_place_elems(base),
                    };
                    if boxed.ty(&body.local_decls, tcx).ty.is_box() {
                        entry.1 = Some(boxed);
                    }
                }
            }
        }
    }
    let mut derefs: IndexMap<Local, PathKey> = IndexMap::default();
    for (&local, &(count, of)) in &copies {
        if count == 1
            && let Some(of) = of
        {
            let key = path_key_through(of, &derefs);
            derefs.insert(local, key);
        }
    }
    let is_flag = |local: Local| body.local_decls[local].ty.is_bool() && written.get(&local) == Some(&true);
    let tests_a_flag = |block: BasicBlock| {
        matches!(&body.basic_blocks[block].terminator().kind, TerminatorKind::SwitchInt { discr: Operand::Copy(place) | Operand::Move(place), .. }
            if place.as_local().is_some_and(is_flag))
    };
    // A block that tests a discriminant alone, `_d = discriminant(p);
    // switchInt(move _d)`: what it tests, the variant of each value, and
    // how many variants it has.
    let tested = |block: BasicBlock| -> Option<Tested> {
        let data = &body.basic_blocks[block];
        let TerminatorKind::SwitchInt {
            discr: Operand::Copy(d) | Operand::Move(d),
            ..
        } = &data.terminator().kind
        else {
            return None;
        };
        let StatementKind::Assign(assign) = &data.statements.last()?.kind else {
            return None;
        };
        let Rvalue::Discriminant(of) = &assign.1 else {
            return None;
        };
        if assign.0 != *d {
            return None;
        }
        let rustc_middle::ty::Adt(adt, _) = of.ty(&body.local_decls, tcx).ty.kind() else {
            return None;
        };
        let values = (adt.discriminants(tcx))
            .map(|(variant, value)| (value.val, variant.as_u32()))
            .collect();
        Some((path_key_through(*of, &derefs), values, adt.variants().len() as u32))
    };
    let reachable = running(body);
    let awaited = awaited(body);
    let predecessors = body.basic_blocks.predecessors();
    let mut found = Vec::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        let TerminatorKind::Drop { place, .. } = &data.terminator().kind else {
            continue;
        };
        let ty = place.ty(&body.local_decls, tcx).ty;
        if !reachable.contains(block) || !js_drops(ty) || awaited.contains(&place.local) {
            continue;
        }
        // Up through the tests of discriminants that lead to it, each the
        // only way in, to a test of a flag, if one.
        let (mut at, mut flagged, mut except) = (block, false, Vec::new());
        for _ in 0..8 {
            let into = &predecessors[at];
            if !into.is_empty() && into.iter().all(|&from| tests_a_flag(from)) {
                flagged = true;
                break;
            }
            let [from] = into.as_slice() else { break };
            let Some((of, values, count)) = tested(*from) else {
                break;
            };
            let TerminatorKind::SwitchInt { targets, .. } = &body.basic_blocks[*from].terminator().kind else {
                break;
            };
            let variant_of = |value: u128| values.iter().find(|(v, _)| *v == value).map(|&(_, variant)| variant);
            let to_it: Vec<u32> = (targets.iter())
                .filter(|&(_, to)| to == at)
                .filter_map(|(v, _)| variant_of(v))
                .collect();
            let elsewhere: Vec<u32> = (targets.iter())
                .filter(|&(_, to)| to != at)
                .filter_map(|(v, _)| variant_of(v))
                .collect();
            match targets.otherwise() == at {
                true => except.extend(elsewhere.into_iter().map(|variant| (of.clone(), variant))),
                false => except.extend((0..count).filter(|v| !to_it.contains(v)).map(|v| (of.clone(), v))),
            }
            at = *from;
        }
        let info = data.terminator().source_info;
        found.push(Theirs {
            site: (info.span, info.scope, data.is_cleanup),
            place: path_key_through(packed_moved(tcx, body, data, *place), &derefs),
            when: if flagged { When::Flagged } else { When::Always },
            except,
        });
    }
    found
}

/// The place a `Drop` of `data`, of `place`, drops: of a part of a packed
/// struct that may be misaligned, what rustc's `AddMovesForPackedDrops`
/// moved into a temporary of its own to drop it there, `_t = move ((_5.1:
/// Misalign).1.0); drop(_t)`, the part, as rust-js's body drops it in place
/// (a field's address is never misaligned in JS); else `place`.
fn packed_moved<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    data: &mir::BasicBlockData<'tcx>,
    place: Place<'tcx>,
) -> Place<'tcx> {
    let Some(local) = place.as_local() else {
        return place;
    };
    let Some(StatementKind::Assign(assign)) = data.statements.last().map(|statement| &statement.kind) else {
        return place;
    };
    let (Some(to), Rvalue::Use(Operand::Move(from), _)) = (assign.0.as_local(), &assign.1) else {
        return place;
    };
    // The pass's temporary, which no variable is; of a place in a packed
    // struct, which alone it moves.
    let variable = (body.var_debug_info.iter())
        .any(|info| matches!(info.value, mir::VarDebugInfoContents::Place(at) if at.local == local));
    let packed = (from.iter_projections()).any(|(base, _)| {
        base.ty(&body.local_decls, tcx)
            .ty
            .ty_adt_def()
            .is_some_and(|adt| adt.repr().packed())
    });
    match to == local && !variable && packed {
        true => *from,
        false => place,
    }
}

/// What the comparison says of a `Drop`: rust-js's elaboration drops each
/// part of it as rustc's does, or one otherwise; only one side asks a flag
/// where the other knows, which only running it says alike or not; which
/// of rustc's `Drop`s it is can't be said; or there's no body of the other
/// side's to compare it with: a library's body the crate compiles, whose
/// elaborated body rustc keeps in its own crate, or a constant's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Verdict {
    Agree,
    Disagree,
    FlagOurs,
    FlagTheirs,
    Unmatched,
    Uncompared,
    Unreached,
}

impl Verdict {
    /// How `--drops-shadow` writes it.
    fn word(self) -> &'static str {
        match self {
            Verdict::Agree => "agree",
            Verdict::Disagree => "disagree",
            Verdict::FlagOurs => "flag-ours",
            Verdict::FlagTheirs => "flag-theirs",
            Verdict::Unmatched => "unmatched",
            Verdict::Uncompared => "uncompared",
            Verdict::Unreached => "unreached",
        }
    }
}

impl When {
    /// How a refusal says it: "rust-js drops it always".
    fn word(self) -> &'static str {
        match self {
            When::Never => "never",
            When::Flagged => "as its drop flag says",
            When::Always => "always",
        }
    }
}

/// A part the two sides drop otherwise: the part, rust-js's answer,
/// rustc's, and whether it's the rest of a value parts of which are paths
/// of their own, which `rest_drops` can't always take apart (#2177).
type Differ = (String, When, When, bool);

/// A part of a dropped place, as `parts_of` compares it: how it's said,
/// whether it's the rest of a part with parts of its own, when rust-js's
/// elaboration drops it, and when each of rustc's `Drop`s there that covers
/// it does, one each; `split` where none covers it but some drop its parts,
/// all alike, its one entry standing for them.
// Flags, not a state: two facts of one part, each true or not apart.
struct Part {
    what: String,
    rest: bool,
    ours: When,
    theirs: Vec<When>,
    split: bool,
}

/// Each part that differs, as a refusal and `--drops-shadow` say it.
fn said(differ: &[Differ]) -> String {
    (differ.iter())
        .map(|(what, ours, theirs, _)| format!("`{what}`: rust-js drops it {}, rustc {}", ours.word(), theirs.word()))
        .collect::<Vec<_>>()
        .join("; ")
}

/// A place of `body` as its source names it: its variable, where one is
/// it, then each step, `(*(node as variant 1).0)`.
fn spelled(body: &mir::Body<'_>, (local, steps): &PathKey) -> String {
    let mut out = (body.var_debug_info.iter())
        .find_map(|info| match info.value {
            mir::VarDebugInfoContents::Place(place) if place.local == *local && place.projection.is_empty() => {
                Some(info.name.to_string())
            }
            _ => None,
        })
        .unwrap_or_else(|| format!("{local:?}"));
    for step in steps {
        out = match step.strip_prefix('@') {
            Some(variant) => format!("({out} as variant {variant})"),
            None if step == "*" => format!("(*{out})"),
            None => out + step,
        };
    }
    out
}

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// Of each `Drop` of `mir`'s body that drops what JS drops, whether
    /// rust-js's `elaboration` drops each part of it when rustc's does:
    /// `agree`, `disagree` with each part that differs, or `unmatched` with
    /// why it can't be said. A disagreement on the normal path is refused,
    /// in every compile; each verdict is a line where the session records
    /// them (`--drops-shadow`).
    pub(super) fn shadow_drops(
        &self,
        mir: &Mir<'tcx>,
        elaboration: Option<&Elaboration<'tcx>>,
        quiet_guards: &IndexSet<Local>,
    ) {
        let tcx = self.tcx;
        // Nothing to compare: nothing refused, its lines written only where
        // the session records them.
        let recording = rust_js_lower::session::caches(tcx).drops_shadow.is_some();
        if !recording && (mir.elaborated.is_none() || elaboration.is_none()) {
            return;
        }
        let body = &mir.body;
        // Of a type as rust-js's body has it, normalized where it's lowered:
        // a copied default's rustc body is only instantiated,
        // `<Holder<I> as Iterator>::Item`.
        let js_drops = |ty: Ty<'tcx>| {
            let unnormalized = rustc_middle::ty::Unnormalized::new_wip(ty);
            let ty = (tcx.try_normalize_erasing_regions(self.typing_env, unnormalized)).unwrap_or(ty);
            self.drops(ty) != Drops::Nothing
        };
        let reachable = running(body);
        let awaited = awaited(body);
        let sites: Vec<(BasicBlock, Place<'tcx>)> = (body.basic_blocks.iter_enumerated())
            .filter_map(|(block, data)| match &data.terminator().kind {
                TerminatorKind::Drop { place, .. }
                    if reachable.contains(block)
                        && !awaited.contains(&place.local)
                        && js_drops(place.ty(&body.local_decls, tcx).ty)
                        && !(place.projection.is_empty() && quiet_guards.contains(&place.local)) =>
                {
                    Some((block, *place))
                }
                _ => None,
            })
            .collect();
        if sites.is_empty() {
            return;
        }
        let function = body.source.def_id();
        let source_map = tcx.sess.source_map();
        let site_of = |block: BasicBlock| {
            let info = body.basic_blocks[block].terminator().source_info;
            (info.span, info.scope, body.basic_blocks[block].is_cleanup)
        };
        let mut out: Vec<(Verdict, BasicBlock, String)> = Vec::new();
        let (Some(elaborated), Some(elaboration)) = (&mir.elaborated, elaboration) else {
            let why = if mir.elaborated.is_none() {
                "no rustc body"
            } else {
                "no rust-js elaboration"
            };
            out.extend(
                sites
                    .iter()
                    .map(|&(block, _)| (Verdict::Uncompared, block, why.to_string())),
            );
            self.say_shadowed(function, &out, &site_of, source_map);
            return;
        };
        let mut theirs = theirs(tcx, elaborated, &js_drops);
        // A closure's body as `call_once` runs it: its drop of itself, where
        // it returns and where it unwinds, is rustc's `ClosureOnce` shim's,
        // which calls its body by `&mut` and then drops it, each where
        // rust-js's body writes its own, of the path it's on (#2177).
        if mir.called_once {
            let shim = self.shim_drops_of_self(body);
            for &(block, place) in sites
                .iter()
                .filter(|(_, place)| place.local == SELF && place.projection.is_empty())
            {
                let site = site_of(block);
                theirs.extend((shim.iter()).filter(|t| t.site.2 == site.2).map(|t| Theirs {
                    site,
                    place: path_key(place),
                    when: t.when,
                    except: Vec::new(),
                }));
            }
        }
        // What rust-js's elaboration drops of all of `place` where `block`
        // ends, as the writer asks it (`Elaboration::style`, and behind a
        // pointer `Elaboration::drops`); none where it drops it a part at a
        // time.
        let whole = |block: BasicBlock, place: Place<'tcx>| match elaboration.move_data.rev_lookup.find(place.as_ref())
        {
            LookupResult::Exact(path) => match elaboration.style(block, path, true) {
                Style::Dead => Some(When::Never),
                Style::Static => Some(When::Always),
                Style::Conditional => Some(When::Flagged),
                Style::Open => None,
            },
            LookupResult::Parent(_) => Some(if elaboration.drops(block, place) {
                When::Always
            } else {
                When::Never
            }),
        };
        let overlapping = |a: &PathKey, b: &PathKey| holds(a, b) || holds(b, a);
        let reached = self.cleanups_reached(body, elaboration, &reachable);
        for &(block, place) in &sites {
            let site = site_of(block);
            // A cleanup no terminator that unwinds in JS reaches runs
            // nothing in JS: rustc's reaches it where what can't throw in
            // JS unwinds, or a drop of what JS drops nothing of (#2112's
            // change D3).
            if site.2 && !reached.contains(block) {
                out.push((
                    Verdict::Unreached,
                    block,
                    "no terminator that unwinds in JS reaches it".to_string(),
                ));
                continue;
            }
            let key = path_key(place);
            // Two of rust-js's at one place where they're written, of what
            // holds the other: rustc's can't be told apart one by one.
            let alike: Vec<(BasicBlock, Place<'tcx>)> = (sites.iter())
                .filter(|&&(other, other_place)| site_of(other) == site && overlapping(&key, &path_key(other_place)))
                .copied()
                .collect();
            let at_site: Vec<&Theirs> = (theirs.iter())
                .filter(|theirs| theirs.site == site && overlapping(&key, &theirs.place))
                .collect();
            // Compared as a multiset: what each side drops there, whole, of
            // each place, a drop that drops nothing left out, as rustc's
            // elaboration leaves a dead one out. Unlike, or of a part at a
            // time on either side, it's unmatched.
            let multiset = || -> Option<bool> {
                let mut ours: Vec<(PathKey, When)> = Vec::new();
                for &(other, other_place) in &alike {
                    let when = whole(other, other_place)?;
                    if when != When::Never {
                        ours.push((path_key(other_place), when));
                    }
                }
                let mut rustcs: Vec<(PathKey, When)> = Vec::new();
                for t in theirs
                    .iter()
                    .filter(|t| t.site == site && alike.iter().any(|&(_, p)| overlapping(&path_key(p), &t.place)))
                {
                    if !t.except.is_empty() {
                        return None;
                    }
                    rustcs.push((t.place.clone(), t.when));
                }
                ours.sort();
                rustcs.sort();
                Some(ours == rustcs)
            };
            let path = match elaboration.move_data.rev_lookup.find(place.as_ref()) {
                LookupResult::Exact(path) => path,
                // Behind a pointer, which has no move path: all of it, as
                // the writer drops it, against rustc's `Drop` there, its
                // place read through `Derefer`'s copies.
                LookupResult::Parent(_) => {
                    let ours = whole(block, place).unwrap_or(When::Always);
                    let theirs_when = (at_site.iter()).filter(|t| t.covers(&key)).map(|t| t.when).max();
                    out.push(match theirs_when {
                        _ if alike.len() > 1 && multiset() == Some(true) => (
                            Verdict::Agree,
                            block,
                            "behind a pointer, alike drops as a multiset".to_string(),
                        ),
                        _ if alike.len() > 1 => (
                            Verdict::Unmatched,
                            block,
                            "ambiguous: behind a pointer, drops written alike".to_string(),
                        ),
                        None => (
                            Verdict::Unmatched,
                            block,
                            "no rustc drop where it's written: behind a pointer".to_string(),
                        ),
                        Some(theirs_when) if theirs_when == ours => {
                            (Verdict::Agree, block, "behind a pointer, whole".to_string())
                        }
                        Some(theirs_when) => (
                            Verdict::Disagree,
                            block,
                            said(&[(
                                format!("{} behind a pointer", spelled(body, &key)),
                                ours,
                                theirs_when,
                                false,
                            )]),
                        ),
                    });
                    continue;
                }
            };
            let parts = self.parts_compared(body, elaboration, block, path, &at_site);
            out.push(match parts {
                Err(why) => (Verdict::Unmatched, block, why),
                Ok((differ, compared)) if differ.is_empty() => (Verdict::Agree, block, format!("{compared} compared")),
                Ok(_) if alike.len() > 1 && multiset() == Some(true) => (
                    Verdict::Agree,
                    block,
                    format!("alike drops as a multiset, {}", alike.len()),
                ),
                Ok((differ, _)) if alike.len() > 1 => {
                    match self.alike_parts(body, elaboration, (&alike, &key), &at_site) {
                        Some(Ok(parts)) => (
                            Verdict::Agree,
                            block,
                            format!("alike drops part by part as a multiset, {parts} parts"),
                        ),
                        Some(Err((verdict, unlike))) => (verdict, block, unlike.join("; ")),
                        None => (
                            Verdict::Unmatched,
                            block,
                            format!("ambiguous: two drops written alike; {}", said(&differ)),
                        ),
                    }
                }
                Ok((differ, _)) if at_site.is_empty() => (
                    Verdict::Unmatched,
                    block,
                    format!("no rustc drop where it's written; {}", said(&differ)),
                ),
                // A difference only of whether a flag is asked runs alike
                // where the flag holds the other's answer, which only
                // running it says: counted apart. Judged by the parts that
                // differ where any does, the rest of a value beside them
                // following them; and a difference only of the rest is a
                // difference all the same: a box's contents rustc frees by
                // a call are read as the box's (`theirs`), so none is left
                // the comparison can't stand behind (#2177).
                Ok((differ, _)) => {
                    let parts: Vec<&Differ> = differ.iter().filter(|&&(.., rest)| !rest).collect();
                    let judged = if parts.is_empty() {
                        differ.iter().collect()
                    } else {
                        parts
                    };
                    let ours = judged.iter().all(|&&(_, ours, ..)| ours == When::Flagged);
                    let theirs = judged.iter().all(|&&(_, _, theirs, _)| theirs == When::Flagged);
                    match (ours, theirs) {
                        // rust-js asks a flag where rustc knows.
                        (true, false) => (Verdict::FlagOurs, block, said(&differ)),
                        // rustc asks one where rust-js knows.
                        (false, true) => (Verdict::FlagTheirs, block, said(&differ)),
                        _ => (Verdict::Disagree, block, said(&differ)),
                    }
                }
            });
        }
        self.say_shadowed(function, &out, &site_of, source_map);
    }

    /// Of a closure's body as `call_once` runs it, `body`, rustc's drops of
    /// the closure itself, `_1` whole, as its `ClosureOnce` shim makes them
    /// (`Instance::fn_once_adapter_instance`): one where the call returns
    /// and one where it unwinds, each sited in the shim (`Mir::called_once`).
    fn shim_drops_of_self(&self, body: &mir::Body<'tcx>) -> Vec<Theirs> {
        let tcx = self.tcx;
        let rustc_middle::ty::Closure(closure, args) = *body.local_decls[SELF].ty.kind() else {
            return Vec::new();
        };
        let shim = rustc_middle::ty::Instance::fn_once_adapter_instance(tcx, closure, args);
        let shim_body = tcx.instance_mir(shim.def);
        // Its `_1` is the trait's `Self`, the shim's own generic body's: what
        // it drops is the closure, which JS drops, as rust-js's body asks.
        (theirs(tcx, shim_body, &|_| true).into_iter())
            .filter(|t| t.place == (SELF, Vec::new()))
            .collect()
    }

    /// Each of `function`'s verdicts said: on the normal path, a
    /// disagreement refused, as the JS would drop otherwise than Rust does,
    /// and an unmatched drop, which can't be said to drop as Rust does, so
    /// the comparison fails closed (#2112's change D2); not one of a body
    /// with no other side to compare, `uncompared`. A cleanup's apart, as rust-js's body has no unwinding of what can't
    /// throw (ADR 0301), whose cleanups rustc's reaches. And each a line
    /// where the session records them (`--drops-shadow`). The function's
    /// path is printed only where one is said: printing a path walks every
    /// crate's modules, 37 ms of a compile (#2314).
    fn say_shadowed(
        &self,
        function: DefId,
        verdicts: &[(Verdict, BasicBlock, String)],
        site_of: &dyn Fn(BasicBlock) -> SiteKey,
        source_map: &rustc_span::source_map::SourceMap,
    ) {
        let path = std::cell::OnceCell::new();
        let function = || path.get_or_init(|| self.tcx.def_path_str(function)).as_str();
        for (verdict, block, detail) in verdicts {
            let (span, _, cleanup) = site_of(*block);
            let why = match verdict {
                _ if cleanup => continue,
                Verdict::Disagree => {
                    "is elaborated otherwise than rustc's drop elaboration, so its JS would drop otherwise than Rust"
                }
                Verdict::Unmatched => {
                    "can't be compared with rustc's drop elaboration, so its JS can't be said to drop as Rust does"
                }
                Verdict::Agree | Verdict::FlagOurs | Verdict::FlagTheirs | Verdict::Uncompared | Verdict::Unreached => {
                    continue;
                }
            };
            let function = function();
            rust_js_lower::refusal::error(
                self.tcx,
                span,
                format!("rust-js: this drop in `{function}` {why}: {detail} (#2112's change D)"),
            );
        }
        let caches = rust_js_lower::session::caches(self.tcx);
        let Some(lines) = &caches.drops_shadow else {
            return;
        };
        lines
            .borrow_mut()
            .extend(verdicts.iter().map(|(verdict, block, detail)| {
                let (span, _, cleanup) = site_of(*block);
                let at = source_map.span_to_diagnostic_string(span);
                let path = if cleanup { "cleanup" } else { "normal" };
                format!("{}\t{path}\t{}\t{at}\t{detail}", verdict.word(), function())
            }));
    }

    /// The cleanup blocks of `body` a terminator that unwinds in JS
    /// (`unwinds`) reaches, of those it runs to, `reachable`.
    fn cleanups_reached(
        &self,
        body: &mir::Body<'tcx>,
        elaboration: &Elaboration<'tcx>,
        reachable: &rustc_index::bit_set::DenseBitSet<BasicBlock>,
    ) -> rustc_index::bit_set::DenseBitSet<BasicBlock> {
        let mut reached = rustc_index::bit_set::DenseBitSet::new_empty(body.basic_blocks.len());
        let mut next: Vec<BasicBlock> = (body.basic_blocks.iter_enumerated())
            .filter(|&(block, data)| {
                !data.is_cleanup && reachable.contains(block) && self.unwinds(body, elaboration, block)
            })
            .filter_map(|(_, data)| match data.terminator().unwind() {
                Some(mir::UnwindAction::Cleanup(start)) => Some(*start),
                _ => None,
            })
            .collect();
        while let Some(block) = next.pop() {
            if reached.insert(block) {
                next.extend(body.basic_blocks[block].terminator().successors());
            }
        }
        reached
    }

    /// Whether what of `node`, a path with parts of its own, none of them
    /// holds, has what JS drops: the rest an open drop drops by its
    /// `rest_path`'s state.
    pub(super) fn rest_holds_glue(
        &self,
        body: &mir::Body<'tcx>,
        move_data: &MoveData<'tcx>,
        node: MovePathIndex,
    ) -> bool {
        let mut below = Vec::new();
        rustc_mir_dataflow::on_all_children_bits(move_data, node, |child| {
            if child != node {
                below.push(path_key(move_data.move_paths[child].place));
            }
        });
        self.rest_drops(body, move_data.move_paths[node].place, &below, 4)
    }

    /// Whether what of `place` is none of `parts`, nor holds one, has what
    /// JS drops: a field, a capture or another variant, `depth` down.
    fn rest_drops(&self, body: &mir::Body<'tcx>, place: Place<'tcx>, parts: &[PathKey], depth: u32) -> bool {
        let tcx = self.tcx;
        let key = path_key(place);
        if parts.contains(&key) {
            return false;
        }
        if !parts.iter().any(|part| holds(&key, part)) {
            let at = place.ty(&body.local_decls, tcx);
            return match at.variant_index {
                Some(_) => !self.variant_drops_nothing(body, place),
                None => self.drops(at.ty) != Drops::Nothing,
            };
        }
        if depth == 0 {
            return true;
        }
        let at = place.ty(&body.local_decls, tcx);
        let field = |index: usize, ty: Ty<'tcx>| {
            let place = tcx.mk_place_field(place, rustc_abi::FieldIdx::from_usize(index), ty);
            self.rest_drops(body, place, parts, depth - 1)
        };
        match at.ty.kind() {
            rustc_middle::ty::Adt(adt, args) if adt.is_enum() && at.variant_index.is_none() => {
                (adt.variants().iter_enumerated()).any(|(variant, _)| {
                    let down = tcx.mk_place_downcast(place, *adt, variant);
                    self.rest_drops(body, down, parts, depth - 1)
                })
            }
            rustc_middle::ty::Adt(adt, args) => {
                let variant = adt.variant(at.variant_index.unwrap_or(rustc_abi::FIRST_VARIANT));
                (variant.fields.iter().enumerate()).any(|(index, f)| field(index, f.ty(tcx, args).skip_normalization()))
            }
            rustc_middle::ty::Tuple(items) => items.iter().enumerate().any(|(index, ty)| field(index, ty)),
            // What a closure, an `async` closure or block holds, its
            // captures, as the writer takes it apart (`held_types`).
            _ if let Some((_, held)) = rust_js_lower::recognition::held_types(at.ty) => {
                held.iter().enumerate().any(|(index, ty)| field(index, ty))
            }
            // An array of a known length, each element in turn, as its
            // `ConstantIndex` paths take it apart.
            rustc_middle::ty::Array(_, len) if let Some(len) = len.try_to_target_usize(tcx) => (0..len).any(|offset| {
                let at = ProjectionElem::ConstantIndex {
                    offset,
                    min_length: len,
                    from_end: false,
                };
                self.rest_drops(body, tcx.mk_place_elem(place, at), parts, depth - 1)
            }),
            _ => true,
        }
    }

    /// Whether `place` is a variant of an enum, `(k as Text)`, none of whose
    /// fields JS drops.
    pub(super) fn variant_drops_nothing(&self, body: &mir::Body<'tcx>, place: Place<'tcx>) -> bool {
        let at = place.ty(&body.local_decls, self.tcx);
        let (Some(variant), rustc_middle::ty::Adt(adt, args)) = (at.variant_index, at.ty.kind()) else {
            return false;
        };
        (adt.variant(variant).fields.iter())
            .all(|field| self.drops(field.ty(self.tcx, args).skip_normalization()) == Drops::Nothing)
    }

    /// Of `path`, dropped where `block` ends, how many of its parts were
    /// compared, and each part rust-js's elaboration drops otherwise than
    /// rustc's `Drop`s there, `theirs`, do: its leaves, and the rest of it
    /// where it has parts of its own (`parts_of`), each against the most
    /// any of rustc's there does.
    fn parts_compared(
        &self,
        body: &mir::Body<'tcx>,
        elaboration: &Elaboration<'tcx>,
        block: BasicBlock,
        path: MovePathIndex,
        theirs: &[&Theirs],
    ) -> Result<(Vec<Differ>, usize), String> {
        let parts = self.parts_of(body, elaboration, block, path, theirs)?;
        let differ = (parts.iter())
            .filter_map(|part| {
                let most = part.theirs.iter().copied().max().unwrap_or(When::Never);
                (part.ours != most).then(|| (part.what.clone(), part.ours, most, part.rest))
            })
            .collect();
        Ok((differ, parts.len()))
    }

    /// Of drops of one place written alike, `alike` (each `key`), each part
    /// compared as a multiset (`parts_of`): when rust-js's drops drop it,
    /// but those that drop it never, which rustc's elaboration leaves out,
    /// against when each of rustc's `Drop`s there, `theirs`, does. The
    /// number of parts that agree, or each that doesn't, said; `None` where
    /// they aren't all of one place, or a part's rustc drops are of its
    /// parts (`split`), which can't be counted one each.
    fn alike_parts(
        &self,
        body: &mir::Body<'tcx>,
        elaboration: &Elaboration<'tcx>,
        (alike, key): (&[(BasicBlock, Place<'tcx>)], &PathKey),
        theirs: &[&Theirs],
    ) -> Option<Result<usize, (Verdict, Vec<String>)>> {
        if alike.iter().any(|&(_, place)| path_key(place) != *key) {
            return None;
        }
        let LookupResult::Exact(path) = elaboration.move_data.rev_lookup.find(alike[0].1.as_ref()) else {
            return None;
        };
        let each: Vec<Vec<Part>> = (alike.iter())
            .map(|&(block, _)| self.parts_of(body, elaboration, block, path, theirs).ok())
            .collect::<Option<_>>()?;
        if each.iter().flatten().any(|part| part.split) || each.iter().any(|parts| parts.len() != each[0].len()) {
            return None;
        }
        // Each part that differs, said, and what's left of each side's
        // drops of it once those alike are paired off.
        let mut unlike: Vec<(String, Vec<When>, Vec<When>)> = Vec::new();
        for (index, part) in each[0].iter().enumerate() {
            let mut ours: Vec<When> = (each.iter())
                .map(|parts| parts[index].ours)
                .filter(|&when| when != When::Never)
                .collect();
            let mut rustc = part.theirs.clone();
            ours.sort();
            rustc.sort();
            if ours == rustc {
                continue;
            }
            let said = format!(
                "`{}`: rust-js's alike drops drop it [{}], rustc's [{}]",
                part.what,
                ours.iter().map(|when| when.word()).collect::<Vec<_>>().join(", "),
                rustc.iter().map(|when| when.word()).collect::<Vec<_>>().join(", ")
            );
            let (mut only_ours, mut only_rustc) = (ours.clone(), rustc);
            for when in &ours {
                if let Some(at) = only_rustc.iter().position(|w| w == when) {
                    only_rustc.remove(at);
                    only_ours.remove(only_ours.iter().position(|w| w == when).unwrap());
                }
            }
            unlike.push((said, only_ours, only_rustc));
        }
        if unlike.is_empty() {
            return Some(Ok(each[0].len()));
        }
        // Only a flag one side asks where the other knows, of every part
        // that differs, as one drop's differences are judged.
        let flagged = |whens: &[When]| whens.iter().all(|&when| when == When::Flagged);
        let known = |whens: &[When]| whens.iter().all(|&when| when != When::Flagged);
        let asks =
            |asking: &[When], knowing: &[When]| asking.len() == knowing.len() && flagged(asking) && known(knowing);
        let ours_ask = unlike.iter().all(|(_, ours, rustc)| asks(ours, rustc));
        let rustcs_ask = unlike.iter().all(|(_, ours, rustc)| asks(rustc, ours));
        let verdict = match (ours_ask, rustcs_ask) {
            (true, false) => Verdict::FlagOurs,
            (false, true) => Verdict::FlagTheirs,
            _ => Verdict::Disagree,
        };
        Some(Err((verdict, unlike.into_iter().map(|(said, ..)| said).collect())))
    }

    /// Of `path`, dropped where `block` ends, each part compared: its
    /// leaves, but a variant none of whose fields JS drops, and the rest of
    /// each part with parts of its own, itself too, where that rest holds
    /// what JS drops (`rest_drops`); each with what rust-js drops of it and
    /// what each of rustc's `Drop`s there, `theirs`, that covers it does,
    /// one each. A leaf none of rustc's covers but some drop its parts of,
    /// all alike, is `split`: its one entry stands for those.
    fn parts_of(
        &self,
        body: &mir::Body<'tcx>,
        elaboration: &Elaboration<'tcx>,
        block: BasicBlock,
        path: MovePathIndex,
        theirs: &[&Theirs],
    ) -> Result<Vec<Part>, String> {
        let move_data = &elaboration.move_data;
        let ours_of = |node: MovePathIndex| match elaboration.presence(block, node) {
            (false, _) => When::Never,
            (true, false) => When::Always,
            (true, true) => When::Flagged,
        };
        let key_of = |node: MovePathIndex| path_key(move_data.move_paths[node].place);
        let mut leaves = Vec::new();
        rustc_mir_dataflow::on_all_children_bits(move_data, path, |child| {
            if move_data.move_paths[child].first_child.is_none() {
                leaves.push(child);
            }
        });
        let mut parts = Vec::new();
        for &leaf in &leaves {
            // A variant none of whose fields JS drops, `Text(Vec<u8>)`'s:
            // dropped or not, nothing runs.
            if self.variant_drops_nothing(body, move_data.move_paths[leaf].place) {
                continue;
            }
            let key = key_of(leaf);
            // A `Drop` of it or of what holds it; else those of its parts,
            // all alike.
            let covering: Vec<When> = (theirs.iter()).filter(|t| t.covers(&key)).map(|t| t.when).collect();
            let (whens, split) = match covering.is_empty() {
                false => (covering, false),
                true => {
                    let under: IndexSet<When> = (theirs.iter())
                        .filter(|t| holds(&key, &t.place) && t.place != key)
                        .map(|t| t.when)
                        .collect();
                    match under.len() {
                        0 => (Vec::new(), false),
                        1 => (vec![under[0]], true),
                        _ => return Err(format!("`{}`: rustc drops its parts unalike", spelled(body, &key))),
                    }
                }
            };
            parts.push(Part {
                what: spelled(body, &key),
                rest: false,
                ours: ours_of(leaf),
                theirs: whens,
                split,
            });
        }
        // The rest of each part of it that has parts of its own, itself
        // too: rustc's `Drop`s of all of it, or of what of it is under none
        // of those parts, `(_15.0)` of a `Kind` one variant of which moved,
        // as its own flag says.
        let mut inner = Vec::new();
        rustc_mir_dataflow::on_all_children_bits(move_data, path, |child| {
            if move_data.move_paths[child].first_child.is_some() {
                inner.push(child);
            }
        });
        for &node in &inner {
            let key = key_of(node);
            if !self.rest_holds_glue(body, move_data, node) {
                continue;
            }
            let below: Vec<PathKey> = (leaves.iter().chain(&inner))
                .filter(|&&other| other != node)
                .map(|&other| key_of(other))
                .filter(|other| holds(&key, other))
                .collect();
            let rest = (theirs.iter())
                .filter(|t| {
                    t.covers(&key)
                        || holds(&key, &t.place) && !below.iter().any(|part| t.covers(part) || holds(part, &t.place))
                })
                .map(|t| t.when)
                .collect();
            parts.push(Part {
                what: format!("{} (the rest)", spelled(body, &key)),
                rest: true,
                // As the writer drops it: of a variant, by the enum's own
                // state, under its test of the discriminant, as rustc's
                // ladder drops it (`Elaboration::rest_path`).
                ours: ours_of(elaboration.rest_path(node)),
                theirs: rest,
                split: false,
            });
        }
        Ok(parts)
    }
}
