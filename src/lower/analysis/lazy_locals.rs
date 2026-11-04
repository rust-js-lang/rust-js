//! Which thread-locals are made as their module loads, as JS's `const x =
//! f();` and `export const` are, and which where they're first used, as
//! Rust makes each (the user's decisions, ADR 1006, amended): one is made
//! as its module loads unless that would show (#554). What its initializer
//! runs, `may_run`'s answer, is asked: whether it prints or panics, reads
//! state something other than the loading code writes, or writes state
//! something else touches (`Shows`). React DOM's `const
//! isScrollEndEventSupported = isEventSupported("scrollend")`, which calls
//! bindings alone, is made as its module loads. A private one that would
//! show is made where it's first used, `$local(() => ..)`; a `pub` one is
//! refused, as JS's importers read an `export const`. The pipeline asks
//! too, of the modules' imports, where its module's load can't make one
//! first (`load_cycles::out_of_order`): one that runs what rust-js can't
//! follow, reads itself, or runs a module in an import cycle with its own,
//! made where it's first used, the crate lowered again so
//! (`made_where_used`), or refused. What a thread-local's `with` lends is
//! told apart by its struct's fields (`Place`, #616): React Flight's `const {
//! TaintRegistryObjects, .. } = ReactSharedInternals;` reads fields of the
//! server internals that nothing writes after the load, where functions set
//! its dispatchers'.

use std::collections::{HashMap, HashSet};

use rustc_abi::FieldIdx;
use rustc_hir as hir;
use rustc_middle::mir::TerminatorKind;
use rustc_middle::thir::{ExprId, ExprKind, PatKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

use super::may_run::Runs;
use crate::lower::Body;
use crate::lower::body_queries::strip;
use crate::lower::recognition::{is_local_with, local_key_access, local_key_sets, print_of};

/// The thread-locals made where they're first used, and each other one,
/// made as its module loads, by its key.
pub(in crate::lower) struct LazyLocals {
    pub(in crate::lower) lazy: HashSet<LocalDefId>,
    /// Each made as its module loads, as JS's `const x = f();` is, and what
    /// its initializer may run and read there, all `may_run` finds, the
    /// one rule; `None` where it may run what rust-js can't follow. The pipeline asks, of the
    /// modules' imports, where its module's load can't make it
    /// (`load_cycles::out_of_order`, ADR 1006, amended).
    pub(in crate::lower) at_load: HashMap<LocalDefId, Option<Vec<DefId>>>,
}

/// The thread-locals, by their keys, of `inits`, each initializer's key:
/// those made where they're first used already, `made_where_used`, which a
/// lowering of the crate found their module's load can't make; each whose
/// initializer would show it's made as its module loads (`shows`), made
/// where it's first used, a `pub` one refused; and each other, made as its
/// module loads, and what it may run and read there, as `runs` finds of
/// `bodies`.
pub(super) fn lazy_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    (inits, made_where_used): (&HashMap<LocalDefId, LocalDefId>, &HashSet<LocalDefId>),
    (runs, bodies): (&Runs<'_, 'tcx>, &[&Body<'tcx>]),
    counted: &super::Counted<'tcx>,
) -> LazyLocals {
    let mut found = LazyLocals {
        lazy: made_where_used.clone(),
        at_load: HashMap::new(),
    };
    // What each initializer may run and read as its module loads, all
    // `may_run` finds, which follows each thread-local it reads to its
    // own initializer: the one rule (the user's decision, rd120).
    let mut keyed: Vec<(LocalDefId, LocalDefId)> = (inits.iter())
        .filter(|&(&init, key)| !made_where_used.contains(key) && runs.body(init.to_def_id()).is_some())
        .map(|(&init, &key)| (init, key))
        .collect();
    keyed.sort_by_key(|&(_, key)| tcx.def_span(key));
    for &(init, key) in &keyed {
        found.at_load.insert(key, runs.of(init.to_def_id()));
    }
    // Each that would show it's made as its module loads, made where it's
    // first used, until none more is: a body only it ran runs outside the
    // load then, and what that touches may make another show (#554).
    let initializers: HashSet<DefId> = inits.keys().map(|init| init.to_def_id()).collect();
    let mut touches = Touches::of(tcx, (bodies, counted), &initializers);
    // What a reference copied out of state may be held by too: each
    // thread-local made of one, and each state a loading body writes one to.
    let made_by: Vec<(DefId, Vec<DefId>)> = (keyed.iter())
        .map(|&(init, key)| {
            let ran = std::iter::once(init.to_def_id()).chain(found.at_load[&key].iter().flatten().copied());
            (key.to_def_id(), ran.collect())
        })
        .chain(
            (bodies.iter())
                .filter(|body| crate::lower::bindings::is_on_load(tcx, body.def_id))
                .map(|body| {
                    (
                        body.def_id,
                        std::iter::once(body.def_id)
                            .chain(runs.of(body.def_id).into_iter().flatten())
                            .collect(),
                    )
                }),
        )
        .collect();
    touches.join_copies(&made_by);
    // Each function only calls reach, by its calls (ADR 0577).
    let only_called = super::only_called::only_called(tcx);
    loop {
        let loading = loading(tcx, &keyed, &found.lazy, (bodies, &only_called));
        let outside = Outside::of(&touches, &loading);
        // A namespace thread-local (ADR 1061) is its module's exports, which
        // only JS reads, so its first use is its module's load: made then,
        // it shows nothing Rust would order otherwise.
        let shown: Vec<(LocalDefId, Shows)> = (keyed.iter())
            .filter(|(_, key)| !found.lazy.contains(key))
            .filter(|(_, key)| !crate::lower::classes::is_value_namespace(tcx, key.to_def_id()))
            .filter_map(|&(init, key)| {
                let reached = found.at_load[&key].as_deref()?;
                Some((key, shows(&touches, &outside, init.to_def_id(), reached)?))
            })
            .collect();
        if shown.is_empty() {
            break;
        }
        for (key, why) in shown {
            found.lazy.insert(key);
            found.at_load.remove(&key);
            if tcx.visibility(key).is_public() {
                refuse_exported(tcx, key, why);
            }
        }
    }
    found
}

/// What a body touches of the crate's state, a thread-local of what may
/// change, a `static mut` or a `static` of interior mutability (`changes`),
/// and whether it prints or panics, which Rust orders.
struct Touched {
    /// What it reads or writes.
    reads: Vec<Place>,
    /// What it may write: a thread-local it `set`s, `take`s, `replace`s or
    /// lends, by `with_borrow_mut` or `with`, or hands on; a `static mut` it
    /// writes or takes a pointer to; any `static` of interior mutability.
    /// Of a `with`, the fields its closure reads of what it lends, each
    /// written, as a cell may be (`fields_lent`).
    writes: Vec<Place>,
    /// Whether it prints, or calls what never returns, a `panic!`'s.
    ordered: Option<Ordered>,
    /// The states whose read may give out a reference to what may change,
    /// which what it's given to may hold (#635): `PTR.get()`'s `&'static
    /// S`, `A.with(|a| *a)`, a `static`'s own, `&COUNTER`.
    copies: Vec<DefId>,
}

/// A place of the crate's state, by its name: the whole of it, or, of a
/// thread-local, one field of the struct it holds, through the references
/// it holds, as its `with` lends it, `registry` of `INTERNALS.with(|i|
/// i.registry.get())` (#616); and whether what that field points to may be
/// read or written too (`through`): not of a `Cell`'s `set`, nor its `get`
/// or `replace` of what points to nothing that may change, which read or
/// write the field alone.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Place {
    state: DefId,
    field: Option<FieldIdx>,
    through: bool,
}

impl Place {
    fn whole(state: DefId) -> Self {
        Place {
            state,
            field: None,
            through: true,
        }
    }
}

/// How a `with`'s closure uses a field of what it's lent (`fields_lent`):
/// whether it writes it, whether it gives out what it holds, a `Cell`'s
/// `get` or `replace`, and whether it lends it, to any other call or
/// place, which may read and write it and what it points to.
#[derive(Clone, Copy, Default)]
struct FieldUse {
    writes: bool,
    gives: bool,
    lends: bool,
    /// Whether it's an initializer's value as it is (`field_as_is`), which
    /// the thread-local holds then, read alone.
    as_is: bool,
}

/// A field of what a struct's field holds: its type, and what it may point
/// to that may change, which another field may point to too (`pointees`).
struct FieldOf<'tcx> {
    ty: Ty<'tcx>,
    pointees: Option<HashSet<Ty<'tcx>>>,
}

/// What a body does that Rust orders, where it's run.
#[derive(Clone, Copy)]
enum Ordered {
    /// `println!` and the like, std's `_print` or `_eprint`.
    Prints,
    /// A call of what never returns: a `panic!`, an `assert!`'s failure,
    /// `unreachable!()`, `process::exit`.
    Panics,
}

/// Each body's `Touched`, by its id, and the bodies in the crate's order;
/// each field a `with` lends, by its state and field; and what each place
/// touched may share with another state's (`aliases`).
struct Touches<'tcx> {
    order: Vec<DefId>,
    of: HashMap<DefId, Touched>,
    fields: HashMap<(DefId, FieldIdx), FieldOf<'tcx>>,
    aliases: HashMap<Place, Aliases<'tcx>>,
    /// Each state that may hold a reference another holds or is, by the
    /// first of its kin (`join_copies`); one that's apart isn't here.
    kin: HashMap<DefId, DefId>,
}

/// What a place may share with another state's place, by the types that
/// aren't `Freeze` of what it touches (#635): `None` where it may be
/// anything. Two states' places are one where what one reaches through a
/// pointer may be what the other does, or a `static`'s own: a `&'static S`
/// two thread-locals hold, or one to an atomic `static`.
#[derive(Default)]
struct Aliases<'tcx> {
    /// Of a `static`, what it holds itself, which a reference may point to.
    named: Option<HashSet<Ty<'tcx>>>,
    /// What it reaches through a pointer: of a field of a struct its state
    /// holds a reference to, the field; and what it points to, where that's
    /// read or written (`Place::through`).
    reached: Option<HashSet<Ty<'tcx>>>,
}

impl<'tcx> Aliases<'tcx> {
    /// Of `place`, of `fields`, the types of the fields `with`s lend.
    fn of(tcx: TyCtxt<'tcx>, fields: &HashMap<(DefId, FieldIdx), FieldOf<'tcx>>, place: Place) -> Self {
        let none = || Some(HashSet::new());
        let Some((held, named)) = held_of(tcx, place.state) else {
            return Aliases {
                named: none(),
                reached: none(),
            };
        };
        match place.field {
            None => Aliases {
                named: if named { reach(tcx, held, true, false) } else { none() },
                reached: reach(tcx, held, false, place.through),
            },
            Some(field) => Aliases {
                named: none(),
                reached: reach(
                    tcx,
                    fields[&(place.state, field)].ty,
                    behind_reference(held),
                    place.through,
                ),
            },
        }
    }

    /// Whether it may share anything with another state's place.
    fn shares(&self) -> bool {
        let some = |types: &Option<HashSet<Ty<'tcx>>>| types.as_ref().is_none_or(|types| !types.is_empty());
        some(&self.named) || some(&self.reached)
    }

    /// Whether it and `other`, another state's, may be one place.
    fn meet(&self, other: &Self) -> bool {
        let meet = |a: &Option<HashSet<Ty<'tcx>>>, b: &Option<HashSet<Ty<'tcx>>>| match (a, b) {
            (Some(a), Some(b)) => !a.is_disjoint(b),
            (None, Some(b)) | (Some(b), None) => !b.is_empty(),
            (None, None) => true,
        };
        meet(&self.reached, &other.reached) || meet(&self.reached, &other.named) || meet(&self.named, &other.reached)
    }
}

/// Whether the struct `held` is, or holds, is reached through a reference:
/// a `&'static S`'s, which another state may hold too, not a `Box`'s.
fn behind_reference(mut held: Ty<'_>) -> bool {
    let mut behind = false;
    loop {
        match held.kind() {
            ty::Ref(_, pointee, _) => {
                behind = true;
                held = *pointee;
            }
            ty::Adt(adt, args) if adt.is_box() => held = args.type_at(0),
            _ => return behind,
        }
    }
}

impl<'tcx> Touches<'tcx> {
    /// Of `bodies`, `initializers` the thread-locals' initializers among them.
    fn of(
        tcx: TyCtxt<'tcx>,
        (bodies, counted): (&[&Body<'tcx>], &super::Counted<'tcx>),
        initializers: &HashSet<DefId>,
    ) -> Self {
        let mut state: HashMap<DefId, bool> = HashMap::new();
        let mut changing = |id: DefId| *state.entry(id).or_insert_with(|| changes(tcx, id));
        let by_id: HashMap<DefId, &Body<'tcx>> = bodies.iter().map(|body| (body.def_id, *body)).collect();
        let mut found = Touches {
            order: bodies.iter().map(|body| body.def_id).collect(),
            of: HashMap::new(),
            fields: HashMap::new(),
            aliases: HashMap::new(),
            kin: HashMap::new(),
        };
        for body in bodies {
            let thir = &body.thir;
            // The state an initializer's value is as it is, read by its name:
            // its own memory alone is read there, what it points to where
            // the thread-local is read, the same at the load as later.
            let given_as_is = (initializers.contains(&body.def_id))
                .then(|| body.root().and_then(|root| given_as_is(tcx, thir, root)))
                .flatten();
            let mut touched = Touched {
                reads: Vec::new(),
                writes: Vec::new(),
                ordered: None,
                copies: Vec::new(),
            };
            // Each thread-local read where it's only read, `KEY` of
            // `KEY.get()` or `KEY.with_borrow(f)`, and each written, by a
            // method that sets it or lends its cell.
            let mut read_only = HashSet::new();
            for expr in thir.exprs.iter() {
                if let ExprKind::Call { fun, ref args, .. } = expr.kind
                    && let Some((id, _)) = crate::lower::fn_def(thir[fun].ty)
                    && (local_key_access(tcx, id).is_some() || is_local_with(tcx, id))
                    && !local_key_sets(tcx, id)
                    && let Some(&first) = args.first()
                    && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
                {
                    read_only.insert(strip(thir, arg));
                }
            }
            // Each thread-local a `with` lends its closure, and the fields of
            // its struct the closure reads alone, where it reads no more.
            let mut lent: HashMap<ExprId, Vec<(FieldIdx, Ty<'tcx>, FieldUse)>> = HashMap::new();
            for expr in thir.exprs.iter() {
                if let ExprKind::Call { fun, ref args, .. } = expr.kind
                    && let Some((id, _)) = crate::lower::fn_def(thir[fun].ty)
                    && is_local_with(tcx, id)
                    && let [key, f] = args[..]
                    && let ExprKind::Borrow { arg: key, .. } = thir[strip(thir, key)].kind
                    && let ExprKind::Closure(ref closure) = thir[strip(thir, f)].kind
                    && let Some(closure) = by_id.get(&closure.closure_id.to_def_id())
                    && let Some(fields) = fields_lent(
                        tcx,
                        counted,
                        &closure.thir,
                        given_as_is == Some(AsIs::Lent(strip(thir, key))),
                    )
                {
                    lent.insert(strip(thir, key), fields);
                }
            }
            for (at, expr) in thir.exprs.iter_enumerated() {
                let written = match expr.kind {
                    ExprKind::NamedConst { def_id, .. } | ExprKind::StaticRef { def_id, .. }
                        if def_id.is_local() && changing(def_id) =>
                    {
                        let Some(fields) = lent.get(&at) else {
                            let whole = Place {
                                through: given_as_is != Some(AsIs::Read(at)),
                                ..Place::whole(def_id)
                            };
                            touched.reads.push(whole);
                            // A `static mut` is written where it's assigned,
                            // below; anything else that isn't only read may be.
                            // And what a read gives out may be written
                            // through, `PTR.get()`'s `&'static S` (#635).
                            let points = (found.aliases.entry(whole))
                                .or_insert_with(|| Aliases::of(tcx, &HashMap::new(), whole))
                                .reached
                                .as_ref()
                                .is_none_or(|reached| !reached.is_empty());
                            let gives_out = points && whole.through;
                            // What's read as it is gives it out too.
                            let copied = held_of(tcx, def_id).is_some_and(|(held, named)| {
                                named || pointees(tcx, held).is_none_or(|pointees| !pointees.is_empty())
                            });
                            if copied {
                                touched.copies.push(def_id);
                            }
                            if gives_out || (!read_only.contains(&at) && !tcx.is_mutable_static(def_id)) {
                                touched.writes.push(whole);
                            }
                            continue;
                        };
                        // Lent to a closure that reads none of it, it's read,
                        // made if it wasn't.
                        if fields.is_empty() {
                            touched.reads.push(Place::whole(def_id));
                        }
                        for &(field, ty, used) in fields {
                            let of = (found.fields.entry((def_id, field))).or_insert_with(|| FieldOf {
                                ty: tcx.erase_and_anonymize_regions(ty),
                                pointees: pointees(tcx, ty),
                            });
                            // What it gives out may be written through later.
                            let points = of.pointees.as_ref().is_none_or(|pointees| !pointees.is_empty());
                            let through = used.lends || (used.gives && points);
                            let place = Place {
                                state: def_id,
                                field: Some(field),
                                through,
                            };
                            touched.reads.push(place);
                            if used.writes || through {
                                touched.writes.push(place);
                            }
                            // A reference to it, or to what it points to,
                            // given out, may be held by what it's given to.
                            let behind = held_of(tcx, def_id).is_some_and(|(held, _)| behind_reference(held));
                            if (used.lends || used.gives || used.as_is) && (points || behind) {
                                touched.copies.push(def_id);
                            }
                        }
                        continue;
                    }
                    ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } => lhs,
                    ExprKind::RawBorrow { arg, .. } => arg,
                    ExprKind::Borrow {
                        borrow_kind: rustc_middle::mir::BorrowKind::Mut { .. },
                        arg,
                    } => arg,
                    _ => continue,
                };
                if let Some(def_id) = super::validation::static_of(thir, written)
                    && def_id.is_local()
                    && changing(def_id)
                {
                    touched.writes.push(Place::whole(def_id));
                }
            }
            if let Some(mir) = &body.mir {
                for block in mir.body.basic_blocks.iter().filter(|block| !block.is_cleanup) {
                    let TerminatorKind::Call { func, target, .. } = &block.terminator().kind else {
                        continue;
                    };
                    let printed = func.const_fn_def().is_some_and(|(f, _)| print_of(tcx, f).is_some());
                    touched.ordered = touched.ordered.or(match () {
                        _ if printed => Some(Ordered::Prints),
                        _ if target.is_none() => Some(Ordered::Panics),
                        _ => None,
                    });
                }
            }
            found.of.insert(body.def_id, touched);
        }
        let places: Vec<Place> = (found.of.values())
            .flat_map(|touched| touched.reads.iter().chain(&touched.writes))
            .copied()
            .collect();
        for place in places {
            if !found.aliases.contains_key(&place) {
                let aliases = Aliases::of(tcx, &found.fields, place);
                found.aliases.insert(place, aliases);
            }
        }
        found
    }

    /// Whether `a` and `b` may be one place: of one state, the whole of it
    /// either, or one field; or two fields one of which may point to the
    /// other, or to what the other points to, where what they point to is
    /// read or written, `Rc`s of one cell.
    fn overlap(&self, a: Place, b: Place) -> bool {
        if a.state != b.state {
            return self.kin_of(a.state) == self.kin_of(b.state) && self.aliases[&a].meet(&self.aliases[&b]);
        }
        let (Some(f), Some(g)) = (a.field, b.field) else {
            return true;
        };
        if f == g {
            return true;
        }
        let (f, g) = (&self.fields[&(a.state, f)], &self.fields[&(b.state, g)]);
        // Whether what `from` points to may be `to`, or what `to` points to,
        // where that's read or written, `to_through`.
        let reaches = |from: &FieldOf<'tcx>, to: &FieldOf<'tcx>, to_through: bool| match &from.pointees {
            None => true,
            Some(from) => {
                from.contains(&to.ty)
                    || (to_through
                        && match &to.pointees {
                            None => !from.is_empty(),
                            Some(to) => !from.is_disjoint(to),
                        })
            }
        };
        (a.through && reaches(f, g, b.through)) || (b.through && reaches(g, f, a.through))
    }
}

impl Touches<'_> {
    /// The first of `state`'s kin, itself where it has none.
    fn kin_of(&self, state: DefId) -> DefId {
        self.kin.get(&state).copied().unwrap_or(state)
    }

    /// Of `made_by`, each thread-local's key with what its initializer
    /// runs, and each `js::on_load!` with what it runs: each state whose
    /// read may give out a reference (`Touched::copies`) is kin with what's
    /// made of it, the key, and with what the same bodies write, which may
    /// hold the reference: `static B: &'static S = A.with(|a| *a)`'s `B` and
    /// `A` (#635). A body that may run after the load writes what it copies
    /// out of, so only those the modules' load runs are asked; all are, as
    /// which are made then changes.
    fn join_copies(&mut self, made_by: &[(DefId, Vec<DefId>)]) {
        let mut kin: HashMap<DefId, DefId> = HashMap::new();
        fn first(kin: &HashMap<DefId, DefId>, mut state: DefId) -> DefId {
            while let Some(&up) = kin.get(&state).filter(|&&up| up != state) {
                state = up;
            }
            state
        }
        for (made, ran) in made_by {
            let touched = || ran.iter().filter_map(|id| self.of.get(id));
            let copies: Vec<DefId> = touched().flat_map(|touched| touched.copies.iter().copied()).collect();
            if copies.is_empty() {
                continue;
            }
            let holders = (touched().flat_map(|touched| touched.writes.iter().map(|place| place.state)))
                .chain(std::iter::once(*made));
            for holder in holders.chain(copies.iter().copied()).collect::<Vec<_>>() {
                let (a, b) = (first(&kin, copies[0]), first(&kin, holder));
                if a != b {
                    kin.insert(b, a);
                }
            }
        }
        let states: Vec<DefId> = kin.keys().copied().collect();
        self.kin = states.into_iter().map(|state| (state, first(&kin, state))).collect();
    }
}

/// The state `root`, a body's value, is as it is, read by its name, where
/// it's that: `*SERVER_INTERNALS`, `PTR.get()`, or an `Option`'s `expect`
/// or `unwrap` of one, React's `SERVER_INTERNALS.expect(..)`, reborrowed or
/// not; or the thread-local whose `with` gives the closure's value, which
/// `fields_lent` asks of the field it's of (#635).
fn given_as_is<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, root: ExprId) -> Option<AsIs> {
    let mut e = root;
    let mut lent = false;
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::NamedConst { .. } | ExprKind::StaticRef { .. } if lent => return Some(AsIs::Lent(e)),
            ExprKind::NamedConst { .. } | ExprKind::StaticRef { .. } => return Some(AsIs::Read(e)),
            ExprKind::Deref { arg } => e = arg,
            // A reborrow, `&*`, of what it reads.
            ExprKind::Borrow {
                borrow_kind: rustc_middle::mir::BorrowKind::Shared,
                arg,
            } if matches!(thir[strip(thir, arg)].kind, ExprKind::Deref { .. }) => e = arg,
            ExprKind::Block { block } if thir[block].stmts.is_empty() => e = thir[block].expr?,
            ExprKind::Call { fun, ref args, .. } => {
                let (id, _) = crate::lower::fn_def(thir[fun].ty)?;
                let key_get = crate::lower::recognition::local_key_cell_method(tcx, id)
                    == Some(crate::lower::recognition::LocalKeyCellMethod::Get);
                // A `with` whose closure's value is a field as it is
                // (`fields_lent`'s `as_is`), or reads none of it.
                let with = crate::lower::recognition::is_local_with(tcx, id);
                if key_get || (with && !lent) {
                    let ExprKind::Borrow { arg, .. } = thir[strip(thir, *args.first()?)].kind else {
                        return None;
                    };
                    lent = with;
                    e = arg;
                } else if crate::lower::body_queries::is_option_unwrap(tcx, thir[fun].ty) {
                    e = *args.first()?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
}

/// What an initializer's value is as it is (`given_as_is`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AsIs {
    /// The state read by its name there.
    Read(ExprId),
    /// What a `with` of the thread-local named there gives its closure.
    Lent(ExprId),
}

/// The field `root`, a closure's value, is as it is: a copy of it, a
/// `Cell`'s `get` of it, or a clone of it, or of what a `RefCell`'s
/// `borrow` of it holds, that's the value itself (`clones_itself`: an
/// `Option` of an `Rc` whose counts nothing reads, React Flight's
/// `internals.taint_registry_pending_requests.borrow().clone()`).
fn field_as_is<'tcx>(
    tcx: TyCtxt<'tcx>,
    counted: &super::Counted<'tcx>,
    thir: &Thir<'tcx>,
    root: ExprId,
) -> Option<ExprId> {
    use crate::lower::recognition::{CellMethod, HandleStep, cell_own_method, handle_step};
    let lent = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Borrow { arg, .. } => Some(strip(thir, arg)),
        _ => None,
    };
    let mut e = strip(thir, root);
    if let ExprKind::Block { block } = thir[e].kind {
        e = strip(thir, thir[block].stmts.is_empty().then_some(thir[block].expr?)?);
    }
    let mut cloned = false;
    loop {
        match thir[e].kind {
            ExprKind::Field { .. } => return Some(e),
            ExprKind::Deref { arg } if cloned => e = strip(thir, arg),
            ExprKind::Call { fun, ref args, .. } => {
                let (id, fn_args) = crate::lower::fn_def(thir[fun].ty)?;
                let first = *args.first()?;
                if cell_own_method(tcx, id) == Some(CellMethod::Get) && !cloned {
                    return lent(first).filter(|&field| matches!(thir[field].kind, ExprKind::Field { .. }));
                }
                match handle_step(tcx, id, fn_args) {
                    _ if tcx.is_lang_item(id, rustc_hir::attrs::lang_items::LangItem::CloneFn) && !cloned => {
                        let ty = fn_args.types().next()?;
                        if !crate::lower::analysis::mutation::clones_itself(tcx, counted, ty) {
                            return None;
                        }
                        cloned = true;
                        e = lent(first)?;
                    }
                    Some(HandleStep::GuardDeref | HandleStep::Borrow) if cloned => e = lent(first)?,
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
}

/// Of `thir`, the body of a closure a thread-local's `with` is given, the
/// fields it reads of the struct it's lent, each with its type and how it's
/// used, where it reads nothing else of it: each use of its parameter,
/// `|internals|` or `|&internals|`, is a field of what it points to,
/// `internals.registry`, through its references alone. `None` where one
/// isn't, which may change any field: the struct lent whole, to a function,
/// a method, a binding or a closure that captures it, or a reference to it
/// copied out (#616).
fn fields_lent<'tcx>(
    tcx: TyCtxt<'tcx>,
    counted: &super::Counted<'tcx>,
    thir: &Thir<'tcx>,
    as_is: bool,
) -> Option<Vec<(FieldIdx, Ty<'tcx>, FieldUse)>> {
    use crate::lower::recognition::{CellMethod, cell_own_method};
    // Its own parameter, after the closure's environment, which has no pattern.
    let [param] = &(thir.params.iter())
        .filter_map(|param| param.pat.as_deref())
        .collect::<Vec<_>>()[..]
    else {
        return None;
    };
    let named = match &param.kind {
        PatKind::Deref { subpattern, .. } => &subpattern.kind,
        kind => kind,
    };
    let PatKind::Binding {
        var, subpattern: None, ..
    } = *named
    else {
        return None;
    };
    // Each place a `Cell`'s `get`, `set` or `replace` is given, and how
    // it's used: read or written, and whether what it held is given out.
    let celled: HashMap<ExprId, FieldUse> = (thir.exprs.iter())
        .filter_map(|expr| {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                return None;
            };
            let writes = match cell_own_method(tcx, crate::lower::fn_def(thir[fun].ty)?.0)? {
                CellMethod::Get => false,
                CellMethod::Writes => true,
                _ => return None,
            };
            let ExprKind::Borrow { arg, .. } = thir[strip(thir, *args.first()?)].kind else {
                return None;
            };
            let used = FieldUse {
                writes,
                gives: !expr.ty.is_unit(),
                ..FieldUse::default()
            };
            Some((strip(thir, arg), used))
        })
        .collect();
    // Of an initializer's `with` whose closure's value is a field as it is,
    // the field: its own memory alone is read there (#635).
    let given = as_is
        .then(|| {
            thir.exprs
                .last_index()
                .and_then(|root| field_as_is(tcx, counted, thir, root))
        })
        .flatten();
    let mut fields: Vec<(FieldIdx, Ty<'tcx>, FieldUse)> = Vec::new();
    let mut read_through = HashSet::new();
    for (at, expr) in thir.exprs.iter_enumerated() {
        let ExprKind::Field { lhs, name, .. } = expr.kind else {
            continue;
        };
        let mut base = strip(thir, lhs);
        while let ExprKind::Deref { arg } = thir[base].kind {
            base = strip(thir, arg);
        }
        if !matches!(thir[base].kind, ExprKind::VarRef { id } if id == var) {
            continue;
        }
        // A struct's or a tuple's fields are apart; a union's aren't.
        let apart = match thir[lhs].ty.kind() {
            ty::Adt(adt, _) => adt.is_struct(),
            ty::Tuple(_) => true,
            _ => false,
        };
        if !apart {
            return None;
        }
        read_through.insert(base);
        let lent = FieldUse {
            writes: true,
            gives: true,
            lends: true,
            as_is: false,
        };
        let used = match given == Some(at) {
            true => FieldUse {
                as_is: true,
                ..FieldUse::default()
            },
            false => celled.get(&at).copied().unwrap_or(lent),
        };
        match fields.iter_mut().find(|(field, _, _)| *field == name) {
            Some((_, _, was)) => {
                was.writes |= used.writes;
                was.gives |= used.gives;
                was.lends |= used.lends;
                was.as_is |= used.as_is;
            }
            None => fields.push((name, expr.ty, used)),
        }
    }
    let other_use = (thir.exprs.iter_enumerated())
        .any(|(at, expr)| matches!(expr.kind, ExprKind::VarRef { id } if id == var) && !read_through.contains(&at));
    (!other_use).then_some(fields)
}

/// What a value of `ty` may point to that may change, which another value
/// may point to too: `reach`'s, of its own memory not, of what it points
/// to.
fn pointees<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<HashSet<Ty<'tcx>>> {
    reach(tcx, ty, false, true)
}

/// The types that aren't `Freeze` of what a value of `ty` holds that may be
/// pointed to: of its own memory where it's reached through a pointer,
/// `behind`, and of what it reaches through a reference, an `Rc`, an `Arc`
/// or a `Weak`, where that's asked, `through`; through the fields of
/// structs and enums and the type arguments of std's. `None` where that
/// may be anything: a raw pointer, which may be cast, a `dyn` or a closure,
/// whose captures its type doesn't name, or a type rustc doesn't say.
fn reach<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, behind: bool, through: bool) -> Option<HashSet<Ty<'tcx>>> {
    use crate::lower::recognition::{is_std_crate, rc_pointee, weak_pointee};
    let typing_env = ty::TypingEnv::fully_monomorphized();
    let mut found = HashSet::new();
    let mut todo = vec![(ty, behind)];
    let mut seen = HashSet::new();
    while let Some((ty, behind)) = todo.pop() {
        let ty = tcx.erase_and_anonymize_regions(ty);
        if !seen.insert((ty, behind)) {
            continue;
        }
        if behind && !ty.is_freeze(tcx, typing_env) {
            found.insert(ty);
        }
        if let Some(pointee) = rc_pointee(tcx, ty).or_else(|| weak_pointee(tcx, ty)) {
            if through {
                todo.push((pointee, true));
            }
            continue;
        }
        if crate::lower::pointers::raw_pointee(tcx, ty).is_some() {
            if through {
                return None;
            }
            continue;
        }
        match *ty.kind() {
            ty::Dynamic(..) | ty::Closure(..) | ty::Param(..) | ty::Alias(..) if through || behind => return None,
            ty::Ref(_, pointee, _) if through => todo.push((pointee, true)),
            // A marker holds nothing: a JS object's, `WeakMap`'s.
            ty::Adt(adt, _) if adt.is_phantom_data() => {}
            ty::Adt(adt, args) if is_std_crate(tcx, adt.did()) => todo.extend(args.types().map(|ty| (ty, behind))),
            ty::Adt(adt, args) => {
                todo.extend((adt.all_fields()).map(|field| (field.ty(tcx, args).skip_normalization(), behind)));
            }
            ty::Array(item, _) | ty::Slice(item) => todo.push((item, behind)),
            ty::Tuple(items) => todo.extend(items.iter().map(|ty| (ty, behind))),
            _ => {}
        }
    }
    Some(found)
}

/// What `id`, a thread-local's key or a `static`, holds, and whether it's
/// a `static`, whose own memory a reference may point to; `None` of
/// anything else.
fn held_of(tcx: TyCtxt<'_>, id: DefId) -> Option<(Ty<'_>, bool)> {
    use rustc_hir::def::DefKind;
    let ty = || tcx.type_of(id).instantiate_identity().skip_normalization();
    match tcx.def_kind(id) {
        DefKind::Static { .. } => Some((ty(), true)),
        DefKind::Const { .. } => match ty().kind() {
            ty::Adt(key, args)
                if crate::lower::recognition::is_std_def(
                    tcx,
                    key.did(),
                    crate::lower::recognition::StdItem::LocalKey,
                ) =>
            {
                Some((args.type_at(0), false))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Whether `id`, a thread-local's key or a `static`, is state that may
/// change: a `static mut`; a `static` or a thread-local of what has a
/// cell, through the fields of the crate's types, the type arguments of
/// std's, and what a reference or a pointer points to; a mutable raw
/// pointer. Not a function, a closure or a `dyn`, which holds no state
/// that's named, nor a JS value, a binding's type, `&'static
/// Class<fn(Rc<Fiber>)>`, of what its own fields hold: its JS object a
/// binding's call may change, as what bindings do as a module loads is (the
/// guide's difference 25).
fn changes(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let typing_env = ty::TypingEnv::fully_monomorphized();
    if tcx.is_mutable_static(id) {
        return true;
    }
    let Some((held, _)) = held_of(tcx, id) else {
        return false;
    };
    let mut todo: Vec<Ty<'_>> = vec![held];
    let mut seen = HashSet::new();
    while let Some(ty) = todo.pop() {
        if !seen.insert(ty) {
            continue;
        }
        if let Some((pointee, mutability)) = crate::lower::pointers::raw_pointee(tcx, ty) {
            if mutability.is_mut() {
                return true;
            }
            todo.push(pointee);
            continue;
        }
        match *ty.kind() {
            ty::Adt(adt, args) if adt.did().is_local() => {
                todo.extend(adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization()));
            }
            ty::Adt(..) if !ty.is_freeze(tcx, typing_env) => return true,
            ty::Adt(adt, args) if crate::lower::recognition::is_std_crate(tcx, adt.did()) => todo.extend(args.types()),
            ty::Ref(_, pointee, _) | ty::Array(pointee, _) | ty::Slice(pointee) => todo.push(pointee),
            ty::Tuple(items) => todo.extend(items.iter()),
            _ => {}
        }
    }
    false
}

/// The bodies that run only as the modules load: each thread-local's
/// initializer made then, of `keyed`'s whose key isn't `lazy`, each
/// `js::on_load!`'s; a function only calls reach, of `only_called`, each
/// of whose calls is in one of those; and a closure one of those gives std
/// to run there (`run_in_place`).
fn loading<'tcx>(
    tcx: TyCtxt<'tcx>,
    keyed: &[(LocalDefId, LocalDefId)],
    lazy: &HashSet<LocalDefId>,
    (bodies, only_called): (&[&Body<'tcx>], &HashMap<DefId, Vec<hir::HirId>>),
) -> HashSet<DefId> {
    let mut loading: HashSet<DefId> = (keyed.iter())
        .filter(|(_, key)| !lazy.contains(key))
        .map(|(init, _)| init.to_def_id())
        .chain(
            (bodies.iter())
                .map(|body| body.def_id)
                .filter(|&id| crate::lower::bindings::is_on_load(tcx, id)),
        )
        .collect();
    let in_place: Vec<(DefId, DefId)> = (bodies.iter())
        .flat_map(|body| {
            run_in_place(tcx, body)
                .into_iter()
                .map(|closure| (closure, body.def_id))
        })
        .collect();
    loop {
        let before = loading.len();
        for (&f, calls) in only_called {
            if !loading.contains(&f)
                && (calls.iter()).all(|&call| loading.contains(&tcx.hir_enclosing_body_owner(call).to_def_id()))
            {
                loading.insert(f);
            }
        }
        for &(closure, at) in &in_place {
            if loading.contains(&at) {
                loading.insert(closure);
            }
        }
        if loading.len() == before {
            return loading;
        }
    }
}

/// Each closure `body` gives std's call, made there, that the call runs
/// before it returns, or never, keeping none of it: one whose type is in
/// neither what the call gives back nor what else it's given, as a
/// thread-local's `with(|x| ..)`'s or `Option::map(|x| ..)`'s is, where an
/// iterator's `map` keeps it in what it gives, and a `Vec`'s `push` in what
/// it's given. Std runs it, if at all, where `body` calls it.
fn run_in_place<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<DefId> {
    let thir = &body.thir;
    let mut found = Vec::new();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let Some((id, fn_args)) = crate::lower::fn_def(thir[fun].ty) else {
            continue;
        };
        if !crate::lower::recognition::is_std_crate(tcx, id) {
            continue;
        }
        let sig =
            tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate(tcx, fn_args).skip_normalization());
        for (at, &arg) in args.iter().enumerate() {
            let ExprKind::Closure(ref closure) = thir[strip(thir, arg)].kind else {
                continue;
            };
            let closure = closure.closure_id.to_def_id();
            let holds = |ty: Ty<'tcx>| {
                ty.walk().any(
                    |part| matches!(part.as_type().map(|part| *part.kind()), Some(ty::Closure(c, _)) if c == closure),
                )
            };
            let kept = holds(sig.output())
                || (sig.inputs().iter().enumerate()).any(|(other, &input)| other != at && holds(input));
            if !kept {
                found.push(closure);
            }
        }
    }
    found
}

/// What the bodies that may run outside the modules' load touch: each
/// place of state they may write, by the first that does, and each they
/// read or write, by the first that does.
struct Outside {
    writes: Placed,
    touches: Placed,
}

/// Places, each by the first body that touches it: by their state, and,
/// of those that may share with another state's, in the bodies' order.
#[derive(Default)]
struct Placed {
    by_state: HashMap<DefId, Vec<(Place, DefId)>>,
    shared: Vec<(Place, DefId)>,
}

impl Placed {
    fn add(&mut self, touches: &Touches<'_>, place: Place, by: DefId) {
        let places = self.by_state.entry(place.state).or_default();
        if !places.iter().any(|&(was, _)| was == place) {
            places.push((place, by));
            if touches.aliases[&place].shares() {
                self.shared.push((place, by));
            }
        }
    }

    /// The first body that touches a place that may be `place`: of its
    /// state, else of another's that may share with it (#635).
    fn by(&self, touches: &Touches<'_>, place: Place) -> Option<DefId> {
        let of_state = || {
            (self.by_state.get(&place.state)?.iter())
                .find(|&&(other, _)| touches.overlap(place, other))
                .map(|&(_, by)| by)
        };
        of_state().or_else(|| {
            (touches.aliases[&place].shares())
                .then(|| {
                    (self.shared.iter())
                        .find(|&&(other, _)| other.state != place.state && touches.overlap(place, other))
                        .map(|&(_, by)| by)
                })
                .flatten()
        })
    }
}

impl Outside {
    fn of(touches: &Touches<'_>, loading: &HashSet<DefId>) -> Self {
        let mut outside = Outside {
            writes: Placed::default(),
            touches: Placed::default(),
        };
        for id in touches.order.iter().filter(|id| !loading.contains(id)) {
            let touched = &touches.of[id];
            for &place in &touched.writes {
                outside.writes.add(touches, place, *id);
            }
            for &place in touched.reads.iter().chain(&touched.writes) {
                outside.touches.add(touches, place, *id);
            }
        }
        outside
    }
}

/// Why making a thread-local as its module loads would show, where Rust
/// makes it at its first use.
enum Shows {
    /// Its initializer runs a body, the first, that prints or panics.
    Ordered(DefId, Ordered),
    /// It reads state, the first, that a body that may run after the
    /// modules load, the second, writes.
    Reads(DefId, DefId),
    /// It writes state, the first, that a body that may run after the
    /// modules load, the second, reads or writes.
    Writes(DefId, DefId),
}

/// Whether making `init`'s thread-local as its module loads would show:
/// of what it runs and reads there, `reached`, a body that prints or
/// panics; state it reads that what may run outside the load, `outside`,
/// writes; or state it writes that that touches (the user's decision,
/// #554).
fn shows(touches: &Touches<'_>, outside: &Outside, init: DefId, reached: &[DefId]) -> Option<Shows> {
    let ran = || {
        std::iter::once(&init)
            .chain(reached)
            .filter_map(|id| Some((*id, touches.of.get(id)?)))
    };
    if let Some((id, ordered)) = ran().find_map(|(id, touched)| Some((id, touched.ordered?))) {
        return Some(Shows::Ordered(id, ordered));
    }
    let mut read = ran().flat_map(|(_, touched)| touched.reads.iter());
    if let Some((state, by)) = read.find_map(|&place| Some((place.state, outside.writes.by(touches, place)?))) {
        return Some(Shows::Reads(state, by));
    }
    let mut written = ran().flat_map(|(_, touched)| touched.writes.iter());
    written
        .find_map(|&place| Some((place.state, outside.touches.by(touches, place)?)))
        .map(|(state, by)| Shows::Writes(state, by))
}

/// A `pub` thread-local whose initializer would show it's made as its
/// module loads, `why`: refused, as JS's importers read it as an `export
/// const`, made then (ADR 1006, amended, #554).
fn refuse_exported(tcx: TyCtxt<'_>, key: LocalDefId, why: Shows) {
    let span = tcx.def_ident_span(key).unwrap_or_else(|| tcx.def_span(key));
    let name = |id: DefId| tcx.def_path_str(tcx.typeck_root_def_id(id));
    let what = match why {
        Shows::Ordered(by, Ordered::Prints) => {
            format!("a `pub` thread-local whose initializer prints, in `{}`", name(by))
        }
        Shows::Ordered(by, Ordered::Panics) => {
            format!("a `pub` thread-local whose initializer may panic, in `{}`", name(by))
        }
        Shows::Reads(state, by) => format!(
            "a `pub` thread-local whose initializer reads `{}`, which `{}` may change after the modules load",
            name(state),
            name(by)
        ),
        Shows::Writes(state, by) => format!(
            "a `pub` thread-local whose initializer changes `{}`, which `{}` may read after the modules load",
            name(state),
            name(by)
        ),
    };
    let why = "JS's importers read it as an `export const`, made as its module loads, where Rust makes it at its first use: make it `pub(crate)`, made where it's first used, and export a `pub fn` that reads it (ADR 1006)";
    crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Because(&what, why));
}
