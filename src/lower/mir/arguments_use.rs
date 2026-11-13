//! What a body does with a `fmt::Arguments` it makes, `format_args!`'s,
//! the one owner of it (#1565): one std formats where it's made, given
//! by value to `format`, `print!`'s `_print`, a `write_fmt` or `panic_fmt`,
//! or asked `as_str()`, is its text, made there; one given on, to a
//! function of the crate's or a library's, stored, returned or lent, is a
//! function that makes its text, which is called where Rust formats it, so
//! its values' `Display` and `Debug` run there, and as often as it's
//! formatted. Its operands are evaluated where it's made, as Rust's are.

use crate::lower::Std;
use crate::lower::recognition::Recognition;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::visit::{MutatingUseContext, NonUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{self, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty;

/// What a body does with the `fmt::Arguments` it makes into a local.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum ArgumentsUse {
    /// Given by value to std's formatting alone, or asked `as_str()`: its
    /// text, made where it's made.
    InPlace,
    /// Given by value to a `write!` into a writer of the crate's own, or a
    /// `W: fmt::Write`'s, which std writes a piece at a time: each write,
    /// made where it's written (#1652).
    Written,
    /// Given on: a function that makes its text.
    GivenOn,
}

/// What `body` does with the `fmt::Arguments` it makes into `local`.
pub(in crate::lower) fn arguments_use<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    body: &mir::Body<'tcx>,
    local: Local,
) -> ArgumentsUse {
    match formatted_by_std(recognition, body, local, 2) {
        true if crate::lower::recognition::writes_piecewise() && written_into_writer(recognition, body, local) => {
            ArgumentsUse::Written
        }
        true => ArgumentsUse::InPlace,
        false => ArgumentsUse::GivenOn,
    }
}

/// Whether `local` is given by value to a `write_fmt`, or `fmt::write`,
/// into a writer that std writes a piece at a time, `Std::UserWrite`'s, of
/// the crate's own: not a `W: fmt::Write`'s, whose `write_fmt` its
/// dictionary says, given the function (#1652, step 6).
fn written_into_writer<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &mir::Body<'tcx>, local: Local) -> bool {
    let tcx = recognition.tcx;
    body.basic_blocks.iter().any(|data| match &data.terminator().kind {
        TerminatorKind::Call { func, args, .. } => {
            args.iter().any(|arg| {
                arg.node
                    .place()
                    .is_some_and(|p| p.local == local && p.projection.is_empty())
            }) && crate::lower::fn_def(func.ty(&body.local_decls, tcx)).is_some_and(|(id, generic_args)| {
                recognition.writes_arguments_piecewise(id, generic_args)
                    && !recognition.writes_through_a_dictionary(id, generic_args)
            })
        }
        _ => false,
    })
}

/// Whether `local`, an `Arguments` std formats where it's made
/// (`ArgumentsUse::InPlace`), is given by value to a `Formatter`'s
/// `write_fmt`, the writer's own or one a struct holds, `held.0`: of a
/// writer shown into a writer, which writes through its sink, each piece a
/// call of that writer's (#1652, ADR 1135).
pub(in crate::lower) fn written_into_a_formatter<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    body: &mir::Body<'tcx>,
    local: Local,
) -> bool {
    let tcx = recognition.tcx;
    let in_place = arguments_use(recognition, body, local) == ArgumentsUse::InPlace;
    in_place
        && body.basic_blocks.iter().any(|data| match &data.terminator().kind {
            TerminatorKind::Call { func, args, .. } => {
                args.iter().any(|arg| {
                    arg.node
                        .place()
                        .is_some_and(|p| p.local == local && p.projection.is_empty())
                }) && crate::lower::fn_def(func.ty(&body.local_decls, tcx))
                    .is_some_and(|(id, generic_args)| recognition.formatter_write_fmt(id, generic_args))
            }
            _ => false,
        })
}

/// Whether each use of `local`, which holds an `Arguments` or what holds
/// one, `assert_eq!`'s `Some(args)`, is std's by value, or `as_str()`'s,
/// through as many such holders as `depth`.
fn formatted_by_std<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    body: &mir::Body<'tcx>,
    local: Local,
    depth: u8,
) -> bool {
    struct Uses {
        local: Local,
        uses: Vec<mir::Location>,
    }
    impl<'tcx> Visitor<'tcx> for Uses {
        fn visit_local(&mut self, local: Local, context: PlaceContext, location: mir::Location) {
            let idle = matches!(
                context,
                PlaceContext::NonUse(NonUseContext::StorageLive | NonUseContext::StorageDead)
                    | PlaceContext::MutatingUse(MutatingUseContext::Drop)
            ) || context.is_place_assignment()
                || !context.is_use();
            if local == self.local && !idle {
                self.uses.push(location);
            }
        }
    }
    let tcx = recognition.tcx;
    let holds_arguments = |ty: ty::Ty<'tcx>| {
        ty.walk().filter_map(|part| part.as_type()).any(
            |part| matches!(part.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::FormatArguments)),
        )
    };
    let mut uses = Uses {
        local,
        uses: Vec::new(),
    };
    uses.visit_body(body);
    let given_to_std = |location: mir::Location| {
        let data = &body.basic_blocks[location.block];
        if location.statement_index < data.statements.len() {
            let statement = &data.statements[location.statement_index];
            return match &statement.kind {
                // What borrowck alone reads.
                StatementKind::FakeRead(_) | StatementKind::PlaceMention(_) => true,
                StatementKind::Assign(assign) => match &assign.1 {
                    // Lent to `as_str()` alone.
                    Rvalue::Ref(_, _, place) if place.local == local && place.projection.is_empty() => assign
                        .0
                        .as_local()
                        .is_some_and(|lent| asked_as_str(recognition, body, lent)),
                    // Held by what std alone is given, `Some(args)`.
                    Rvalue::Aggregate(_, items) if depth > 0 => {
                        items.iter().all(|item| {
                            matches!(item, Operand::Move(place) | Operand::Copy(place)
                            if place.local != local || place.projection.is_empty())
                        }) && (assign.0.as_local())
                            .is_some_and(|holder| formatted_by_std(recognition, body, holder, depth - 1))
                    }
                    _ => false,
                },
                _ => false,
            };
        }
        match &data.terminator().kind {
            TerminatorKind::Call { func, args, .. } => {
                let decls = &body.local_decls;
                // Not a std trait's method an impl of the crate's defines,
                // `write_fmt`'s of a writer that writes it itself (#1695),
                // nor one a dictionary says, which may be (#1652, step 6).
                let std = crate::lower::fn_def(func.ty(decls, tcx)).is_some_and(|(id, generic_args)| {
                    crate::lower::recognition::is_std_crate(tcx, id)
                        && !resolves_to_own(recognition, id, generic_args)
                        && !recognition.writes_through_a_dictionary(id, generic_args)
                });
                std && args.iter().all(|arg| match &arg.node {
                    Operand::Copy(place) | Operand::Move(place) if place.local == local => {
                        place.projection.is_empty() && holds_arguments(arg.node.ty(decls, tcx))
                    }
                    _ => true,
                })
            }
            TerminatorKind::Drop { .. } => true,
            _ => false,
        }
    };
    uses.uses.into_iter().all(given_to_std)
}

/// Whether `lent`, a reference to an `Arguments`, is read only by its
/// `as_str()`.
fn asked_as_str<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &mir::Body<'tcx>, lent: Local) -> bool {
    let tcx = recognition.tcx;
    let mut reads = 0;
    let mut asked = 0;
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind
                && mentions(&assign.1, lent)
            {
                reads += 1;
            }
        }
        if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
            && args.iter().any(|a| a.node.place().is_some_and(|p| p.local == lent))
        {
            reads += 1;
            let known = crate::lower::fn_def(func.ty(&body.local_decls, tcx))
                .and_then(|(id, generic_args)| recognition.classify(id, generic_args));
            if known == Some(Std::FmtAsStr) {
                asked += 1;
            }
        }
    }
    reads > 0 && reads == asked
}

/// Whether `rvalue` reads `local`.
fn mentions(rvalue: &Rvalue<'_>, local: Local) -> bool {
    struct Found(Local, bool);
    impl<'tcx> Visitor<'tcx> for Found {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            self.1 |= local == self.0 && context.is_use();
        }
    }
    let mut found = Found(local, false);
    found.visit_rvalue(rvalue, mir::Location::START);
    found.1
}

/// Whether `local`, of `body`, holds an `Arguments` the body made that's
/// its text: made by `format_args!` (`Arguments::new` or `from_str`) and
/// formatted where it's made (`ArgumentsUse::InPlace`), or its writes
/// (`ArgumentsUse::Written`). Any other place
/// of an `Arguments`, a parameter's, a field's, or one given on, holds a
/// function that makes its text.
pub(in crate::lower) fn holds_text<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    body: &mir::Body<'tcx>,
    local: Local,
) -> bool {
    let tcx = recognition.tcx;
    let made = body.basic_blocks.iter().any(|data| {
        matches!(&data.terminator().kind, TerminatorKind::Call { func, destination, .. }
            if destination.as_local() == Some(local)
                && crate::lower::fn_def(func.ty(&body.local_decls, tcx))
                    .and_then(|(id, generic_args)| recognition.classify(id, generic_args))
                    .is_some_and(|known| matches!(known, Std::FmtNew | Std::FmtStr)))
    });
    made && arguments_use(recognition, body, local) != ArgumentsUse::GivenOn
}

/// Whether a call of `id`, a std trait's method, of `generic_args`, runs a
/// function rust-js compiled: the method of an impl of the crate's, or of a
/// library's, that defines it, not its trait's default. Of a type only a caller knows, its
/// dictionary's entry says (`impl_entries`).
pub(super) fn resolves_to_own<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    id: rustc_span::def_id::DefId,
    generic_args: ty::GenericArgsRef<'tcx>,
) -> bool {
    let tcx = recognition.tcx;
    let Some(trait_) = tcx.trait_of_assoc(id) else {
        return false;
    };
    let tr = ty::TraitRef::from_assoc(tcx, trait_, generic_args);
    !rustc_middle::ty::TypeVisitableExt::has_non_region_param(&tr)
        && recognition
            .user_impl(tr)
            .is_some_and(|imp| tcx.impl_item_implementor_ids(imp).contains_key(&id))
}
