//! rustc's own drop elaboration of the crate's bodies, beside rust-js's
//! (`drops.rs`), compared drop by drop where the session records it,
//! `--drops-shadow` (#2112's change D, step 1): rust-js lowers the built
//! MIR and elaborates its drops itself, of the parts JS drops; rustc's
//! `ElaborateDrops` is crate-private and runs after passes that renumber the
//! blocks, so each `Drop` is found again in rustc's body by where it's
//! written, its span, scope and whether it unwinds, and what of the dropped
//! place each side drops there, never, always or as a flag says, is
//! compared. A `Drop` that can't be found again, or is found twice, is
//! counted apart from a disagreement: `unmatched`.

use super::drops::{Elaboration, Style};
use super::prepared::Mir;
use crate::collections::{IndexMap, IndexSet};
use crate::lower::FnCx;
use crate::lower::drops::Drops;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{
    self, BasicBlock, Local, Operand, Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_mir_dataflow::move_paths::{LookupResult, MovePathIndex};
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

/// Keeps rustc's body of `local`, its drops elaborated, beside rust-js's
/// own of it, where the session compares them (`--drops-shadow`): of a
/// function or a closure, not a coroutine, whose elaborated body is its
/// state machine's.
pub(in crate::lower) fn with_elaborated<'tcx>(tcx: TyCtxt<'tcx>, local: LocalDefId, mir: &mut Mir<'tcx>) {
    if crate::lower::session::caches(tcx).drops_shadow.is_none()
        || !matches!(tcx.def_kind(local), DefKind::Fn | DefKind::AssocFn | DefKind::Closure)
        || tcx.is_coroutine(local.to_def_id())
    {
        return;
    }
    let elaborated = tcx.mir_drops_elaborated_and_const_checked(local);
    if !elaborated.is_stolen() {
        mir.elaborated = Some(elaborated.borrow().clone());
    }
}

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
                let pointer = body.local_decls[local].ty.is_ref() || body.local_decls[local].ty.is_raw_ptr();
                if let Rvalue::Use(Operand::Copy(of), _) | Rvalue::CopyForDeref(of) = &assign.1
                    && pointer
                    && !of.projection.is_empty()
                {
                    entry.1 = Some(*of);
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
    let reachable = mir::traversal::reachable_as_bitset(body);
    let predecessors = body.basic_blocks.predecessors();
    let mut found = Vec::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        let TerminatorKind::Drop { place, .. } = &data.terminator().kind else {
            continue;
        };
        let ty = place.ty(&body.local_decls, tcx).ty;
        if !reachable.contains(block) || !js_drops(ty) {
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
            place: path_key_through(*place, &derefs),
            when: if flagged { When::Flagged } else { When::Always },
            except,
        });
    }
    found
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of each `Drop` of `mir`'s body that drops what JS drops, whether
    /// rust-js's `elaboration` drops each part of it when rustc's does, a
    /// line each, where the session records them (`--drops-shadow`):
    /// `agree`, `disagree` with each part that differs, or `unmatched` with
    /// why it can't be said.
    pub(super) fn shadow_drops(
        &self,
        mir: &Mir<'tcx>,
        elaboration: Option<&Elaboration<'tcx>>,
        quiet_guards: &IndexSet<Local>,
    ) {
        let tcx = self.tcx;
        let Some(lines) = &crate::lower::session::caches(tcx).drops_shadow else {
            return;
        };
        let body = &mir.body;
        // Of a type as rust-js's body has it, normalized where it's lowered:
        // a copied default's rustc body is only instantiated,
        // `<Holder<I> as Iterator>::Item`.
        let js_drops = |ty: Ty<'tcx>| {
            let unnormalized = rustc_middle::ty::Unnormalized::new_wip(ty);
            let ty = (tcx.try_normalize_erasing_regions(self.typing_env, unnormalized)).unwrap_or(ty);
            self.drops(ty) != Drops::Nothing
        };
        let reachable = mir::traversal::reachable_as_bitset(body);
        let sites: Vec<(BasicBlock, Place<'tcx>)> = (body.basic_blocks.iter_enumerated())
            .filter_map(|(block, data)| match &data.terminator().kind {
                TerminatorKind::Drop { place, .. }
                    if reachable.contains(block)
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
        let function = tcx.def_path_str(body.source.def_id());
        let source_map = tcx.sess.source_map();
        let site_of = |block: BasicBlock| {
            let info = body.basic_blocks[block].terminator().source_info;
            (info.span, info.scope, body.basic_blocks[block].is_cleanup)
        };
        let mut out = Vec::new();
        let line = |verdict: &str, block: BasicBlock, detail: &str| {
            let (span, _, cleanup) = site_of(block);
            let at = source_map.span_to_diagnostic_string(span);
            // A cleanup's apart: rust-js's body has no unwinding of what
            // can't throw (ADR 0301), whose cleanups rustc's reaches.
            let path = if cleanup { "cleanup" } else { "normal" };
            format!("{verdict}\t{path}\t{function}\t{at}\t{detail}")
        };
        let (Some(elaborated), Some(elaboration)) = (&mir.elaborated, elaboration) else {
            let why = if mir.elaborated.is_none() {
                "no rustc body"
            } else {
                "no rust-js elaboration"
            };
            out.extend(sites.iter().map(|&(block, _)| line("unmatched", block, why)));
            lines.borrow_mut().extend(out);
            return;
        };
        let theirs = theirs(tcx, elaborated, &js_drops);
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
        for &(block, place) in &sites {
            let site = site_of(block);
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
                    match theirs_when {
                        _ if alike.len() > 1 && multiset() == Some(true) => {
                            out.push(line("agree", block, "behind a pointer, alike drops as a multiset"))
                        }
                        _ if alike.len() > 1 => out.push(line(
                            "unmatched",
                            block,
                            "ambiguous: behind a pointer, drops written alike",
                        )),
                        None => out.push(line(
                            "unmatched",
                            block,
                            "no rustc drop where it's written: behind a pointer",
                        )),
                        Some(theirs_when) if theirs_when == ours => {
                            out.push(line("agree", block, "behind a pointer, whole"))
                        }
                        Some(theirs_when) => out.push(line(
                            "disagree",
                            block,
                            &format!("{key:?} behind a pointer: rust-js {ours:?}, rustc {theirs_when:?}"),
                        )),
                    }
                    continue;
                }
            };
            let parts = self.parts_compared(body, elaboration, block, path, &at_site);
            match parts {
                Err(why) => out.push(line("unmatched", block, &why)),
                Ok((differ, compared)) if differ.is_empty() => {
                    out.push(line("agree", block, &format!("{compared} compared")))
                }
                Ok(_) if alike.len() > 1 && multiset() == Some(true) => out.push(line(
                    "agree",
                    block,
                    &format!("alike drops as a multiset, {}", alike.len()),
                )),
                Ok((differ, _)) if alike.len() > 1 => out.push(line(
                    "unmatched",
                    block,
                    &format!("ambiguous: two drops written alike; {}", differ.join("; ")),
                )),
                Ok((differ, _)) if at_site.is_empty() => out.push(line(
                    "unmatched",
                    block,
                    &format!("no rustc drop where it's written; {}", differ.join("; ")),
                )),
                // A difference only of whether a flag is asked runs alike
                // where the flag holds the other's answer, which only
                // running it says: counted apart.
                Ok((differ, _)) => {
                    let ours = differ.iter().all(|d| d.contains("rust-js Flagged"));
                    let theirs = differ.iter().all(|d| d.contains("rustc Flagged"));
                    let verdict = match (ours, theirs) {
                        // rust-js asks a flag where rustc knows.
                        (true, false) => "flag-ours",
                        // rustc asks one where rust-js knows.
                        (false, true) => "flag-theirs",
                        _ => "disagree",
                    };
                    out.push(line(verdict, block, &differ.join("; ")))
                }
            }
        }
        lines.borrow_mut().extend(out);
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
            rustc_middle::ty::Closure(_, args) => {
                (args.as_closure().upvar_tys().iter().enumerate()).any(|(index, ty)| field(index, ty))
            }
            _ => true,
        }
    }

    /// Whether `place` is a variant of an enum, `(k as Text)`, none of whose
    /// fields JS drops.
    fn variant_drops_nothing(&self, body: &mir::Body<'tcx>, place: Place<'tcx>) -> bool {
        let at = place.ty(&body.local_decls, self.tcx);
        let (Some(variant), rustc_middle::ty::Adt(adt, args)) = (at.variant_index, at.ty.kind()) else {
            return false;
        };
        (adt.variant(variant).fields.iter())
            .all(|field| self.drops(field.ty(self.tcx, args).skip_normalization()) == Drops::Nothing)
    }

    /// Of `path`, dropped where `block` ends, how many of its parts were
    /// compared, and each part rust-js's
    /// elaboration drops otherwise than rustc's `Drop`s there, `theirs`,
    /// do: its leaves, and the rest of it where it has parts of its own.
    fn parts_compared(
        &self,
        body: &mir::Body<'tcx>,
        elaboration: &Elaboration<'tcx>,
        block: BasicBlock,
        path: MovePathIndex,
        theirs: &[&Theirs],
    ) -> Result<(Vec<String>, usize), String> {
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
        let mut differ = Vec::new();
        let mut compared = 0;
        let mut compare = |what: String, ours: When, theirs: When| {
            compared += 1;
            if ours != theirs {
                differ.push(format!("{what}: rust-js {ours:?}, rustc {theirs:?}"));
            }
        };
        for &leaf in &leaves {
            // A variant none of whose fields JS drops, `Text(Vec<u8>)`'s:
            // dropped or not, nothing runs.
            if self.variant_drops_nothing(body, move_data.move_paths[leaf].place) {
                continue;
            }
            let key = key_of(leaf);
            // A `Drop` of it or of what holds it; else those of its parts,
            // all alike.
            let covering = (theirs.iter()).filter(|t| t.covers(&key)).map(|t| t.when).max();
            let theirs_when = match covering {
                Some(when) => when,
                None => {
                    let under: IndexSet<When> = (theirs.iter())
                        .filter(|t| holds(&key, &t.place) && t.place != key)
                        .map(|t| t.when)
                        .collect();
                    match under.len() {
                        0 => When::Never,
                        1 => under[0],
                        _ => return Err(format!("{key:?}: rustc drops its parts unalike")),
                    }
                }
            };
            compare(format!("{key:?}"), ours_of(leaf), theirs_when);
        }
        // The rest of it, where parts of it are paths of their own: rustc's
        // `Drop`s of it, or of what of it is under none of those parts.
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
            let below: Vec<PathKey> = (leaves.iter().chain(&inner))
                .filter(|&&other| other != node)
                .map(|&other| key_of(other))
                .filter(|other| holds(&key, other))
                .collect();
            if !self.rest_drops(body, move_data.move_paths[node].place, &below, 4) {
                continue;
            }
            let rest = (theirs.iter())
                .filter(|t| {
                    t.covers(&key)
                        || holds(&key, &t.place) && !below.iter().any(|part| t.covers(part) || holds(part, &t.place))
                })
                .map(|t| t.when)
                .max()
                .unwrap_or(When::Never);
            compare(format!("{key:?} (the rest)"), ours_of(node), rest);
        }
        Ok((differ, compared))
    }
}
