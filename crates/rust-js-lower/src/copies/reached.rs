//! What JS may change in place, value by value (#2408): a clone whose value
//! reaches JS, or whose source does, is copied, as JS may change either
//! while the other is still Rust's. Where a value reaches JS:
//!
//! - an exported function's result, and what it holds;
//! - a returned closure's result;
//! - what Rust gives a JS function, a binding's or a callback's;
//! - a constant read where any of those is made of it.
//!
//! Found by following values backwards through each body's MIR from those
//! places, and across calls by which of a function's parameters flow into
//! its result. Where the flow can't be followed, the copy is kept: a store
//! through a `&mut` into what reaches JS, a value behind a reference, an
//! `Rc` or a guard, which aliases what it came from, and a library's or
//! std's function, whose result may carry any of its arguments.
//!
//! The answer is per function: the types its clones copy, beside the
//! crate's `Changed` (ADR 0052), which is per type and stays as it is, so
//! no library's contract changes (ADR 0914).

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{self, AggregateKind, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

use crate::Body;
use crate::recognition::StdItem;

/// Each function's types whose clones it copies, as JS may change them: by
/// the function's typeck root, as its closures are lowered with it.
pub type JsCopies<'tcx> = IndexMap<DefId, IndexSet<Ty<'tcx>>>;

/// What reaches JS of the crate's bodies, as `JsCopies`.
pub fn js_copies<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    foreign: &crate::library::Foreign<'_, 'tcx>,
) -> JsCopies<'tcx> {
    let bodies: IndexMap<DefId, &mir::Body<'tcx>> = (all_bodies.iter())
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
        .collect();
    // Which of each function's parameters flow into its result, to a
    // fixpoint: each starts with none, and only grows.
    let mut summaries: IndexMap<DefId, IndexSet<usize>> = IndexMap::default();
    loop {
        let mut changed = false;
        for (&id, body) in &bodies {
            let flow = Flow::of(tcx, body, &summaries, true, false);
            let params: IndexSet<usize> = (1..=body.arg_count)
                .filter(|&at| flow.tainted(mir::Local::from_usize(at)))
                .map(|at| at - 1)
                .collect();
            if summaries.get(&id) != Some(&params) {
                summaries.insert(id, params);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Which functions' results reach JS, to a fixpoint: JS-facing ones, and
    // each a body whose result reaches JS calls for what reaches JS, or a
    // closure such a body makes and gives JS.
    let mut reaching: IndexSet<DefId> = (bodies.keys().copied())
        .filter(|&id| crate::analysis::js_facing(tcx, foreign, id))
        .collect();
    let mut found: JsCopies<'tcx> = IndexMap::default();
    loop {
        let before = reaching.len();
        found.clear();
        for (&id, body) in &bodies {
            let flow = Flow::of(tcx, body, &summaries, reaching.contains(&id), true);
            reaching.extend(flow.reaching.iter().copied());
            if !flow.copied.is_empty() {
                (found.entry(tcx.typeck_root_def_id(id)).or_default()).extend(flow.copied.iter().copied());
            }
        }
        if reaching.len() == before {
            break;
        }
    }
    found
}

/// One body's flow: which of its locals' values reach JS.
struct Flow<'tcx> {
    /// Each local's alias class: a borrow, an `Rc` and a guard share their
    /// source's storage, so what's written through one is written to it.
    class: Vec<usize>,
    tainted: IndexSet<usize>,
    /// The callees whose results reach JS from here: local functions and
    /// closures, which then follow their own.
    reaching: IndexSet<DefId>,
    /// The types this body's clones copy.
    copied: IndexSet<Ty<'tcx>>,
}

impl<'tcx> Flow<'tcx> {
    fn of(
        tcx: TyCtxt<'tcx>,
        body: &mir::Body<'tcx>,
        summaries: &IndexMap<DefId, IndexSet<usize>>,
        result_reaches: bool,
        sinks: bool,
    ) -> Flow<'tcx> {
        let mut flow = Flow {
            class: (0..body.local_decls.len()).collect(),
            tainted: IndexSet::default(),
            reaching: IndexSet::default(),
            copied: IndexSet::default(),
        };
        flow.alias(tcx, body);
        if result_reaches {
            flow.taint(mir::RETURN_PLACE);
        }
        loop {
            let before = (flow.tainted.len(), flow.copied.len(), flow.reaching.len());
            flow.step(tcx, body, summaries, sinks);
            if (flow.tainted.len(), flow.copied.len(), flow.reaching.len()) == before {
                break;
            }
        }
        flow
    }

    fn find(&mut self, local: usize) -> usize {
        let mut at = local;
        while self.class[at] != at {
            self.class[at] = self.class[self.class[at]];
            at = self.class[at];
        }
        at
    }

    fn union(&mut self, a: mir::Local, b: mir::Local) {
        let (a, b) = (self.find(a.as_usize()), self.find(b.as_usize()));
        if a != b {
            self.class[a] = b;
        }
    }

    fn taint(&mut self, local: mir::Local) -> bool {
        let class = self.find(local.as_usize());
        self.tainted.insert(class)
    }

    fn tainted(&self, local: mir::Local) -> bool {
        let mut at = local.as_usize();
        while self.class[at] != at {
            at = self.class[at];
        }
        self.tainted.contains(&at)
    }

    /// Joins what shares storage: a borrow with what it borrows, a value
    /// holding a reference, a raw pointer or an `Rc` with what it came from.
    fn alias(&mut self, tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>) {
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (place, rvalue) = &**assign;
                match rvalue {
                    Rvalue::Ref(_, _, from) | Rvalue::RawPtr(_, from) | Rvalue::CopyForDeref(from) => {
                        self.union(place.local, from.local)
                    }
                    _ => {
                        for from in places_of(rvalue) {
                            if aliasing(tcx, from.ty(&body.local_decls, tcx).ty) {
                                self.union(place.local, from.local);
                            }
                        }
                    }
                }
            }
            if let TerminatorKind::Call { args, destination, .. } = &data.terminator().kind
                && aliasing(tcx, destination.ty(&body.local_decls, tcx).ty)
            {
                for arg in args {
                    if let Some(from) = arg.node.place()
                        && aliasing(tcx, from.ty(&body.local_decls, tcx).ty)
                    {
                        self.union(destination.local, from.local);
                    }
                }
            }
        }
    }

    /// One pass over the body, each value that flows into what reaches JS
    /// made to reach it too.
    fn step(
        &mut self,
        tcx: TyCtxt<'tcx>,
        body: &mir::Body<'tcx>,
        summaries: &IndexMap<DefId, IndexSet<usize>>,
        sinks: bool,
    ) {
        let decls = &body.local_decls;
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (place, rvalue) = &**assign;
                // What a number or a string is made of can't reach JS through it.
                if !self.tainted(place.local) || !may_share(tcx, place.ty(decls, tcx).ty) {
                    continue;
                }
                for from in places_of(rvalue) {
                    if may_share(tcx, from.ty(decls, tcx).ty) {
                        self.taint(from.local);
                    }
                }
                // A constant made where JS gets it: a copy of it at the read.
                for constant in constants_of(rvalue) {
                    if copies_on_clone(tcx, constant) {
                        parts(tcx, constant, &mut self.copied);
                    }
                }
                // A closure JS gets: its result reaches JS too.
                if let Rvalue::Aggregate(kind, _) = rvalue
                    && let AggregateKind::Closure(id, _) = **kind
                {
                    self.reaching.insert(id);
                }
            }
            let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let arg_places: Vec<mir::Place<'tcx>> = args.iter().filter_map(|arg| arg.node.place()).collect();
            let result = self.tainted(destination.local) && may_share(tcx, destination.ty(decls, tcx).ty);
            let callee = func.const_fn_def();
            let callable = callee.and_then(|(id, args)| called_through_fn(tcx, id, args));
            // What a JS function is given reaches JS: a binding's arguments,
            // and a callback's, one not the crate's own closure.
            let to_js = sinks
                && match callee {
                    Some((id, _)) => crate::bindings::is_binding(tcx, id) || matches!(callable, Some(None)),
                    None => true,
                };
            let flows: Vec<bool> = match (callee, callable) {
                _ if to_js => vec![true; arg_places.len()],
                // The crate's closure, called: its result reaches JS where the
                // call's does, and what it's given may be in it.
                (_, Some(Some(closure))) => {
                    if result {
                        self.reaching.insert(closure);
                    }
                    vec![result; arg_places.len()]
                }
                (Some((id, _)), _) if let Some(params) = summaries.get(&id) => {
                    if result {
                        self.reaching.insert(id);
                    }
                    (0..arg_places.len()).map(|at| result && params.contains(&at)).collect()
                }
                // A library's or std's: its result may carry any argument.
                _ => vec![result; arg_places.len()],
            };
            for (place, flows) in arg_places.iter().zip(&flows) {
                if *flows && may_share(tcx, place.ty(decls, tcx).ty) {
                    self.taint(place.local);
                }
            }
            // A store through a `&mut` into what reaches JS: whatever the call
            // is given may be stored there.
            let stores = arg_places.iter().any(|place| {
                matches!(place.ty(decls, tcx).ty.kind(), ty::Ref(_, _, ty::Mutability::Mut))
                    && self.tainted(place.local)
            });
            if stores {
                for place in &arg_places {
                    if may_share(tcx, place.ty(decls, tcx).ty) {
                        self.taint(place.local);
                    }
                }
            }
            // A clone whose copy or source reaches JS: what it clones is
            // copied here, each type behind a reference it's given. A clone
            // only read through its fields, `let (a, b) = pair.clone()`, is
            // never a value JS gets whole: only its parts are copied.
            if let Some((id, _)) = callee
                && cloning(tcx, id)
                && (result || arg_places.iter().any(|place| self.tainted(place.local)))
            {
                let whole = used_whole(body, destination.local);
                for place in &arg_places {
                    for referent in referents(place.ty(decls, tcx).ty) {
                        if !copies_on_clone(tcx, referent) {
                            continue;
                        }
                        if whole {
                            parts(tcx, referent, &mut self.copied);
                        } else {
                            for part in parts_of(tcx, referent) {
                                parts(tcx, part, &mut self.copied);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The places an rvalue reads.
fn places_of<'a, 'tcx>(rvalue: &'a Rvalue<'tcx>) -> Vec<mir::Place<'tcx>> {
    let operands: Vec<&Operand<'tcx>> = match rvalue {
        Rvalue::Use(op, _) | Rvalue::Cast(_, op, _) | Rvalue::Repeat(op, _) | Rvalue::UnaryOp(_, op) => vec![op],
        Rvalue::Aggregate(_, ops) => ops.iter().collect(),
        Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) | Rvalue::CopyForDeref(place) => return vec![*place],
        _ => Vec::new(),
    };
    operands.into_iter().filter_map(Operand::place).collect()
}

/// The types of the constants an rvalue reads.
fn constants_of<'tcx>(rvalue: &Rvalue<'tcx>) -> Vec<Ty<'tcx>> {
    let operands: Vec<&Operand<'tcx>> = match rvalue {
        Rvalue::Use(op, _) | Rvalue::Cast(_, op, _) | Rvalue::Repeat(op, _) => vec![op],
        Rvalue::Aggregate(_, ops) => ops.iter().collect(),
        _ => Vec::new(),
    };
    operands
        .into_iter()
        .filter_map(|op| op.constant().map(|c| c.ty()))
        .collect()
}

/// Whether a value of `ty` may be an object JS changes in place, or hold
/// one: anything but a number, a `bool`, a `char`, a string or a function.
fn may_share<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => false,
        ty::FnDef(..) | ty::FnPtr(..) => false,
        ty::Tuple(items) => items.iter().any(|item| may_share(tcx, item)),
        ty::Ref(_, inner, _) => may_share(tcx, *inner),
        ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::String) => false,
        // An `Option` is its value or `undefined`.
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) => {
            may_share(tcx, args.type_at(0))
        }
        _ => true,
    }
}

/// Whether a value of `ty` may share storage with another: one holding a
/// reference or a raw pointer, or an `Rc`, an `Arc` or a `Weak`.
fn aliasing<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.walk().any(|part| match part.as_type() {
        Some(t) => match t.kind() {
            ty::Ref(..) => true,
            _ if crate::pointers::raw_pointee(tcx, t).is_some() => true,
            ty::Adt(adt, _) => {
                crate::recognition::rc_pointee(tcx, t).is_some()
                    || crate::recognition::weak_pointee(tcx, t).is_some()
                    || crate::recognition::is_std_def(tcx, adt.did(), StdItem::Arc)
            }
            _ => false,
        },
        None => part.as_region().is_some(),
    })
}

/// The types behind references in `ty`: what a clone of it may copy.
fn referents<'tcx>(ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    ty.walk()
        .filter_map(|part| match part.as_type()?.kind() {
            ty::Ref(_, inner, _) => Some(*inner),
            _ => None,
        })
        .collect()
}

/// Whether `id` clones what it's given: `Clone::clone`, and std's that do
/// by another name.
fn cloning(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let of_clone =
        (tcx.trait_of_assoc(id)).is_some_and(|t| tcx.is_lang_item(t, rustc_hir::attrs::lang_items::LangItem::Clone));
    of_clone
        || (crate::recognition::is_std_crate(tcx, id)
            && matches!(
                tcx.item_name(id).as_str(),
                "to_vec" | "to_owned" | "cloned" | "copied" | "clone_from"
            ))
}

/// Of a call of `id` through a function trait, `Fn::call` and its kin: the
/// crate's closure it calls, `Some(Some(..))`, or `Some(None)` for any other
/// function, a callback JS may give. `None` of any other call.
fn called_through_fn<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Option<DefId>> {
    let trait_id = tcx.trait_of_assoc(id)?;
    tcx.fn_trait_kind_from_def_id(trait_id)?;
    match args.type_at(0).peel_refs().kind() {
        ty::Closure(closure, _) => Some(Some(*closure)),
        ty::FnDef(..) => None,
        _ => Some(None),
    }
}

/// Whether a clone of `ty` is a copy of its own, one that may share: not a
/// reference's, an `Rc`'s, an `Arc`'s or a `Weak`'s, which share their value
/// by design, nor a function's, which JS can't change, nor an `Option` of
/// any of those.
pub(super) fn copies_on_clone<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let shares = match ty.kind() {
        // An `Option` copies what its value does: of a reference, nothing.
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) => {
            return copies_on_clone(tcx, args.type_at(0));
        }
        // A reference's clone is the reference, and a raw pointer's.
        ty::Ref(..) => true,
        _ if crate::pointers::raw_pointee(tcx, ty).is_some() => true,
        ty::Closure(..) | ty::Dynamic(..) | ty::FnDef(..) | ty::FnPtr(..) => true,
        ty::Adt(adt, _) => {
            crate::recognition::rc_pointee(tcx, ty).is_some()
                || crate::recognition::weak_pointee(tcx, ty).is_some()
                || crate::recognition::is_std_def(tcx, adt.did(), StdItem::Arc)
        }
        _ => false,
    };
    !shares && may_share(tcx, ty)
}

/// `ty` and what a clone of it copies with it, into `copied`: an
/// `Option`'s value, a tuple's items, an array's, a struct's or an enum's
/// fields, a std type's arguments, a `Vec`'s items. A copy is decided part
/// by part, so a clone of an `Option<Vec<u32>>` copies the `Vec` because
/// the `Vec` is recorded too. What shares, an `Rc` or a reference, ends
/// the walk: its clone copies nothing.
fn parts<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, copied: &mut IndexSet<Ty<'tcx>>) {
    if !copies_on_clone(tcx, ty) || !copied.insert(ty) {
        return;
    }
    for part in parts_of(tcx, ty) {
        parts(tcx, part, copied);
    }
}

/// What a value of `ty` holds that a clone of it clones in turn: a tuple's
/// items, an array's, a struct's or an enum's fields, a std type's
/// arguments, an `Option`'s value or a `Vec`'s items.
fn parts_of<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    match ty.kind() {
        ty::Tuple(items) => items.iter().collect(),
        ty::Array(item, _) | ty::Slice(item) => vec![*item],
        ty::Adt(adt, args) if crate::recognition::is_std_crate(tcx, adt.did()) => args.types().collect(),
        ty::Adt(adt, args) => (adt.all_fields())
            .map(|field| field.ty(tcx, args).skip_normalization())
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether `local` is read, moved or borrowed whole anywhere in `body`,
/// not only through its fields: what's written to it, dropped or ended
/// isn't a use of its value.
fn used_whole(body: &mir::Body<'_>, local: mir::Local) -> bool {
    struct Whole {
        local: mir::Local,
        found: bool,
    }
    impl<'tcx> Visitor<'tcx> for Whole {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, _: mir::Location) {
            let read = match context {
                PlaceContext::NonUse(_) => false,
                // A `let` pattern's fake read, a mention or a discriminant's
                // read isn't a use of the value.
                PlaceContext::NonMutatingUse(used) => !matches!(
                    used,
                    NonMutatingUseContext::Inspect
                        | NonMutatingUseContext::PlaceMention
                        | NonMutatingUseContext::Projection
                ),
                PlaceContext::MutatingUse(used) => {
                    matches!(used, MutatingUseContext::Borrow | MutatingUseContext::RawBorrow)
                }
            };
            if read && place.local == self.local && place.projection.is_empty() {
                self.found = true;
            }
        }
    }
    // What a function returns is used whole by its caller.
    if local == mir::RETURN_PLACE {
        return true;
    }
    let mut whole = Whole { local, found: false };
    whole.visit_body(body);
    whole.found
}
