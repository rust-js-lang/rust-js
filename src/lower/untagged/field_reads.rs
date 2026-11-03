//! An untagged enum's method that reads the same field of each variant's
//! value (rd317): `match self { Parent::Container(container) =>
//! &container.children, Parent::Instance(instance) => &instance.children }`,
//! the test renderer's `parentInstance.children`, upstream's read of
//! `Instance | Container`. Its call is that read, where its JS is the read
//! of its parameter alone (`read_of_its_parameter`).

use rustc_middle::thir::{ExprKind, PatKind};
use rustc_middle::ty::{self, TyCtxt};

use crate::js::{self, Expr};
use crate::lower::Body;
use crate::lower::bindings;
use crate::lower::body_queries::strip;

/// Whether `body` is an inherent method of an untagged enum
/// of no type parameters, of the crate's, whose one parameter is its `self`,
/// by reference or by value, and whose body is a `match` of it, an arm of
/// each variant, each giving its one value's field, borrowed or read, of
/// the same JS key, each a struct's field: `&container.children` and
/// `&instance.children`. What it gives is the field of whichever variant
/// the value is, which JS reads of the value itself, as an untagged
/// enum's value is its variant's (ADR 0214).
pub(in crate::lower) fn reads_each_variants_field<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> bool {
    let (def_id, thir) = (body.def_id, &body.thir);
    let Some(imp) = tcx.inherent_impl_of_assoc(def_id) else {
        return false;
    };
    let ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
    let Some((adt, _)) = super::untagged_adt(tcx, ty) else {
        return false;
    };
    if tcx.generics_of(def_id).requires_monomorphization(tcx) || crate::lower::recognition::is_noinline(tcx, def_id) {
        return false;
    }
    let [param] = &thir.params.raw[..] else { return false };
    let Some(PatKind::Binding {
        var: this,
        subpattern: None,
        ..
    }) = param.pat.as_deref().map(|pat| &pat.kind)
    else {
        return false;
    };
    // The body's value, its block's of no statements.
    let Some(root) = body.root() else { return false };
    let mut value = strip(thir, root);
    if let ExprKind::Block { block } = thir[value].kind {
        let block = &thir[block];
        let (true, Some(expr)) = (block.stmts.is_empty(), block.expr) else {
            return false;
        };
        value = strip(thir, expr);
    }
    // A reborrow of what it gives, `&*match self { .. }`.
    if let ExprKind::Borrow { arg, .. } = thir[value].kind
        && let ExprKind::Deref { arg } = thir[strip(thir, arg)].kind
    {
        value = strip(thir, arg);
    }
    let ExprKind::Match {
        scrutinee, ref arms, ..
    } = thir[value].kind
    else {
        return false;
    };
    let mut scrutinee = strip(thir, scrutinee);
    if let ExprKind::Deref { arg } = thir[scrutinee].kind {
        scrutinee = strip(thir, arg);
    }
    if !matches!(thir[scrutinee].kind, ExprKind::VarRef { id } if id == *this) || arms.len() != adt.variants().len() {
        return false;
    }
    let mut keys: Option<Vec<bindings::FieldKey>> = None;
    let mut seen = Vec::new();
    for &arm in arms.iter() {
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
        let PatKind::Binding {
            var: payload,
            subpattern: None,
            ..
        } = field.pattern.kind
        else {
            return false;
        };
        if arm.guard.is_some() || adt.variant(variant_index).fields.len() != 1 || seen.contains(&variant_index) {
            return false;
        }
        seen.push(variant_index);
        // `&payload.f`, or `payload.f` read, through the derefs of its
        // reference: each field's key, outermost last.
        let mut at = strip(thir, arm.body);
        if let ExprKind::Borrow {
            borrow_kind: rustc_middle::mir::BorrowKind::Shared,
            arg,
        } = thir[at].kind
        {
            at = strip(thir, arg);
        }
        let mut path = Vec::new();
        loop {
            match thir[at].kind {
                ExprKind::Field {
                    lhs,
                    variant_index,
                    name,
                } => {
                    let ty::Adt(of, _) = thir[lhs].ty.kind() else {
                        return false;
                    };
                    if !of.is_struct() {
                        return false;
                    }
                    let field = &of.variant(variant_index).fields[name];
                    path.push(bindings::field_key(tcx, field));
                    at = strip(thir, lhs);
                }
                // A deref, or a reborrow under one, `&*&payload.f`.
                ExprKind::Deref { arg }
                | ExprKind::Borrow {
                    borrow_kind: rustc_middle::mir::BorrowKind::Shared,
                    arg,
                } => at = strip(thir, arg),
                ExprKind::VarRef { id } if id == payload && !path.is_empty() => break,
                _ => return false,
            }
        }
        match &keys {
            Some(keys) if *keys != path => return false,
            Some(_) => {}
            None => keys = Some(path),
        }
    }
    true
}

/// Of a function's JS, `function children(parent) { return
/// parent.children; }`, its parameter and the read of it it gives: a path
/// of members or literal keys from that parameter alone, which JS reads
/// the same where it's called of the value given, once (rd317). Made anew,
/// of what's known of the function's own reads nothing, so its caller's
/// lowering takes it as any read of its value.
pub(in crate::lower) fn read_of_its_parameter(function: &js::Function) -> Option<(String, Expr)> {
    let [js::Pattern::Name(param)] = &function.params[..] else {
        return None;
    };
    let [
        js::Stmt {
            kind: js::StmtKind::Return(Some(read)),
            ..
        },
    ] = &function.body[..]
    else {
        return None;
    };
    fn path(at: &Expr, param: &str) -> Option<Expr> {
        match &at.kind {
            js::ExprKind::Member(of, key) => Some(Expr::member(path(of, param)?, key.clone())),
            js::ExprKind::Index(of, key) if matches!(key.kind, js::ExprKind::Str(_) | js::ExprKind::Num(_)) => {
                Some(Expr::index(path(of, param)?, (**key).clone()))
            }
            js::ExprKind::Var(name) if name == param => Some(Expr::var(name)),
            _ => None,
        }
    }
    match read.kind {
        js::ExprKind::Var(_) => None,
        _ => Some((param.clone(), path(read, param)?)),
    }
}
