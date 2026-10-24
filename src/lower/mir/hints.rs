//! What tidy is told of a body (`tidy::Hints`) that its THIR says: where
//! the program writes each `if`'s condition, each `&&` and `||`, each `if`
//! of a `continue`, each loop and whether as a `while`, and each `match`
//! that may read a table. A question module: it emits nothing.

use super::*;

/// Where the program writes what tidy reads a body's JS by.
pub(super) struct Written {
    /// Each `match` whose arms are each a variant of its own, with no field,
    /// guard or binding, and its variants' names (ADR 0233).
    pub tables: Vec<(js::Span, Vec<String>)>,
    /// Each `if`'s condition, and a `match` and the guard of an arm of it.
    pub conditions: Vec<(js::Span, js::Span)>,
    /// Each `if` written with an `else`: its condition, its `then` and its
    /// `else`, which MIR's structure may make a guard of (`written_elses`).
    pub elses: Vec<(js::Span, js::Span, js::Span)>,
    /// Each `&&` or `||` the program wrote.
    pub logicals: Vec<js::Span>,
    /// Each `if` of a `continue`, alone or ending what it does: where its
    /// condition is, and where the `continue` is, which its comment is
    /// before (ADR 0684).
    pub continues: Vec<(js::Span, js::Span)>,
    /// Each loop, whether as a `while`, and its label.
    pub loops: Vec<(js::Span, bool, Option<String>)>,
    /// Each block the program labeled, `'desc: { .. }` (`labeled_blocks`).
    pub blocks: Vec<LabeledBlock>,
    /// Each `match` the program wrote whose every arm tests constants, a
    /// `switch` (ADR 0629): its arms.
    pub switches: Vec<Switch>,
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

/// The jumps the program wrote in a body: where each `return` is, each
/// block it labeled; and, as its structure is made, the JS block of each
/// MIR block a `break` of one goes to, which is its end (ADR 0742).
pub(super) struct Jumps {
    pub returns: Vec<js::Span>,
    pub blocks: Vec<LabeledBlock>,
    pub left: std::collections::HashMap<BasicBlock, usize>,
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
}

/// An arm of a `match` of constants (`Written::switches`): where it, its
/// pattern and its guard are written, and each constant its pattern names,
/// `1 | 2`'s two, none of a last `_`, the `switch`'s `default`.
pub(super) struct SwitchArm {
    pub span: js::Span,
    pub pattern: js::Span,
    pub guard: Option<js::Span>,
    pub constants: Vec<Constant>,
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
}

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
        // branch.
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
                _ => None,
            })
            .collect();
        // Where each `&&` or `||` the program wrote is.
        let logicals: Vec<js::Span> = (self.frame.thir.exprs.iter())
            .filter(|e| matches!(e.kind, thir::ExprKind::LogicalOp { .. }))
            .map(|e| self.js_span(e.span))
            .collect();
        // Where each `if` the program wrote of a `continue` tests: one alone,
        // or one that ends what it does.
        let continues: Vec<(js::Span, js::Span)> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If { cond, then, .. } => {
                    let mut then = then;
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
                    matches!(self.frame.thir[only].kind, thir::ExprKind::Continue { .. }).then(|| {
                        (
                            self.js_span(self.frame.thir[cond].span),
                            self.js_span(self.frame.thir[only].span),
                        )
                    })
                }
                _ => None,
            })
            .chain(self.let_else_continues())
            .collect();
        // Where each loop is written, and whether as a `while`.
        // Each loop's label the program wrote, `'rows`, by the loop.
        let labels: std::collections::HashMap<thir::ExprId, String> = (self.frame.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Scope { value, hir_id, .. } => match self.tcx.hir_expect_expr(hir_id).kind {
                    // The loop it scopes, in scopes of its own too.
                    rustc_hir::ExprKind::Loop(_, Some(label), ..) => Some((
                        self.strip(value),
                        label.ident.name.as_str().trim_start_matches('\'').to_string(),
                    )),
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
        Written {
            tables,
            conditions,
            elses,
            logicals,
            continues,
            loops,
            blocks: self.labeled_blocks(),
            switches,
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
            left: Default::default(),
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
                            label: label.ident.name.as_str().trim_start_matches('\'').to_string(),
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

    /// Each `match` the program wrote, not a macro, `matches!`, of three
    /// arms or more, whose every arm tests constants, with no binding, but
    /// for a last `_`, and whose value isn't read in place, which JS has no
    /// `switch` expression for (ADR 0629): its arms. An arm with a guard is
    /// one where a failed guard goes on to nothing (`fails_to_nothing`).
    /// The one answer to which `match` is a `switch`.
    fn written_switches(&self) -> Vec<Switch> {
        // Each once, of the scopes THIR has of it.
        let mut seen = std::collections::HashSet::new();
        let held = crate::lower::body_queries::held_guards(self.tcx, self.frame.thir);
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
                let arms: Vec<SwitchArm> = (arms.iter().enumerate())
                    .map(|(i, arm)| {
                        // A guard that holds where the arm's pattern matches
                        // tests nothing: the arm is as one without it.
                        let guard = (arm.guard).filter(|&guard| {
                            !held.contains(
                                &self.frame.thir[crate::lower::body_queries::strip(self.frame.thir, guard)].span,
                            )
                        });
                        let constants = match &arm.pattern.kind {
                            thir::PatKind::Wild if i == last && guard.is_none() => Some(Vec::new()),
                            _ => (self.constants_of(&arm.pattern, present))
                                .filter(|_| guard.is_none() || self.fails_to_nothing(&arm.pattern, &arms[i + 1..])),
                        }?;
                        Some(SwitchArm {
                            span: self.js_span(arm.span),
                            pattern: self.js_span(arm.pattern.span),
                            guard: guard.map(|guard| self.js_span(self.frame.thir[guard].span)),
                            constants,
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

    /// Whether a value `a` matches may be one `b` matches, of the patterns
    /// of constants (`constants_of`): unless each constant of one is a value
    /// another of the other's isn't, a variant or the value a constant
    /// pattern is (`body_queries::pattern_value`). Of a string's or a
    /// float's, which that doesn't tell, it may.
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
        match &pattern.kind {
            thir::PatKind::Deref { subpattern, .. } => self.constants_of(subpattern, present),
            // Of a number, a `char` or a string, which `===` compares by its
            // value; not a `bool`, which has two, its `if`'s.
            thir::PatKind::Constant { .. } => {
                let ty = pattern.ty.peel_refs();
                let scalar = ty.is_integral() || ty.is_floating_point() || ty.is_char() || ty.is_str();
                scalar.then(|| vec![Constant::Value(self.js_span(pattern.span))])
            }
            // The registry's symbol, `REACT_MEMO_TYPE`, which `===` compares
            // as it does its key (ADR 0634).
            thir::PatKind::Leaf { .. } if self.recognition().is_symbol_for(pattern.ty) => {
                Some(vec![Constant::Value(self.js_span(pattern.span))])
            }
            // A `Some` of constants, where it's the value itself (ADR
            // 0030), whose test of each says it's there.
            thir::PatKind::Variant {
                adt_def, subpatterns, ..
            } if self.tcx.is_lang_item(adt_def.did(), LangItem::Option)
                && let [some] = &subpatterns[..]
                && !self.boxed_payload(some.pattern.ty) =>
            {
                self.constants_of(&some.pattern, false)
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
                Some(cases.into_iter().map(Constant::Kind).collect())
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
                    js::ExprKind::Binary(js::Op::Eq, s, value) if *s == subject => {
                        Some(vec![Constant::Variant(*value)])
                    }
                    _ => None,
                }
            }
            thir::PatKind::Or { pats } => {
                let each: Option<Vec<Vec<Constant>>> = pats.iter().map(|p| self.constants_of(p, present)).collect();
                Some(each?.concat())
            }
            _ => None,
        }
    }

    /// Where each `let … else { continue; }` the program wrote is, which MIR
    /// tests as an `if` of the `continue` in its place.
    fn let_else_continues(&self) -> Vec<(js::Span, js::Span)> {
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
                    matches!(self.frame.thir[only].kind, thir::ExprKind::Continue { .. })
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

    /// The names each of whose locals is a `bool`, a shared reference to one
    /// too, which JS reads as it, of a closure what it captures too; and the
    /// names each of whose locals is an integer.
    pub(super) fn typed_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> (std::collections::HashSet<String>, std::collections::HashSet<String>) {
        // A name each of whose locals is a `bool`, and one each of whose
        // locals is an integer.
        let mut booleans = std::collections::HashSet::new();
        let mut others = std::collections::HashSet::new();
        let mut numbers = std::collections::HashSet::new();
        let mut not_numbers = std::collections::HashSet::new();
        for (local, name) in state.locals.names.iter_enumerated().filter(|(_, n)| !n.is_empty()) {
            // A shared reference to one too, `*show_hint`'s, which JS reads as it.
            let ty = mir_body.local_decls[local].ty;
            match ty.is_integral() {
                true => numbers.insert(name.clone()),
                false => not_numbers.insert(name.clone()),
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
            for (ty, (capture, _)) in args.as_closure().upvar_tys().iter().zip(&state.captures) {
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
        (booleans, numbers)
    }
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
