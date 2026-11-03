//! Which `self` a call of a namespace's method gives is the namespace as
//! Rust names it, and what holds it there (ADR 1037): the MIR answers of
//! `classes.rs`'s namespaces.

use super::{Member, is_namespace, member};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Whether `receiver`, the `self` a call of a namespace's method in `body`
/// gives it, is the namespace as Rust names it, whose call is the function
/// itself, not a value of it JS may have made of another instance of its
/// module, whose call is that value's (ADR 1037): the unit struct's
/// constant, through what MIR holds it in, or the `self` of the namespace's
/// own method `body` is. The one answer of both the lowering and which
/// library functions the crate compiles (ADR 0914, rd311).
pub(crate) fn names_namespace<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    receiver: &rustc_middle::mir::Operand<'tcx>,
) -> bool {
    namespace_held_in(tcx, body, receiver).is_some()
}

/// Where `names_namespace` finds the namespace `receiver` names: the
/// locals MIR holds it in on the way, with each one's write; `None` of a
/// receiver that's no namespace as Rust names it.
fn namespace_held_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    receiver: &rustc_middle::mir::Operand<'tcx>,
) -> Option<Vec<(rustc_middle::mir::Local, rustc_middle::mir::Location)>> {
    use rustc_middle::mir::{Operand, Rvalue, StatementKind};
    let owner = body.source.def_id();
    // Each local's one write, where it is, or `None` of one written more
    // or less than once.
    let one_write = |local: rustc_middle::mir::Local| {
        let mut writes = (body.basic_blocks.iter_enumerated())
            .flat_map(|(block, data)| {
                (data.statements.iter().enumerate())
                    .map(move |(statement_index, s)| (rustc_middle::mir::Location { block, statement_index }, s))
            })
            .filter_map(|(at, statement)| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.local == local => Some((at, &assign.1)),
                _ => None,
            });
        match (writes.next(), writes.next()) {
            (Some(write), None) => Some(write),
            _ => None,
        }
    };
    let mut held = Vec::new();
    let mut operand = receiver.clone();
    for _ in 0..body.local_decls.len() {
        let local = match &operand {
            Operand::Constant(_) => return Some(held),
            Operand::Copy(place) | Operand::Move(place) if place.projection.is_empty() => place.local,
            _ => return None,
        };
        if local.as_usize() <= body.arg_count {
            return (local.as_usize() == 1
                && !tcx.is_closure_like(owner)
                && member(tcx, owner) == Some(Member::Namespace))
            .then_some(held);
        }
        let (at, write) = one_write(local)?;
        let called = (body.basic_blocks.iter()).any(|data| {
            matches!(&data.terminator().kind, rustc_middle::mir::TerminatorKind::Call { destination, .. }
                if destination.local == local)
        });
        if called {
            return None;
        }
        held.push((local, at));
        operand = match write {
            Rvalue::Use(operand, _) => operand.clone(),
            Rvalue::CopyForDeref(place) => Operand::Copy(*place),
            // A reborrow, `&*r`.
            Rvalue::Ref(_, _, place) if matches!(place.projection[..], [rustc_middle::mir::PlaceElem::Deref]) => {
                Operand::Copy(place.local.into())
            }
            // `&Reconciler` of the struct made where it's borrowed.
            Rvalue::Ref(_, _, place) if place.projection.is_empty() => {
                let (made_at, made) = one_write(place.local)?;
                let made = match made {
                    Rvalue::Aggregate(kind, _) => {
                        matches!(**kind, rustc_middle::mir::AggregateKind::Adt(adt, ..) if is_namespace(tcx, adt))
                    }
                    Rvalue::Use(Operand::Constant(_), _) => true,
                    _ => false,
                };
                return made.then(|| {
                    held.push((place.local, made_at));
                    held
                });
            }
            _ => return None,
        };
    }
    None
}

/// The writes in `body` of what holds a namespace only the calls that name
/// it read (`names_namespace`), each call's `self` left out: no JS, so a
/// library's namespace nothing reads is never imported, React DOM's
/// `ReactFiberReconciler.attempt_synchronous_hydration(fiber)` only
/// `attemptSynchronousHydration(fiber)` (rd310, ADR 1037).
pub(crate) fn unread_namespaces<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
) -> impl Iterator<Item = rustc_middle::mir::Location> {
    use rustc_middle::mir::visit::{PlaceContext, Visitor};
    use rustc_middle::mir::{Local, Location, StatementKind, TerminatorKind};
    // Each read of a local and where it is; not a `let`'s fake read, which
    // is no JS.
    struct Reads<'b, 'tcx>(&'b rustc_middle::mir::Body<'tcx>, HashMap<Local, Vec<Location>>);
    impl<'tcx> Visitor<'tcx> for Reads<'_, 'tcx> {
        fn visit_local(&mut self, local: Local, context: PlaceContext, at: Location) {
            let unwritten = (self.0.stmt_at(at).left()).is_some_and(|s| {
                matches!(
                    s.kind,
                    StatementKind::FakeRead(_) | StatementKind::PlaceMention(_) | StatementKind::AscribeUserType(..)
                )
            });
            if context.is_use() && !context.is_place_assignment() && !context.is_storage_marker() && !unwritten {
                self.1.entry(local).or_default().push(at);
            }
        }
    }
    let mut held: HashMap<Local, Location> = HashMap::new();
    // Where each named receiver is read, its call.
    let mut named_at: HashSet<Location> = HashSet::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
            && let Some((callee, _)) = func.const_fn_def()
            && member(tcx, callee) == Some(Member::Namespace)
            && let Some(receiver) = args.first()
            && let Some(locals) = namespace_held_in(tcx, body, &receiver.node)
        {
            named_at.insert(body.terminator_loc(block));
            held.extend(locals);
        }
    }
    let mut reads = Reads(body, HashMap::new());
    reads.visit_body(body);
    // A local whose reads are all a named call's `self` or another such
    // local's write, until none is left out.
    loop {
        let writes: HashSet<Location> = held.values().copied().collect();
        let before = held.len();
        held.retain(|local, _| {
            (reads.1.get(local).into_iter().flatten()).all(|at| named_at.contains(at) || writes.contains(at))
        });
        if held.len() == before {
            return held.into_values();
        }
    }
}

/// The value a namespace thread-local's initializer, `body`, takes apart
/// into `fields`, its module's exports (rd610, the user's decision, ADR
/// 1037, amended): `{ const renderer = create(3); return { render:
/// renderer.render, .. }; }` is `create(3)`, `export const { render, .. } =
/// create(3);`, where its literal takes each field, once, in the exports'
/// order, from the same-named field of the one value it makes and reads
/// nowhere else: JS's destructuring reads the same properties of the same
/// value in the same order, once each, a getter's too. A value with fields
/// left to drop, whose drops are statements, isn't one.
pub(crate) fn taken_apart(fields: &[String], body: &[crate::js::Stmt]) -> Option<crate::js::Expr> {
    use crate::js::{ExprKind, Prop, Stmt, StmtKind};
    let [
        Stmt {
            kind: StmtKind::Const(made, value),
            ..
        },
        Stmt {
            kind: StmtKind::Return(Some(literal)),
            ..
        },
    ] = body
    else {
        return None;
    };
    let ExprKind::Object(props, _) = &literal.kind else {
        return None;
    };
    let mut keys = Vec::new();
    for prop in props {
        let Prop::Field(key, read) = prop else { return None };
        let ExprKind::Member(of, field) = &read.kind else {
            return None;
        };
        if field != key || !matches!(&of.kind, ExprKind::Var(v) if v == made) {
            return None;
        }
        keys.push(key.as_str());
    }
    (keys.iter().copied().eq(fields.iter().map(String::as_str))).then(|| value.clone())
}

/// Whether each of the crate's namespaces' methods (ADR 1037) is its
/// module's export by its property's name, `close`, which a call through
/// the namespace's value, the module imported whole, reads: one its module
/// names apart, `close$1`, of a function of its module that has that name,
/// is refused, as a call through the value would be that function's
/// (#585, ADR 0262's "can't be preserved" case). Not one that's the
/// function it passes its parameters on to, `forwarded`, which that
/// function is (#566). `false` where one is refused.
pub(in crate::lower) fn exported_by_property(
    tcx: TyCtxt<'_>,
    fns: &HashMap<DefId, crate::lower::FnInfo>,
    forwarded: &HashMap<DefId, DefId>,
) -> bool {
    let mut valid = true;
    let mut methods: Vec<_> = (fns.iter())
        .filter(|&(&id, _)| member(tcx, id) == Some(Member::Namespace) && !forwarded.contains_key(&id))
        .collect();
    methods.sort_by_key(|&(&id, _)| tcx.def_span(id).lo());
    for (&method, info) in methods {
        let property = super::property(tcx, method);
        if info.exported() == property {
            continue;
        }
        let other =
            (fns.iter()).find(|&(&id, other)| id != method && other.module == info.module && other.name == property);
        let message = match other {
            Some((&other, _)) => format!(
                "rust-js: the namespace's method `{}` and `{}` are both `{property}` in JS, and a call through \
                 the namespace reads its module's `{property}`, which would be `{}`; give one another name with \
                 `#[rust_js::name = \"..\"]`, or have the method only pass its parameters on to `{}`, which it then is",
                tcx.def_path_str(method),
                tcx.def_path_str(other),
                tcx.def_path_str(other),
                tcx.def_path_str(other),
            ),
            None => format!(
                "rust-js: the namespace's method `{}` is `{property}` in JS, which its module can't export it \
                 by, and a call through the namespace reads `{property}`; give it another name with \
                 `#[rust_js::name = \"..\"]`",
                tcx.def_path_str(method),
            ),
        };
        let mut error = tcx.dcx().struct_span_err(tcx.def_span(method), message);
        if let Some((&other, _)) = other {
            error = error.with_span_note(tcx.def_span(other), format!("`{property}` here"));
        }
        error.emit();
        valid = false;
    }
    valid
}
