//! A local used before any path has written it whole: `_5.1 = ..` before
//! any `_5 = ..`, or `&raw mut _1`, or a read of `_1`, where nothing wrote
//! `_1` (#1382). Only custom MIR writes it, as rustc refuses it of Rust
//! (E0381, "partially assigned binding isn't fully initialized", "used
//! binding isn't initialized"), and a local is declared where it's written
//! whole, so its JS would read a name it never declared. Refused, never
//! lowered; asked of custom MIR's bodies alone.

use super::*;
use rustc_middle::mir::visit::{PlaceContext, Visitor};

/// Of a body written in custom MIR (`injection_phase`), the span of the
/// first use of a local, or of the return place, that some path from the body's start reaches before any write of it whole: a
/// read, a borrow, a write through it, or a write of a part of it, a
/// field, an index or a variant's. A parameter is written whole as the
/// body starts; a local's storage ending leaves it unwritten again. A
/// `drop`, which drop elaboration flags, isn't a use; nor is a read of a
/// type of one value, `()`, which `one_value` says, as no write changes it.
pub(super) fn partly_written<'tcx>(body: &mir::Body<'tcx>, one_value: impl Fn(Ty<'tcx>) -> bool) -> Option<Span> {
    // Rust's own bodies are rustc's, which E0381 holds to it, and the
    // MIR rust-js lowers is rewritten (`mir_threaded`, a library's
    // optimized MIR), where a local may be written by its parts.
    body.injection_phase?;
    let blocks = &body.basic_blocks;
    let locals = body.local_decls.len();
    let any: Vec<bool> = body.local_decls.iter().map(|decl| one_value(decl.ty)).collect();
    // Of each block, the locals every path to its start wrote whole: a
    // must-analysis, each block unvisited until a predecessor says what it
    // wrote, then only less.
    let mut start: Vec<Option<Vec<bool>>> = vec![None; blocks.len()];
    start[mir::START_BLOCK.index()] = Some((0..locals).map(|l| (1..=body.arg_count).contains(&l)).collect());
    let order: Vec<BasicBlock> = blocks.reverse_postorder().to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for &block in &order {
            let Some(written) = start[block.index()].clone() else {
                continue;
            };
            let data = &blocks[block];
            let mut walk = Walk {
                written,
                any: &any,
                unwritten: None,
            };
            for (index, statement) in data.statements.iter().enumerate() {
                let location = mir::Location {
                    block,
                    statement_index: index,
                };
                match &statement.kind {
                    StatementKind::Assign(assign) => {
                        walk.visit_rvalue(&assign.1, location);
                        walk.write(&assign.0);
                    }
                    StatementKind::StorageDead(local) => walk.written[local.index()] = false,
                    _ => {}
                }
                if walk.unwritten.is_some() {
                    return Some(statement.source_info.span);
                }
            }
            let terminator = data.terminator();
            let location = mir::Location {
                block,
                statement_index: data.statements.len(),
            };
            match &terminator.kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    walk.visit_operand(func, location);
                    for arg in args {
                        walk.visit_operand(&arg.node, location);
                    }
                    walk.write(destination);
                }
                TerminatorKind::SwitchInt { discr, .. } => walk.visit_operand(discr, location),
                TerminatorKind::Assert { cond, .. } => walk.visit_operand(cond, location),
                _ => {}
            }
            if walk.unwritten.is_some() {
                return Some(terminator.source_info.span);
            }
            for next in terminator.successors() {
                let met = match &start[next.index()] {
                    None => walk.written.clone(),
                    Some(before) => before.iter().zip(&walk.written).map(|(a, b)| *a && *b).collect(),
                };
                if start[next.index()].as_ref() != Some(&met) {
                    start[next.index()] = Some(met);
                    changed = true;
                }
            }
        }
    }
    None
}

/// The locals written whole so far in a block, those of a type of one
/// value, and the first used before it's written.
struct Walk<'a> {
    written: Vec<bool>,
    any: &'a [bool],
    unwritten: Option<Local>,
}

impl Walk<'_> {
    /// `place` written: whole, its local now written; a part of it, or
    /// through it, a use of its local.
    fn write(&mut self, place: &Place<'_>) {
        for elem in place.projection {
            if let PlaceElem::Index(index) = elem {
                self.used(index);
            }
        }
        match place.projection.is_empty() {
            true => self.written[place.local.index()] = true,
            false => self.used(place.local),
        }
    }

    fn used(&mut self, local: Local) {
        if !self.written[local.index()] && !self.any[local.index()] && self.unwritten.is_none() {
            self.unwritten = Some(local);
        }
    }
}

impl<'tcx> Visitor<'tcx> for Walk<'_> {
    fn visit_local(&mut self, local: Local, _: PlaceContext, _: mir::Location) {
        self.used(local);
    }
}
