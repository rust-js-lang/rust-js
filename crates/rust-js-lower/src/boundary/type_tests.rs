//! A parameter a body asks a `js` type test of first (#2403, ADR 1192,
//! amended, the lead's choice): `js::is_function(&callback)` before
//! anything else reads `callback`, on every path. Its body takes any value
//! JS gives, React's SchedulerMock's `null`, `{}` or `42` of a callback,
//! and asks it what it is itself, so the check of its type is skipped: the
//! test is its check. Found of MIR, by `js`'s catalog of its type tests
//! (`bindings::is_type_test`), never of the JS.

use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{self, Local, Location, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_span::def_id::DefId;

use crate::FnCx;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The Rust parameters of `body` a type test asks of before any other
    /// use of each, on every path: each by its index.
    pub fn type_tested(&self, body: &mir::Body<'tcx>) -> Vec<usize> {
        (body.args_iter())
            .filter(|&arg| self.tested_first(body, arg))
            .map(|arg| arg.as_usize() - 1)
            .collect()
    }

    /// Whether a type test of `arg`, given it or what one statement makes of
    /// it, `&arg` or its copy, comes before each of its other uses.
    fn tested_first(&self, body: &mir::Body<'tcx>, arg: Local) -> bool {
        // What's made of it once, `_5 = &_1`, where.
        let made: Vec<(Local, Location)> = (body.basic_blocks.iter_enumerated())
            .flat_map(|(block, data)| {
                (data.statements.iter().enumerate()).filter_map(move |(statement_index, statement)| {
                    let StatementKind::Assign(assign) = &statement.kind else {
                        return None;
                    };
                    let (place, rvalue) = &**assign;
                    let of_arg = match rvalue {
                        Rvalue::Ref(_, _, from) | Rvalue::RawPtr(_, from) => from.as_local() == Some(arg),
                        Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _) => from.as_local() == Some(arg),
                        _ => false,
                    };
                    (of_arg && place.as_local().is_some()).then_some((place.local, Location { block, statement_index }))
                })
            })
            .collect();
        let tests: Vec<Location> = (body.basic_blocks.iter_enumerated())
            .filter_map(|(block, data)| {
                let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                    return None;
                };
                let (callee, _) = func.const_fn_def()?;
                if !crate::bindings::is_type_test(self.tcx, callee) {
                    return None;
                }
                let given = |local: Local| local == arg || made.iter().any(|&(of, _)| of == local);
                (args.iter())
                    .any(|operand| {
                        operand
                            .node
                            .place()
                            .is_some_and(|place| place.as_local().is_some_and(given))
                    })
                    .then(|| body.terminator_loc(block))
            })
            .collect();
        if tests.is_empty() {
            return false;
        }
        let mut uses = Uses { arg, at: Vec::new() };
        uses.visit_body(body);
        let dominators = body.basic_blocks.dominators();
        tests.iter().any(|&test| {
            (uses.at.iter()).all(|&used| {
                used == test
                    || made.iter().any(|&(_, at)| at == used && at.dominates(test, dominators))
                    || test.dominates(used, dominators)
            })
        })
    }

    /// Whether the call of the crate's binding `binding` that ends `block`
    /// gives what its body asked a `js` type test of first, the same rule
    /// as a parameter's (the lead's choice): the binding asked before the
    /// call, on every path, `if js::is_number(width) { width() }`. So its
    /// result isn't checked: the test is its check.
    pub fn binding_result_tested(&self, body: &mir::Body<'tcx>, block: mir::BasicBlock, binding: DefId) -> bool {
        let call = body.terminator_loc(block);
        let dominators = body.basic_blocks.dominators();
        (body.basic_blocks.iter_enumerated()).any(|(at, data)| {
            let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                return false;
            };
            let Some((callee, _)) = func.const_fn_def() else {
                return false;
            };
            crate::bindings::is_type_test(self.tcx, callee)
                && args.iter().any(|arg| {
                    arg.node
                        .constant()
                        .is_some_and(|c| matches!(c.ty().kind(), rustc_middle::ty::FnDef(id, _) if *id == binding))
                })
                && body.terminator_loc(at).dominates(call, dominators)
        })
    }
}

/// Where `arg` is used, but where its storage alone is.
struct Uses {
    arg: Local,
    at: Vec<Location>,
}

impl<'tcx> Visitor<'tcx> for Uses {
    fn visit_local(&mut self, local: Local, context: PlaceContext, location: Location) {
        if local == self.arg && !matches!(context, PlaceContext::NonUse(_)) {
            self.at.push(location);
        }
    }
}
