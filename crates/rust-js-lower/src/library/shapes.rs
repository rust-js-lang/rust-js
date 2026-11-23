//! What a library's contract calls a type, its shape: its constructor's
//! key and its arguments', `_` of what may be any (ADR 0852), and whether
//! a type may be one a shape names, an `Rc`'s or a `RefCell`'s (ADR 0863).

use rust_js_program::library::RcShape;
use rustc_middle::ty::TyCtxt;

/// What a library's contract calls each of `tys`, sorted, each once (ADR
/// 0863): as it calls a type an `Rc` it counts points at (ADR 0852).
pub(crate) fn shapes<'tcx>(
    tcx: TyCtxt<'tcx>,
    tys: impl IntoIterator<Item = rustc_middle::ty::Ty<'tcx>>,
) -> Vec<RcShape> {
    let mut shapes: Vec<RcShape> = tys.into_iter().map(|ty| rc_shape(tcx, ty)).collect();
    shapes.sort();
    shapes.dedup();
    shapes
}

/// May `ty`, a `RefCell`'s type, be one `shape` names, a library's, as
/// `analysis::may_be_cell` says of two (ADR 0863): a type parameter of
/// either any type, a slice's what it's made of, and a `dyn`'s any type
/// where a value may be made one in place (ADR 0960).
pub(crate) fn may_be_cell_shape<'tcx>(tcx: TyCtxt<'tcx>, shape: &RcShape, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
    may_be_cell_shape_at(tcx, (shape, ty), true)
}

/// `may_be_cell_shape` of `shape` and `ty`, where `unsized_here`, a `dyn`
/// there may be any type made one in place.
fn may_be_cell_shape_at<'tcx>(
    tcx: TyCtxt<'tcx>,
    (shape, ty): (&RcShape, rustc_middle::ty::Ty<'tcx>),
    unsized_here: bool,
) -> bool {
    use rustc_middle::ty;
    let dyn_shape = shape.of.as_deref().is_some_and(|of| of.starts_with("dyn "));
    match ty.kind() {
        ty::Dynamic(..) if unsized_here => true,
        _ if dyn_shape && unsized_here => true,
        ty::Slice(item) | ty::Array(item, _) => {
            shape.of.as_deref().is_none_or(|of| matches!(of, "[]" | "[;]"))
                && shape
                    .args
                    .first()
                    .is_none_or(|arg| may_be_cell_shape_at(tcx, (arg, *item), false))
        }
        _ => match (&shape.of, crate::analysis::head(tcx, ty)) {
            (Some(of), Some((head, args))) => {
                let tail = crate::analysis::unsized_in_place(tcx, ty);
                head_key(tcx, &head) == *of
                    && shape.args.len() == args.len()
                    && (shape.args.iter().zip(args).enumerate())
                        .all(|(i, (shape, arg))| may_be_cell_shape_at(tcx, (shape, arg), tail == Some(i)))
            }
            _ => true,
        },
    }
}

/// What a library's contract calls `head`, a type's constructor (ADR 0852).
pub(super) fn head_key(tcx: TyCtxt<'_>, head: &crate::analysis::Head) -> String {
    match head {
        crate::analysis::Head::Def(kind, id) => format!("{kind}{}", super::key(tcx, *id)),
        crate::analysis::Head::Named(name) => name.clone(),
    }
}

/// What a library's contract calls `ty`, a type an `Rc` it counts points
/// at, `_` where it may be any (ADR 0852).
pub(crate) fn rc_shape<'tcx>(tcx: TyCtxt<'tcx>, ty: rustc_middle::ty::Ty<'tcx>) -> RcShape {
    match crate::analysis::head(tcx, ty) {
        Some((head, args)) => RcShape {
            of: Some(head_key(tcx, &head)),
            args: args.into_iter().map(|arg| rc_shape(tcx, arg)).collect(),
        },
        None => RcShape {
            of: None,
            args: Vec::new(),
        },
    }
}

/// May `ty` be the type `shape` names, a library's, a type parameter of
/// either any type (ADR 0852)? As `rc_counts::may_be` says of two types.
pub(crate) fn may_be_shape<'tcx>(tcx: TyCtxt<'tcx>, shape: &RcShape, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
    match (&shape.of, crate::analysis::head(tcx, ty)) {
        (Some(of), Some((head, args))) => {
            *of == head_key(tcx, &head)
                && shape.args.len() == args.len()
                && shape
                    .args
                    .iter()
                    .zip(args)
                    .all(|(shape, arg)| may_be_shape(tcx, shape, arg))
        }
        _ => true,
    }
}
