//! How much of std rust-js lowers (ADR 0314): each stable inherent method
//! of its data structures, each provided method of the iterator traits and
//! each method of the adapters' impls, and whether recognition knows it,
//! for the ratchet in `test/std-coverage.test.ts`, which no method may
//! leave. And, beside each, what its body in std is made of (ADR
//! 0561): whether rust-js could lower it from the MIR the
//! toolchain ships, or it reaches what JS hasn't, addressable memory.

use std::collections::HashMap;
use std::fmt::Write as _;

use rustc_middle::mir::{self, Operand, RuntimeChecks, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

use super::recognition::{self, Recognition, registry};

/// What a std method's body is made of, through every std body it calls
/// whose MIR the toolchain ships; in order, each worse than the last, so
/// a body is the worst it reaches.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Body {
    /// MIR all the way down to the vocabulary rust-js lowers: a trait's
    /// method on a type parameter, a closure's call, `?`, a drop, a panic.
    Safe,
    /// Reaches a std-private trait's method on a type parameter, one only
    /// std can name, `SpecFold`, `ZipImpl`, `TrustedRandomAccess`: a
    /// dictionary rust-js would have to build for each type given.
    Spec,
    /// Reaches a std body the toolchain ships no MIR of, before any memory.
    Opaque,
    /// Reaches addressable memory: a raw pointer made or read through, a
    /// transmute, an intrinsic, or one of `std::ptr`'s.
    Memory,
    /// The method itself has no MIR: not generic, not inlined.
    None,
}

impl Body {
    fn name(self) -> &'static str {
        match self {
            Body::Safe => "safe",
            Body::Spec => "spec",
            Body::Opaque => "opaque",
            Body::Memory => "memory",
            Body::None => "none",
        }
    }
}

/// Each type's stable methods, `+ Vec::push` where rust-js knows it and
/// `- Vec::shrink_to` where it doesn't, each with what its body is made
/// of, under `# Vec 35 of 160; bodies: 20 safe, 1 spec, 3 opaque, 10
/// memory, 2 none`.
pub fn report<'tcx>(tcx: TyCtxt<'tcx>, dependencies: &crate::library::Dependencies) -> String {
    let instances = super::Instances::default();
    let foreign = super::library::Foreign::new(tcx, dependencies, &instances);
    let mut bodies = Bodies::default();
    let mut out = String::new();
    for (name, measured) in registry::measured(tcx) {
        let mut methods: Vec<(String, bool, Body)> = Vec::new();
        let (owners, provided) = match measured {
            registry::Measured::Impls(impls) => (impls, false),
            registry::Measured::Trait(def) => (vec![def], true),
        };
        for owner in owners {
            for item in tcx.associated_items(owner).in_definition_order() {
                let def_id = item.def_id;
                // A trait's method is measured where it's provided, a body. An
                // impl's method is as stable, and as hidden, as the trait's it
                // implements, or as its impl, which has the attribute.
                let trait_item = item.trait_item_def_id();
                let shown = trait_item.unwrap_or(def_id);
                let stability = tcx.lookup_stability(shown).or_else(|| tcx.lookup_stability(owner));
                if !matches!(item.kind, ty::AssocKind::Fn { .. })
                    || (provided && !item.defaultness(tcx).has_value())
                    || stability.is_none_or(|s| s.is_unstable())
                    || tcx.is_doc_hidden(shown)
                    || tcx.is_doc_hidden(def_id)
                {
                    continue;
                }
                let recognition = Recognition {
                    tcx,
                    typing_env: ty::TypingEnv::non_body_analysis(tcx, def_id),
                    trait_impls: &[],
                    foreign: &foreign,
                };
                let args = ty::GenericArgs::identity_for_item(tcx, def_id);
                // One known for some of its own type arguments counts: a `str`'s
                // `split` of a `&str` or a `char` pattern, its patterns rust-js takes,
                // and a slice's `get` of an index.
                let parent_count = tcx.generics_of(def_id).parent_count;
                let given = |pattern: ty::Ty<'tcx>| {
                    ty::GenericArgs::for_item(tcx, def_id, |param, _| match param.kind {
                        ty::GenericParamDefKind::Type { .. } if param.index as usize >= parent_count => pattern.into(),
                        _ => args[param.index as usize],
                    })
                };
                // An impl's method is called as its trait's, on the impl's type:
                // `Iterator::next` of a `Peekable<I>`, which is what `classify` is asked.
                let of_trait = matches!(tcx.def_kind(owner), rustc_hir::def::DefKind::Impl { of_trait: true });
                let as_called = match (trait_item, of_trait.then(|| tcx.impl_opt_trait_ref(owner)).flatten()) {
                    (Some(trait_item), Some(trait_ref)) => {
                        Some((trait_item, trait_ref.instantiate_identity().skip_normalization().args))
                    }
                    _ => None,
                };
                // Known as an idiom `classify` writes, or as std's own body,
                // compiled into `@rust-js/std` (ADR 0561).
                let known = recognition::is_compiled_std(tcx, def_id)
                    || match as_called {
                        Some((trait_item, trait_args)) => recognition.classify(trait_item, trait_args).is_some(),
                        None => [ty::Ty::new_static_str(tcx), tcx.types.char, tcx.types.usize]
                            .into_iter()
                            .any(|pattern| recognition.classify(def_id, given(pattern)).is_some()),
                    };
                let body = bodies.of(tcx, def_id);
                let method = format!("{name}::{}", tcx.item_name(def_id));
                match methods.iter_mut().find(|(m, _, _)| *m == method) {
                    // A method of two impls, a slice's `to_vec` say: its
                    // worse body, as the reader must take it.
                    Some((_, k, b)) => {
                        *k |= known;
                        *b = (*b).max(body);
                    }
                    None => methods.push((method, known, body)),
                }
            }
        }
        if methods.is_empty() {
            continue;
        }
        methods.sort();
        let known = methods.iter().filter(|(_, k, _)| *k).count();
        let count = |kind: Body| methods.iter().filter(|(_, _, b)| *b == kind).count();
        let _ = writeln!(
            out,
            "# {name} {known} of {}; bodies: {} safe, {} spec, {} opaque, {} memory, {} none",
            methods.len(),
            count(Body::Safe),
            count(Body::Spec),
            count(Body::Opaque),
            count(Body::Memory),
            count(Body::None)
        );
        for (method, known, body) in methods {
            let _ = writeln!(out, "{} {method}\t{}", if known { "+" } else { "-" }, body.name());
        }
    }
    out
}

/// What each std function's body is made of, each asked once: the measure
/// follows calls, so one function's answer is many methods'.
#[derive(Default)]
struct Bodies {
    known: HashMap<DefId, Body>,
}

impl Bodies {
    fn of(&mut self, tcx: TyCtxt<'_>, def_id: DefId) -> Body {
        if let Some(&body) = self.known.get(&def_id) {
            return body;
        }
        if !tcx.is_mir_available(def_id) {
            self.known.insert(def_id, Body::None);
            return Body::None;
        }
        // A cycle, `fold` through `try_fold` through `fold`, is read as safe
        // until a body on it says otherwise, as its other calls do.
        self.known.insert(def_id, Body::Safe);
        let body = tcx.optimized_mir(def_id);
        let typing_env = ty::TypingEnv::post_analysis(tcx, def_id);
        let mut worst = Body::Safe;
        for block in reachable(tcx, body) {
            let data = &body.basic_blocks[block];
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && super::pointers::mir_addresses_memory(tcx, body, assign)
                {
                    worst = Body::Memory;
                }
            }
            let Some(terminator) = &data.terminator else { continue };
            let (TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. }) = &terminator.kind else {
                continue;
            };
            let Some((callee, args)) = func.const_fn_def() else {
                continue;
            };
            let called = match super::items::resolve_instance_in(tcx, typing_env, callee, args) {
                Ok(Some(instance)) => match instance.def {
                    // A callee the toolchain ships, read as this body is.
                    ty::InstanceKind::Item(def) => self.of_callee(tcx, def),
                    ty::InstanceKind::Intrinsic(def) if recognition::memory_intrinsic(tcx, def) => Body::Memory,
                    // A shim, a drop, a `dyn`'s or a closure's call: the vocabulary.
                    _ => Body::Safe,
                },
                // A trait's method of a type parameter: the vocabulary, of a
                // trait a program can name; std's own, a specialization's.
                _ => match tcx.trait_of_assoc(callee) {
                    Some(trait_def) if recognition::std_private(tcx, trait_def) => Body::Spec,
                    _ => Body::Safe,
                },
            };
            worst = worst.max(called);
        }
        self.known.insert(def_id, worst);
        worst
    }

    /// A callee's body: a panic's or an abort's, which never returns, is
    /// the vocabulary (ADR 0012), and so is `std::ptr`'s, memory.
    fn of_callee(&mut self, tcx: TyCtxt<'_>, def: DefId) -> Body {
        if recognition::addresses_memory(tcx, def) {
            return Body::Memory;
        }
        if matches!(
            tcx.def_kind(def),
            rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
        ) && tcx.fn_sig(def).skip_binder().output().skip_binder().is_never()
        {
            return Body::Safe;
        }
        match self.of(tcx, def) {
            // A callee with no MIR is opaque to its caller.
            Body::None => Body::Opaque,
            other => other,
        }
    }
}

/// Whether `data` assumes `UbChecks`, a precondition's broken arm, which
/// never runs, as rust-js reads it as off (ADRs 0364, 0917).
pub(in crate::lower) fn assumes_ub_checks(data: &mir::BasicBlockData<'_>) -> bool {
    data.statements.iter().any(|statement| {
        matches!(&statement.kind, StatementKind::Intrinsic(intrinsic)
            if matches!(&**intrinsic, mir::NonDivergingIntrinsic::Assume(Operand::RuntimeChecks(RuntimeChecks::UbChecks))))
    })
}

/// The blocks a body runs, with `UbChecks` read as off (ADR 0364): a
/// precondition check's blocks are left out, as rust-js leaves them out,
/// of `UbChecks` or of `check_language_ub()`, which reads it, and a block
/// that assumes it, `assume(UbChecks)`, a precondition's broken arm; what
/// std's bodies compiled call is found in these (ADR 0917).
pub(in crate::lower) fn reachable(tcx: TyCtxt<'_>, body: &mir::Body<'_>) -> Vec<mir::BasicBlock> {
    let mut ub_checks: Vec<mir::Local> = (body.basic_blocks.iter())
        .flat_map(|data| &data.statements)
        .filter_map(|statement| match &statement.kind {
            StatementKind::Assign(assign)
                if matches!(
                    assign.1,
                    Rvalue::Use(Operand::RuntimeChecks(RuntimeChecks::UbChecks), _)
                ) =>
            {
                assign.0.as_local()
            }
            _ => None,
        })
        .collect();
    ub_checks.extend((body.basic_blocks.iter()).filter_map(|data| {
        match &data.terminator.as_ref()?.kind {
            TerminatorKind::Call { func, destination, .. }
                if func
                    .const_fn_def()
                    .is_some_and(|(callee, _)| recognition::is_check_language_ub(tcx, callee)) =>
            {
                destination.as_local()
            }
            _ => None,
        }
    }));
    let mut seen = vec![mir::START_BLOCK];
    let mut at = 0;
    while at < seen.len() {
        let block = seen[at];
        at += 1;
        let Some(terminator) = &body.basic_blocks[block].terminator else {
            continue;
        };
        let next: Vec<mir::BasicBlock> = match &terminator.kind {
            TerminatorKind::SwitchInt { discr, targets }
                if matches!(discr, Operand::RuntimeChecks(RuntimeChecks::UbChecks))
                    || discr
                        .place()
                        .and_then(|place| place.as_local())
                        .is_some_and(|local| ub_checks.contains(&local)) =>
            {
                vec![targets.target_for_value(0)]
            }
            _ => terminator.successors().collect(),
        };
        for block in next
            .into_iter()
            .filter(|&block| !assumes_ub_checks(&body.basic_blocks[block]))
        {
            if !seen.contains(&block) {
                seen.push(block);
            }
        }
    }
    seen
}
