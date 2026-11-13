//! Which compiler-made labels a body's JS keeps after tidy, by the MIR
//! shape that left each, where a compile records it, `--label-census
//! <file>` (#1461): what the structurer knew of each `bbN` it emitted, the
//! terminator kinds of the block's predecessors, and how far the JS's
//! jumps to it reach, the loops and blocks they cross. A row a shape, its
//! count the labels of it, ranked, is where the next label is taken away,
//! at its emitter, by the decisions walk, or left as the jump it is.
//! Recorded by statics, as `--tidy-fired`'s passes are.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use rustc_middle::mir::{BasicBlock, Body};

use crate::js::{Stmt, StmtKind};

/// Whether this compile records the labels left.
static RECORDS: AtomicBool = AtomicBool::new(false);
/// Each shape, by its key, and how many labels of it were left.
static CENSUS: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());

/// Records the labels left, for the rest of the compile.
pub(in crate::lower) fn record() {
    RECORDS.store(true, Ordering::Relaxed);
}

/// Each `bbN` label `body`'s JS, `stmts`, keeps, noted by its shape: of
/// its own statements alone, not a closure's, which its own body notes.
pub(super) fn note(body: &Body<'_>, stmts: &[Stmt]) {
    if !RECORDS.load(Ordering::Relaxed) {
        return;
    }
    let mut left: BTreeMap<String, Left> = BTreeMap::new();
    walk(stmts, &mut Vec::new(), &mut left);
    if left.is_empty() {
        return;
    }
    let mut census = CENSUS.lock().expect("no note panics holding it");
    for (label, found) in left {
        *census.entry(key(body, &label, &found)).or_default() += 1;
    }
}

/// Each shape and its count, a line each, `count<TAB>shape`, the most
/// first: what `--label-census` writes.
pub(in crate::lower) fn report() -> String {
    let census = CENSUS.lock().expect("no note panics holding it");
    let mut rows: Vec<(&String, &usize)> = census.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    rows.into_iter()
        .map(|(shape, count)| format!("{count}\t{shape}\n"))
        .collect()
}

/// A `bbN` label left: what it labels, and the most loops and blocks one of
/// its jumps crosses on its way out to it.
struct Left {
    kind: &'static str,
    jumps: usize,
    loops: usize,
    blocks: usize,
}

/// A statement a jump may leave: a loop, a block or a switch.
#[derive(Clone, Copy, PartialEq)]
enum Around {
    Loop,
    Block,
}

/// `stmts` walked, `around` what each is inside, from the outermost, its
/// label where it has one: each `bbN` left, and each jump to one.
fn walk(stmts: &[Stmt], around: &mut Vec<(Around, Option<String>)>, left: &mut BTreeMap<String, Left>) {
    for stmt in stmts {
        let label = |label: &Option<String>| label.clone().filter(|name| super::queries::is_block_label(name));
        let (inside, kind, label, bodies): (Around, &'static str, Option<String>, Vec<&[Stmt]>) = match &stmt.kind {
            StmtKind::While { label: l, body, .. } => (Around::Loop, "while", label(l), vec![body]),
            StmtKind::DoWhile { label: l, body, .. } => (Around::Loop, "do-while", label(l), vec![body]),
            StmtKind::ForOf { label: l, body, .. } => (Around::Loop, "for-of", label(l), vec![body]),
            StmtKind::For { label: l, body, .. } => (Around::Loop, "for", label(l), vec![body]),
            StmtKind::Labeled(l, body) => (Around::Block, "block", label(&Some(l.clone())), vec![body]),
            StmtKind::Switch { cases, default, .. } => {
                let mut bodies: Vec<&[Stmt]> = cases.iter().map(|case| case.body.as_slice()).collect();
                bodies.extend(default.as_deref());
                (Around::Block, "switch", None, bodies)
            }
            StmtKind::If(_, then, otherwise) => {
                walk(then, around, left);
                if let Some(otherwise) = otherwise {
                    walk(otherwise, around, left);
                }
                continue;
            }
            StmtKind::Try(body, finally) | StmtKind::TryCatch(body, _, finally) => {
                walk(body, around, left);
                walk(finally, around, left);
                continue;
            }
            StmtKind::Break(Some(to)) | StmtKind::Continue(Some(to)) => {
                if let Some(at) = around
                    .iter()
                    .rposition(|(_, label)| label.as_deref() == Some(to.as_str()))
                    && let Some(found) = left.get_mut(to)
                {
                    let crossed = &around[at + 1..];
                    found.jumps += 1;
                    found.loops = found
                        .loops
                        .max(crossed.iter().filter(|(a, _)| *a == Around::Loop).count());
                    found.blocks = found
                        .blocks
                        .max(crossed.iter().filter(|(a, _)| *a == Around::Block).count());
                }
                continue;
            }
            _ => continue,
        };
        if let Some(name) = &label {
            left.entry(name.clone()).or_insert(Left {
                kind,
                jumps: 0,
                loops: 0,
                blocks: 0,
            });
        }
        around.push((inside, label));
        for body in bodies {
            walk(body, around, left);
        }
        around.pop();
    }
}

/// `label`'s shape: what it labels, what decided each way to its block in
/// MIR (`decider`), and how many jumps reach it and the most each crosses.
fn key(body: &Body<'_>, label: &str, found: &Left) -> String {
    let block = super::queries::label_block(label);
    let from = match block.filter(|&b| b.as_usize() < body.basic_blocks.len()) {
        Some(block) => {
            let mut kinds: Vec<String> = (body.basic_blocks.predecessors()[block].iter())
                .map(|&pred| decider(body, pred))
                .collect();
            kinds.sort_unstable();
            kinds.dedup();
            kinds.join(" + ")
        }
        None => "?".into(),
    };
    let jumps = match found.jumps {
        1 => "1 jump".to_string(),
        n => format!("{n} jumps"),
    };
    format!(
        "{}, reached from {from}; {jumps}, crossing {} loops, {} blocks",
        found.kind, found.loops, found.blocks
    )
}

/// What decided the way through `block`: its terminator, or, of a `Goto`
/// or a `Drop` its one predecessor alone reaches, that one's, back to
/// where the way was chosen, a `SwitchInt`, a `Call`, a guard's
/// `FalseEdge`; said with the drops passed on the way there.
fn decider(body: &Body<'_>, mut block: BasicBlock) -> String {
    let preds = body.basic_blocks.predecessors();
    let mut drops = false;
    let mut seen = vec![block];
    loop {
        let kind = &body.basic_blocks[block].terminator().kind;
        let passes = matches!(
            kind,
            rustc_middle::mir::TerminatorKind::Goto { .. } | rustc_middle::mir::TerminatorKind::Drop { .. }
        );
        drops |= matches!(kind, rustc_middle::mir::TerminatorKind::Drop { .. });
        match preds[block].as_slice() {
            [only] if passes && !seen.contains(only) => {
                seen.push(*only);
                block = *only;
            }
            _ => {
                let name = kind.name();
                return if drops {
                    format!("{name} through drops")
                } else {
                    name.to_string()
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::walk;
    use crate::js::{Case, Expr, Span, Stmt, StmtKind};

    fn stmt(kind: StmtKind) -> Stmt {
        kind.at(Span::NONE)
    }

    fn while_true(body: Vec<Stmt>) -> Stmt {
        stmt(StmtKind::While {
            label: None,
            cond: Expr::bool(true),
            body,
        })
    }

    // `bb3: { while (true) { if (c) break bb3; } }`: a block, one jump to
    // it, crossing the loop; `bb7: for` left from a `switch`'s case,
    // crossing the switch, and a label the program wrote is no row.
    #[test]
    fn each_compiler_label_kept_and_how_far_its_jumps_reach() {
        let left_from_loop = stmt(StmtKind::Labeled(
            "bb3".into(),
            vec![while_true(vec![stmt(StmtKind::If(
                Expr::var("c"),
                vec![stmt(StmtKind::Break(Some("bb3".into())))],
                None,
            ))])],
        ));
        let switch = stmt(StmtKind::Switch {
            discriminant: Expr::var("x"),
            cases: vec![Case {
                values: vec![Expr::int(1)],
                body: vec![stmt(StmtKind::Break(Some("bb7".into())))],
                falls: false,
            }],
            default: None,
        });
        let left_from_switch = stmt(StmtKind::While {
            label: Some("bb7".into()),
            cond: Expr::bool(true),
            body: vec![switch],
        });
        let written = stmt(StmtKind::Labeled(
            "outer".into(),
            vec![stmt(StmtKind::Break(Some("outer".into())))],
        ));
        let mut left = BTreeMap::new();
        walk(&[left_from_loop, left_from_switch, written], &mut Vec::new(), &mut left);
        let found: Vec<_> = (left.iter())
            .map(|(name, l)| (name.as_str(), l.kind, l.jumps, l.loops, l.blocks))
            .collect();
        assert_eq!(found, [("bb3", "block", 1, 1, 0), ("bb7", "while", 1, 0, 1)]);
    }
}
