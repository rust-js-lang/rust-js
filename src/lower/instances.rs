//! The library functions a crate compiles itself (ADR 0914). A library's
//! generic function takes the copies and drops its own crate's calls give
//! it, as a program's does, so its JS is what it would be with no consumer,
//! the one JS calls. A consumer whose call gives it a value to copy or drop
//! that it takes nothing for compiles it from the library's MIR, as Rust's
//! codegen does, beside its first caller, and calls its own; so are the
//! library's private functions that one calls. One that reads the library's
//! private state, which no other crate can reach, is refused.

use crate::collections::{IndexMap, IndexSet};
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeFoldable, TypeVisitableExt};
use rustc_span::def_id::{DefId, LocalModId};

use super::Body;

mod failing;
mod needed;
pub(super) use needed::{Analyzed, copy_reaches, is_state, needed, reads};

/// The library functions the crate compiles itself, each placed in the
/// module of the crate's that first calls it (ADR 0914). Decided by the
/// pipeline once the crate's bodies are analyzed (`needed`), and given back
/// to it with their bodies, until none more is needed.
#[derive(Default)]
pub struct Instances {
    placed: IndexMap<DefId, LocalModId>,
}

impl Instances {
    /// Whether `id`, a library's function or a closure of one, is compiled
    /// in this crate.
    pub(super) fn contains(&self, tcx: TyCtxt<'_>, id: DefId) -> bool {
        !id.is_local() && self.module(tcx, id).is_some()
    }

    /// The module of the crate's an instance is placed in, its closures'
    /// too, and a closure one makes of a function rustc inlined into it,
    /// placed as itself (`instance_bodies`, #1029).
    pub(super) fn module(&self, tcx: TyCtxt<'_>, id: DefId) -> Option<LocalModId> {
        (self
            .placed
            .get(&tcx.typeck_root_def_id(id))
            .or_else(|| self.placed.get(&id)))
        .copied()
    }

    /// Each instance `needed` found, with the others.
    pub fn extend(&mut self, more: Vec<(DefId, LocalModId)>) {
        self.placed.extend(more);
    }

    /// The trait impls of its libraries the crate compiles itself, which
    /// their JS doesn't export (ADR 0914, amended), in the order they're
    /// declared.
    pub(super) fn impls(&self, tcx: TyCtxt<'_>) -> Vec<DefId> {
        let mut impls: Vec<DefId> = (self.placed.keys().copied())
            .filter(|&id| matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true }))
            .collect();
        impls.sort_by_key(|&id| tcx.def_path_hash(id));
        impls
    }
}

/// What the crate's lowering asked for of its libraries that their JS
/// doesn't export (`Foreign::compiles`), each where it was first asked: a
/// trait impl, or its method, the impl with each of its methods, beside
/// it, whose dictionary names them (ADR 0914, amended), and its constants,
/// whose initializers it lowers where rustc can't compute them, `CAPACITY
/// = CAP` of arrayvec's `impl<T, const CAP: usize> ArrayVecImpl for
/// ArrayVec<T, CAP>` (ADR 0176, amended).
pub(super) fn asked(tcx: TyCtxt<'_>, asked: Vec<(DefId, LocalModId)>) -> Vec<(DefId, LocalModId)> {
    let mut more: Vec<(DefId, LocalModId)> = Vec::new();
    for (id, module) in asked {
        let id = tcx.trait_impl_of_assoc(id).unwrap_or(id);
        let methods = match tcx.def_kind(id) {
            DefKind::Impl { of_trait: true } => (tcx.associated_item_def_ids(id).iter().copied())
                .filter(|&item| match tcx.def_kind(item) {
                    DefKind::AssocFn => tcx.is_mir_available(item),
                    DefKind::AssocConst => true,
                    _ => false,
                })
                .collect(),
            _ => Vec::new(),
        };
        for id in std::iter::once(id).chain(methods) {
            if !more.iter().any(|&(placed, _)| placed == id) {
                more.push((id, module));
            }
        }
    }
    more
}

/// The provided methods of `imp`'s trait that `imp` doesn't write.
pub(in crate::lower) fn defaults_used(tcx: TyCtxt<'_>, imp: DefId) -> impl Iterator<Item = DefId> + '_ {
    let written = tcx.impl_item_implementor_ids(imp);
    (tcx.provided_trait_methods(tcx.impl_trait_id(imp)))
        .map(|item| item.def_id)
        .filter(move |method| !written.contains_key(method))
}

/// The impls of `imp`'s trait's supertraits for its type, whose
/// dictionaries its own holds, `Clone()` of `trait Dup: Clone` (#1927).
fn supertrait_impls(tcx: TyCtxt<'_>, imp: DefId) -> Vec<DefId> {
    let tr = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
    let typing_env = ty::TypingEnv::post_analysis(tcx, imp);
    let supers = rustc_middle::ty::elaborate::supertraits(tcx, ty::Binder::dummy(tr)).skip(1);
    supers
        .filter_map(|of| {
            let of = tcx.instantiate_bound_regions_with_erased(of);
            super::traits::evidence_impl(tcx, typing_env, of).map(|(id, _)| id)
        })
        .collect()
}

/// Which of a library's functions and impls may leave out a copy, or a
/// clone, of what the crate changes in place, which its library's JS,
/// copying what its own crate changes, may then share: the crate compiles
/// its own of each it uses (ADR 0914, amended). As its library's contract
/// says, which names the types each leaves copies of out (`Library::elided`,
/// #2112's change D). Asked of each once.
pub(super) struct Copying<'a, 'tcx> {
    recognition: super::recognition::Recognition<'a, 'tcx>,
    changed: super::copies::Changed<'a, 'tcx>,
    known: std::cell::RefCell<IndexMap<DefId, bool>>,
}

impl<'a, 'tcx> Copying<'a, 'tcx> {
    /// Of the crate's impls and libraries, `trait_impls` and `foreign`, and
    /// what it changes in place.
    pub(super) fn new(
        (trait_impls, foreign): (&'a [DefId], &'a super::library::Foreign<'a, 'tcx>),
        changed: super::copies::Changed<'a, 'tcx>,
    ) -> Self {
        let recognition = super::recognition::Recognition {
            tcx: foreign.tcx(),
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls,
            foreign,
        };
        Copying {
            recognition,
            changed,
            known: Default::default(),
        }
    }

    /// Whether `id`, a library's function or trait impl the crate uses,
    /// leaves out a copy or a clone of what the crate changes in place and
    /// its library doesn't: one of the shapes its contract says it, or what
    /// of its crate's it calls, leaves copies of out may be one. An impl's
    /// supertraits' impls of its type too, whose dictionaries its own
    /// holds, `pointDup.Clone()` of `trait Dup: Clone` (#1927).
    pub(super) fn copies(&self, id: DefId) -> bool {
        let tcx = self.recognition.tcx;
        if id.is_local() {
            return false;
        }
        if let Some(&known) = self.known.borrow().get(&id) {
            return known;
        }
        let Some(elided) = self.recognition.foreign.elided(id) else {
            return false;
        };
        let copies = self.elided_changed(elided)
            || matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
                && supertrait_impls(tcx, id).into_iter().any(|imp| self.copies(imp));
        self.known.borrow_mut().insert(id, copies);
        copies
    }

    /// Whether any of `elided`, the shapes a library's function leaves
    /// copies of out, may be a type the crate changes in place, or a `Vec`
    /// of it does. What the library changes too is none of them: its JS
    /// copies that as Rust does (rd606).
    fn elided_changed(&self, elided: &[crate::library::RcShape]) -> bool {
        let tcx = self.recognition.tcx;
        (elided.iter()).any(|shape| {
            (self.changed.mutated.iter().chain(self.changed.vecs))
                .any(|&ty| super::library::may_be_shape(tcx, shape, tcx.erase_and_anonymize_regions(ty)))
        })
    }
}

/// The bodies of each of `more`, a library's functions, from the MIR its
/// metadata keeps, and of each closure they make (ADR 0914), each closure
/// placed where what made it is: one of a function rustc inlined into the
/// instance, `joined`'s in `fold_with`, is no closure of an instance's own
/// by its typeck root (#1029).
pub fn instance_bodies<'tcx>(
    tcx: TyCtxt<'tcx>,
    more: &[(DefId, LocalModId)],
) -> (Vec<Body<'tcx>>, Vec<(DefId, LocalModId)>) {
    let (bodies, makers) = super::analysis::foreign_bodies(tcx, more.iter().map(|&(id, _)| id).collect());
    // In the order they were found, each maker before what it made.
    let mut placed: IndexMap<DefId, LocalModId> = more.iter().copied().collect();
    let mut made = Vec::new();
    for (&closure, maker) in &makers {
        if let Some(&module) = placed.get(maker) {
            placed.insert(closure, module);
            made.push((closure, module));
        }
    }
    (bodies, made)
}

/// Each function a body of MIR alone names, which its THIR would: one of a
/// library's the crate compiles (ADR 0914).
pub(super) fn named(body: &Body<'_>) -> Vec<DefId> {
    let Some(mir) = &body.mir else { return Vec::new() };
    called(&mir.body).into_iter().map(|(id, _)| id).collect()
}

/// Each function `body` calls or names as a value, as the call graph's
/// `names` give them, that's given its type parameters' values: not one of rest
/// arguments given where JS alone reads it (`read_by_js_alone`), which no
/// call gives anything, so needs no copy of its own (ADR 0051, amended,
/// #565).
pub(super) fn given_calls_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    names: &super::call_graph::Names<'tcx>,
    body: &Body<'tcx>,
) -> Vec<(DefId, ty::GenericArgsRef<'tcx>, rustc_span::Span)> {
    let alone = match body.root() {
        Some(_) => read_by_js_alone(tcx, Some(&body.thir), None),
        None => read_by_js_alone(tcx, None, body.mir.as_ref().map(|mir| &mir.body)),
    };
    (names.names_of(body.def_id).iter().copied())
        .filter(|&(callee, _, at, _)| !alone.iter().any(|&(f, given)| f == callee && given.contains(at)))
        .map(|(callee, args, at, _)| (callee, args, at))
        .collect()
}

/// Each function of rest arguments `body` gives a binding where JS alone
/// reads it, `js::function::name(&signal_oper::<()>)`, and where it's
/// given: its JS function itself, its own JS, which the binding calls with
/// what Rust gives it alone, or not at all, so it's no call that gives its
/// type parameters anything, `()` here, that would need a copy of it
/// (`FnCx::rest_function_itself`, #565). alien-signals' `isSignal` compares
/// `signalOper`'s name with what `signal` binds, its own JS.
pub(in crate::lower) fn read_by_js_alone<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: Option<&rustc_middle::thir::Thir<'tcx>>,
    mir: Option<&mir::Body<'tcx>>,
) -> Vec<(DefId, rustc_span::Span)> {
    use crate::lower::binding_calls::{Itself, function_itself};
    let mut alone = Vec::new();
    let mut given = |callee: DefId, index: usize, ty: Ty<'tcx>, span: rustc_span::Span| {
        if crate::lower::bindings::is_binding(tcx, callee)
            && function_itself(tcx, callee, index) == Some(Itself::Unreturned)
            && let ty::FnDef(f, _) = *ty.peel_refs().kind()
            && crate::lower::bindings::takes_rest(tcx, f)
        {
            alone.push((f, span));
        }
    };
    for expr in thir.iter().flat_map(|thir| thir.exprs.iter()) {
        if let rustc_middle::thir::ExprKind::Call { fun, args, .. } = &expr.kind
            && let Some(thir) = thir
            && let ty::FnDef(callee, _) = *thir[*fun].ty.kind()
        {
            for (index, &arg) in args.iter().enumerate() {
                given(callee, index, thir[arg].ty, thir[arg].span);
            }
        }
    }
    for data in mir.iter().flat_map(|mir| mir.basic_blocks.iter()) {
        if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
            && let Some(mir) = mir
            && let ty::FnDef(callee, _) = *func.ty(mir, tcx).kind()
        {
            for (index, arg) in args.iter().enumerate() {
                given(callee, index, arg.node.ty(mir, tcx), arg.span);
            }
        }
    }
    alone
}

/// Each function `body` calls or names as a value, and its generic
/// arguments there: a reference to one too, `&f` made a `&dyn Fn`, which
/// MIR promotes to a constant of its own, `&FnDef` (#1210).
pub(in crate::lower) fn called<'tcx>(body: &mir::Body<'tcx>) -> Vec<(DefId, ty::GenericArgsRef<'tcx>)> {
    struct Called<'tcx>(Vec<(DefId, ty::GenericArgsRef<'tcx>)>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Called<'tcx> {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            if let Some(called) = super::fn_def(constant.ty().peel_refs()) {
                self.0.push(called);
            }
        }
    }
    let mut called = Called(Vec::new());
    mir::visit::Visitor::visit_body(&mut called, body);
    called.0
}

/// Whether a value of `ty` may hold one of type parameter `index`: `ty` is
/// of it, but a closure only by what it captures and what it's given or
/// gives back. A closure's type is of each of its maker's type parameters,
/// whose values it holds none of unless it captures one, so React's
/// `runWithFiberInDEV`'s `CURRENT.with_borrow(|current| current.clone())`
/// gives std no `()` it's given (gap rd85).
pub(in crate::lower) fn holds_param(ty: Ty<'_>, index: u32) -> bool {
    let mut walk = ty.walk();
    while let Some(arg) = walk.next() {
        let Some(ty) = arg.as_type() else { continue };
        match ty.kind() {
            ty::Param(param) if param.index == index => return true,
            ty::Closure(_, args) => {
                walk.skip_current_subtree();
                let closure = args.as_closure();
                let sig = closure.sig().skip_binder();
                if holds_param(closure.tupled_upvars_ty(), index)
                    || sig.inputs_and_output.iter().any(|ty| holds_param(ty, index))
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// A fact what callers give generic code decides, as whether a caller the
/// crate doesn't see gets its own copy of the function for it asks.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Fact<'f, 'a, 'tcx> {
    /// A copy, a `Clone` dictionary, an unboxed `Some` (ADRs 0289, 0914,
    /// 0051): a consumer whose call gives more than the export takes
    /// compiles its own copy (`gives_more`), and JS's is a copy apart
    /// (`js_given`), so the crate's own calls are every caller of its.
    CopiedApart(&'f crate::lower::library::Foreign<'a, 'tcx>),
    /// A place an object's stored, a clone apart (#1114, #1119): no caller
    /// compiles its own for these, so a `pub` function's are any a
    /// consumer or JS may give (#1234).
    Shared,
    /// An `Rc` counted (#1009): what JS gives drops nothing and reads no
    /// count (ADRs 0289, 0914), so only a consumer's call, of a library's,
    /// gives what's counted, and a consumer whose call counts what it gives
    /// otherwise than the export (`ParamFact::Counted`) compiles its own, as
    /// it does for a copy (`gives_more`, #1234): a library's function a
    /// consumer can compile is seen by its own calls. A trait's method or an
    /// impl's, which a dictionary calls, isn't (#1233). A program's callers
    /// are its own calls: a `pub` function's, and a trait's method std or a
    /// library calls by its dictionary, `Debug::fmt` of `{:?}`, `Hash::hash`
    /// of a `HashSet`'s, given a type the program names where it makes that
    /// dictionary (#1112).
    Counted { library: bool },
    /// What a caller makes knowing, its hint (#1236): each caller the crate
    /// sees marks what it gives, whatever others do, which nothing marks.
    Marked,
    /// How a closure the crate makes is called where generic code calls
    /// what it's given, by `FnOnce` or by `&` (#2031): only the crate's own
    /// calls give its closures, as `Marked`'s; and an impl's own parameter
    /// is what's in its place in the impl's type: of an impl of the crate's
    /// own trait, in what the namings of the trait's method give it; of
    /// another crate's, which code the crate can't see may call, in each
    /// type the crate's bodies hold, `Held` (`givings`).
    Called(&'f Held<'tcx>),
}

/// Whether every caller of `owner`, a function or a trait, that may give
/// it what decides `fact` is one the crate sees: the one rule of the
/// callers' instances' carrier, `params_given`'s and `instances_of`'s
/// (ADR 1115), which asks whether a caller the crate doesn't see gets its
/// own copy for `fact`. Of `CopiedApart`, a function of the crate's, or of
/// a library's it compiles itself, called by name: not a trait's method,
/// nor an impl's, which a dictionary calls (#1233). Of `Shared`, one no
/// other crate or JS can name.
pub(in crate::lower) fn seen_whole(tcx: TyCtxt<'_>, owner: DefId, fact: Fact<'_, '_, '_>) -> bool {
    match fact {
        Fact::CopiedApart(foreign) => {
            let id = tcx.typeck_root_def_id(owner);
            (id.is_local() || foreign.is_instance(id))
                && !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
                && ((tcx.trait_of_assoc(id).is_none() && tcx.trait_impl_of_assoc(id).is_none())
                    // A method of the crate's own trait no other crate can
                    // name, each of whose namings `params_given` follows
                    // (#1233).
                    || method_family(tcx, id).is_some())
        }
        Fact::Shared => !tcx.visibility(owner).is_public(),
        Fact::Counted { library } => {
            !library || !tcx.visibility(owner).is_public() || consumer_compiles(tcx, tcx.typeck_root_def_id(owner))
        }
        Fact::Marked | Fact::Called(_) => true,
    }
}

/// Whether a consumer of `id`'s crate, a library, compiles its own of it
/// where its call gives more than its export takes (ADR 0914): a function,
/// not a trait's method nor an impl's, which a dictionary calls, nor a
/// binding (`needed::compiled_here`, #1234).
pub(in crate::lower) fn consumer_compiles(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)
        && tcx.trait_of_assoc(id).is_none()
        && tcx.trait_impl_of_assoc(id).is_none()
        && !crate::lower::bindings::is_binding(tcx, id)
}

/// The namings a method of a trait is one of, where every caller of each
/// is the crate's: the trait's declaration of it and each impl's, of a trait
/// of the crate's that's no other crate's to name or implement. A call of
/// any decides what each takes, as generic code calls them by the trait's
/// dictionary, passing what the declaration takes (#1233). `None` of a
/// method of another trait, a function of none.
fn method_family(tcx: TyCtxt<'_>, id: DefId) -> Option<Vec<DefId>> {
    let declared = tcx.trait_item_of(id).or(tcx.trait_of_assoc(id).map(|_| id))?;
    let tr = tcx.parent(declared);
    if !tr.is_local() || tcx.visibility(tr).is_public() {
        return None;
    }
    let mut family = vec![declared];
    for imp in tcx.all_impls(tr) {
        if let Some(&method) = tcx.impl_item_implementor_ids(imp).get(&declared) {
            family.push(method);
        }
    }
    Some(family)
}

/// Of `items`, each type parameter `bounded` names that some caller gives
/// what isn't `itself`, carried through generic callers: a caller's own type
/// parameter given on is given what that one is. A function `seen_whole`
/// says isn't, whose callers aren't all its crate's calls, is given every
/// type. The carrier's second question beside `instances_of`'s (ADR 1115):
/// `instances_of` answers which types, of a function whose visibility says
/// its callers are seen; this whether any fails `itself`, of one
/// `seen_whole` says is, of `Fact::CopiedApart`, for the copies, clones and
/// `Some`s a caller gives (ADRs 0289, 0914, 0051), as a consumer that gives
/// more compiles its own and JS's is a copy apart.
pub(super) fn params_given<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    callers: &Callers<'tcx>,
    (items, is_item): (impl Iterator<Item = &'a DefId>, &dyn Fn(DefId) -> bool),
    seen_whole: &dyn Fn(DefId) -> bool,
    bounded: &dyn Fn(DefId) -> Vec<u32>,
    itself: &dyn Fn(Ty<'tcx>) -> bool,
) -> IndexSet<(DefId, u32)> {
    let mut given = IndexSet::default();
    for &id in items {
        if !seen_whole(id) {
            given.extend(bounded(id).into_iter().map(|index| (id, index)));
        } else if method_family(tcx, id).is_some() {
            // Of a trait's method, its trait's and its impl's parameters,
            // which a call through the trait names as `Self` alone, any type
            // (#1233), as `given_on_the_way` takes them.
            let parents = tcx.generics_of(id).parent_count as u32;
            given.extend(
                bounded(id)
                    .into_iter()
                    .filter(|&index| index < parents)
                    .map(|index| (id, index)),
            );
        }
    }
    let mut passed = Vec::new();
    for (&callee, calls) in callers {
        // A trait's method's naming is each of its family's (`method_family`):
        // what one takes of a parameter of its own, each does.
        let family = method_family(tcx, callee).unwrap_or_else(|| vec![callee]);
        if !family.iter().any(|&member| is_item(member)) {
            continue;
        }
        let parents = |id: DefId| tcx.generics_of(id).parent_count as u32;
        let targets = |index: u32| -> Vec<(DefId, u32)> {
            match index.checked_sub(parents(callee)) {
                // Each member, the declaration too, by which a call through
                // the trait decides what it passes.
                Some(own) => (family.iter())
                    .map(|&member| (member, parents(member) + own))
                    .filter(|&(member, at)| bounded(member).contains(&at))
                    .collect(),
                None => vec![(callee, index)]
                    .into_iter()
                    .filter(|&(id, _)| is_item(id))
                    .collect(),
            }
        };
        for &(caller, args, _) in calls {
            for index in bounded(callee) {
                let Some(given_ty) = args.get(index as usize).and_then(|arg| arg.as_type()) else {
                    continue;
                };
                match given_ty.kind() {
                    ty::Param(param) => {
                        passed.extend(targets(index).into_iter().map(|to| ((caller, param.index), to)));
                    }
                    _ if itself(given_ty) => {}
                    _ => given.extend(targets(index)),
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for &(from, to) in &passed {
            if given.contains(&from) && given.insert(to) {
                changed = true;
            }
        }
    }
    given
}

/// What each function's bodies, its closures' too, call or name as a value
/// of the functions `called` says, as `callers_of` has them, by caller, in
/// the order its bodies make them: the carrier's index the other way round,
/// for an analysis that carries what a caller needs to what it calls.
pub(super) fn calls_by_caller<'tcx>(
    tcx: TyCtxt<'tcx>,
    (all_bodies, names): (&[&Body<'tcx>], &super::call_graph::Names<'tcx>),
    called: &dyn Fn(DefId) -> bool,
) -> IndexMap<DefId, Vec<(DefId, ty::GenericArgsRef<'tcx>)>> {
    let mut calls: IndexMap<DefId, Vec<(DefId, ty::GenericArgsRef<'tcx>)>> = IndexMap::default();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id);
        let named = given_calls_of(tcx, names, body)
            .into_iter()
            .filter(|&(callee, ..)| called(callee));
        (calls.entry(caller).or_default()).extend(named.map(|(callee, args, _)| (callee, args)));
    }
    calls
}

/// Each function the crate names, called or as a value, and each naming of
/// it: by what, its body's owner, its generic arguments there, and where.
pub(super) type Callers<'tcx> = IndexMap<DefId, Vec<(DefId, ty::GenericArgsRef<'tcx>, rustc_span::Span)>>;

/// Each type the crate's bodies hold, its expressions' or its MIR's
/// locals', of each of its ADTs, by the ADT, with the function whose body
/// holds it: what an impl's own parameter of an impl of another crate's
/// trait may be (`Fact::Called`, #2031).
pub(super) type Held<'tcx> = IndexMap<DefId, IndexSet<(DefId, Ty<'tcx>)>>;

/// The types `all_bodies` hold, `Held`'s.
pub(super) fn held_of<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> Held<'tcx> {
    let mut held: Held<'tcx> = IndexMap::default();
    for body in all_bodies {
        let owner = tcx.typeck_root_def_id(body.def_id);
        let tys: Vec<Ty<'tcx>> = match body.root() {
            Some(_) => body.thir.exprs.iter().map(|expr| expr.ty).collect(),
            None => (body.mir.iter())
                .flat_map(|mir| mir.body.local_decls.iter().map(|local| local.ty))
                .collect(),
        };
        let mut seen: IndexSet<Ty<'tcx>> = IndexSet::default();
        for ty in tys {
            if !seen.insert(ty) {
                continue;
            }
            for part in ty.walk() {
                if let Some(part) = part.as_type()
                    && let ty::Adt(adt, _) = *part.kind()
                {
                    held.entry(adt.did())
                        .or_default()
                        .insert((owner, tcx.erase_and_anonymize_regions(part)));
                }
            }
        }
    }
    held
}

/// The callers' instances' one carrier (ADR 1115): each naming of each
/// function `all_bodies` call or name as a value that's given its type
/// parameters' values, `given_calls_of`'s, not one of rest arguments JS
/// alone reads, which gives it nothing. Every analysis of what callers give
/// generic code reads it, `instances_of` and `given_params` alike.
pub(super) fn callers_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    names: &super::call_graph::Names<'tcx>,
    all_bodies: &[&Body<'tcx>],
) -> Callers<'tcx> {
    let mut callers: Callers<'tcx> = IndexMap::default();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id);
        for (callee, args, span) in given_calls_of(tcx, names, body) {
            callers.entry(callee).or_default().push((caller, args, span));
        }
    }
    callers
}

/// Each type the crate's namings of `owner` give its type parameter of
/// `index`, of a caller's own parameter what its callers give that, in
/// turn: `None` where one can't be seen, so the parameter may be any type.
/// A `pub` function is called by JS, or a library's by its consumers; a
/// trait's or an impl's method's `Self` and its impl's parameters are a
/// dictionary's, though its own each naming gives, `count_of::<Item>`,
/// whether of the trait's method or the impl's (#1009); and one given what
/// a caller makes of its own type parameters, `Map<Take<I>, F>` of
/// `taken<I>`, each instance of it its callers give, in turn, each such
/// The way to a parameter, each met on it, and what's made on it: each
/// type a generic caller makes of its own parameters, with that caller.
type TheWay<'a, 'tcx> = (&'a mut IndexSet<(DefId, u32)>, &'a mut Made<'tcx>);

/// Each type a generic caller makes of its own parameters and gives on,
/// with that caller (#1240).
pub(super) type Made<'tcx> = IndexSet<(DefId, Ty<'tcx>)>;

/// type kept in `made`, with the caller that makes it (#1240).
fn given_types<'tcx>(
    tcx: TyCtxt<'tcx>,
    (callers, fact): (&Callers<'tcx>, Fact<'_, '_, 'tcx>),
    (owner, index): (DefId, u32),
    (seen, made): TheWay<'_, 'tcx>,
) -> Option<IndexSet<Ty<'tcx>>> {
    // Met again on the way to it, a caller's own: what reaches it is what
    // its other callers give, which they answer for. `seen` is the way, not
    // every parameter met: two of a type's parameters may each be given a
    // caller's one.
    if !seen.insert((owner, index)) {
        return Some(IndexSet::default());
    }
    let given = given_on_the_way(tcx, (callers, fact), (owner, index), (seen, made));
    seen.shift_remove(&(owner, index));
    given
}

fn given_on_the_way<'tcx>(
    tcx: TyCtxt<'tcx>,
    (callers, fact): (&Callers<'tcx>, Fact<'_, '_, 'tcx>),
    (owner, index): (DefId, u32),
    (seen, made): TheWay<'_, 'tcx>,
) -> Option<IndexSet<Ty<'tcx>>> {
    let mut given = IndexSet::default();
    for giving in givings(tcx, (callers, fact), (owner, index))? {
        let Giving::Given { caller, arg, .. } = giving else {
            // An impl's own parameter, `Fact::Called`'s alone (`on_the_way`).
            return None;
        };
        match *arg.kind() {
            ty::Param(param) => given.extend(given_types(tcx, (callers, fact), (caller, param.index), (seen, made))?),
            _ if arg.has_non_region_param() => {
                made.insert((caller, arg));
                given.extend(instances_in(tcx, (callers, fact), (arg, caller), (seen, made))?);
            }
            _ => _ = given.insert(arg),
        }
    }
    Some(given)
}

/// What gives a type parameter of a function a type: a naming of it, its
/// caller and what it gives there; or, of an impl's own parameter, of
/// `Fact::Called`, what its caller holds of the impl's type and the impl's
/// own type, `Once<F>` of `impl<F> Run for Once<F>`'s `Self`, which the
/// parameter is matched in (#2031).
enum Giving<'tcx> {
    Given {
        caller: DefId,
        named: DefId,
        arg: Ty<'tcx>,
    },
    Matched {
        caller: DefId,
        given: Ty<'tcx>,
        pattern: Ty<'tcx>,
    },
}

/// Each naming that gives `owner`'s type parameter `index` a type: of a
/// trait's method, or an impl's of one, each naming of the trait's too, its
/// own parameter where the trait's has it. Of `Fact::Called`, an impl's own
/// parameter too: of an impl of the crate's own trait, which only the crate
/// names, in what each naming of the trait's method gives the trait; of
/// another crate's, which code the crate can't see may call, std's `Drop`
/// or `Iterator`'s, in each type the crate's bodies hold of the impl's
/// (`Held`); and a trait's own method's `Self`, the crate's own trait's, as
/// its namings give it. `None` where one can't be seen, `seen_whole`'s.
fn givings<'tcx>(
    tcx: TyCtxt<'tcx>,
    (callers, fact): (&Callers<'tcx>, Fact<'_, '_, 'tcx>),
    (owner, index): (DefId, u32),
) -> Option<Vec<Giving<'tcx>>> {
    if !seen_whole(tcx, owner, fact) {
        return None;
    }
    let mut givings = Vec::new();
    let mut namings = vec![(owner, index)];
    if let Some(declared) = tcx.trait_item_of(owner).or(tcx.trait_of_assoc(owner).map(|_| owner)) {
        let own = index.checked_sub(tcx.generics_of(owner).parent_count as u32);
        let held = match (own, fact) {
            (Some(_), _) => None,
            (None, Fact::Called(held)) => Some(held),
            (None, _) => return None,
        };
        if !seen_whole(tcx, tcx.parent(declared), fact) {
            return None;
        }
        match own {
            Some(own) if declared != owner => {
                namings.push((declared, tcx.generics_of(declared).parent_count as u32 + own));
            }
            Some(_) => {}
            None => {
                let imp = tcx.parent(owner);
                let tr = tcx.parent(declared);
                if declared == owner && tr.is_local() {
                    return givings_of(tcx, callers, &namings, fact);
                }
                if declared == owner || !matches!(tcx.def_kind(imp), DefKind::Impl { of_trait: true }) {
                    return None;
                }
                let param = ty::GenericArgs::identity_for_item(tcx, imp).type_at(index as usize);
                let holds = |pattern: Ty<'tcx>| pattern.walk().any(|part| part.as_type() == Some(param));
                if !tr.is_local() {
                    // Of `&'a ChunkBy<K, I, F>`'s impl, each `ChunkBy` held.
                    let pattern = tcx.type_of(imp).instantiate_identity().skip_normalization().peel_refs();
                    let ty::Adt(adt, _) = *pattern.kind() else { return None };
                    if !holds(pattern) {
                        return None;
                    }
                    for &(caller, given) in held?.get(&adt.did()).into_iter().flatten() {
                        givings.push(Giving::Matched { caller, given, pattern });
                    }
                    return Some(givings);
                }
                let implemented = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
                for (at, pattern) in implemented.args.iter().enumerate() {
                    let Some(pattern) = pattern.as_type().filter(|&pattern| holds(pattern)) else {
                        continue;
                    };
                    for &(caller, args, _) in callers.get(&declared).into_iter().flatten() {
                        let given = tcx.erase_and_anonymize_regions(args.get(at)?.as_type()?);
                        // A `dyn` of the trait hides which impl it is.
                        if matches!(given.kind(), ty::Dynamic(..)) {
                            return None;
                        }
                        givings.push(Giving::Matched { caller, given, pattern });
                    }
                }
            }
        }
    }
    givings.extend(givings_of(tcx, callers, &namings, fact)?);
    Some(givings)
}

/// What each naming of each of `namings`, a function and its type
/// parameter, gives it there, with its caller. A `dyn` given of
/// `Fact::Called` hides the type it's made of: `None`.
fn givings_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    callers: &Callers<'tcx>,
    namings: &[(DefId, u32)],
    fact: Fact<'_, '_, 'tcx>,
) -> Option<Vec<Giving<'tcx>>> {
    let mut givings = Vec::new();
    for &(named, at) in namings {
        for &(caller, args, _) in callers.get(&named).into_iter().flatten() {
            let arg = tcx.erase_and_anonymize_regions(args.get(at as usize)?.as_type()?);
            if matches!(fact, Fact::Called(_)) && matches!(arg.kind(), ty::Dynamic(..)) {
                return None;
            }
            givings.push(Giving::Given { caller, named, arg });
        }
    }
    Some(givings)
}

/// What matching `ty` with `pattern`, an impl's type of its own parameters,
/// gives the impl's parameter `index`.
enum Matching<'tcx> {
    /// The type in its place.
    Bound(Ty<'tcx>),
    /// A part of `pattern` where `ty` has a caller's type parameter, of
    /// that index, whose values are matched with it in turn.
    Deferred(Ty<'tcx>, u32),
}

/// Matches `ty` with `pattern`, `out` what it gives parameter `index`:
/// false where `ty` isn't of it, so isn't that impl's; `None` where it
/// can't be told.
fn match_in<'tcx>(pattern: Ty<'tcx>, ty: Ty<'tcx>, index: u32, out: &mut Vec<Matching<'tcx>>) -> Option<bool> {
    let holds = |pattern: Ty<'tcx>| {
        pattern
            .walk()
            .any(|part| matches!(part.as_type().map(|t| *t.kind()), Some(ty::Param(param)) if param.index == index))
    };
    if !pattern.has_non_region_param() {
        return Some(pattern == ty);
    }
    let all = |pairs: &mut dyn Iterator<Item = (Ty<'tcx>, Ty<'tcx>)>, out: &mut Vec<Matching<'tcx>>| {
        let mut all = true;
        for (pattern, ty) in pairs {
            all &= match_in(pattern, ty, index, out)?;
        }
        Some(all)
    };
    match (*pattern.kind(), *ty.kind()) {
        (ty::Param(param), _) => {
            if param.index == index {
                out.push(Matching::Bound(ty));
            }
            Some(true)
        }
        (_, ty::Param(param)) => {
            if holds(pattern) {
                out.push(Matching::Deferred(pattern, param.index));
            }
            Some(true)
        }
        (ty::Adt(of, pattern_args), ty::Adt(is, args)) => match of == is {
            true => {
                let mut pairs = (pattern_args.iter().zip(args.iter()))
                    .filter_map(|(pattern, given)| Some((pattern.as_type()?, given.as_type()?)));
                all(&mut pairs, out)
            }
            false => Some(false),
        },
        (ty::Tuple(patterns), ty::Tuple(tys)) => match patterns.len() == tys.len() {
            true => all(&mut patterns.iter().zip(tys.iter()), out),
            false => Some(false),
        },
        (ty::Ref(_, pattern, of), ty::Ref(_, ty, is)) => match of == is {
            true => match_in(pattern, ty, index, out),
            false => Some(false),
        },
        (ty::Array(pattern, _), ty::Array(ty, _)) | (ty::Slice(pattern), ty::Slice(ty)) => {
            match_in(pattern, ty, index, out)
        }
        (ty::Adt(..) | ty::Tuple(_) | ty::Ref(..) | ty::Array(..) | ty::Slice(_), _) => Some(false),
        _ if !holds(pattern) => Some(true),
        _ => None,
    }
}

/// What's on the way to a value, `on_the_way`'s: each function's type
/// parameter whose value it is, and each closure it may be, called by
/// value or by `&`.
#[derive(Default)]
pub(super) struct OnTheWay<'tcx> {
    pub(super) params: [IndexSet<(DefId, u32)>; 2],
    pub(super) closures: [IndexSet<DefId>; 2],
    /// Of a call by value, each boxed `Fn` or `FnMut` given as the value of
    /// a parameter a function's naming gives it, the function and the
    /// parameter's index with its `dyn`'s type: where the call that gives
    /// it gives it as one whose call drops it (#2033).
    pub(super) boxes_given: IndexSet<(DefId, u32, Ty<'tcx>)>,
    /// And each `dyn` of one given otherwise, in what a call's given, or
    /// given back (#2050): no call gives it as one whose call drops it.
    pub(super) boxes_held: IndexSet<Ty<'tcx>>,
}

/// How a value `on_the_way` meets is given: as the call's own argument,
/// which the call's lowering gives as it is; as what a naming gives a
/// function's parameter, the function and the parameter; or held in what
/// it's given, or gives back.
#[derive(Clone, Copy)]
enum As {
    Called,
    Param(DefId, u32),
    Held,
}

/// How deep impls' parameters, each matched in what's in its impl's type's
/// place, `on_the_way` matches one in another, past which it can't tell.
const MOST_MATCHED: usize = 4;

/// What's on the way to `arg`, what `caller` gives a call by value, or by
/// `&` where `by_ref`, into `on`, by index `by_ref as usize`: each of the
/// crate's functions' type parameters whose value it is, given on as it is,
/// in a `Box` or an `AssertUnwindSafe`, or by a reference, by which the
/// call calls what's referred to by `&`; and each closure rustc infers `Fn`
/// or `FnMut` it may be, carried through every generic caller by the
/// carrier's namings (`givings` of `Fact::Called`, ADR 1115). A value given
/// in a made type, `Once<F>` of an `F`, a call of the type's doesn't call,
/// but an impl's own parameter matched in it, `impl<F> Run for Once<F>`'s
/// `F`. Each parameter is followed once, not on each way to it, as
/// `instances_of` does. False where a caller can't be seen (#2031).
pub(super) fn on_the_way<'tcx>(
    tcx: TyCtxt<'tcx>,
    (callers, held): (&Callers<'tcx>, &Held<'tcx>),
    (caller, arg): (DefId, Ty<'tcx>),
    by_ref: bool,
    on: &mut OnTheWay<'tcx>,
) -> bool {
    // A parameter's values, each matched in turn with what's pending, the
    // last first: the impls' types whose own parameter is wanted of them.
    type Task<'tcx> = (DefId, u32, bool, Vec<(Ty<'tcx>, u32)>);
    let mut todo: Vec<Task<'tcx>> = Vec::new();
    let mut met: IndexSet<Task<'tcx>> = IndexSet::default();
    // What a value of `ty`, `caller`'s, is on the way.
    let flow = |caller: DefId,
                mut ty: Ty<'tcx>,
                (mut by_ref, given): (bool, As),
                pending: &[(Ty<'tcx>, u32)],
                todo: &mut Vec<Task<'tcx>>,
                on: &mut OnTheWay<'tcx>|
     -> bool {
        if let Some((&(pattern, index), rest)) = pending.split_last() {
            let mut out = Vec::new();
            if match_in(pattern, ty, index, &mut out).is_none() {
                return false;
            }
            for matching in out {
                match matching {
                    Matching::Bound(ty) => todo.push((caller, u32::MAX, by_ref, [rest, &[(ty, u32::MAX)]].concat())),
                    // Met again, an impl's type matched in its own: a type
                    // that wraps itself, whose values the outer match follows.
                    Matching::Deferred(part, _) if rest.contains(&(part, index)) => {}
                    Matching::Deferred(part, param) => {
                        todo.push((caller, param, by_ref, [rest, &[(part, index)]].concat()))
                    }
                }
            }
            return true;
        }
        let mut itself = true;
        loop {
            match *ty.kind() {
                ty::Param(param) => todo.push((caller, param.index, by_ref, Vec::new())),
                ty::Dynamic(predicates, ..)
                    if !by_ref
                        && (predicates.principal_def_id()).is_some_and(|tr| {
                            matches!(
                                tcx.fn_trait_kind_from_def_id(tr),
                                Some(ty::ClosureKind::Fn | ty::ClosureKind::FnMut)
                            )
                        }) =>
                {
                    match given {
                        As::Called if itself => {}
                        As::Param(named, index) if itself => _ = on.boxes_given.insert((named, index, ty)),
                        _ => _ = on.boxes_held.insert(ty),
                    }
                }
                ty::Closure(id, args) if args.as_closure().kind() != ty::ClosureKind::FnOnce => {
                    on.closures[by_ref as usize].insert(id);
                }
                ty::Ref(_, inner, _) => {
                    (ty, by_ref) = (inner, true);
                    continue;
                }
                ty::Adt(adt, args)
                    if ty.is_box() || crate::lower::recognition::is_assert_unwind_safe(tcx, adt.did()) =>
                {
                    // A box in a box isn't what's given.
                    itself &= !(ty.is_box() && args.type_at(0).is_box());
                    ty = args.type_at(0);
                    continue;
                }
                _ => {}
            }
            return true;
        }
    };
    if !flow(caller, arg, (by_ref, As::Called), &[], &mut todo, on) {
        return false;
    }
    while let Some(task) = todo.pop() {
        if !met.insert(task.clone()) {
            continue;
        }
        let (owner, index, by_ref, mut pending) = task;
        if pending.len() > MOST_MATCHED {
            return false;
        }
        // A bound type, matched with no further impl, flows as it is.
        if index == u32::MAX {
            let Some((ty, _)) = pending.pop() else { continue };
            if !flow(owner, ty, (by_ref, As::Held), &pending, &mut todo, on) {
                return false;
            }
            continue;
        }
        if pending.is_empty() {
            on.params[by_ref as usize].insert((owner, index));
        }
        let Some(givings) = givings(tcx, (callers, Fact::Called(held)), (owner, index)) else {
            return false;
        };
        for giving in givings {
            let flowed = match giving {
                Giving::Given { caller, named, arg } => {
                    let given = match pending.is_empty() {
                        true => As::Param(named, index),
                        false => As::Held,
                    };
                    flow(caller, arg, (by_ref, given), &pending, &mut todo, on)
                }
                Giving::Matched { caller, given, pattern } => {
                    let pending = [pending.as_slice(), &[(pattern, index)]].concat();
                    flow(caller, given, (by_ref, As::Held), &pending, &mut todo, on)
                }
            };
            if !flowed {
                return false;
            }
        }
    }
    true
}

/// At most this many instances of one pointee, `Slot<T, U>`'s every pair of
/// what its callers give: past it, it may be any type (#1009).
const MOST_INSTANCES: usize = 64;

/// What `pointee`, which holds `owner`'s type parameters, `Slot<T>`, may be:
/// each type its callers give each parameter put in its place, `Slot<u32>`;
/// `None` where one's callers can't be seen (`given_types`), or past
/// `MOST_INSTANCES` (#1009).
pub(super) fn instances_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    (callers, fact): (&Callers<'tcx>, Fact<'_, '_, 'tcx>),
    pointee: Ty<'tcx>,
    owner: DefId,
) -> Option<IndexSet<Ty<'tcx>>> {
    instances_in(
        tcx,
        (callers, fact),
        (pointee, owner),
        (&mut IndexSet::default(), &mut IndexSet::default()),
    )
}

/// What each caller the crate sees gives `pointee`, a type of `owner`'s
/// type parameters, whatever callers it doesn't see give: what they mark
/// knowing their hint (`Fact::Marked`, #1236); and, made on the way, each
/// type a generic caller makes of its own parameters and gives it, with
/// that caller, which makes it as its instances are made (#1240).
pub(super) fn seen_instances_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    callers: &Callers<'tcx>,
    pointee: Ty<'tcx>,
    owner: DefId,
) -> (IndexSet<Ty<'tcx>>, Made<'tcx>) {
    let mut made = IndexSet::default();
    let instances = instances_in(
        tcx,
        (callers, Fact::Marked),
        (pointee, owner),
        (&mut IndexSet::default(), &mut made),
    );
    (instances.unwrap_or_default(), made)
}

/// Each instance of `pointee`, a type of `owner`'s type parameters, its
/// callers give, each choice of a type for each parameter.
fn instances_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    (callers, fact): (&Callers<'tcx>, Fact<'_, '_, 'tcx>),
    (pointee, owner): (Ty<'tcx>, DefId),
    (seen, made): TheWay<'_, 'tcx>,
) -> Option<IndexSet<Ty<'tcx>>> {
    let mut params: Vec<u32> = (pointee.walk())
        .filter_map(|part| match *part.as_type()?.kind() {
            ty::Param(param) => Some(param.index),
            _ => None,
        })
        .collect();
    params.sort_unstable();
    params.dedup();
    // Each choice of a type for each parameter, by its index.
    let mut choices: Vec<IndexMap<u32, Ty<'tcx>>> = vec![IndexMap::default()];
    for index in params {
        let given = given_types(tcx, (callers, fact), (owner, index), (seen, made))?;
        choices = (choices.into_iter())
            .flat_map(|chosen| {
                given.iter().map(move |&ty| {
                    let mut chosen = chosen.clone();
                    chosen.insert(index, ty);
                    chosen
                })
            })
            .collect();
        if choices.len() > MOST_INSTANCES {
            return None;
        }
    }
    Some(
        (choices.iter())
            .map(|chosen| {
                tcx.erase_and_anonymize_regions(pointee.fold_with(&mut ty::BottomUpFolder {
                    tcx,
                    ty_op: |t: Ty<'tcx>| match *t.kind() {
                        ty::Param(param) => chosen.get(&param.index).copied().unwrap_or(t),
                        _ => t,
                    },
                    lt_op: |r| r,
                    ct_op: |c| c,
                }))
            })
            .collect(),
    )
}
