//! What `Debug` takes, crate-wide: whether anything shows a value pretty,
//! `{:#?}`, and which derives are shown (ADRs 0060, 0137).

use crate::collections::{IndexMap, IndexSet};
use crate::lower::format_args::{Piece, decode_template};
use crate::lower::recognition::{
    FormatterQuery, formatter_query, is_arguments_new, is_debug_argument, is_formatter_pad,
};
use crate::lower::{Body, fn_def, strip};
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Thir};
use rustc_middle::ty;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// A `format_args!`'s `Arguments::new` call, its placeholders, and the
/// argument at each, `Argument::new_display(&x)` and the like, where they
/// can be told: `None` for one whose arguments can't.
pub(in crate::lower) type FormatCall = (ExprId, Vec<Piece>, Option<Vec<ExprId>>);

/// Each `format_args!` in `thir`, its `FormatCall`.
pub(in crate::lower) fn format_calls(tcx: TyCtxt<'_>, thir: &Thir<'_>) -> Vec<FormatCall> {
    let through = |mut e: ExprId| loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
            _ => break e,
        }
    };
    // The `let`s a `format_args!` keeps its arguments' array in.
    let lets: IndexMap<LocalVarId, ExprId> = thir
        .stmts
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            rustc_middle::thir::StmtKind::Let {
                pattern,
                initializer: Some(init),
                ..
            } => match pattern.kind {
                rustc_middle::thir::PatKind::Binding { var, .. } => Some((var, *init)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    thir.exprs
        .iter_enumerated()
        .filter_map(|(call, expr)| {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                return None;
            };
            let (id, _) = fn_def(thir[fun].ty)?;
            if !is_arguments_new(tcx, id) || args.len() != 2 {
                return None;
            }
            let ExprKind::Literal { lit, .. } = thir[through(args[0])].kind else {
                return None;
            };
            let rustc_ast::LitKind::ByteStr(ref bytes, _) = lit.node else {
                return None;
            };
            let pieces = decode_template(bytes.as_byte_str())?;
            let array = match thir[through(args[1])].kind {
                ExprKind::VarRef { id } => lets.get(&id).map(|&init| through(init)),
                _ => Some(through(args[1])),
            };
            let fields = match array.map(|a| &thir[a].kind) {
                Some(ExprKind::Array { fields }) => Some(fields.iter().map(|&field| through(field)).collect()),
                _ => None,
            };
            Some((call, pieces, fields))
        })
        .collect()
}

/// Does any body show a value with `{:#?}`, a `Debug` argument's alternate
/// placeholder, or call `Formatter::alternate` (ADR 0137)? Another
/// alternate placeholder, `{:#}` or `{:#x}`, is no reason; one whose
/// argument can't be told is taken to be.
pub(super) fn uses_pretty_debug(tcx: TyCtxt<'_>, all_bodies: &[&Body<'_>]) -> bool {
    all_bodies.iter().any(|body| {
        let thir = &body.thir;
        let asks = thir.exprs.iter().any(|expr| {
            matches!(expr.kind, ExprKind::Call { fun, .. }
                if matches!(*thir[fun].ty.kind(), ty::FnDef(id, _)
                    if formatter_query(tcx, id) == Some(FormatterQuery::Alternate)))
        });
        asks || format_calls(tcx, thir).into_iter().any(|(_, pieces, fields)| {
            let alternates: Vec<usize> = pieces
                .iter()
                .filter_map(|piece| match piece {
                    Piece::Argument(index, spec) if spec.alternate => Some(*index),
                    _ => None,
                })
                .collect();
            if alternates.is_empty() {
                return false;
            }
            let Some(fields) = fields else { return true };
            alternates.iter().any(|&index| {
                let Some(&field) = fields.get(index) else { return true };
                match thir[field].kind {
                    ExprKind::Call { fun, .. } => match thir[fun].ty.kind() {
                        &ty::FnDef(made_by, _) => is_debug_argument(tcx, made_by),
                        _ => true,
                    },
                    _ => true,
                }
            })
        })
    })
}

/// Whether the crate asks `{:x?}` or `{:X?}` of anything (ADR 0559): then
/// what its writers show of an integer is in hex where their options say.
pub(super) fn uses_debug_hex(tcx: TyCtxt<'_>, all_bodies: &[&Body<'_>]) -> bool {
    all_bodies.iter().any(|body| {
        format_calls(tcx, &body.thir).into_iter().any(|(_, pieces, _)| {
            pieces
                .iter()
                .any(|piece| matches!(piece, Piece::Argument(_, spec) if spec.debug_hex.is_some()))
        })
    })
}

/// Does any body give a placeholder's options, a width, a precision, a sign
/// or zeros, to a value that isn't a number, a `bool`, a `char`, a string
/// or `()`: one of the crate's own, a generic `T`, a `dyn` (ADR 0058)? Or
/// ask a `Formatter` for them, `f.width()` or `f.pad(s)` (ADR 0143)? Then
/// its writers take them, and its dictionaries apply them. One whose
/// argument can't be told is taken to be.
pub(super) fn uses_format_options(tcx: TyCtxt<'_>, all_bodies: &[&Body<'_>]) -> bool {
    let asks = |id: DefId| {
        formatter_query(tcx, id).is_some_and(|query| query != FormatterQuery::Alternate) || is_formatter_pad(tcx, id)
    };
    if all_bodies.iter().any(|body| {
        body.thir.exprs.iter().any(|expr| {
            matches!(expr.kind, ExprKind::Call { fun, .. } if matches!(*body.thir[fun].ty.kind(), ty::FnDef(id, _) if asks(id)))
        })
    }) {
        return true;
    }
    let primitive = |ty: ty::Ty<'_>| {
        ty.is_numeric()
            || ty.is_bool()
            || ty.is_char()
            || ty.is_str()
            || ty.is_unit()
            || matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::String))
    };
    all_bodies.iter().any(|body| {
        let thir = &body.thir;
        format_calls(tcx, thir).into_iter().any(|(_, pieces, fields)| {
            let given: Vec<usize> = pieces
                .iter()
                .filter_map(|piece| match piece {
                    Piece::Argument(index, spec)
                        if spec.width.is_some()
                            || spec.precision.is_some()
                            || spec.width_from.is_some()
                            || spec.precision_from.is_some()
                            || spec.plus
                            || spec.zero
                            || spec.debug_hex.is_some() =>
                    {
                        Some(*index)
                    }
                    _ => None,
                })
                .collect();
            if given.is_empty() {
                return false;
            }
            let Some(fields) = fields else { return true };
            given.iter().any(|&index| {
                // `Argument::new_display(&x)`: what `x` is.
                let Some(&field) = fields.get(index) else { return true };
                let ExprKind::Call { ref args, .. } = thir[field].kind else {
                    return true;
                };
                args.first().is_none_or(|&arg| !primitive(thir[arg].ty.peel_refs()))
            })
        })
    })
}

/// Whether `body` reads the options of a `Formatter` it's given or
/// captures, where a caller's `{:>4}`, `{:#?}` or `{:x?}` changes what it
/// writes: anything done with one but its text written, `write!(f, ..)`,
/// `f.write_str(s)` or `f.write_char(c)`, which a placeholder's options
/// leave alone. `f.width()`, `f.pad(s)`, a debug builder, and `f` given on,
/// `self.0.fmt(f)` or to a closure, read them (#1026).
pub(super) fn reads_formatter_options(tcx: TyCtxt<'_>, body: &Body<'_>) -> bool {
    let thir = &body.thir;
    let formatter = |ty: ty::Ty<'_>| {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _)
            if crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::Formatter))
    };
    let named = |e: ExprId| {
        let mut e = e;
        loop {
            e = strip(thir, e);
            match thir[e].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
                ExprKind::VarRef { .. } | ExprKind::UpvarRef { .. } => break Some(e),
                _ => break None,
            }
        }
    };
    // Each `f` its text is written through, the first argument of a write.
    let writes: Vec<ExprId> = (thir.exprs.iter())
        .filter_map(|expr| match &expr.kind {
            ExprKind::Call { fun, args, .. }
                if fn_def(thir[*fun].ty).is_some_and(|(id, _)| {
                    matches!(tcx.item_name(id).as_str(), "write_fmt" | "write_str" | "write_char")
                }) =>
            {
                args.first().and_then(|&f| named(f))
            }
            _ => None,
        })
        .collect();
    (thir.exprs.iter_enumerated()).any(|(id, expr)| {
        matches!(expr.kind, ExprKind::VarRef { .. } | ExprKind::UpvarRef { .. })
            && formatter(expr.ty)
            && !writes.contains(&id)
    })
}

/// What a crate's writers take beyond their arguments, its own answer of
/// what its code formats, and which of its functions read a `Formatter`'s
/// options, which a library's manifest says (ADRs 0058, 0137, 0559).
pub(in crate::lower) struct WriterFacts {
    /// Whether it shows anything with `{:#?}`, or asks a `Formatter` if
    /// it's alternate: then its `Debug` functions take whether (ADR 0137).
    /// `@rust-js/std`'s do: its consumers are every program.
    pub(in crate::lower) pretty_debug: bool,
    /// Whether it gives a placeholder's options to a value it doesn't apply
    /// them to itself: then its writers take them (ADR 0058).
    pub(in crate::lower) format_options: bool,
    /// Whether it asks `{:x?}` of anything (ADR 0559).
    pub(in crate::lower) debug_hex: bool,
    /// Its functions, closures' by their own, that read a `Formatter`'s
    /// options (`reads_formatter_options`, #1026).
    pub(in crate::lower) reads_options: IndexSet<DefId>,
}

impl WriterFacts {
    pub(super) fn of<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>], std_bodies: bool) -> Self {
        WriterFacts {
            pretty_debug: std_bodies || uses_pretty_debug(tcx, all_bodies),
            format_options: uses_format_options(tcx, all_bodies),
            debug_hex: uses_debug_hex(tcx, all_bodies),
            reads_options: (all_bodies.iter())
                .filter(|body| reads_formatter_options(tcx, body))
                .map(|body| tcx.typeck_root_def_id(body.def_id))
                .collect(),
        }
    }

    /// What its writers take, which a library's writer it reaches is
    /// compiled here to take where its library's doesn't (#1026).
    pub(in crate::lower) fn writes(&self) -> Writes {
        Writes {
            options: self.pretty_debug || self.format_options,
            hex: self.debug_hex,
        }
    }
}

/// What a crate's writers take beyond their arguments: their `Formatter`'s
/// options, whether to be pretty or a placeholder's (ADRs 0058, 0137); and
/// whether an integer's `{:?}` may be hex (ADR 0559). A crate's own answer,
/// `WriterFacts::writes`; a library's in its manifest.
#[derive(Clone, Copy, Default)]
pub(in crate::lower) struct Writes {
    pub(in crate::lower) options: bool,
    pub(in crate::lower) hex: bool,
}
