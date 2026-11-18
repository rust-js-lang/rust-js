//! What of the crate is a value at hand, one a builtin or the program
//! made, whose property reads meet no getter of the program's (#2106, ADR
//! 1173). Split from runs_nothing.rs by concept.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::bindings::{JsForm, creates, is_binding, js_form};
use crate::lower::recognition::{Recognition, Std, StdItem};
use crate::lower::std_types::option::OptionPlaceOp;
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Pat, PatKind, StmtKind, Thir};
use rustc_span::def_id::{DefId, LocalDefId};

mod local_cells;
mod slots;

/// The crate's variables whose value is one a builtin made, at hand (#2106):
/// what a constructor makes, `new X`, or a binding that `creates` its
/// result, `regExp.exec(s)`; through `unwrap`, `expect`, `clone` and `?`,
/// a `let`, an `if let` or a `match` of one, and the parameter of an
/// `Option`'s or a `Result`'s combinator's closure given one's payload,
/// `.and_then(|matched| matched.get(1))`, a closure's capture of one too.
/// Never what a builtin hands back from what a JS caller stored, `Map.get`'s.
/// A variable is at hand only where every write of it is: its binding's
/// and each assignment's, to it or to a closure's capture of it; one lent
/// `&mut` or bound `ref mut`, whose writes aren't seen, never is. And
/// what's passed on (#2111): a cast of one, `js::cast(Some(js::unknown(x)))`;
/// a parameter of a crate-private function that never escapes, where every
/// call gives one; and what `KEY.with(|value| ..)` gives of a thread-local
/// whose initializer makes one, never replaced, the
/// crate's own or a library's that its contract says so of
/// (`ItemFact::AtHand`).
#[derive(Default)]
pub(in crate::lower) struct AtHand {
    vars: IndexSet<LocalVarId>,
    /// The crate's thread-locals no code but its own reaches every write of
    /// which is at hand, its slots (#2114): what they hold read back is.
    slots: IndexSet<DefId>,
    /// What a slot's `with_borrow` and `with_borrow_mut` closures are lent,
    /// at hand as the slot is: what's written through it is of its writes.
    slot_params: IndexSet<LocalVarId>,
    /// The crate's closures whose value is at hand, `get_or_insert_with(||
    /// js::map::new(None))`'s.
    closures: IndexSet<DefId>,
    /// The variables of a local cell its closures share every write of
    /// which is at hand (`local_cells`, #2114): what `borrow()` reads of it
    /// is.
    cells: IndexSet<LocalVarId>,
    /// The crate's thread-locals whose value is at hand, which its
    /// contract says of them (`ItemFact::AtHand`).
    keys: IndexSet<DefId>,
    /// The functions only calls reach (`only_called`) whose value, their
    /// body's and each `return`'s, is at hand (#2116): a call of one is.
    returns: IndexSet<DefId>,
}

/// The combinators whose closure is given the payload of what they're of.
const PAYLOAD_COMBINATORS: [&str; 9] = [
    "and_then",
    "map",
    "map_or",
    "map_or_else",
    "is_some_and",
    "is_none_or",
    "is_ok_and",
    "filter",
    "inspect",
];

impl AtHand {
    /// Of each of the crate's bodies, until it holds; `inits`, each
    /// thread-local's initializer to its key.
    pub(in crate::lower) fn of_crate<'tcx>(
        recognition: &Recognition<'_, 'tcx>,
        whole: &[&crate::lower::Body<'tcx>],
        (inits, closed): (&IndexMap<LocalDefId, LocalDefId>, &IndexSet<DefId>),
    ) -> Self {
        let tcx = recognition.tcx;
        let mut at_hand = AtHand::default();
        let bodies: Vec<(DefId, &Thir<'tcx>)> = whole.iter().map(|body| (body.def_id, &body.thir)).collect();
        let bodies = &bodies[..];
        // A body's own recognition, of its typing env, which a call's generic
        // arguments there, a const parameter's, are of.
        let in_body = |thir: &Thir<'tcx>| {
            let body = bodies
                .iter()
                .find(|(_, of)| std::ptr::eq(*of, thir))
                .map(|&(body, _)| body);
            Recognition {
                typing_env: body.map_or(recognition.typing_env, |body| {
                    rustc_middle::ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(body))
                }),
                ..*recognition
            }
        };
        let thir_of = |id: DefId| bodies.iter().find(|(body, _)| *body == id).map(|(_, thir)| *thir);
        let (assigned, lent) = writes(bodies);
        let calls = Calls::of(tcx, bodies);
        // Each of the crate's thread-locals, by its initializer's root: one
        // whose value is at hand is one no code replaces, as what's at hand
        // is never a `Cell`'s nor a `RefCell`'s.
        let initialized_by: &IndexMap<LocalDefId, LocalDefId> = inits;
        let inits: Vec<(DefId, &Thir<'tcx>, ExprId)> = (initialized_by.iter())
            .filter_map(|(&init, &key)| {
                let body = whole.iter().find(|body| body.def_id == init.to_def_id())?;
                Some((key.to_def_id(), &body.thir, body.root()?))
            })
            .collect();
        // Every thread-local's key has an initializer; one whose body isn't
        // among the bodies is a `const { .. }`'s, which makes no JS object, a
        // binding being no `const fn`.
        let initialized: IndexSet<DefId> = initialized_by.values().map(|key| key.to_def_id()).collect();
        let slot_writes = slots::of(&in_body, bodies, (closed, &initialized), &inits);
        let local_cells = local_cells::of(&in_body, bodies);
        let roots: Vec<(DefId, &Thir<'tcx>, ExprId)> = (whole.iter())
            .filter(|body| tcx.is_closure_like(body.def_id))
            .filter_map(|body| Some((body.def_id, &body.thir, body.root()?)))
            .collect();
        // Each function only calls reach, by its body: what it gives is
        // what its callers get, `only_called`'s, the one owner of whose
        // callers are known (ADR 0577).
        let only_called = (crate::lower::session::caches(tcx).only_called.get().cloned()).unwrap_or_default();
        let functions: Vec<(DefId, &Thir<'tcx>, ExprId)> = (whole.iter())
            .filter(|body| only_called.contains_key(&body.def_id))
            .filter_map(|body| Some((body.def_id, &body.thir, body.root()?)))
            .collect();
        let bind = |pattern: &Pat<'tcx>, found: &mut Vec<LocalVarId>| {
            pattern.walk_always(|p| {
                if let PatKind::Binding { var, .. } = p.kind {
                    found.push(var);
                }
            });
        };
        loop {
            let mut found = Vec::new();
            for &(_, thir) in bodies {
                for stmt in thir.stmts.iter() {
                    if let StmtKind::Let {
                        pattern,
                        initializer: Some(value),
                        ..
                    } = &stmt.kind
                        && at_hand.holds(&in_body(thir), thir, *value)
                    {
                        bind(pattern, &mut found);
                    }
                }
                for e in thir.exprs.iter() {
                    match &e.kind {
                        ExprKind::Let { expr, pat } if at_hand.holds(&in_body(thir), thir, *expr) => {
                            bind(pat, &mut found)
                        }
                        ExprKind::Match { scrutinee, arms, .. } if at_hand.holds(&in_body(thir), thir, *scrutinee) => {
                            for &arm in arms.iter() {
                                bind(&thir[arm].pattern, &mut found);
                            }
                        }
                        ExprKind::Call { fun, args, .. } => {
                            if let Some(pattern) =
                                at_hand.payload_parameter(&in_body(thir), thir, (*fun, args), &thir_of)
                            {
                                bind(pattern, &mut found);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // A closure whose value is at hand; a slot every write of which is.
            let grown = at_hand.closures.len() + at_hand.slots.len() + at_hand.cells.len() + at_hand.returns.len();
            for &(function, thir, root) in &functions {
                let returns = |e: &rustc_middle::thir::Expr<'tcx>| match e.kind {
                    ExprKind::Return { value: Some(value) } => Some(value),
                    _ => None,
                };
                if !at_hand.returns.contains(&function)
                    && at_hand.holds(&in_body(thir), thir, root)
                    && (thir.exprs.iter())
                        .filter_map(returns)
                        .all(|value| at_hand.holds(&in_body(thir), thir, value))
                {
                    at_hand.returns.insert(function);
                }
            }
            for &(closure, thir, root) in &roots {
                if !at_hand.closures.contains(&closure) && at_hand.holds(&in_body(thir), thir, root) {
                    at_hand.closures.insert(closure);
                }
            }
            for (&key, written) in &slot_writes {
                let Some((writes, params)) = written else { continue };
                if !at_hand.slots.contains(&key) && writes.iter().all(|write| at_hand.written(&in_body, write)) {
                    at_hand.slots.insert(key);
                    at_hand.slot_params.extend(params.iter().copied());
                }
            }
            for group in &local_cells {
                let Some(writes) = &group.writes else { continue };
                if !group.members.iter().all(|member| at_hand.cells.contains(member))
                    && writes.iter().all(|write| at_hand.written(&in_body, write))
                {
                    at_hand.cells.extend(group.members.iter().copied());
                }
            }
            let grown =
                at_hand.closures.len() + at_hand.slots.len() + at_hand.cells.len() + at_hand.returns.len() != grown;
            // A thread-local whose initializer makes what's at hand.
            for &(key, thir, root) in &inits {
                if !at_hand.keys.contains(&key) && at_hand.holds(&in_body(thir), thir, root) {
                    at_hand.keys.insert(key);
                }
            }
            // What `KEY.with(|value| ..)` gives, of a key at hand; and a
            // parameter of a function every call of which gives one.
            for &(_, thir) in bodies {
                for e in thir.exprs.iter() {
                    if let ExprKind::Call { fun, args, .. } = &e.kind
                        && let Some(pattern) = at_hand.given_by_key(&in_body(thir), thir, (*fun, args), &thir_of)
                    {
                        bind(pattern, &mut found);
                    }
                }
            }
            for (&callee, sites) in &calls.sites {
                let Some(callee_thir) = thir_of(callee) else { continue };
                for (at, param) in callee_thir.params.iter().enumerate() {
                    let Some(pattern) = param.pat.as_deref() else { continue };
                    if (sites.iter()).all(|&(thir, args)| {
                        args.get(at)
                            .is_some_and(|&arg| at_hand.holds(&in_body(thir), thir, arg))
                    }) {
                        bind(pattern, &mut found);
                    }
                }
            }
            // Each condition holds of more as more is at hand, so this least
            // fixed point is sound: a variable a cycle of writes alone gives
            // is never at hand.
            found.retain(|var| {
                !lent.contains(var)
                    && (assigned.get(var).into_iter().flatten())
                        .all(|&(thir, value)| at_hand.holds(&in_body(thir), thir, value))
            });
            let before = at_hand.vars.len();
            at_hand.vars.extend(found);
            if at_hand.vars.len() == before && !grown {
                return at_hand;
            }
        }
    }

    /// Of a call, in `thir`, of an `Option`'s or a `Result`'s combinator
    /// whose subject is at hand: its closure's parameter, given the payload.
    fn payload_parameter<'a, 'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        thir: &Thir<'tcx>,
        (fun, args): (ExprId, &[ExprId]),
        thir_of: &dyn Fn(DefId) -> Option<&'a Thir<'tcx>>,
    ) -> Option<&'a rustc_middle::thir::Pat<'tcx>> {
        let (callee, _) = crate::lower::fn_def(thir[fun].ty)?;
        let [subject, .., last] = args else {
            return None;
        };
        let of = thir[*subject].ty.peel_refs();
        let combinator = PAYLOAD_COMBINATORS.contains(&recognition.tcx.item_name(callee).as_str())
            && (recognition.is_lang_adt(of, LangItem::Option) || recognition.is_std_type(of, StdItem::Result));
        if !combinator || !self.holds(recognition, thir, *subject) {
            return None;
        }
        let ExprKind::Closure(closure) = &thir[crate::lower::body_queries::strip(thir, *last)].kind else {
            return None;
        };
        let closure_thir = thir_of(closure.closure_id.to_def_id())?;
        (closure_thir.params.iter())
            .find(|param| param.self_kind.is_none() && param.pat.is_some())
            .and_then(|param| param.pat.as_deref())
    }

    /// Of a call, in `thir`, of a thread-local's `with` whose key is at
    /// hand, the crate's (`keys`) or a library's (`ItemFact::AtHand`): its
    /// closure's parameter, given the value.
    fn given_by_key<'a, 'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        thir: &Thir<'tcx>,
        (fun, args): (ExprId, &[ExprId]),
        thir_of: &dyn Fn(DefId) -> Option<&'a Thir<'tcx>>,
    ) -> Option<&'a Pat<'tcx>> {
        let (callee, generic_args) = crate::lower::fn_def(thir[fun].ty)?;
        let [key, closure] = args else {
            return None;
        };
        if recognition.classify(callee, generic_args) != Some(Std::LocalWith { tried: false })
            || !recognition.is_std_type(thir[*key].ty.peel_refs(), StdItem::LocalKey)
        {
            return None;
        }
        let ExprKind::Borrow { arg, .. } = thir[crate::lower::body_queries::strip(thir, *key)].kind else {
            return None;
        };
        let ExprKind::NamedConst { def_id: key, .. } = thir[crate::lower::body_queries::strip(thir, arg)].kind else {
            return None;
        };
        let at_hand = self.keys.contains(&key)
            || (!key.is_local()
                && (recognition.foreign.item(key))
                    .is_some_and(|item| item.calling.has(crate::library::ItemFact::AtHand)));
        if !at_hand {
            return None;
        }
        let ExprKind::Closure(closure) = &thir[crate::lower::body_queries::strip(thir, *closure)].kind else {
            return None;
        };
        let closure_thir = thir_of(closure.closure_id.to_def_id())?;
        (closure_thir.params.iter())
            .find(|param| param.self_kind.is_none() && param.pat.is_some())
            .and_then(|param| param.pat.as_deref())
    }

    /// The crate's thread-locals whose value is at hand, which its contract
    /// says of them (`ItemFact::AtHand`).
    pub(in crate::lower) fn keys(&self) -> impl Iterator<Item = DefId> + '_ {
        self.keys.iter().copied()
    }

    /// Whether a slot's write is of what's at hand.
    fn written<'r, 'tcx>(
        &self,
        in_body: &dyn Fn(&Thir<'tcx>) -> Recognition<'r, 'tcx>,
        write: &slots::Write<'_, 'tcx>,
    ) -> bool {
        match *write {
            slots::Write::Value(thir, value) => self.holds(&in_body(thir), thir, value),
            slots::Write::Closure(closure) => self.closures.contains(&closure),
            slots::Write::Made => true,
        }
    }

    /// Whether `made`, in `thir`, the maker `get_or_insert_with` is given,
    /// makes what's at hand: a closure whose value is, or a builtin's
    /// constructor, or one that creates its result.
    fn makes_at_hand<'tcx>(&self, recognition: &Recognition<'_, 'tcx>, thir: &Thir<'tcx>, made: ExprId) -> bool {
        let tcx = recognition.tcx;
        match &thir[crate::lower::body_queries::strip(thir, made)].kind {
            ExprKind::Closure(c) => self.closures.contains(&c.closure_id.to_def_id()),
            _ => crate::lower::fn_def(thir[made].ty).is_some_and(|(maker, _)| {
                is_binding(tcx, maker) && (matches!(js_form(tcx, maker), JsForm::New(_)) || creates(tcx, maker))
            }),
        }
    }

    /// Whether `e`, in `thir`, is a value at hand.
    pub(in crate::lower) fn holds<'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        thir: &Thir<'tcx>,
        e: ExprId,
    ) -> bool {
        let tcx = recognition.tcx;
        let e = crate::lower::body_queries::strip(thir, e);
        match &thir[e].kind {
            ExprKind::Deref { arg } | ExprKind::Borrow { arg, .. } => self.holds(recognition, thir, *arg),
            ExprKind::VarRef { id } => self.vars.contains(id) || self.slot_params.contains(id),
            // `None`, which holds no object.
            ExprKind::Adt(adt) if recognition.is_lang_adt(thir[e].ty, LangItem::Option) && adt.fields.is_empty() => {
                true
            }
            // A value of each way it may be made.
            ExprKind::Match { arms, .. } => {
                !arms.is_empty() && arms.iter().all(|&arm| self.holds(recognition, thir, thir[arm].body))
            }
            ExprKind::If {
                then,
                else_opt: Some(otherwise),
                ..
            } => self.holds(recognition, thir, *then) && self.holds(recognition, thir, *otherwise),
            // `Some(x)` of what's at hand, as a cast is given it.
            ExprKind::Adt(adt) if recognition.is_lang_adt(thir[e].ty, LangItem::Option) && adt.fields.len() == 1 => {
                self.holds(recognition, thir, adt.fields[0].expr)
            }
            // A block's value, a thread-local initializer's.
            ExprKind::Block { block } => thir[*block]
                .expr
                .is_some_and(|value| self.holds(recognition, thir, value)),
            ExprKind::UpvarRef { var_hir_id, .. } => self.vars.contains(var_hir_id),
            ExprKind::Call { fun, args, .. } => match crate::lower::fn_def(thir[*fun].ty) {
                // A cast of what's at hand is the value passed on,
                // `js::cast(Some(js::unknown(tag)))` (#2111).
                Some((callee, _)) if is_binding(tcx, callee) && matches!(js_form(tcx, callee), JsForm::This) => {
                    args.first().is_some_and(|&of| self.holds(recognition, thir, of))
                }
                // `value.method?.()` of a method that creates its result, of a
                // receiver at hand: what it makes (#2116).
                Some((callee, generic_args))
                    if is_binding(tcx, callee)
                        && matches!(js_form(tcx, callee), JsForm::OptionalCall { unchecked: false }) =>
                {
                    let method = generic_args.types().next().and_then(|path| match path.kind() {
                        rustc_middle::ty::Tuple(path) => path.iter().find_map(crate::lower::fn_def),
                        _ => None,
                    });
                    let receiver = args.first().and_then(|&path| match &thir[crate::lower::body_queries::strip(thir, path)].kind {
                        ExprKind::Tuple { fields } => fields.first().copied(),
                        _ => None,
                    });
                    method.is_some_and(|(method, _)| is_binding(tcx, method) && creates(tcx, method))
                        && receiver.is_some_and(|receiver| self.holds(recognition, thir, receiver))
                }
                Some((callee, _)) if is_binding(tcx, callee) => {
                    matches!(js_form(tcx, callee), JsForm::New(_) | JsForm::Null) || creates(tcx, callee)
                }
                // What a slot holds, read back (#2114): its `get`, `take`'s
                // or `replace`'s old value, and what a `with_borrow`'s or a
                // `with_borrow_mut`'s closure gives of it.
                Some((callee, generic_args)) if slots::closed_key(tcx, thir, (callee, args), &self.slots).is_some() => {
                    match recognition.classify(callee, generic_args) {
                        Some(Std::CellGet | Std::CellTake | Std::CellReplace) => true,
                        Some(Std::LocalBorrow { .. }) => args.get(1).is_some_and(|&closure| {
                            matches!(&thir[crate::lower::body_queries::strip(thir, closure)].kind,
                                ExprKind::Closure(c) if self.closures.contains(&c.closure_id.to_def_id()))
                        }),
                        _ => false,
                    }
                }
                // An `Option`'s value made where it's none, of what's at hand.
                Some((callee, generic_args))
                    if let Some(Std::OptionPlace(
                        op @ (OptionPlaceOp::GetOrInsertWith | OptionPlaceOp::GetOrInsert | OptionPlaceOp::Insert),
                    )) = recognition.classify(callee, generic_args) =>
                {
                    let [of, given] = &args[..] else { return false };
                    self.holds(recognition, thir, *of)
                        && match op {
                            OptionPlaceOp::GetOrInsertWith => self.makes_at_hand(recognition, thir, *given),
                            _ => self.holds(recognition, thir, *given),
                        }
                }
                // What an `Rc` or a guard points to is what it's of.
                Some((callee, generic_args))
                    if crate::lower::recognition::is_rc_deref(tcx, callee, generic_args)
                        || recognition.is_guard_deref(callee, generic_args) =>
                {
                    args.first().is_some_and(|&of| self.holds(recognition, thir, of))
                }
                // What a local cell its closures share holds, read back
                // (#2114): its `borrow()`, `replace`'s or `take`'s old value.
                Some((callee, generic_args))
                    if matches!(
                        recognition.classify(callee, generic_args),
                        Some(Std::Borrow { mutable: false, .. } | Std::CellReplace | Std::CellTake)
                    ) && args
                        .first()
                        .and_then(|&receiver| local_cells::receiver_member(recognition, thir, receiver))
                        .is_some_and(|at| matches!(thir[at].kind,
                            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } if self.cells.contains(&id))) =>
                {
                    true
                }
                // A call of a function every value of which is at hand (#2116).
                Some((callee, _)) if self.returns.contains(&callee) => true,
                // What `catch_unwind` gives where its closure returns: its value.
                Some((callee, _))
                    if recognition.is_std(callee) && matches!(tcx.item_name(callee).as_str(), "catch_unwind") =>
                {
                    args.first().is_some_and(|&made| self.makes_at_hand(recognition, thir, made))
                }
                // An `Option`'s or a `Result`'s value, or else its fallback,
                // each at hand: `unwrap_or(v)`, `or_else(|| ..)`.
                Some((callee, _))
                    if args.first().is_some_and(|&of| {
                        let of = thir[of].ty.peel_refs();
                        recognition.is_lang_adt(of, LangItem::Option) || recognition.is_std_type(of, StdItem::Result)
                    }) && matches!(tcx.item_name(callee).as_str(), "or" | "unwrap_or" | "or_else" | "unwrap_or_else") =>
                {
                    let [of, fallback] = &args[..] else { return false };
                    self.holds(recognition, thir, *of)
                        && match matches!(tcx.item_name(callee).as_str(), "or" | "unwrap_or") {
                            true => self.holds(recognition, thir, *fallback),
                            false => self.makes_at_hand(recognition, thir, *fallback),
                        }
                }
                Some((callee, _)) => {
                    matches!(
                        tcx.item_name(callee).as_str(),
                        "unwrap" | "expect" | "unwrap_unchecked" | "clone" | "branch"
                    ) && args.first().is_some_and(|&of| self.holds(recognition, thir, of))
                }
                None => false,
            },
            _ => false,
        }
    }
}

/// Of each variable, each value assigned to it, in its body.
type Assigned<'a, 'tcx> = IndexMap<LocalVarId, Vec<(&'a Thir<'tcx>, ExprId)>>;

/// Of each variable of `bodies`, the values assigned to it, `x = value`'s,
/// a closure's to its capture of it too; and the variables lent `&mut`,
/// `&raw mut` or bound `ref mut` in a pattern of them, whose writes through
/// the loan aren't seen.
fn writes<'a, 'tcx>(bodies: &[(DefId, &'a Thir<'tcx>)]) -> (Assigned<'a, 'tcx>, IndexSet<LocalVarId>) {
    let mut assigned = Assigned::default();
    let mut lent = IndexSet::default();
    for &(_, thir) in bodies {
        let lends_mut = |pattern: &Pat<'tcx>| {
            let mut lends = false;
            pattern.walk_always(|p| {
                lends |= matches!(
                    p.kind,
                    PatKind::Binding {
                        mode: rustc_hir::BindingMode(rustc_hir::ByRef::Yes(_, Mutability::Mut), _),
                        ..
                    }
                );
            });
            lends
        };
        for stmt in thir.stmts.iter() {
            if let StmtKind::Let {
                pattern,
                initializer: Some(value),
                ..
            } = &stmt.kind
                && lends_mut(pattern)
            {
                lent.extend(root(thir, *value));
            }
        }
        for e in thir.exprs.iter() {
            match &e.kind {
                ExprKind::Assign { lhs, rhs } => match root_written(thir, *lhs) {
                    Some((var, true)) => assigned.entry(var).or_default().push((thir, *rhs)),
                    Some((var, false)) => {
                        lent.insert(var);
                    }
                    None => {}
                },
                ExprKind::AssignOp { lhs: arg, .. }
                | ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                }
                | ExprKind::RawBorrow {
                    mutability: Mutability::Mut,
                    arg,
                } => lent.extend(root(thir, *arg)),
                ExprKind::Let { expr, pat } if lends_mut(pat) => lent.extend(root(thir, *expr)),
                ExprKind::Match { scrutinee, arms, .. } if arms.iter().any(|&arm| lends_mut(&thir[arm].pattern)) => {
                    lent.extend(root(thir, *scrutinee));
                }
                _ => {}
            }
        }
    }
    (assigned, lent)
}

/// The variable the place `e`, in `thir`, is part of, through its fields,
/// elements and derefs: `x`, of `x.0[i]` or `*x`.
fn root(thir: &Thir<'_>, e: ExprId) -> Option<LocalVarId> {
    root_written(thir, e).map(|(var, _)| var)
}

/// `root`'s, and whether the place is the variable itself, `x`, not a part
/// of it or what it points to.
fn root_written(thir: &Thir<'_>, e: ExprId) -> Option<(LocalVarId, bool)> {
    let e = crate::lower::body_queries::strip(thir, e);
    match &thir[e].kind {
        ExprKind::VarRef { id } => Some((*id, true)),
        ExprKind::UpvarRef { var_hir_id, .. } => Some((*var_hir_id, true)),
        ExprKind::Field { lhs: of, .. } | ExprKind::Index { lhs: of, .. } | ExprKind::Deref { arg: of } => {
            root(thir, *of).map(|var| (var, false))
        }
        _ => None,
    }
}

/// The crate's calls of its functions whose every call it sees (#2111):
/// each call's arguments, by the function called.
struct Calls<'a, 'tcx> {
    sites: IndexMap<DefId, Vec<(&'a Thir<'tcx>, &'a [ExprId])>>,
}

impl<'a, 'tcx> Calls<'a, 'tcx> {
    /// Of `bodies`: each call of a function only calls reach, `only_called`'s
    /// (ADR 0577), the one owner of whose callers are all known: no JS
    /// caller nor another crate sees it, nor is it named as a value or a
    /// trait's method. What each call here gives it is all it's given.
    fn of(tcx: rustc_middle::ty::TyCtxt<'tcx>, bodies: &[(DefId, &'a Thir<'tcx>)]) -> Self {
        let only_called = (crate::lower::session::caches(tcx).only_called.get().cloned()).unwrap_or_default();
        let mut sites: IndexMap<DefId, Vec<(&'a Thir<'tcx>, &'a [ExprId])>> = IndexMap::default();
        for &(_, thir) in bodies {
            for e in thir.exprs.iter() {
                if let ExprKind::Call { fun, args, .. } = &e.kind
                    && let Some((callee, _)) = crate::lower::fn_def(thir[*fun].ty)
                    && only_called.contains_key(&callee)
                {
                    sites.entry(callee).or_default().push((thir, &args[..]));
                }
            }
        }
        Calls { sites }
    }
}
