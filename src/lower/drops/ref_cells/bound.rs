//! What a body's patterns bind of its `RefCell` fields: a field's own
//! place by reference, whose uses are the field's, or a cell of its own
//! moved out (ADRs 0362, 0863).

use super::*;

/// What a body's patterns bind of its `RefCell` fields (`bound_ref_cells`).
struct BoundRefCells<'a, 'b, 'tcx> {
    recognition: &'b crate::lower::recognition::Recognition<'a, 'tcx>,
    thir: &'b rustc_middle::thir::Thir<'tcx>,
    places: BoundPlaces,
    found: Vec<(DefId, rustc_span::Span)>,
}

/// Each binding of a body that's a field's own place, of the field it is.
pub(super) type BoundPlaces = IndexMap<rustc_middle::thir::LocalVarId, DefId>;

/// Of a body, each `RefCell` field of a struct or of an enum's variant its
/// patterns bind, not as `_` (ADR 0362): one bound by reference, `cleanup`
/// of `let Kind::Effect { cleanup, .. } = &e.kind`, is that field's own
/// place, by its binding, whose uses are the field's (ADR 0863, amended,
/// #503); one bound by value, moved out, a cell of its own, lent there.
pub(super) fn bound_ref_cells<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    thir: &rustc_middle::thir::Thir<'tcx>,
) -> (BoundPlaces, Vec<(DefId, rustc_span::Span)>) {
    let mut bound = BoundRefCells {
        recognition,
        thir,
        places: IndexMap::default(),
        found: Vec::new(),
    };
    for expr in thir.exprs.iter() {
        if let ExprKind::Let { ref pat, .. } = expr.kind {
            rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
        }
    }
    for stmt in thir.stmts.iter() {
        if let ThirStmt::Let { ref pattern, .. } = stmt.kind {
            rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pattern);
        }
    }
    for arm in thir.arms.iter() {
        rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, &arm.pattern);
    }
    for param in thir.params.iter() {
        if let Some(pat) = &param.pat {
            rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
        }
    }
    (bound.places, bound.found)
}

/// The read of a binding that's a field's own place `expr` is, `cleanup`
/// or `*cleanup`, and the field (`bound_ref_cells`).
pub(super) fn bound_place(
    thir: &rustc_middle::thir::Thir<'_>,
    places: &BoundPlaces,
    expr: rustc_middle::thir::ExprId,
) -> Option<(rustc_middle::thir::ExprId, DefId)> {
    use crate::lower::body_queries::strip;
    let expr = strip(thir, expr);
    let read = match thir[expr].kind {
        ExprKind::Deref { arg } => strip(thir, arg),
        _ => expr,
    };
    match thir[read].kind {
        ExprKind::VarRef { id } => places.get(&id).map(|&field| (read, field)),
        _ => None,
    }
}

impl<'b, 'tcx> rustc_middle::thir::visit::Visitor<'b, 'tcx> for BoundRefCells<'_, 'b, 'tcx> {
    fn thir(&self) -> &'b rustc_middle::thir::Thir<'tcx> {
        self.thir
    }

    fn visit_pat(&mut self, pat: &'b Pat<'tcx>) {
        use rustc_middle::thir::PatKind;
        // A struct's, or a variant's (#503).
        let variant = match pat.kind {
            PatKind::Leaf { ref subpatterns }
                if let ty::Adt(adt, _) = pat.ty.kind()
                    && adt.is_struct() =>
            {
                Some((adt.non_enum_variant(), subpatterns))
            }
            PatKind::Variant {
                adt_def,
                variant_index,
                ref subpatterns,
                ..
            } => Some((adt_def.variant(variant_index), subpatterns)),
            _ => None,
        };
        if let Some((variant, subpatterns)) = variant {
            for field in subpatterns {
                if matches!(field.pattern.kind, PatKind::Wild)
                    || !self.recognition.is_std_type(field.pattern.ty, StdItem::RefCell)
                {
                    continue;
                }
                let did = variant.fields[field.field].did;
                match field.pattern.kind {
                    // By reference, the field's own place (ADR 0863, amended).
                    PatKind::Binding {
                        mode: rustc_hir::BindingMode(rustc_hir::ByRef::Yes(..), _),
                        var,
                        subpattern: None,
                        ..
                    } if object_field(self.recognition.tcx, did) => _ = self.places.insert(var, did),
                    _ => self.found.push((did, field.pattern.span)),
                }
            }
        }
        rustc_middle::thir::visit::walk_pat(self, pat);
    }
}
