//! `js::Is<T>`, a type predicate to TypeScript (#1828, ADR 1155): a
//! function that gives one is declared `isISODate(value: unknown): value
//! is ISODate`, so which parameter it guards is proved here, of its MIR:
//! the one each `js::Is::when` of its body is given, as it was passed,
//! a parameter TypeScript types `unknown`, whose false branch narrows
//! nothing. The one owner lowering refuses by and declarations name by.

use rustc_middle::mir::{Body, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;

use crate::lower::recognition::{is_js_is, trusted_js_predicate};

/// Whether `ty` is a `js::Is<T>`, and its `T`.
pub(in crate::lower) fn predicate_of<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if is_js_is(tcx, adt.did()) => Some(args.type_at(0)),
        _ => None,
    }
}

/// Why a `js::Is::when` isn't one TypeScript can say, said where it's
/// written: what to write instead.
const REFUSED: &str = "rust-js: `js::Is::when` guards a parameter of the function that gives it, as it was passed, \
    one TypeScript types `unknown`, `value: Option<&js::Unknown>`: TypeScript narrows a predicate's parameter both ways, \
    and of another type its false branch would say what isn't so. Of any other value, return a `bool`, declared `boolean`; \
    a predicate that narrows both ways is #1828's to reopen";

const TRUSTED_REFUSED: &str = "rust-js: `js::Is::assert` guards an unchanged parameter of the function that gives it; of another value, return a `bool`, declared `boolean`";

/// The parameter, by its index among `body`'s, each `js::Is::when` of it
/// guards; `None` where it calls none; `Err` where one guards no parameter
/// of the function's own, declared `unknown` and never changed, or two
/// guard two, with where.
pub(in crate::lower) fn guarded<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
) -> Result<Option<usize>, (Span, &'static str)> {
    let mut guarded = None;
    for data in body.basic_blocks.iter() {
        let TerminatorKind::Call {
            func, args, fn_span, ..
        } = &data.terminator().kind
        else {
            continue;
        };
        let Some((did, _)) = func.const_fn_def() else {
            continue;
        };
        let Some(trusted) = trusted_js_predicate(tcx, did) else {
            continue;
        };
        let refused = if trusted { TRUSTED_REFUSED } else { REFUSED };
        let param = args.first().and_then(|arg| passed(body, &arg.node)).filter(|&local| {
            body.local_decls[local].mutability == rustc_ast::Mutability::Not
                && (trusted || any_value(tcx, body.local_decls[local].ty))
        });
        let Some(param) = param else {
            return Err((*fn_span, refused));
        };
        let index = param.as_usize() - 1;
        if guarded.is_some_and(|given| given != index) {
            return Err((*fn_span, refused));
        }
        guarded = Some(index);
    }
    // One it gives is made here, of its parameter, or TypeScript couldn't
    // say of which.
    if guarded.is_none() && predicate_of(tcx, body.return_ty()).is_some() {
        return Err((body.span, MADE_HERE));
    }
    Ok(guarded)
}

/// Why a function that gives a `js::Is` it doesn't make is refused.
const MADE_HERE: &str = "rust-js: a function that gives a `js::Is` makes it here, `js::Is::when(value, ..)` of its own \
    parameter, which its declaration names, `value is T`: return that, or a `bool`, declared `boolean`";

/// The parameter `operand` is, as it was passed: the parameter itself, or
/// a temporary MIR assigned it once from one.
fn passed(body: &Body<'_>, operand: &Operand<'_>) -> Option<Local> {
    let place = operand.place()?;
    if !place.projection.is_empty() {
        return None;
    }
    let local = place.local;
    if (1..=body.arg_count).contains(&local.as_usize()) {
        return Some(local);
    }
    let mut from = None;
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (to, value) = &**assign;
            if to.local != local {
                continue;
            }
            let Rvalue::Use(Operand::Copy(source) | Operand::Move(source), _) = value else {
                return None;
            };
            if from.is_some()
                || !source.projection.is_empty()
                || !(1..=body.arg_count).contains(&source.local.as_usize())
            {
                return None;
            }
            from = Some(source.local);
        }
    }
    from
}

/// Whether `ty` is a value TypeScript types `unknown` or `any`, whose
/// false branch narrows nothing (`declarations::any_shape`): a
/// `js::Unknown`, or an `Option` of a reference to one, whose `None` is
/// `undefined`.
fn any_value<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = match ty.kind() {
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) => {
            args.type_at(0)
        }
        _ => ty,
    };
    matches!(ty.kind(), ty::Ref(_, pointee, _)
        if matches!(pointee.kind(), ty::Adt(adt, _) if crate::lower::declarations::any_shape(tcx, adt.did())))
}
