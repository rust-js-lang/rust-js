//! Which of the crate's `RefCell` fields a body uses whole, lent, stored,
//! moved or read by a derive, and whether each such use is a lend to a
//! parameter only ever borrowed: `CellForm`'s use rules (#2490's design
//! point 6, its whole-use proof A), split from `cell_form.rs` by concept.

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{self, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty;
use rustc_span::def_id::DefId;

use super::lent_field;
use crate::recognition::{Recognition, Std, StdItem};

/// How a `RefCell` field is used whole (`whole_uses`): the first use found,
/// and whether every one is a lend to a parameter of the crate's that's
/// only ever borrowed, or given on to another such (`param_borrows_only`),
/// which a rule of the field's own borrows through the parameter would
/// pass (#2490's whole-use proofs, measured).
pub(crate) struct Whole {
    pub(crate) why: String,
    pub(crate) lent_to_borrows: bool,
}

/// Of the crate's `bodies`, each `RefCell` field of a struct or a variant
/// one uses whole, and how (design point 6's use rules): lent, `&x.f`, to
/// what isn't the cell's own borrow or write (`borrows_its_cell`), its
/// `Clone`'s, its `Debug`'s, a `mem::replace`'s; stored, that reference put
/// in a place; or moved or copied out.
pub(crate) fn whole_uses<'b, 'tcx: 'b>(
    recognition: &Recognition<'_, 'tcx>,
    bodies: impl Iterator<Item = &'b mir::Body<'tcx>>,
) -> IndexMap<DefId, Whole> {
    let tcx = recognition.tcx;
    let bodies: Vec<&mir::Body<'tcx>> = bodies.collect();
    let by_id: IndexMap<DefId, &mir::Body<'tcx>> = bodies.iter().map(|body| (body.source.def_id(), *body)).collect();
    let mut wholes: IndexMap<DefId, Whole> = IndexMap::default();
    let cell_field = |body: &mir::Body<'tcx>, place: mir::Place<'tcx>| {
        lent_field(tcx, body, place).filter(|_| recognition.is_std_type(place.ty(body, tcx).ty, StdItem::RefCell))
    };
    for &body in &bodies {
        // Each local that's a reference to a `RefCell` field, `&x.f`, or
        // that reference given on to a local, `&*r` or `r` moved.
        let mut lent: IndexMap<Local, DefId> = IndexMap::default();
        loop {
            let before = lent.len();
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    let StatementKind::Assign(assign) = &statement.kind else {
                        continue;
                    };
                    let (place, rvalue) = &**assign;
                    if !place.projection.is_empty() {
                        continue;
                    }
                    let field = match rvalue {
                        Rvalue::Ref(_, _, of) => cell_field(body, *of).map(|field| field.did).or_else(|| {
                            (of.projection.as_slice() == [mir::PlaceElem::Deref])
                                .then(|| lent.get(&of.local).copied())
                                .flatten()
                        }),
                        Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..) if of.projection.is_empty() => {
                            lent.get(&of.local).copied()
                        }
                        _ => None,
                    };
                    if let Some(field) = field {
                        lent.entry(place.local).or_insert(field);
                    }
                }
            }
            if lent.len() == before {
                break;
            }
        }
        let mut note = |field: DefId, why: String, lent_to_borrows: bool| {
            let whole = wholes.entry(field).or_insert(Whole {
                why,
                lent_to_borrows: true,
            });
            whole.lent_to_borrows &= lent_to_borrows;
        };
        let at = |span: rustc_span::Span| tcx.sess.source_map().span_to_diagnostic_string(span);
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (place, rvalue) = &**assign;
                let mut operands: Vec<&Operand<'tcx>> = Vec::new();
                match rvalue {
                    Rvalue::Use(operand, ..) => operands.push(operand),
                    Rvalue::Aggregate(_, fields) => operands.extend(fields.iter()),
                    _ => {}
                }
                for operand in operands {
                    let Some(of) = operand.place() else { continue };
                    // A field moved or copied out whole.
                    if let Some(field) = cell_field(body, of) {
                        note(field.did, format!("moved at {}", at(statement.source_info.span)), false);
                    }
                    // A reference to one put in a place, not a local's.
                    if of.projection.is_empty()
                        && let Some(&field) = lent.get(&of.local)
                        && (!place.projection.is_empty() || matches!(rvalue, Rvalue::Aggregate(..)))
                    {
                        note(field, format!("stored at {}", at(statement.source_info.span)), false);
                    }
                }
            }
            let TerminatorKind::Call {
                func, args, fn_span, ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let callee = func.const_fn_def();
            for (index, arg) in args.iter().enumerate() {
                let Some(of) = arg.node.place() else { continue };
                if let Some(field) = cell_field(body, of) {
                    note(field.did, format!("moved at {}", at(*fn_span)), false);
                }
                let Some(&field) = lent.get(&of.local).filter(|_| of.projection.is_empty()) else {
                    continue;
                };
                // The cell's own call, a borrow of it; a `swap`'s of either
                // cell, which borrows each mutably (#659's rule).
                let own = callee.is_some_and(|(id, args)| {
                    (index == 0 && crate::drops::borrows_its_cell(recognition, id, args))
                        || (index <= 1 && matches!(recognition.classify(id, args), Some(Std::CellSwap)))
                });
                if !own {
                    let to = callee.map_or_else(|| "a fn pointer".to_string(), |(id, _)| tcx.def_path_str(id));
                    let borrows_only = callee.is_some_and(|(id, _)| {
                        param_borrows_only(recognition, &by_id, (id, index), &mut IndexSet::default())
                    });
                    note(field, format!("lent to `{to}` at {}", at(*fn_span)), borrows_only);
                }
            }
        }
    }
    // A derive rust-js writes itself, of which no body is lowered: its
    // `Clone`, `Debug` or `PartialEq` reads each `RefCell` field whole,
    // through the cell's own impl. Not a `Default`'s, which makes them.
    for (_, imp) in crate::traits::declared_impls(tcx) {
        let of = tcx
            .impl_trait_ref(imp)
            .instantiate_identity()
            .skip_normalization()
            .def_id;
        let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
        if crate::recognition::known_derive(tcx, imp)
            && !crate::recognition::is_std_def(tcx, of, StdItem::Default)
            && let ty::Adt(adt, _) = self_ty.kind()
            && !adt.is_union()
        {
            for field in adt.all_fields() {
                let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                if recognition.is_std_type(ty, StdItem::RefCell) {
                    let why = format!("derived `{}`", tcx.def_path_str(of));
                    wholes
                        .entry(field.did)
                        .or_insert(Whole {
                            why,
                            lent_to_borrows: true,
                        })
                        .lent_to_borrows = false;
                }
            }
        }
    }
    wholes
}

/// Whether `callee`'s parameter at `index`, a `&RefCell`, is only ever
/// borrowed, written by the cell's own call (`borrows_its_cell`), or given
/// on to a parameter of another body of the crate's that is, through its
/// copies and reborrows: a lend of a field to it is as the field's own
/// borrows. Of a callee with no body here, or seen already, no.
fn param_borrows_only<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    by_id: &IndexMap<DefId, &mir::Body<'tcx>>,
    (callee, index): (DefId, usize),
    seen: &mut IndexSet<(DefId, usize)>,
) -> bool {
    let Some(&body) = by_id.get(&callee) else { return false };
    if !seen.insert((callee, index)) || index >= body.arg_count {
        return false;
    }
    let mut lent: IndexSet<Local> = IndexSet::default();
    lent.insert(Local::from_usize(index + 1));
    loop {
        let before = lent.len();
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (place, rvalue) = &**assign;
                let from = match rvalue {
                    Rvalue::Ref(_, _, of) if of.projection.as_slice() == [mir::PlaceElem::Deref] => Some(of.local),
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..) if of.projection.is_empty() => {
                        Some(of.local)
                    }
                    _ => None,
                };
                if place.projection.is_empty() && from.is_some_and(|from| lent.contains(&from)) {
                    lent.insert(place.local);
                }
            }
        }
        if lent.len() == before {
            break;
        }
    }
    let uses_lent = |operand: &Operand<'tcx>| operand.place().is_some_and(|place| lent.contains(&place.local));
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            let derived = place.projection.is_empty()
                && matches!(rvalue, Rvalue::Ref(_, _, of) | Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..)
                    if lent.contains(&of.local));
            // Any other read of it, a raw pointer of it, a cast, a
            // comparison, takes it for what it is, not a borrow.
            let mut mentioned = Mentioned {
                lent: &lent,
                found: false,
            };
            mir::visit::Visitor::visit_rvalue(&mut mentioned, rvalue, mir::Location::START);
            if mentioned.found && !derived {
                return false;
            }
        }
        match &data.terminator().kind {
            TerminatorKind::Call { func, args, .. } => {
                for (at, arg) in args.iter().enumerate() {
                    if !uses_lent(&arg.node) {
                        continue;
                    }
                    let Some((id, generic_args)) = func.const_fn_def() else {
                        return false;
                    };
                    let own = at == 0 && crate::drops::borrows_its_cell(recognition, id, generic_args);
                    if !own && !param_borrows_only(recognition, by_id, (id, at), seen) {
                        return false;
                    }
                }
            }
            TerminatorKind::Return if lent.contains(&mir::RETURN_PLACE) => return false,
            _ => {}
        }
    }
    true
}

/// Whether an rvalue reads any of `lent`'s locals (`param_borrows_only`).
struct Mentioned<'l> {
    lent: &'l IndexSet<Local>,
    found: bool,
}

impl<'tcx> mir::visit::Visitor<'tcx> for Mentioned<'_> {
    fn visit_local(&mut self, local: Local, _: mir::visit::PlaceContext, _: mir::Location) {
        self.found |= self.lent.contains(&local);
    }
}
