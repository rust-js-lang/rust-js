//! An untagged enum whose `Display` writes each variant's one value with
//! `{}`, a string, an integer, a `bool` or a `char`: webpack's module id of
//! `string | number` (#557). JS's template makes Rust's text of each, so a
//! plain `{}` of the enum is the value in JS's template, `${id}`, as an
//! untagged enum's value is its variant's (ADR 0214).

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};

use super::untagged_adt;
use crate::lower::analysis::{FormatCall, format_calls};
use crate::lower::body_queries::strip;
use crate::lower::format_args::Piece;
use crate::lower::format_spec::Spec;
use crate::lower::recognition::{WriteCall, fmt_trait_called};
use crate::lower::{Body, FnCx, fn_def};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `ty` is an untagged enum of the crate's, of no type
    /// parameters, whose `Display` writes each variant's one value as it
    /// is: what JS's template makes of the value, where no options are
    /// given (`writes_each_value`).
    pub(in crate::lower) fn displays_its_value(&self, ty: Ty<'tcx>) -> bool {
        let Some((adt, _)) = untagged_adt(self.tcx, ty) else {
            return false;
        };
        if self.tcx.generics_of(adt.did()).requires_monomorphization(self.tcx) {
            return false;
        }
        let Some(fmt) = self.recognition().display_fmt() else {
            return false;
        };
        let args = self.tcx.mk_args(&[self.tcx.erase_and_anonymize_regions(ty).into()]);
        let Ok(Some(instance)) = self.resolve_instance(fmt, args) else {
            return false;
        };
        (self.krate.bodies.get(&instance.def_id())).is_some_and(|body| self.writes_each_value(body))
    }

    /// Whether `body`, an untagged enum's `Display::fmt(&self, f)`, is a
    /// `match self` of an arm of each variant, binding its one value, each
    /// writing it to `f` as it is: `write!(f, "{x}")`, `x.fmt(f)`,
    /// `f.write_str(x)` or `f.write_char(x)`, of a value whose JS text is
    /// Rust's, a string's, an integer's, a `bool`'s or a `char`'s. A
    /// `write!` gives its value no options and `x.fmt(f)` the `Formatter`'s,
    /// so only where none are given do they write the same.
    fn writes_each_value(&self, body: &Body<'tcx>) -> bool {
        let thir = &body.thir;
        let [this, formatter] = &thir.params.raw[..] else {
            return false;
        };
        let (Some(this), Some(formatter)) = (bound(this.pat.as_deref()), bound(formatter.pat.as_deref())) else {
            return false;
        };
        let Some(root) = body.root() else { return false };
        let mut value = strip(thir, root);
        if let ExprKind::Block { block } = thir[value].kind {
            let block = &thir[block];
            let (true, Some(expr)) = (block.stmts.is_empty(), block.expr) else {
                return false;
            };
            value = strip(thir, expr);
        }
        let ExprKind::Match {
            scrutinee, ref arms, ..
        } = thir[value].kind
        else {
            return false;
        };
        let ty::Adt(adt, _) = thir[scrutinee].ty.peel_refs().kind() else {
            return false;
        };
        if !is_var(self.tcx, thir, scrutinee, this) || arms.len() != adt.variants().len() {
            return false;
        }
        let calls = format_calls(self.tcx, thir);
        let mut seen = Vec::new();
        arms.iter().all(|&arm| {
            let arm = &thir[arm];
            let mut pattern = &arm.pattern;
            while let PatKind::Deref { subpattern, .. } = &pattern.kind {
                pattern = subpattern;
            }
            let PatKind::Variant {
                variant_index,
                ref subpatterns,
                ..
            } = pattern.kind
            else {
                return false;
            };
            let [field] = &subpatterns[..] else { return false };
            let Some(payload) = bound(Some(&field.pattern)) else {
                return false;
            };
            if arm.guard.is_some() || adt.variant(variant_index).fields.len() != 1 || seen.contains(&variant_index) {
                return false;
            }
            seen.push(variant_index);
            let shown = self.recognition().untagged_payload(field.pattern.ty);
            if !(self.is_string_like(shown) || shown.is_integral() || shown.is_bool()) {
                return false;
            }
            let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, arm.body)].kind else {
                return false;
            };
            let Some((called, _)) = fn_def(thir[fun].ty) else {
                return false;
            };
            match (self.recognition().write_call(called, false), &args[..]) {
                // `x.fmt(f)`, `Display::fmt(x, f)`.
                (WriteCall::Display, &[value, to]) => {
                    is_var(self.tcx, thir, value, payload) && is_var(self.tcx, thir, to, formatter)
                }
                // `write!(f, "{x}")`, of one plain placeholder of `x`'s
                // `Display`; `f.write_str(x)` or `f.write_char(x)`.
                (WriteCall::Text, &[to, written]) => {
                    is_var(self.tcx, thir, to, formatter)
                        && (is_var(self.tcx, thir, written, payload)
                            || self.writes_plainly(thir, &calls, arguments_made(thir, written), payload))
                }
                _ => false,
            }
        })
    }

    /// Whether `call` is a `format_args!`'s `Arguments::new` of one plain
    /// placeholder, `{}`, of `payload`'s `Display`.
    fn writes_plainly(&self, thir: &Thir<'tcx>, calls: &[FormatCall], call: ExprId, payload: LocalVarId) -> bool {
        let Some((_, pieces, Some(fields))) = calls.iter().find(|(at, ..)| *at == call) else {
            return false;
        };
        let pieces: Vec<&Piece> = (pieces.iter())
            .filter(|piece| !matches!(piece, Piece::Text(text) if text.is_empty()))
            .collect();
        let ([Piece::Argument(0, spec)], [argument]) = (&pieces[..], &fields[..]) else {
            return false;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[*argument].kind else {
            return false;
        };
        *spec == Spec::plain()
            && fn_def(thir[fun].ty)
                .is_some_and(|(made_by, _)| fmt_trait_called(self.tcx, made_by) == Some(self.display_trait()))
            && matches!(&args[..], &[value] if is_var(self.tcx, thir, captured(thir, value).unwrap_or(value), payload))
    }
}

/// The variable a parameter's or a field's pattern binds, as it is.
fn bound(pattern: Option<&rustc_middle::thir::Pat<'_>>) -> Option<LocalVarId> {
    match pattern?.kind {
        PatKind::Binding {
            var, subpattern: None, ..
        } => Some(var),
        _ => None,
    }
}

/// Whether `e` is `var`, through borrows and derefs of it, `&**x` or
/// `&mut *f`, a `Deref` call too: of a value `writes_each_value` takes,
/// std's, a `String`'s to its `str` or an `Rc<str>`'s, each the same value
/// in JS.
fn is_var<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, mut e: ExprId, var: LocalVarId) -> bool {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow {
                borrow_kind: BorrowKind::Shared | BorrowKind::Mut { .. },
                arg,
            }
            | ExprKind::Deref { arg } => e = arg,
            ExprKind::Call { fun, ref args, .. }
                if let [arg] = args[..]
                    && let Some((called, _)) = fn_def(thir[fun].ty)
                    && tcx
                        .trait_of_assoc(called)
                        .is_some_and(|t| tcx.is_lang_item(t, LangItem::Deref)) =>
            {
                e = arg;
            }
            ExprKind::VarRef { id } => return id == var,
            _ => return false,
        }
    }
}

/// What `format_args!` keeps a placeholder's argument in, `&*args.0` of
/// its `let args = (&x,)`: `&x`.
fn captured(thir: &Thir<'_>, mut e: ExprId) -> Option<ExprId> {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
            ExprKind::Field { lhs, name, .. } => {
                let ExprKind::VarRef { id } = thir[strip(thir, lhs)].kind else {
                    return None;
                };
                let init = thir.stmts.iter().find_map(|stmt| match &stmt.kind {
                    StmtKind::Let {
                        pattern,
                        initializer: Some(init),
                        ..
                    } if matches!(pattern.kind, PatKind::Binding { var, .. } if var == id) => Some(*init),
                    _ => None,
                })?;
                let ExprKind::Tuple { ref fields } = thir[strip(thir, init)].kind else {
                    return None;
                };
                return fields.get(name.as_usize()).copied();
            }
            _ => return None,
        }
    }
}

/// The `Arguments::new` call that the `format_args!` `e` is, past the
/// block its arguments are kept in.
fn arguments_made(thir: &Thir<'_>, mut e: ExprId) -> ExprId {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Block { block } if let Some(expr) = thir[block].expr => e = expr,
            _ => return e,
        }
    }
}
