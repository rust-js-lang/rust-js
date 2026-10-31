//! What tidy is told of a body (`tidy::Hints`) that its THIR says: where
//! the program writes each `if`'s condition, each `&&` and `||`, each `if`
//! of a `continue` or a loop's `break`, each loop and whether as a `while`,
//! and each `match` that may read a table. A question module: it emits
//! nothing.

use super::*;
use crate::lower::body_queries::{self, PatternValue};

mod negations;
mod present_breaks;

/// Where the program writes what tidy reads a body's JS by.
pub(super) struct Written {
    /// Each `match` whose arms are each a variant of its own, with no field,
    /// guard or binding, and its variants' names (ADR 0233).
    pub tables: Vec<(js::Span, Vec<String>)>,
    /// Each `if`'s condition, and a `match` and the guard of an arm of it.
    pub conditions: Vec<(js::Span, js::Span)>,
    /// Each `if`'s condition alone, an `if let`'s too: a test the program
    /// wrote, which no earlier test decides away (ADR 0564, amended).
    pub ifs: Vec<js::Span>,
    /// Each `if` written without an `else`: its condition and its `then`,
    /// which no pass makes a guard of (`written_then`).
    pub thens: Vec<(js::Span, js::Span)>,
    /// Each `if` written with an `else`: its condition, its `then` and its
    /// `else`, which MIR's structure may make a guard of (`written_elses`).
    pub elses: Vec<(js::Span, js::Span, js::Span)>,
    /// Of them, each a statement, of no value, `if c { x = a; } else { x =
    /// b; }`, its condition: an `if` the program wrote of a late `let`,
    /// which stays one, not a value's conditional (rd187).
    pub statement_elses: Vec<js::Span>,
    /// Where each `else if` is written, an `else` that's an `if`: a link
    /// of a chain, which an `if let`'s binding, its JS `const`, keeps from
    /// being an `else if` (`written_elses`).
    pub else_ifs: Vec<js::Span>,
    /// Each `else if`'s own condition: a link of a chain after its first,
    /// whose value is the chain's, never two arms of its own folded
    /// (`returned_values`, ADR 0969, amended).
    pub links: Vec<js::Span>,
    /// Where each `let … else`'s `else` is written, and each `?`: a guard's
    /// exit, which stays where it's written, not a tail it's alike
    /// (`shared_tails`), and comes first (`let_else_first`).
    pub let_elses: Vec<js::Span>,
    /// Where each `let Some(x) = v.get(i) else { break; }` is written, of
    /// items that take up room: where it breaks, `i` is the slice's length
    /// or past it (`body_queries::present_item_of_sized`), a `for`'s test
    /// of it (rd260).
    pub present_breaks: Vec<js::Span>,
    /// Each `&&` or `||` the program wrote.
    pub logicals: Vec<js::Span>,
    /// Each `js::strict_equals(a, b)` the program wrote, a chain of which
    /// of one subject is a `switch` (ADR 0958).
    pub strict_tests: Vec<js::Span>,
    /// Each `!( .. )` the program wrote of an `&&` or `||`: where what it
    /// negates is, and whether that's an `&&` (`written_negations`).
    pub negations: Vec<(js::Span, bool)>,
    /// Each `if` of a `continue`, alone or ending what it does: where its
    /// condition is, and where the `continue` is, which its comment is
    /// before (ADR 0684).
    pub continues: Vec<(js::Span, js::Span)>,
    /// Each `if` of a loop's `break`, as of a `continue`: where its
    /// condition is, and where the `break` is (ADR 0684, amended).
    pub breaks: Vec<(js::Span, js::Span)>,
    /// Each `if … else` whose `else` ends in a `continue`, as `continues`
    /// of a `then`, `} else { ..; continue; }`: kept where it's written
    /// (`kept_jumps`, ADR 0684, amended).
    pub else_continues: Vec<(js::Span, js::Span)>,
    /// And each whose `else` ends in a loop's `break`, `} else { break; }`.
    pub else_breaks: Vec<(js::Span, js::Span)>,
    /// Each loop, whether as a `while`, and its label.
    pub loops: Vec<(js::Span, bool, Option<String>)>,
    /// Each block the program labeled, `'desc: { .. }` (`labeled_blocks`).
    pub blocks: Vec<LabeledBlock>,
    /// Each `match` the program wrote whose every arm tests constants, a
    /// `switch` (ADR 0629): its arms.
    pub switches: Vec<Switch>,
    /// Where each arm's pattern of each `match` the program wrote is,
    /// `matches!`'s too: a test of what's matched against its constant, not
    /// of its truth (ADR 0896, amended).
    pub patterns: Vec<js::Span>,
    /// Each arm's guard of each `match` the program wrote, and where that
    /// `match` tests: what rustc keeps from changing while it runs (#221).
    pub guards: Vec<Guard>,
}

/// A `match` arm's guard (`Written::guards`): where it's written, and where
/// its `match`'s scrutinee and each arm's pattern are, what MIR's tests of
/// that `match` read. rustc keeps a guard from changing its scrutinee, as
/// it borrows each place the `match` tests while the guard runs (E0510):
/// a test of a pattern after it reads what it would have before.
pub(super) struct Guard {
    pub span: js::Span,
    pub tested: Vec<js::Span>,
}

/// A block the program labeled, `'desc: { .. }`: where it's written, its
/// label, and where each `break` the program wrote of it is, `break 'desc
/// v`, of which the JS block MIR's breaks leave is its (ADR 0742).
#[derive(Clone)]
pub(super) struct LabeledBlock {
    pub span: js::Span,
    pub label: String,
    pub breaks: Vec<js::Span>,
}

/// The jumps the program wrote in a body: where each `return` is, and each
/// block it labeled (ADR 0742).
pub(super) struct Jumps {
    pub returns: Vec<js::Span>,
    pub blocks: Vec<LabeledBlock>,
}

/// The fewest cases a `match` of constants is a `switch` of, each constant
/// of a pattern one and a last `_` one (ADR 0629): of two, `if (n === 0) {
/// .. } else { .. }`, as JS writes two ways, and as SonarJS's
/// `no-small-switch` asks; of one arm of seven constants, a switch.
const SWITCH_CASES: usize = 3;

/// What a variant's test of a value tests, as a constant's names it: a
/// name no JS the program writes has.
pub(super) const SUBJECT: &str = "$subject";

/// A `match` of constants (`Written::switches`): where it's written, its
/// `switch`'s, and its arms (ADR 0629).
pub(super) struct Switch {
    pub span: js::Span,
    pub arms: Vec<SwitchArm>,
    /// Whether each constant its patterns name is a number, a string or
    /// the registry's symbol (`PatternValue`), but a last `_`'s: what a
    /// `switch` of an `Option`'s value takes of no case, `null` among them
    /// (#221).
    pub valued: bool,
}

/// An arm of a `match` of constants (`Written::switches`): where it, its
/// pattern and its guard are written, and each constant its pattern names,
/// `1 | 2`'s two, none of a last `_`, the `switch`'s `default`; of an arm
/// the one before falls to, those the one before doesn't name. `falls`,
/// whether it's a guarded arm whose failed guard goes on to the next arm,
/// which names each constant it does: its case falls through to the next's
/// (ADR 0905).
pub(super) struct SwitchArm {
    pub span: js::Span,
    pub pattern: js::Span,
    pub guard: Option<js::Span>,
    pub constants: Vec<Constant>,
    pub falls: bool,
}

impl SwitchArm {
    /// How many cases it is: a `_`'s `default`, else a case of each
    /// constant but an array's, which `typeof` says is an object: `Object |
    /// Array`'s `"object"` is one, and an array's alone none, an `if`
    /// before the `switch` (ADR 0668); nor is `null`'s, an `if` in `case
    /// "object"` (ADR 0676).
    fn cases(&self) -> usize {
        use crate::lower::untagged::TypeofCase;
        match self.constants.is_empty() {
            true => 1,
            false => (self.constants.iter())
                .filter(|c| !matches!(c, Constant::Kind(TypeofCase::Tested(_) | TypeofCase::Inside(..))))
                .count(),
        }
    }
}

/// A constant an arm of a `match` of constants names: a variant, by the
/// value JS has of it, `"Function"`, which a last arm, untested, is a case
/// of; or a literal or a `const`, `n === 1`, where it's written, which its
/// `case` is, its comment after it (ADR 0630).
/// Or a variant of an untagged enum, by what `typeof` says of it, its
/// `case` of a `switch (typeof value)`, the value being `SUBJECT` (ADR
/// 0668).
#[derive(Clone)]
pub(super) enum Constant {
    Variant(js::Expr),
    Value(js::Span),
    Kind(crate::lower::untagged::TypeofCase),
    /// What a guard reads and compares with what's matched, `END.with(|e|
    /// e == name)`'s: a name's, `case END:`, its test says (rd113).
    Read,
}

/// A constant a pattern names (`Constant`), and the value it is, where
/// that's told (`body_queries::pattern_value`).
type Valued<'tcx> = (Constant, Option<PatternValue<'tcx>>);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Where the program writes what tidy reads the body's JS by, of the
    /// body THIR has.
    pub(super) fn written(&self) -> Written {
        // Where each `match` is written whose arms are each a variant of its
        // own, with no field, guard or binding: one that may read a table by
        // what's matched (ADR 0233), as tidy's `table_reads` tells.
        let tables: Vec<(js::Span, Vec<String>)> = (self.frame.thir.exprs.iter())
            .filter_map(|e| {
                let thir::ExprKind::Match { scrutinee, ref arms, .. } = e.kind else {
                    return None;
                };
                let ty::Adt(adt, _) = self.frame.thir[scrutinee].ty.peel_refs().kind() else {
                    return None;
                };
                let each = (arms.iter()).all(|&arm| {
                    let arm = &self.frame.thir[arm];
                    arm.guard.is_none()
                        && matches!(&arm.pattern.kind, thir::PatKind::Variant { subpatterns, .. } if subpatterns.is_empty())
                });
                // Each variant's name too, to know the one an `else` is of.
                let names = adt.variants().iter().map(|v| bindings::variant_name(self.tcx, v)).collect();
                (adt.is_enum() && arms.len() >= 2 && each).then(|| (self.js_span(e.span), names))
            })
            .collect();
        // Where each `if`'s condition is written, of the body THIR has:
        // tests in one are `&&`'s; and a `match` and the guard of an arm of
        // it, whose pattern's test and the guard are.
        let conditions: Vec<(js::Span, js::Span)> = (self.frame.thir.exprs.iter())
            .flat_map(|e| match e.kind {
                thir::ExprKind::If { cond, .. } => {
                    let cond = self.js_span(self.frame.thir[cond].span);
                    vec![(cond, cond)]
                }
                // From `match` to its scrutinee's end, where MIR tests it:
                // an arm's pattern, one test of each of its parts, `(Some(x),
                // Some(y))`'s `a != null && b != null` (ADR 0614), and its
                // guard.
                thir::ExprKind::Match {
                    scrutinee, ref arms, ..
                } => {
                    let tested = js::Span {
                        lo: self.js_span(e.span).lo,
                        hi: self.js_span(self.frame.thir[scrutinee].span).hi,
                    };
                    let guards = (arms.iter()).filter_map(|&arm| {
                        let guard = self.frame.thir[arm].guard?;
                        Some((tested, self.js_span(self.frame.thir[guard].span)))
                    });
                    // And an arm's pattern, where MIR tests a range's two
                    // bounds, `90..=100`'s `score >= 90 && score <= 100`.
                    let patterns = (arms.iter()).map(|&arm| {
                        let pattern = self.js_span(self.frame.thir[arm].pattern.span);
                        (pattern, pattern)
                    });
                    std::iter::once((tested, tested))
                        .chain(guards)
                        .chain(patterns)
                        .collect()
                }
                _ => Vec::new(),
            })
            .collect();
        // Where each `if` with an `else` is written, its condition and each
        // branch; a `while`'s too, `if c { body } else { break }` of its
        // desugaring, its `else` the whole loop.
        // Where each arm's guard is: a `match` in one is a test, not branches.
        let guards: Vec<js::Span> = (self.frame.thir.arms.iter())
            .filter_map(|arm| arm.guard.map(|guard| self.js_span(self.frame.thir[guard].span)))
            .collect();
        let elses: Vec<(js::Span, js::Span, js::Span)> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If {
                    cond,
                    then,
                    else_opt: Some(otherwise),
                    ..
                } => Some((
                    self.js_span(self.frame.thir[cond].span),
                    self.js_span(self.frame.thir[then].span),
                    self.js_span(self.frame.thir[otherwise].span),
                )),
                // A `match` of two arms of no guard, its last arm the `else`,
                // as an `if`'s, `if (current === null) { .. } else { .. }`
                // (ADR 0237, amended, the user's decision, 2026-10-28): its
                // test where MIR tests it, from `match` to its scrutinee.
                thir::ExprKind::Match {
                    scrutinee, ref arms, ..
                } if arms.len() == 2
                    && arms.iter().all(|&arm| self.frame.thir[arm].guard.is_none())
                    // Not of an arm that's a jump alone, `None => break`,
                    // a guard's way out, as `let … else` writes it.
                    && !arms.iter().any(|&arm| self.jump_alone(self.frame.thir[arm].body))
                    && !guards.iter().any(|g| {
                        let at = self.js_span(e.span);
                        g.lo <= at.lo && at.hi <= g.hi
                    }) =>
                {
                    // Or at its first arm's pattern, a string's `s === "a"`.
                    let pattern = self.js_span(self.frame.thir[arms[0]].pattern.span);
                    let tested = js::Span {
                        lo: self.js_span(e.span).lo,
                        hi: self.js_span(self.frame.thir[scrutinee].span).hi.max(pattern.hi),
                    };
                    let body = |arm: thir::ArmId| self.js_span(self.frame.thir[self.frame.thir[arm].body].span);
                    Some((tested, body(arms[0]), body(arms[1])))
                }
                _ => None,
            })
            .collect();
        let statement_elses: Vec<js::Span> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If {
                    cond,
                    else_opt: Some(_),
                    ..
                } if e.ty.is_unit() => Some(self.js_span(self.frame.thir[cond].span)),
                _ => None,
            })
            .collect();
        let else_ifs: Vec<js::Span> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If {
                    else_opt: Some(otherwise),
                    ..
                } if matches!(self.frame.thir[self.strip(otherwise)].kind, thir::ExprKind::If { .. }) => {
                    Some(self.js_span(self.frame.thir[otherwise].span))
                }
                _ => None,
            })
            .collect();
        let links: Vec<js::Span> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If {
                    else_opt: Some(otherwise),
                    ..
                } => match self.frame.thir[self.strip(otherwise)].kind {
                    thir::ExprKind::If { cond, .. } => Some(self.js_span(self.frame.thir[cond].span)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        // Where each `&&` or `||` the program wrote is.
        let logicals: Vec<js::Span> = (self.frame.thir.exprs.iter())
            .filter(|e| matches!(e.kind, thir::ExprKind::LogicalOp { .. }))
            .map(|e| self.js_span(e.span))
            .collect();
        // Where each `js::strict_equals(a, b)` is (ADR 0958).
        let strict_tests: Vec<js::Span> = (self.frame.thir.exprs.iter())
            .filter(|e| {
                matches!(e.kind, thir::ExprKind::Call { fun, .. }
                    if fn_def(self.frame.thir[fun].ty).is_some_and(|(f, _)| {
                        crate::lower::bindings::is_binding(self.tcx, f)
                            && matches!(crate::lower::bindings::js_form(self.tcx, f), crate::lower::bindings::JsForm::StrictEquals)
                    }))
            })
            .map(|e| self.js_span(e.span))
            .collect();
        let negations = self.negations();
        let continue_of = |kind: &thir::ExprKind<'tcx>| matches!(kind, thir::ExprKind::Continue { .. });
        let continues = self.jump_ifs(&continue_of, false);
        let else_continues = self.jump_ifs(&continue_of, true);
        // Of a loop's `break`, not a block's the program labeled, whose
        // `break`s are where they're written already (ADR 0790).
        let labeled: std::collections::HashSet<rustc_hir::ItemLocalId> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Scope { hir_id, .. } => match self.tcx.hir_expect_expr(hir_id).kind {
                    rustc_hir::ExprKind::Block(block, Some(_)) => Some(block.hir_id.local_id),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let break_of = |kind: &thir::ExprKind<'tcx>| matches!(kind, thir::ExprKind::Break { label, .. } if !labeled.contains(&label.local_id));
        let breaks = self.jump_ifs(&break_of, false);
        let else_breaks = self.jump_ifs(&break_of, true);
        // Where each loop is written, and whether as a `while`.
        // Each loop's label the program wrote, `'rows`, by the loop.
        let labels: std::collections::HashMap<thir::ExprId, String> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Scope { value, hir_id, .. } => match self.tcx.hir_expect_expr(hir_id).kind {
                    // The loop it scopes, in scopes of its own too.
                    rustc_hir::ExprKind::Loop(_, Some(label), ..) => Some((self.strip(value), written_label(&label))),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let loops: Vec<(js::Span, bool, Option<String>)> = (self.frame.thir.exprs.iter_enumerated())
            .filter_map(|(id, e)| match e.kind {
                thir::ExprKind::Loop { .. } => {
                    let written = e.span.source_callsite();
                    // Where it's written, not the body it's in, which a closure's
                    // span holds too.
                    let whiles = (self.frame.thir.exprs.iter()).any(|w| {
                        w.span.is_desugaring(rustc_span::DesugaringKind::WhileLoop)
                            && w.span.source_callsite().source_equal(written)
                    });
                    Some((self.js_span(written), whiles, labels.get(&id).cloned()))
                }
                _ => None,
            })
            .collect();
        let switches = self.written_switches();
        let patterns = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Match {
                    ref arms,
                    match_source: rustc_hir::MatchSource::Normal,
                    ..
                } => Some(arms),
                _ => None,
            })
            .flatten()
            .map(|&arm| self.js_span(self.frame.thir[arm].pattern.span))
            .collect();
        let match_guards = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Match {
                    scrutinee, ref arms, ..
                } => Some((e.span, scrutinee, arms)),
                _ => None,
            })
            .flat_map(|(span, scrutinee, arms)| {
                let tested: Vec<js::Span> = std::iter::once(js::Span {
                    lo: self.js_span(span).lo,
                    hi: self.js_span(self.frame.thir[scrutinee].span).hi,
                })
                .chain((arms.iter()).map(|&arm| self.js_span(self.frame.thir[arm].pattern.span)))
                .collect();
                (arms.iter())
                    .filter_map(|&arm| self.frame.thir[arm].guard)
                    .map(move |guard| Guard {
                        span: self.js_span(self.frame.thir[guard].span),
                        tested: tested.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        // Each `if`'s condition alone, as written, not a desugaring's.
        let ifs = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If { cond, .. } if !e.span.from_expansion() => {
                    Some(self.js_span(self.frame.thir[cond].span))
                }
                _ => None,
            })
            .collect();
        // Each `if` without an `else`, as written: its condition and its
        // `then`, which stays its body (ADR 0879, amended).
        let thens = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If {
                    cond,
                    then,
                    else_opt: None,
                    ..
                } if !e.span.from_expansion() => Some((
                    self.js_span(self.frame.thir[cond].span),
                    self.js_span(self.frame.thir[then].span),
                )),
                _ => None,
            })
            .collect();
        // And each `?`, whose way out is a guard's too, `if (x == null) {
        // return null; }` before what reads its value (q1).
        let tries = (self.frame.thir.exprs.iter()).filter_map(|e| match e.kind {
            thir::ExprKind::Match {
                match_source: rustc_hir::MatchSource::TryDesugar(_),
                ..
            } => Some(self.js_span(e.span)),
            _ => None,
        });
        // And each arm that never goes on of a `match` a `let` is given, `let
        // pairs = match e { Some(p) if .. => p, _ => { return; } };`, a
        // `let … else` of a guard (ag6).
        let thir = &self.frame.thir;
        let arm_exits = (thir.stmts.iter())
            .filter_map(|stmt| match stmt.kind {
                thir::StmtKind::Let {
                    initializer: Some(init),
                    else_block: None,
                    ..
                } => match &thir[self.strip(init)].kind {
                    thir::ExprKind::Match {
                        arms,
                        match_source: rustc_hir::MatchSource::Normal,
                        ..
                    } => Some(arms.iter().map(|&arm| &thir[thir[arm].body])),
                    _ => None,
                },
                _ => None,
            })
            .flatten()
            .filter(|body| self.diverges(body))
            .map(|body| self.js_span(body.span));
        let let_elses = (self.frame.thir.stmts.iter())
            .filter_map(|stmt| match stmt.kind {
                thir::StmtKind::Let {
                    else_block: Some(block),
                    ..
                } => Some(self.js_span(self.frame.thir[block].span)),
                _ => None,
            })
            .chain(tries)
            .chain(arm_exits)
            .collect();
        let present_breaks = self.present_breaks();
        Written {
            present_breaks,
            tables,
            conditions,
            ifs,
            thens,
            elses,
            statement_elses,
            else_ifs,
            links,
            let_elses,
            logicals,
            strict_tests,
            negations,
            continues,
            breaks,
            else_continues,
            else_breaks,
            loops,
            blocks: self.labeled_blocks(),
            switches,
            patterns,
            guards: match_guards,
        }
    }

    /// The jumps the program wrote in the body (`Jumps`), none left yet.
    pub(super) fn written_jumps(&self) -> Jumps {
        Jumps {
            returns: (self.frame.thir.exprs.iter())
                .filter(|e| matches!(e.kind, thir::ExprKind::Return { .. }))
                .map(|e| self.js_span(e.span))
                .collect(),
            blocks: self.labeled_blocks(),
        }
    }

    /// Each block the program labeled, `'desc: { .. }`, and where each
    /// `break` of it is (ADR 0742).
    fn labeled_blocks(&self) -> Vec<LabeledBlock> {
        let thir = &self.frame.thir;
        // By the block a `break` of it names, the labeled expression's.
        let mut blocks: Vec<(rustc_hir::ItemLocalId, LabeledBlock)> = (thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Scope { hir_id, .. } => match self.tcx.hir_expect_expr(hir_id).kind {
                    rustc_hir::ExprKind::Block(block, Some(label)) => Some((
                        block.hir_id.local_id,
                        LabeledBlock {
                            span: self.js_span(e.span.source_callsite()),
                            label: written_label(&label),
                            breaks: Vec::new(),
                        },
                    )),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        for e in thir.exprs.iter() {
            if let thir::ExprKind::Break { label, .. } = e.kind
                && let Some((_, block)) = blocks.iter_mut().find(|(id, _)| *id == label.local_id)
            {
                block.breaks.push(self.js_span(e.span));
            }
        }
        blocks.into_iter().map(|(_, block)| block).collect()
    }

    /// Whether `e` never goes on: a `return`, a `break` or a `continue`, a
    /// block that ends with one, or a value of `!`, given as another type.
    fn diverges(&self, e: &thir::Expr<'tcx>) -> bool {
        let thir = &self.frame.thir;
        match &e.kind {
            thir::ExprKind::Scope { value, .. } => self.diverges(&thir[*value]),
            thir::ExprKind::Return { .. }
            | thir::ExprKind::Break { .. }
            | thir::ExprKind::Continue { .. }
            | thir::ExprKind::NeverToAny { .. } => true,
            thir::ExprKind::Block { block } => {
                let block = &thir[*block];
                match (block.expr, block.stmts.last()) {
                    (Some(value), _) => self.diverges(&thir[value]),
                    (None, Some(&last)) => {
                        matches!(thir[last].kind, thir::StmtKind::Expr { expr, .. } if self.diverges(&thir[expr]))
                    }
                    (None, None) => false,
                }
            }
            _ => e.ty.is_never(),
        }
    }

    /// Each `match` the program wrote, not a macro, `matches!`, of three
    /// arms or more, whose every arm tests constants, with no binding, but
    /// for a last `_` or name alone, `current => ..`, its `default` (ADR
    /// 0629, amended), and whose value isn't read in place, which JS has no
    /// `switch` expression for (ADR 0629): its arms. An arm with a guard is
    /// one where a failed guard goes on to nothing (`fails_to_nothing`).
    /// The one answer to which `match` is a `switch`.
    fn written_switches(&self) -> Vec<Switch> {
        // Each once, of the scopes THIR has of it.
        let mut seen = std::collections::HashSet::new();
        let held = &self.frame.facts.held_guards;
        (self.frame.thir.exprs.iter())
            .filter_map(|e| {
                let thir::ExprKind::Scope { mut value, hir_id, .. } = e.kind else {
                    return None;
                };
                // Through what Rust's adjustments made of it, `&*match ..`.
                while let thir::ExprKind::Deref { arg: inner }
                | thir::ExprKind::Borrow { arg: inner, .. }
                | thir::ExprKind::NeverToAny { source: inner }
                | thir::ExprKind::PointerCoercion { source: inner, .. }
                | thir::ExprKind::Use { source: inner } = self.frame.thir[value].kind
                {
                    value = inner;
                }
                let e = &self.frame.thir[value];
                let thir::ExprKind::Match {
                    scrutinee,
                    ref arms,
                    match_source,
                    ..
                } = e.kind
                else {
                    return None;
                };
                // What's matched is there, `classify(Some(value))`'s (ADR 0676).
                let present = crate::lower::body_queries::given_present(self.tcx, self.frame.thir, scrutinee).is_some();
                if match_source != rustc_hir::MatchSource::Normal
                    || e.span.from_expansion()
                    || self.read_in_place(hir_id)
                    || !seen.insert(value)
                {
                    return None;
                }
                let last = arms.len().checked_sub(1)?;
                let arms: Vec<&thir::Arm<'tcx>> = arms.iter().map(|&arm| &self.frame.thir[arm]).collect();
                // A guard that holds where the arm's pattern matches tests
                // nothing: the arm is as one without it.
                let guards: Vec<Option<thir::ExprId>> = (arms.iter())
                    .map(|arm| {
                        (arm.guard).filter(|&guard| {
                            !held.contains(
                                &self.frame.thir[crate::lower::body_queries::strip(self.frame.thir, guard)].span,
                            )
                        })
                    })
                    .collect();
                // Whether each guarded arm's failed guard goes on to the next
                // arm, which names each constant it does and more (ADR 0905):
                // its case falls through to the next's, which is a case as
                // any arm is, guarded or not.
                // And a guarded arm just before a last `_` that does
                // something, which takes each value it does: its case falls
                // through to the `default` (ADR 0636, amended).
                let catch_all = |arm: &thir::Arm<'tcx>| {
                    arm.guard.is_none()
                        && matches!(
                            arm.pattern.kind,
                            thir::PatKind::Wild | thir::PatKind::Binding { subpattern: None, .. }
                        )
                };
                let falls: Vec<bool> = (0..arms.len())
                    .map(|i| {
                        guards[i].is_some()
                            && i < last
                            && (self.falls_to(&arms[i].pattern, &arms[i + 1].pattern, present)
                                || (i + 1 == last
                                    && catch_all(arms[last])
                                    && !self.fails_to_nothing(&arms[i].pattern, &arms[i + 1..])))
                    })
                    .collect();
                // A guarded arm that does nothing, `I if legacy_off => {}`,
                // before one of its constants alone and no guard, `I => {
                // .. }`: one case of them, `case I: if (legacyOff) { break; }
                // ..`, as React's completeWork writes it, the guard's `if` in
                // its statements as MIR tests it (ADR 0905, amended).
                let joined: Vec<bool> = (0..arms.len())
                    .map(|i| {
                        guards[i].is_some()
                            && i < last
                            && guards[i + 1].is_none()
                            && self.does_nothing(arms[i].body)
                            && matches!(
                                (
                                    self.constant_values(&arms[i].pattern, present),
                                    self.constant_values(&arms[i + 1].pattern, present),
                                ),
                                (Some(a), Some(b)) if a.len() == b.len() && a.iter().all(|v| b.contains(v))
                            )
                    })
                    .collect();
                let valued = (arms.iter().enumerate()).all(|(i, arm)| match arm.pattern.kind {
                    thir::PatKind::Wild | thir::PatKind::Binding { subpattern: None, .. } => {
                        i == last && arm.guard.is_none()
                    }
                    _ => self.constant_values(&arm.pattern, present).is_some(),
                });
                let arms: Vec<SwitchArm> = (arms.iter().enumerate())
                    .filter(|&(i, _)| !i.checked_sub(1).is_some_and(|before| joined[before]))
                    .map(|(i, arm)| {
                        if joined[i] {
                            return Some(SwitchArm {
                                span: self.js_span(arm.span.to(arms[i + 1].span)),
                                pattern: self.js_span(arm.pattern.span),
                                guard: None,
                                constants: self.constants_of(&arm.pattern, present)?,
                                falls: false,
                            });
                        }
                        let guard = guards[i];
                        // A name compared, in its guard, with a value read
                        // there, `name if END.with(|e| e == name)`: a case
                        // of that value, `case END:`, written where its test
                        // is (rd113, the user's decision).
                        if let thir::PatKind::Binding {
                            subpattern: None, var, ..
                        } = arm.pattern.kind
                            && let Some(guard) = guard
                            && let Some(reads) = self.reads_compared(guard, var)
                        {
                            return Some(SwitchArm {
                                span: self.js_span(arm.span),
                                pattern: self.js_span(self.frame.thir[guard].span),
                                guard: None,
                                constants: vec![Constant::Read; reads],
                                falls: false,
                            });
                        }
                        let constants = match &arm.pattern.kind {
                            thir::PatKind::Wild | thir::PatKind::Binding { subpattern: None, .. }
                                if i == last && guard.is_none() =>
                            {
                                Some(Vec::new())
                            }
                            _ => (self.constants_of(&arm.pattern, present)).filter(|_| {
                                guard.is_none() || falls[i] || self.fails_to_nothing(&arm.pattern, &arms[i + 1..])
                            }),
                        }?;
                        // Of an arm the one before falls to, the constants it
                        // names alone: the others are that arm's case's.
                        let constants = match i.checked_sub(1).is_some_and(|before| falls[before]) {
                            true if constants.is_empty() => constants,
                            true => {
                                let before = self.constant_values(&arms[i - 1].pattern, present)?;
                                let values = self.constant_values(&arm.pattern, present)?;
                                (constants.into_iter().zip(values))
                                    .filter(|(_, value)| !before.contains(value))
                                    .map(|(constant, _)| constant)
                                    .collect()
                            }
                            false => constants,
                        };
                        Some(SwitchArm {
                            span: self.js_span(arm.span),
                            pattern: self.js_span(arm.pattern.span),
                            guard: guard.map(|guard| self.js_span(self.frame.thir[guard].span)),
                            constants,
                            falls: falls[i],
                        })
                    })
                    .collect::<Option<_>>()?;
                // Of three cases or more, each constant one and a `_` one: two
                // are the `if` and `else`, or the `?:`, a person writes of two
                // ways. Decided with the user.
                let cases: usize = arms.iter().map(SwitchArm::cases).sum();
                (cases >= SWITCH_CASES).then(|| Switch {
                    span: self.js_span(e.span),
                    arms,
                    valued,
                })
            })
            .collect()
    }

    /// Whether the value of the expression `hir_id` is read in place: an
    /// operand's, an argument's, a test's; not a statement's, a `let`'s, an
    /// assignment's, what's returned, nor a block's, an arm's, a branch's
    /// that's one of those.
    fn read_in_place(&self, hir_id: rustc_hir::HirId) -> bool {
        use rustc_hir::{ExprKind, Node};
        let mut child = hir_id;
        for (id, node) in self.tcx.hir_parent_iter(hir_id) {
            match node {
                Node::Stmt(_) | Node::Item(_) | Node::ImplItem(_) | Node::TraitItem(_) => return false,
                // A `let` of a name; one of a pattern reads it in place, `let
                // (a, b) = ..`.
                Node::LetStmt(local) => return !matches!(local.pat.kind, rustc_hir::PatKind::Binding(..)),
                Node::Block(_) | Node::Arm(_) => {}
                Node::Expr(e) => match e.kind {
                    // What's returned, a closure's too, or assigned.
                    ExprKind::Ret(_) | ExprKind::Closure(_) | ExprKind::Assign(..) => return false,
                    ExprKind::Block(..) | ExprKind::Loop(..) | ExprKind::DropTemps(_) => {}
                    // An arm, or a branch, not what's matched or tested.
                    ExprKind::Match(scrutinee, ..) | ExprKind::If(scrutinee, ..) if scrutinee.hir_id != child => {}
                    _ => return true,
                },
                _ => return true,
            }
            child = id;
        }
        true
    }

    /// Whether what Rust tries after a guard of an arm of `pattern` fails,
    /// the arms `later`, does nothing: each that may match a value `pattern`
    /// does is a last `_ => {}` (ADR 0629). Then the arm is a case whose
    /// statements test the guard, `case C: if (on) { .. } break;`.
    fn fails_to_nothing(&self, pattern: &thir::Pat<'tcx>, later: &[&thir::Arm<'tcx>]) -> bool {
        (later.iter()).all(|arm| {
            !self.may_match_alike(pattern, &arm.pattern)
                || (matches!(arm.pattern.kind, thir::PatKind::Wild)
                    && arm.guard.is_none()
                    && self.does_nothing(arm.body))
        })
    }

    /// How many values `guard` compares what's matched with, each read with
    /// no call of the program's, a thread-local's, `END.with(|e| e ==
    /// name)`, `END.with_borrow(..)`, `LIMIT.get() == n`, or a `static`'s,
    /// `name == FIXED`, `||` joining them (rd113, the user's decision): its
    /// arm is a case of each, which a JS `switch` reads where Rust tries the
    /// guard, each case's in order until one matches. Which value, and that
    /// it's a read, its test says (`Constant::Read`).
    fn reads_compared(&self, guard: thir::ExprId, name: thir::LocalVarId) -> Option<usize> {
        let thir = self.frame.thir;
        if let thir::ExprKind::LogicalOp {
            op: thir::LogicalOp::Or,
            lhs,
            rhs,
        } = thir[body_queries::strip(thir, guard)].kind
        {
            return Some(self.reads_compared(lhs, name)? + self.reads_compared(rhs, name)?);
        }
        // What a call is, of the standard library's.
        let called = |e: &thir::Expr<'tcx>| match e.kind {
            thir::ExprKind::Call { fun, .. } => {
                (fn_def(thir[fun].ty)).and_then(|(f, args)| self.recognition().classify(f, args))
            }
            _ => None,
        };
        // Through its borrows, `&*FIXED`'s `FIXED`.
        let peeled = |mut e: thir::ExprId| {
            while let thir::ExprKind::Scope { value: inner, .. }
            | thir::ExprKind::Use { source: inner }
            | thir::ExprKind::Deref { arg: inner }
            | thir::ExprKind::Borrow { arg: inner, .. } = thir[e].kind
            {
                e = inner;
            }
            &thir[e]
        };
        // The name, and a value read: a `static`'s, or a thread-local's
        // `get()`, not another cell's.
        let named = |e: thir::ExprId| matches!(peeled(e).kind, thir::ExprKind::VarRef { id } if id == name);
        let read = |e: thir::ExprId| {
            let e = peeled(e);
            match e.kind {
                thir::ExprKind::StaticRef { .. } => true,
                thir::ExprKind::Call { ref args, .. } => {
                    matches!(called(e), Some(Std::CellGet))
                        && (args.first()).is_some_and(|&key| {
                            self.recognition()
                                .is_std_type(thir[key].ty.peel_refs(), StdItem::LocalKey)
                        })
                }
                _ => false,
            }
        };
        let compared = |lhs, rhs| (named(lhs) && read(rhs)) || (read(lhs) && named(rhs));
        let e = &thir[body_queries::strip(thir, guard)];
        let read = match e.kind {
            thir::ExprKind::Binary {
                op: rustc_middle::mir::BinOp::Eq,
                lhs,
                rhs,
            } => compared(lhs, rhs),
            // `==` of what's compared by reference, `PartialEq::eq(&a, &b)`.
            thir::ExprKind::Call {
                fun,
                ref args,
                from_hir_call: false,
                ..
            } if let [lhs, rhs] = args[..]
                && (fn_def(thir[fun].ty)).is_some_and(|(f, _)| {
                    (self.tcx.trait_of_assoc(f)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::PartialEq))
                }) =>
            {
                compared(lhs, rhs)
            }
            // A thread-local's value, given to a closure that compares it.
            _ => matches!(called(e), Some(Std::LocalWith | Std::LocalBorrow { mutable: false })),
        };
        read.then_some(1)
    }

    /// Whether an arm of `pattern` whose guard fails may fall to the case of
    /// the next arm, of `next` (ADR 0905): `next` names each value `pattern`
    /// does, which Rust tries it of next, and one of its own, its case's,
    /// each value told (`constant_values`).
    fn falls_to(&self, pattern: &thir::Pat<'tcx>, next: &thir::Pat<'tcx>, present: bool) -> bool {
        let (Some(values), Some(next)) = (
            self.constant_values(pattern, present),
            self.constant_values(next, present),
        ) else {
            return false;
        };
        values.iter().all(|v| next.contains(v)) && next.iter().any(|v| !values.contains(v))
    }

    /// The value of each constant `pattern` names, of a value `present`
    /// (`constants_of`), in its order, where each is told
    /// (`body_queries::pattern_value`): a number's, a `char`'s, the
    /// registry's symbol's.
    fn constant_values(&self, pattern: &thir::Pat<'tcx>, present: bool) -> Option<Vec<PatternValue<'tcx>>> {
        (self.valued_constants(pattern, present)?.into_iter())
            .map(|(_, value)| value)
            .collect()
    }

    /// Whether a value `a` matches may be one `b` matches, of the patterns
    /// of constants (`constants_of`): unless each constant of one is a value
    /// another of the other's isn't, a variant or the value a constant
    /// pattern is (`body_queries::pattern_value`). Of a float's, which
    /// that doesn't tell, it may.
    fn may_match_alike(&self, a: &thir::Pat<'tcx>, b: &thir::Pat<'tcx>) -> bool {
        use thir::PatKind;
        match (&a.kind, &b.kind) {
            (PatKind::Deref { subpattern, .. }, _) => self.may_match_alike(subpattern, b),
            (_, PatKind::Deref { subpattern, .. }) => self.may_match_alike(a, subpattern),
            (PatKind::Or { pats }, _) => pats.iter().any(|a| self.may_match_alike(a, b)),
            (_, PatKind::Or { pats }) => pats.iter().any(|b| self.may_match_alike(a, b)),
            (
                PatKind::Variant {
                    variant_index: x,
                    subpatterns: xs,
                    ..
                },
                PatKind::Variant {
                    variant_index: y,
                    subpatterns: ys,
                    ..
                },
            ) => x == y && (xs.iter().zip(ys)).all(|(x, y)| self.may_match_alike(&x.pattern, &y.pattern)),
            _ => match (
                crate::lower::body_queries::pattern_value(self.tcx, a),
                crate::lower::body_queries::pattern_value(self.tcx, b),
            ) {
                (Some(x), Some(y)) => x == y,
                _ => true,
            },
        }
    }

    /// Whether `body`, an arm's, is a jump alone, `break`, `continue` or
    /// `return`, in a block or not.
    fn jump_alone(&self, mut body: thir::ExprId) -> bool {
        loop {
            match self.frame.thir[body].kind {
                thir::ExprKind::Scope { value, .. }
                | thir::ExprKind::Use { source: value }
                | thir::ExprKind::NeverToAny { source: value } => body = value,
                thir::ExprKind::Block { block }
                    if self.frame.thir[block].stmts.is_empty() && self.frame.thir[block].expr.is_some() =>
                {
                    body = self.frame.thir[block].expr.expect("matched");
                }
                thir::ExprKind::Block { block } if self.frame.thir[block].expr.is_none() => {
                    let stmts = &self.frame.thir[block].stmts;
                    let [only] = stmts[..] else {
                        return false;
                    };
                    let thir::StmtKind::Expr { expr, .. } = self.frame.thir[only].kind else {
                        return false;
                    };
                    body = expr;
                }
                thir::ExprKind::Break { .. } | thir::ExprKind::Continue { .. } | thir::ExprKind::Return { .. } => {
                    return true;
                }
                _ => return false,
            }
        }
    }

    /// Whether `body`, an arm's, is `{}`.
    fn does_nothing(&self, mut body: thir::ExprId) -> bool {
        while let thir::ExprKind::Scope { value: inner, .. } = self.frame.thir[body].kind {
            body = inner;
        }
        matches!(self.frame.thir[body].kind, thir::ExprKind::Block { block }
            if self.frame.thir[block].stmts.is_empty() && self.frame.thir[block].expr.is_none())
    }

    /// The variants of an untagged enum `pattern` is, each of a field that's
    /// its value, bound or not: one's, or an or-pattern's of one enum.
    fn untagged_variants(
        &self,
        pattern: &thir::Pat<'tcx>,
    ) -> Option<(ty::AdtDef<'tcx>, ty::GenericArgsRef<'tcx>, Vec<&'tcx ty::VariantDef>)> {
        match &pattern.kind {
            thir::PatKind::Variant {
                adt_def,
                args,
                variant_index,
                subpatterns,
            } if crate::lower::bindings::is_untagged(self.tcx, adt_def.did())
                && crate::lower::untagged::told_by_kind(self.tcx, adt_def.variant(*variant_index))
                && (subpatterns.iter()).all(|field| {
                    matches!(
                        field.pattern.kind,
                        thir::PatKind::Wild | thir::PatKind::Binding { subpattern: None, .. }
                    )
                }) =>
            {
                Some((*adt_def, args, vec![adt_def.variant(*variant_index)]))
            }
            thir::PatKind::Or { pats } => {
                let mut each = pats.iter().map(|p| self.untagged_variants(p));
                let (adt, args, mut variants) = each.next()??;
                for other in each {
                    let (other, _, more) = other?;
                    if other != adt {
                        return None;
                    }
                    variants.extend(more);
                }
                Some((adt, args, variants))
            }
            _ => None,
        }
    }

    /// Each constant `pattern` names, if it names constants alone: a literal,
    /// a `const` (ADR 0590), a variant of no field that JS tests as a value
    /// it's equal to, `k === "Function"` (ADR 0013), each of an or-pattern.
    /// Of a value `present`, `classify(Some(value))`'s, as `untagged_case`
    /// has it (ADR 0676).
    fn constants_of(&self, pattern: &thir::Pat<'tcx>, present: bool) -> Option<Vec<Constant>> {
        let valued = self.valued_constants(pattern, present)?;
        Some(valued.into_iter().map(|(constant, _)| constant).collect())
    }

    /// `constants_of`'s, each with the value its pattern is, where it's told
    /// (`body_queries::pattern_value`).
    fn valued_constants(&self, pattern: &thir::Pat<'tcx>, present: bool) -> Option<Vec<Valued<'tcx>>> {
        let value = |constant| (constant, body_queries::pattern_value(self.tcx, pattern));
        match &pattern.kind {
            thir::PatKind::Deref { subpattern, .. } => self.valued_constants(subpattern, present),
            // Of a number, a `char` or a string, which `===` compares by its
            // value; not a `bool`, which has two, its `if`'s.
            thir::PatKind::Constant { .. } => {
                let ty = pattern.ty.peel_refs();
                let scalar = ty.is_integral() || ty.is_floating_point() || ty.is_char() || ty.is_str();
                scalar.then(|| vec![value(Constant::Value(self.js_span(pattern.span)))])
            }
            // A `#[repr(transparent)]` struct's of such a constant, which is
            // its field in JS (ADR 0584), `EventPriority(2)`.
            thir::PatKind::Leaf { .. }
                if let Some(field) = body_queries::transparent_subpattern(self.tcx, pattern)
                    && let thir::PatKind::Constant { .. } = field.kind
                    && (field.ty.is_integral() || field.ty.is_floating_point() || field.ty.is_char()) =>
            {
                Some(vec![value(Constant::Value(self.js_span(pattern.span)))])
            }
            // The registry's symbol, `REACT_MEMO_TYPE`, which `===` compares
            // as it does its key (ADR 0634).
            thir::PatKind::Leaf { .. } if self.recognition().is_symbol_for(pattern.ty) => {
                Some(vec![value(Constant::Value(self.js_span(pattern.span)))])
            }
            // A `Some` of constants, where it's the value itself (ADR
            // 0030), whose test of each says it's there.
            thir::PatKind::Variant {
                adt_def, subpatterns, ..
            } if self.tcx.is_lang_item(adt_def.did(), LangItem::Option)
                && let [some] = &subpatterns[..]
                && !self.boxed_payload(some.pattern.ty) =>
            {
                self.valued_constants(&some.pattern, false)
            }
            // A variant of an untagged enum, by what `typeof` says of it, its
            // value bound or not, which is the value itself (ADR 0668), or
            // `undefined` or `null` (ADR 0676); or an or-pattern of them,
            // by what they are together, `Object | Array`'s `"object"`, as
            // their test is.
            thir::PatKind::Variant { .. } | thir::PatKind::Or { .. }
                if let Some((adt, args, variants)) = self.untagged_variants(pattern) =>
            {
                let subject = js::Expr::var(SUBJECT);
                let cases = (self.recognition()).untagged_cases(adt, args, &variants, &subject, present)?;
                Some(cases.into_iter().map(|case| (Constant::Kind(case), None)).collect())
            }
            thir::PatKind::Variant {
                adt_def,
                variant_index,
                subpatterns,
                ..
            } if subpatterns.is_empty() && adt_def.variant(*variant_index).fields.is_empty() => {
                let subject = js::Expr::var(SUBJECT);
                let test = self.variant_test(subject.clone(), pattern.ty, *adt_def, *variant_index, false);
                match test.kind {
                    js::ExprKind::Binary(js::Op::Eq, s, variant) if *s == subject => {
                        Some(vec![(Constant::Variant(*variant), None)])
                    }
                    _ => None,
                }
            }
            thir::PatKind::Or { pats } => {
                let each: Option<Vec<Vec<Valued<'tcx>>>> =
                    pats.iter().map(|p| self.valued_constants(p, present)).collect();
                Some(each?.concat())
            }
            _ => None,
        }
    }

    /// Where each `if` the program wrote of a `jump`, a `continue` or a
    /// loop's `break`, tests, one alone or one that ends what it does, and
    /// each `let … else` of one, which MIR tests as an `if` of it in its
    /// place: and where its jump is, which its comment is before (ADR 0684,
    /// amended).
    /// Of an `if`'s `else` block, `of_else`, where it ends in the jump, of
    /// no `let … else`'s.
    fn jump_ifs(&self, jump: &dyn Fn(&thir::ExprKind<'tcx>) -> bool, of_else: bool) -> Vec<(js::Span, js::Span)> {
        let lets = if of_else { Vec::new() } else { self.let_else_jumps(jump) };
        (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If {
                    cond, then, else_opt, ..
                } => {
                    let mut then = if of_else { else_opt? } else { then };
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.frame.thir[then].kind
                    {
                        then = inner;
                    }
                    let thir::ExprKind::Block { block } = self.frame.thir[then].kind else {
                        return None;
                    };
                    let block = &self.frame.thir[block];
                    let only = match (block.stmts.last(), block.expr) {
                        (_, Some(e)) => Some(e),
                        (Some(s), None) => match self.frame.thir[*s].kind {
                            thir::StmtKind::Expr { expr, .. } => Some(expr),
                            _ => None,
                        },
                        _ => None,
                    }?;
                    let mut only = only;
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.frame.thir[only].kind
                    {
                        only = inner;
                    }
                    jump(&self.frame.thir[only].kind).then(|| {
                        (
                            self.js_span(self.frame.thir[cond].span),
                            self.js_span(self.frame.thir[only].span),
                        )
                    })
                }
                _ => None,
            })
            .chain(lets)
            .collect()
    }

    /// Where each `let … else` the program wrote of a `jump` is (`jump_ifs`).
    fn let_else_jumps(&self, jump: &dyn Fn(&thir::ExprKind<'tcx>) -> bool) -> Vec<(js::Span, js::Span)> {
        (self.frame.thir.stmts.iter())
            .filter_map(|stmt| match stmt.kind {
                thir::StmtKind::Let {
                    else_block: Some(block),
                    span,
                    ..
                } => {
                    let block = &self.frame.thir[block];
                    let mut only = match (block.stmts.last(), block.expr) {
                        (_, Some(e)) => e,
                        (Some(s), None) => match self.frame.thir[*s].kind {
                            thir::StmtKind::Expr { expr, .. } => expr,
                            _ => return None,
                        },
                        _ => return None,
                    };
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.frame.thir[only].kind
                    {
                        only = inner;
                    }
                    jump(&self.frame.thir[only].kind)
                        .then(|| (self.js_span(span), self.js_span(self.frame.thir[only].span)))
                }
                _ => None,
            })
            .collect()
    }

    /// Each temporary MIR made, `_5`, by the names a Rust developer's
    /// tooling suggests for it, rust-analyzer's, first to last (ADR 0569):
    /// the method or function that made it, `trailingZeros` of
    /// `a.trailing_zeros()`, `uppercase` of `s.to_uppercase()`, but for one
    /// that names nothing, `unwrap`, `clone`, `new`, `from_str`, and one the
    /// source doesn't call by its name, a macro's, an operator's or a
    /// closure's; then the program's type it holds, `point` of an
    /// `Option<Point>`. One that takes or copies another temporary, `&_5`
    /// as `format_args!` takes what it shows, has that one's names.
    pub(super) fn temporaries_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> std::collections::HashMap<String, Vec<String>> {
        // Each call the source writes by its name, `a.trailing_zeros()`: not
        // one a macro writes, `vec![..]`'s, nor an operator's, `a[i]`'s, nor a
        // closure's, `f(x)`'s `call_mut`.
        let source = self.tcx.sess.source_map();
        let mut called: std::collections::HashMap<Local, rustc_span::def_id::DefId> = std::collections::HashMap::new();
        let mut taken_from: std::collections::HashMap<Local, Local> = std::collections::HashMap::new();
        for data in mir_body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Some(local) = assign.0.as_local()
                    && let Rvalue::Ref(_, _, from) | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..) =
                        &assign.1
                    && let Some(from) = from.as_local()
                {
                    taken_from.insert(local, from);
                }
            }
            if let TerminatorKind::Call {
                func,
                destination,
                fn_span,
                ..
            } = &data.terminator().kind
                && destination.projection.is_empty()
                && let Some((def_id, _)) = func.const_fn_def()
                && !fn_span.from_expansion()
                && let Ok(written) = source.span_to_snippet(*fn_span)
                && written.starts_with(self.tcx.item_name(def_id).as_str())
            {
                called.insert(destination.local, def_id);
            }
        }
        (state.locals.names.iter_enumerated())
            .filter(|(_, name)| super::tidy::is_temporary(name))
            .map(|(local, name)| {
                // What it was taken from, as far as that goes.
                let mut at = local;
                for _ in 0..mir_body.local_decls.len() {
                    match taken_from.get(&at) {
                        Some(&from) if from != at && !called.contains_key(&at) => at = from,
                        _ => break,
                    }
                }
                let suggested = [
                    called.get(&at).and_then(|&def_id| self.call_name(def_id)),
                    self.held_type_name(mir_body.local_decls[local].ty),
                ];
                let names = (suggested.into_iter().flatten())
                    .map(|name| variable_name(&crate::lower::camel_case(&name)))
                    .filter(|name| crate::names::js_ident(name) == *name)
                    .collect();
                (name.clone(), names)
            })
            .collect()
    }

    /// What rust-analyzer names a call's result: a method's name without a
    /// conversion's prefix, `uppercase` of `to_uppercase`, and none of a
    /// method that names nothing; a function's without a constructor's
    /// prefix, `config` of `from_config`; and none of a name that's no
    /// noun of what it gives, `new`, `default`, `str`.
    fn call_name(&self, def_id: rustc_span::def_id::DefId) -> Option<String> {
        const USELESS_METHODS: &[&str] = &[
            "to_string",
            "as_str",
            "to_owned",
            "as_ref",
            "clone",
            "cloned",
            "expect",
            "expect_none",
            "unwrap",
            "unwrap_none",
            "unwrap_or",
            "unwrap_or_default",
            "unwrap_or_else",
            "unwrap_unchecked",
            "iter",
            "into_iter",
            "iter_mut",
            "into_future",
            // And the `Err` side of what rust-analyzer's list has, as it names
            // nothing either.
            "unwrap_err",
            "expect_err",
        ];
        const USELESS_NAMES: &[&str] = &[
            "new", "default", "option", "some", "none", "ok", "err", "str", "string", "from", "into",
        ];
        let name = self.tcx.item_name(def_id);
        let name = name.as_str();
        let method = self
            .tcx
            .opt_associated_item(def_id)
            .is_some_and(|item| item.is_method());
        let name = match method {
            true if USELESS_METHODS.contains(&name) => return None,
            true => ["into_", "as_", "to_"]
                .iter()
                .find_map(|p| name.strip_prefix(p))
                .unwrap_or(name),
            false => ["from_", "with_", "into_"]
                .iter()
                .find_map(|p| name.strip_prefix(p))
                .unwrap_or(name),
        };
        let plain = name.starts_with(|c: char| c.is_ascii_lowercase()) && !name.starts_with('_');
        let keyword = rustc_span::symbol::Ident::from_str(name).is_reserved();
        (plain && !keyword && !USELESS_NAMES.contains(&name)).then(|| name.to_string())
    }

    /// The name a value of `ty` is kept by, its type's, `order` of an
    /// `Order`, through a reference and what JS holds as the value it
    /// holds, an `Option`, a `Box`, an `Rc`: of the program's own or a
    /// library's type, `None` of std's. Not a `Result`'s `Ok`, as
    /// rust-analyzer's is: JS holds a `Result` as `{ TAG, _0 }`, which
    /// `order.TAG` would misname.
    fn held_type_name(&self, ty: Ty<'tcx>) -> Option<String> {
        let mut ty = ty.peel_refs();
        loop {
            ty = match ty.kind() {
                _ if let Some(inner) = self.option_of(ty) => inner.peel_refs(),
                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) => args.type_at(0).peel_refs(),
                _ => break,
            };
        }
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if crate::lower::recognition::is_std_crate(self.tcx, adt.did()) {
            return None;
        }
        Some(self.tcx.item_name(adt.did()).to_string())
    }

    /// The names of the locals a `clone()` makes, `let resolved =
    /// payload.clone()`: where that's the value itself in JS, an uncounted
    /// `Rc`'s, tidy reads it by the name it's of (ADR 0846).
    pub(super) fn clone_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> std::collections::HashSet<String> {
        (mir_body.basic_blocks.iter())
            .filter_map(|block| match &block.terminator().kind {
                mir::TerminatorKind::Call { func, destination, .. }
                    if destination.projection.is_empty()
                        && func
                            .const_fn_def()
                            .is_some_and(|(def_id, _)| self.tcx.is_lang_item(def_id, LangItem::CloneFn)) =>
                {
                    Some(state.locals.names[destination.local].clone())
                }
                _ => None,
            })
            .filter(|name| !name.is_empty())
            .collect()
    }

    /// The names each of whose locals is a `bool`, a shared reference to one
    /// too, which JS reads as it, of a closure what it captures too; the
    /// names each of whose locals is an integer; and those each of whose
    /// locals is a float, a JS number too (ADR 0966).
    pub(super) fn typed_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> (
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
    ) {
        // A name each of whose locals is a `bool`, and one each of whose
        // locals is an integer.
        let mut booleans = std::collections::HashSet::new();
        let mut others = std::collections::HashSet::new();
        let mut numbers = std::collections::HashSet::new();
        let mut not_numbers = std::collections::HashSet::new();
        let mut floats = std::collections::HashSet::new();
        let mut not_floats = std::collections::HashSet::new();
        for (local, name) in state.locals.names.iter_enumerated().filter(|(_, n)| !n.is_empty()) {
            // A shared reference to one too, `*show_hint`'s, which JS reads as it.
            let ty = mir_body.local_decls[local].ty;
            match ty.is_integral() {
                true => numbers.insert(name.clone()),
                false => not_numbers.insert(name.clone()),
            };
            match ty.is_floating_point() {
                true => floats.insert(name.clone()),
                false => not_floats.insert(name.clone()),
            };
            let bool = match ty.kind() {
                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),
                _ => ty.is_bool(),
            };
            match bool {
                true => booleans.insert(name.clone()),
                false => others.insert(name.clone()),
            };
        }
        // And a closure's, of what it captures: its parent's.
        let env = mir_body
            .args_iter()
            .next()
            .map(|local| mir_body.local_decls[local].ty.peel_refs());
        if let Some(ty::Closure(_, args)) = env.map(|ty| ty.kind()) {
            for (ty, (capture, _)) in args.as_closure().upvar_tys().iter().zip(state.facts.captures()) {
                if let js::ExprKind::Var(name) = &capture.kind {
                    match ty.peel_refs().is_bool() {
                        true => booleans.insert(name.clone()),
                        false => others.insert(name.clone()),
                    };
                }
            }
        }
        booleans.retain(|name| !others.contains(name));
        numbers.retain(|name| !not_numbers.contains(name));
        floats.retain(|name| !not_floats.contains(name));
        (booleans, numbers, floats)
    }
}

/// A closure's captures it took by value, each a variable its maker never
/// assigns again from where it makes the closure (`mir_closure_fn`'s
/// copy of one it does): as fixed in it as a parameter, which a clone of
/// it may be read as (ADR 0879).
pub(super) fn kept_captures(state: &State<'_, '_>) -> std::collections::HashSet<String> {
    (state.facts.captures().iter())
        .filter(|(_, place)| !place)
        .filter_map(|(capture, _)| match &capture.kind {
            js::ExprKind::Var(name) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// A type's name as a variable's, lower camel case, as JS names one:
/// `order` of `Order`, `httpServer` of `HTTPServer`, `url` of `URL`.
fn variable_name(type_name: &str) -> String {
    let chars: Vec<char> = type_name.chars().collect();
    // The capitals it starts with, but for one that starts the next word.
    let run = chars.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let lowered = match run > 1 && run < chars.len() {
        true => run - 1,
        false => run,
    };
    (chars.iter().enumerate())
        .map(|(i, c)| match i < lowered.max(1) {
            true => c.to_ascii_lowercase(),
            false => *c,
        })
        .collect()
}

/// A label the program wrote, `'get_tag`, as JS writes it: camelCase,
/// `getTag`, as every name is (ADRs 0573, 0944).
fn written_label(label: &rustc_ast::Label) -> String {
    crate::lower::camel_case(label.ident.name.as_str().trim_start_matches('\''))
}

#[cfg(test)]
mod tests {
    use super::variable_name;

    // A type's name as a variable's: its first capital lowered, a run of them
    // too, but for the one that starts the next word.
    #[test]
    fn a_type_name_is_a_variable_name() {
        assert_eq!(variable_name("Order"), "order");
        assert_eq!(variable_name("HTTPServer"), "httpServer");
        assert_eq!(variable_name("URL"), "url");
        assert_eq!(variable_name("IOError"), "ioError");
        assert_eq!(variable_name("trailingZeros"), "trailingZeros");
    }
}
