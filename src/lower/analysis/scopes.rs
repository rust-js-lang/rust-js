//! A function's scope struct (ADR 1043, the user's rd345 decision): a
//! private struct whose one value a function makes, and that nothing but
//! its own methods and that function's closures ever holds, is the
//! function's scope, as a JS function's variables are its nested
//! functions'. ReactFlightReplyClient's `processReply` keeps its state in
//! its variables, which `resolveToJSON` and `serializeModel`, declared in
//! it, read and write, and call each other by; Rust writes that state as a
//! struct whose methods they are:
//!
//! ```ignore
//! struct Reply { next_part_id: Cell<u32>, .. }
//! impl Reply {
//!     fn resolve_to_json(self: &Rc<Self>, key: &str, value: &Unknown) -> String { .. }
//!     fn serialize_model(self: &Rc<Self>, model: &Unknown, id: u32) -> String { .. }
//! }
//! pub fn process_reply(..) {
//!     let reply = Rc::new(Reply { .. });
//!     let json = reply.serialize_model(root, 0);
//!     ..
//! }
//! ```
//!
//! Its methods are then functions declared in that function, after its
//! value's `let`, each closing over the value and calling the others by
//! their names, `function serializeModel(model, id) { .. }`, never given it.
//!
//! Each call of the function makes one value, so each of its calls is a
//! scope of its own, as each JS call is. Read of each body's MIR, as
//! borrowck reads it, the one the body is lowered from, so what's found is
//! what's lowered. A struct is one where:
//!
//! - it has fields, an object (a unit struct is `undefined`), is not
//!   `pub`, has no type or const parameters, and no trait is
//!   implemented for it: no `Clone` copies it, no `Drop` runs of it, and no
//!   generic code is given it;
//! - every item of its impls is a method of `&self`, `&mut self` or
//!   `self: &Rc<Self>` (or an `Arc`'s), of no type parameters and not
//!   `async`, taking and giving nothing else of the struct, and not a
//!   class's, an object's or a namespace's (ADRs 0761, 0943, 1037);
//! - the crate writes one literal of it, in a function's own body, not a
//!   closure's nor one of its methods', nor a generic or `async` one's,
//!   held by a `let` of that body's outermost block, which each call runs
//!   once, itself or in an `Rc::new` (`Arc::new`);
//! - and nothing holds it but that function, its methods and the closures
//!   made in them: in each of them, what holds it, or a reference to it, or
//!   an `Rc` of it, is given to no call but its methods', as their `self`,
//!   and its `Rc`'s `new`, `clone` and `deref`, nor given back, nor written
//!   anywhere but into another such variable, a field of one, or a closure
//!   made there; a constant holds none. So no other body is ever given it.
//!   Its methods are only called, never named as a value.
//!
//! So whatever holds it, wherever it's read, is the one value the call of
//! the function that made it holds, and JS's closures read that call's.

use crate::lower::Body;
use crate::lower::recognition::{HandleStep, handle_step};
use rustc_hir as hir;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, AggregateKind, Local, Operand, Place, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// A function's scope struct.
pub(crate) struct Scope {
    /// The function whose scope it is.
    pub(crate) function: DefId,
    /// The struct.
    pub(crate) adt: DefId,
    /// The function's variable holding its value, or its `Rc`.
    pub(crate) local: Local,
    /// Its methods, in the order they're written.
    pub(crate) methods: Vec<DefId>,
    /// The `let` of its value, which its methods' functions are declared after.
    pub(crate) written: Span,
}

/// The crate's scope structs.
#[derive(Default)]
pub(crate) struct Scopes {
    scopes: Vec<Scope>,
}

impl Scopes {
    /// Those `bodies` have, read of their MIR.
    pub(crate) fn new<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> Scopes {
        let of: HashMap<DefId, &mir::Body<'tcx>> = (bodies.iter())
            .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
            .collect();
        let implemented = implemented(tcx);
        let scopes = (tcx.hir_crate_items(()).definitions())
            .filter(|&id| tcx.def_kind(id) == DefKind::Struct && !implemented.contains(&id.to_def_id()))
            .filter_map(|adt| scope_of(tcx, &of, bodies, adt.to_def_id()))
            .collect();
        Scopes { scopes }
    }

    /// The scopes `function` makes, in the order their structs are written.
    pub(crate) fn of_function(&self, function: DefId) -> impl Iterator<Item = &Scope> {
        self.scopes.iter().filter(move |s| s.function == function)
    }

    /// The scope `method` is a method of, if any.
    pub(crate) fn of_method(&self, method: DefId) -> Option<&Scope> {
        self.scopes.iter().find(|s| s.methods.contains(&method))
    }

    /// Keep those of the structs `keep` keeps: an `Rc` counted is an object
    /// of its own, `{ value, strong, weak }` (ADR 0320), no scope.
    pub(crate) fn retain(&mut self, keep: impl Fn(DefId) -> bool) {
        self.scopes.retain(|s| keep(s.adt));
    }
}

/// The structs a trait is implemented for here.
fn implemented(tcx: TyCtxt<'_>) -> HashSet<DefId> {
    (tcx.hir_crate_items(()).definitions())
        .filter(|&id| matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true }))
        .filter_map(
            |id| match tcx.type_of(id).instantiate_identity().skip_normalization().kind() {
                ty::Adt(adt, _) => Some(adt.did()),
                _ => None,
            },
        )
        .collect()
}

/// `adt`'s scope, if it's one.
fn scope_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    of: &HashMap<DefId, &mir::Body<'tcx>>,
    bodies: &[&Body<'tcx>],
    adt: DefId,
) -> Option<Scope> {
    let generics = tcx.generics_of(adt);
    if tcx.visibility(adt).is_public() || generics.own_counts().types > 0 || generics.own_counts().consts > 0 {
        return None;
    }
    let def = tcx.adt_def(adt);
    // A unit struct is `undefined`, no object a function closes over (ADR
    // 0020): what its methods are given isn't a variable.
    if def.all_fields().next().is_none()
        || (def.all_fields())
            .any(|field| mentions(tcx.type_of(field.did).instantiate_identity().skip_normalization(), adt))
    {
        return None;
    }
    let methods = methods_of(tcx, adt)?;
    // Its one literal, and the function that writes it.
    let mut literals = Vec::new();
    for body in bodies {
        let Some(mir) = &body.mir else { continue };
        for (promoted, body_of) in std::iter::once((false, &mir.body)).chain(mir.promoted.iter().map(|p| (true, p))) {
            for data in body_of.basic_blocks.iter() {
                for statement in &data.statements {
                    if let StatementKind::Assign(assign) = &statement.kind
                        && let Rvalue::Aggregate(kind, _) = &assign.1
                        && matches!(**kind, AggregateKind::Adt(did, ..) if did == adt)
                    {
                        literals.push((body.def_id, promoted, assign.0));
                    }
                }
            }
        }
    }
    let [(function, false, made)] = literals[..] else {
        return None;
    };
    let body = of.get(&function)?;
    if !matches!(tcx.def_kind(function), DefKind::Fn | DefKind::AssocFn)
        || methods.contains(&function)
        || tcx.generics_of(function).requires_monomorphization(tcx)
        || tcx.asyncness(function).is_async()
    {
        return None;
    }
    let local = held_by(tcx, body, made)?;
    let written = let_of(tcx, function, body, local)?;
    // The bodies that may hold it: the function's, its methods' and their
    // closures'.
    let owners: HashSet<DefId> = methods.iter().copied().chain([function]).collect();
    for body in bodies {
        let Some(mir) = &body.mir else { continue };
        let holds = owners.contains(&tcx.typeck_root_def_id(body.def_id));
        if !methods_only_called(tcx, &mir.body, &methods) || (holds && !held_as_scope(tcx, &mir.body, adt, &methods)) {
            return None;
        }
    }
    Some(Scope {
        function,
        adt,
        local,
        methods,
        written,
    })
}

/// The methods of `adt`'s impls, in the order they're written, if they're
/// all a scope's.
fn methods_of(tcx: TyCtxt<'_>, adt: DefId) -> Option<Vec<DefId>> {
    let mut methods = Vec::new();
    for &imp in tcx.inherent_impls(adt) {
        for &item in tcx.associated_item_def_ids(imp) {
            if tcx.def_kind(item) != DefKind::AssocFn
                || !tcx.associated_item(item).is_method()
                || crate::lower::classes::member(tcx, item).is_some()
                || tcx.generics_of(item).own_counts().types > 0
                || tcx.generics_of(item).own_counts().consts > 0
                || tcx.asyncness(item).is_async()
            {
                return None;
            }
            let sig = tcx.fn_sig(item).instantiate_identity().skip_binder();
            let (receiver, rest) = sig.inputs().split_first()?;
            let ty::Ref(_, pointee, _) = *receiver.kind() else {
                return None;
            };
            let pointee = crate::lower::recognition::rc_pointee(tcx, pointee).unwrap_or(pointee);
            if !matches!(pointee.kind(), ty::Adt(of, _) if of.did() == adt)
                || (rest.iter().chain([&sig.output()])).any(|&ty| mentions(ty, adt))
            {
                return None;
            }
            methods.push(item);
        }
    }
    methods.sort_by_key(|&id| tcx.def_span(id));
    (!methods.is_empty()).then_some(methods)
}

/// Whether `ty` holds a value of `adt`, or a reference to one: not what a
/// closure captures, which its body holds.
pub(crate) fn mentions(ty: Ty<'_>, adt: DefId) -> bool {
    let mut walk = ty.walk();
    while let Some(arg) = walk.next() {
        let Some(ty) = arg.as_type() else { continue };
        match ty.kind() {
            ty::Adt(of, _) if of.did() == adt => return true,
            ty::Closure(..) | ty::CoroutineClosure(..) => walk.skip_current_subtree(),
            _ => {}
        }
    }
    false
}

/// The variable of `body` its literal, `made`, is held by: itself, or the
/// `Rc` it's moved into.
fn held_by<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, made: Place<'tcx>) -> Option<Local> {
    let made = made.as_local()?;
    if named(body, made).is_some() {
        return Some(made);
    }
    (body.basic_blocks.iter()).find_map(|data| match &data.terminator().kind {
        TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } if let Some((id, generic_args)) = crate::lower::fn_def(func.ty(&body.local_decls, tcx))
            && handle_step(tcx, id, generic_args) == Some(HandleStep::RcNew)
            && matches!(&args[..], [arg] if matches!(&arg.node, Operand::Move(p) if p.as_local() == Some(made))) =>
        {
            destination.as_local().filter(|&local| named(body, local).is_some())
        }
        _ => None,
    })
}

/// Where the program names `local`, a variable of its own.
fn named(body: &mir::Body<'_>, local: Local) -> Option<Span> {
    (body.var_debug_info.iter()).find_map(|info| match info.value {
        mir::VarDebugInfoContents::Place(place) if place.as_local() == Some(local) => Some(info.source_info.span),
        _ => None,
    })
}

/// The `let` of `function`'s outermost block that binds `local`, its
/// whole statement.
fn let_of(tcx: TyCtxt<'_>, function: DefId, body: &mir::Body<'_>, local: Local) -> Option<Span> {
    let span = named(body, local)?;
    let hir::ExprKind::Block(block, _) = tcx.hir_body_owned_by(function.as_local()?).value.kind else {
        return None;
    };
    (block.stmts.iter()).find_map(|stmt| match stmt.kind {
        hir::StmtKind::Let(binding)
            if let hir::PatKind::Binding(_, _, ident, None) = binding.pat.kind
                && (ident.span == span || binding.pat.span == span) =>
        {
            Some(stmt.span)
        }
        _ => None,
    })
}

/// Whether `body` names each of `methods` only to call it.
fn methods_only_called<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, methods: &[DefId]) -> bool {
    struct Named<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        methods: &'a [DefId],
        found: usize,
    }
    impl<'tcx> mir::visit::Visitor<'tcx> for Named<'_, 'tcx> {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            if let ty::FnDef(id, _) = *constant.ty().kind()
                && self.methods.contains(&id)
            {
                self.found += 1;
            }
            let _ = self.tcx;
        }
    }
    let mut named = Named { tcx, methods, found: 0 };
    mir::visit::Visitor::visit_body(&mut named, body);
    let called = (body.basic_blocks.iter())
        .filter(|data| {
            matches!(&data.terminator().kind, TerminatorKind::Call { func, .. }
                if crate::lower::fn_def(func.ty(&body.local_decls, tcx)).is_some_and(|(id, _)| methods.contains(&id))
                    && matches!(func, Operand::Constant(_)))
        })
        .count();
    named.found == called
}

/// Whether what holds a value of `adt` in `body`, one of its scope's
/// bodies, is only read as its scope's, as the module's comment says:
/// `receiver`, a method's `self`, the one parameter that may hold it.
fn held_as_scope<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, adt: DefId, methods: &[DefId]) -> bool {
    let holds = |ty: Ty<'tcx>| mentions(ty, adt);
    let place_holds = |place: &Place<'tcx>| holds(place.ty(&body.local_decls, tcx).ty);
    let operand_holds = |operand: &Operand<'tcx>| holds(operand.ty(&body.local_decls, tcx));
    // What it gives back holds none: what's given it is given by a call,
    // which none of the crate's other functions is given.
    if holds(body.local_decls[mir::RETURN_PLACE].ty) {
        return false;
    }
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    let (to, rvalue) = &**assign;
                    let fine = match rvalue {
                        Rvalue::Aggregate(kind, operands) => match **kind {
                            AggregateKind::Adt(did, ..) if did == adt => to.as_local().is_some(),
                            // A closure made here, which captures it.
                            AggregateKind::Closure(..) => true,
                            _ => !operands.iter().any(operand_holds),
                        },
                        Rvalue::Ref(_, _, from)
                        | Rvalue::CopyForDeref(from)
                        | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..)
                            if place_holds(from) =>
                        {
                            true
                        }
                        _ => !holds_any(Ok(statement), &holds, &place_holds),
                    };
                    if !fine {
                        return false;
                    }
                }
                StatementKind::SetDiscriminant { .. } | StatementKind::Intrinsic(_)
                    if holds_any(Ok(statement), &holds, &place_holds) =>
                {
                    return false;
                }
                _ => {}
            }
        }
        match &data.terminator().kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                let called = crate::lower::fn_def(func.ty(&body.local_decls, tcx));
                let fine = match called {
                    // One of its methods, given it as its `self`.
                    Some((id, _)) if methods.contains(&id) => true,
                    // An `Rc` of it, made, cloned or read.
                    Some((id, generic_args))
                        if matches!(
                            handle_step(tcx, id, generic_args),
                            Some(HandleStep::RcNew | HandleStep::RcClone | HandleStep::RcDeref)
                        ) && (generic_args.types()).any(&holds) =>
                    {
                        true
                    }
                    _ => {
                        !holds(func.ty(&body.local_decls, tcx))
                            && !args.iter().any(|arg| operand_holds(&arg.node))
                            && !place_holds(destination)
                    }
                };
                if !fine {
                    return false;
                }
            }
            TerminatorKind::Drop { .. }
            | TerminatorKind::Goto { .. }
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate(_)
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. } => {}
            _ if holds_any(Err(data.terminator()), &holds, &place_holds) => return false,
            _ => {}
        }
    }
    true
}

/// Whether a statement, or a terminator, reads a place `place_holds`, or a
/// constant of a type `holds`.
fn holds_any<'tcx>(
    read: Result<&mir::Statement<'tcx>, &mir::Terminator<'tcx>>,
    holds: &dyn Fn(Ty<'tcx>) -> bool,
    place_holds: &dyn Fn(&Place<'tcx>) -> bool,
) -> bool {
    let mut visitor = Holds {
        found: false,
        holds,
        place_holds,
    };
    match read {
        Ok(statement) => mir::visit::Visitor::visit_statement(&mut visitor, statement, mir::Location::START),
        Err(terminator) => mir::visit::Visitor::visit_terminator(&mut visitor, terminator, mir::Location::START),
    }
    visitor.found
}

/// Finds a place, or a constant, that holds a value of the struct.
struct Holds<'a, 'tcx> {
    found: bool,
    holds: &'a dyn Fn(Ty<'tcx>) -> bool,
    place_holds: &'a dyn Fn(&Place<'tcx>) -> bool,
}

impl<'tcx> mir::visit::Visitor<'tcx> for Holds<'_, 'tcx> {
    fn visit_place(&mut self, place: &Place<'tcx>, _: mir::visit::PlaceContext, _: mir::Location) {
        self.found |= (self.place_holds)(place);
    }

    fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
        self.found |= (self.holds)(constant.ty());
    }
}
