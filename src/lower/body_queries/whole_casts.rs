//! Which casts of an `f64` to an integer are the number as it is, where
//! it's known there to be a whole number in the integer's range (ADR 0302,
//! amended): of a variable given whole numbers, or of what a
//! `#[rust_js::whole]` binding gives.

use super::*;

/// What a comparison of a variable with a constant shows: `x < 3`.
#[derive(Clone, Copy)]
struct Compared {
    var: LocalVarId,
    op: BinOp,
    with: f64,
}

/// The casts to an integer of an `f64` variable known there to be a whole
/// number in the integer's range, each the variable as it is (ADR 0302).
/// What it may hold is what each `let` and `=` gives it: a whole number, an
/// integer `as f64`, or what a `#[rust_js::whole]` binding gives, a
/// WebIDL integer or `indexOf`'s position or -1; narrowed where a test of
/// it holds, if nothing sets it after the test. And a cast of what such a
/// binding gives, `options.length() as usize`.
pub(super) fn whole_casts<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<ExprId> {
    let at = |span: Span| span.source_callsite();
    let pointer_bits = tcx.data_layout.pointer_size().bits();
    // The whole numbers an integer type holds, of 32 bits or fewer: a JS
    // number, not a BigInt.
    let int_range = |t: ty::Ty<'tcx>| {
        let (bits, signed) = match *t.kind() {
            ty::Int(i) => (i.bit_width().unwrap_or(pointer_bits), true),
            ty::Uint(u) => (u.bit_width().unwrap_or(pointer_bits), false),
            _ => return None,
        };
        if bits > 32 {
            return None;
        }
        Some(match signed {
            true => (-(2f64.powi(bits as i32 - 1)), 2f64.powi(bits as i32 - 1) - 1.0),
            false => (0.0, 2f64.powi(bits as i32) - 1.0),
        })
    };
    let var_of = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::VarRef { id } if thir[e].ty.is_floating_point() => Some(id),
        _ => None,
    };
    let casts: Vec<(ExprId, LocalVarId, (f64, f64))> = thir
        .exprs
        .iter_enumerated()
        .filter_map(|(id, expr)| match expr.kind {
            ExprKind::Cast { source } => Some((id, var_of(source)?, int_range(expr.ty)?)),
            _ => None,
        })
        .collect();
    // What a call gives, where it's a `#[rust_js::whole]` binding's.
    let whole_call = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Call { fun, .. } => {
            fn_def(thir[strip(thir, fun)].ty).and_then(|(def_id, _)| crate::lower::bindings::whole_range(tcx, def_id))
        }
        _ => None,
    };
    // A cast of what such a call gives, where each of its numbers is in
    // the integer's range: `options.length() as usize`, `options.length`.
    let mut found: HashSet<ExprId> = (thir.exprs.iter_enumerated())
        .filter_map(|(id, expr)| match expr.kind {
            ExprKind::Cast { source } => {
                let ((lo, hi), (tlo, thi)) = (whole_call(source)?, int_range(expr.ty)?);
                (tlo <= lo && hi <= thi).then_some(id)
            }
            _ => None,
        })
        .collect();
    if casts.is_empty() {
        return found;
    }
    let constant = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Literal { lit, neg } => {
            let value = match lit.node {
                rustc_ast::LitKind::Float(text, _) => text.as_str().replace('_', "").parse::<f64>().ok()?,
                rustc_ast::LitKind::Int(n, _) => n.get() as f64,
                _ => return None,
            };
            Some(if neg { -value } else { value })
        }
        _ => None,
    };
    // The `f64` variable a pattern binds by value, itself.
    let binding = |pattern: &Pat<'tcx>| match pattern.kind {
        PatKind::Binding {
            var,
            mode: BindingMode(ByRef::No, _),
            subpattern: None,
            ..
        } if pattern.ty.is_floating_point() => Some(var),
        _ => None,
    };
    // What a value given to a variable may be, as a range of whole numbers.
    let given = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Literal { .. } => constant(e).filter(|v| v.fract() == 0.0).map(|v| (v, v)),
        ExprKind::Cast { source } => int_range(thir[source].ty),
        ExprKind::Call { .. } => whole_call(e),
        _ => None,
    };
    // Each variable's values, of a plain `let` and each `=` of it.
    let mut values: HashMap<LocalVarId, Option<(f64, f64)>> = HashMap::new();
    let mut bound: HashSet<LocalVarId> = HashSet::new();
    fn join(values: &mut HashMap<LocalVarId, Option<(f64, f64)>>, var: LocalVarId, range: Option<(f64, f64)>) {
        let entry = values.entry(var).or_insert(range);
        *entry = entry.zip(range).map(|((lo, hi), (a, b))| (lo.min(a), hi.max(b)));
    }
    for stmt in thir.stmts.iter() {
        if let thir::StmtKind::Let {
            ref pattern,
            initializer,
            else_block: None,
            ..
        } = stmt.kind
            && let Some(var) = binding(pattern)
        {
            bound.insert(var);
            if let Some(init) = initializer {
                join(&mut values, var, given(init));
            }
        }
    }
    // A `Some`'s number a `let` or an `if let` binds, of what such a
    // binding gives, a nullable WebIDL integer: `let Some(end) =
    // input.selection_end() else { .. }`.
    let lets = (thir.stmts.iter()).filter_map(|stmt| match &stmt.kind {
        thir::StmtKind::Let {
            pattern, initializer, ..
        } => Some((&**pattern, (*initializer)?)),
        _ => None,
    });
    let if_lets = (thir.exprs.iter()).filter_map(|expr| match &expr.kind {
        ExprKind::Let { pat, expr } => Some((&**pat, *expr)),
        _ => None,
    });
    for (pattern, of) in lets.chain(if_lets) {
        if let PatKind::Variant {
            variant_index,
            subpatterns,
            ..
        } = &pattern.kind
            && variant_index.as_u32() == 1
            && let [field] = subpatterns.as_slice()
            && let Some(var) = binding(&field.pattern)
            && let Some(range) = whole_call(of)
        {
            bound.insert(var);
            join(&mut values, var, Some(range));
        }
    }
    let mut sets: HashMap<LocalVarId, usize> = HashMap::new();
    for expr in thir.exprs.iter() {
        if let ExprKind::Assign { lhs, rhs } = expr.kind
            && let ExprKind::VarRef { id } = thir[strip(thir, lhs)].kind
        {
            *sets.entry(id).or_default() += 1;
            join(&mut values, id, given(rhs));
        }
    }
    let changed = changes(thir);
    // A variable only its `let` and its `=`s give values, and those known.
    let known = |var: LocalVarId| {
        let changes = changed.get(&var).map_or(0, Vec::len);
        let only_set = bound.contains(&var) && changes == sets.get(&var).copied().unwrap_or(0);
        values.get(&var).copied().flatten().filter(|_| only_set)
    };
    // What `cond` being `holds` shows of a variable.
    fn shown(
        e: ExprId,
        holds: bool,
        thir: &Thir<'_>,
        compared: &dyn Fn(ExprId) -> Option<Compared>,
        found: &mut Vec<Compared>,
    ) {
        match thir[strip(thir, e)].kind {
            ExprKind::Unary { op: UnOp::Not, arg } => shown(arg, !holds, thir, compared, found),
            ExprKind::LogicalOp { op, lhs, rhs } if matches!(op, LogicalOp::And) == holds => {
                shown(lhs, holds, thir, compared, found);
                shown(rhs, holds, thir, compared, found);
            }
            _ => {
                if let Some(mut c) = compared(e) {
                    if !holds {
                        c.op = match c.op {
                            BinOp::Eq => BinOp::Ne,
                            BinOp::Ne => BinOp::Eq,
                            BinOp::Lt => BinOp::Ge,
                            BinOp::Le => BinOp::Gt,
                            BinOp::Gt => BinOp::Le,
                            BinOp::Ge => BinOp::Lt,
                            _ => return,
                        };
                    }
                    found.push(c);
                }
            }
        }
    }
    // `x < 3`, or `3 > x` as it.
    let compared = |e: ExprId| {
        let ExprKind::Binary { op, lhs, rhs } = thir[strip(thir, e)].kind else {
            return None;
        };
        let flipped = match op {
            BinOp::Lt => BinOp::Gt,
            BinOp::Le => BinOp::Ge,
            BinOp::Gt => BinOp::Lt,
            BinOp::Ge => BinOp::Le,
            BinOp::Eq | BinOp::Ne => op,
            _ => return None,
        };
        match (var_of(lhs), var_of(rhs)) {
            (Some(var), None) => Some(Compared {
                var,
                op,
                with: constant(rhs)?,
            }),
            (None, Some(var)) => Some(Compared {
                var,
                op: flipped,
                with: constant(lhs)?,
            }),
            _ => None,
        }
    };
    let shows = |cond: ExprId, holds: bool| {
        let mut found = Vec::new();
        shown(cond, holds, thir, &compared, &mut found);
        found
    };
    // Where each test holds, from where it's made, and what it shows.
    let mut tested: Vec<(Span, Span, Compared)> = Vec::new();
    for expr in thir.exprs.iter() {
        match expr.kind {
            ExprKind::If {
                cond, then, else_opt, ..
            } => {
                let from = at(thir[cond].span);
                tested.extend(shows(cond, true).into_iter().map(|c| (from, at(thir[then].span), c)));
                if let Some(other) = else_opt {
                    tested.extend(shows(cond, false).into_iter().map(|c| (from, at(thir[other].span), c)));
                }
            }
            // What follows `if c { panic!() }` in its block runs only where
            // `c` didn't hold.
            ExprKind::Block { block } => {
                let block = &thir[block];
                for &stmt in &block.stmts {
                    let thir::StmtKind::Expr { expr: stmt, .. } = thir[stmt].kind else {
                        continue;
                    };
                    let ExprKind::If {
                        cond,
                        then,
                        else_opt: None,
                        ..
                    } = thir[strip(thir, stmt)].kind
                    else {
                        continue;
                    };
                    if !thir[strip(thir, then)].ty.is_never() {
                        continue;
                    }
                    let (from, rest) = (at(thir[stmt].span), at(block.span).with_lo(at(thir[stmt].span).hi()));
                    tested.extend(shows(cond, false).into_iter().map(|c| (from, rest, c)));
                }
            }
            _ => {}
        }
    }
    for (cast, var, (tlo, thi)) in casts {
        let Some((mut lo, mut hi)) = known(var) else { continue };
        let here = at(thir[cast].span);
        // A test holds here if nothing sets the variable after it's made.
        let after = |from: Span| {
            changed
                .get(&var)
                .into_iter()
                .flatten()
                .any(|change| change.hi() > from.lo())
        };
        let holding: Vec<Compared> = tested
            .iter()
            .filter(|(from, region, c)| c.var == var && region.contains(here) && !after(*from))
            .map(|&(_, _, c)| c)
            .collect();
        for c in &holding {
            match c.op {
                BinOp::Ge => lo = lo.max(c.with.ceil()),
                BinOp::Gt => lo = lo.max(c.with.floor() + 1.0),
                BinOp::Le => hi = hi.min(c.with.floor()),
                BinOp::Lt => hi = hi.min(c.with.ceil() - 1.0),
                BinOp::Eq => {
                    lo = lo.max(c.with);
                    hi = hi.min(c.with);
                }
                _ => {}
            }
        }
        // `x != -1` of what's -1 or more is 0 or more.
        let mut narrowed = true;
        while narrowed {
            narrowed = false;
            for c in holding.iter().filter(|c| c.op == BinOp::Ne) {
                if c.with == lo {
                    lo += 1.0;
                    narrowed = true;
                } else if c.with == hi {
                    hi -= 1.0;
                    narrowed = true;
                }
            }
        }
        if tlo <= lo && hi <= thi {
            found.insert(cast);
        }
    }
    found
}
