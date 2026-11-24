//! A brand is a mark on the literal it makes (#2403, ADR 1196): a struct's
//! object or a tuple struct's array, `ObjectLayout::brand`, which every
//! pass that reads or remakes a literal reads past, as a guard's that's
//! elided (ADR 0865). Printed as the call that brands it, `$brand(key,
//! { .. })`, only here, once the module is prepared, before its helpers are
//! found of what it reads.

use super::{Expr, ExprKind, Module};

/// Each literal `module` marks with a brand, the call that brands it.
pub fn printed(module: &mut Module) {
    module.each_expr_mut(&mut |e| {
        let brand = match &mut e.kind {
            ExprKind::Object(_, layout) => layout.brand.take(),
            ExprKind::Array(_, layout) => layout.brand.take(),
            _ => None,
        };
        if let Some(key) = brand {
            let made = std::mem::replace(e, Expr::taken());
            *e = Expr::call(Expr::helper("$brand"), vec![Expr::str(key), made]);
        }
    });
}
