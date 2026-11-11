//! A module's thread-locals and `static mut`s that are its variables,
//! `let X = init` (ADR 0270): which are, their cells as they're made, a
//! nullable one's of `null` (#541, ADR 0586), and what another module reads
//! of them, by name. Split from cell.rs by concept, at 1,370 lines (#1334).

use super::*;

/// A module's own `const X = { value: init }`, a thread-local's cell or a
/// `static mut` (ADR 0096, amended) MIR reads by its `value`, that its
/// module reads and sets only so, by name: `let X = init`, `X` where it's
/// read, `const` where nothing sets it (ADR 0270). A handle on its place,
/// a pointer given on, `$ptr(X, "value")`, reads it whole and keeps it. One
/// it exports is one only where it's `quiet`, as a `static mut` is, which
/// another module only reads, by `get`, a quiet `with_borrow` or a `static
/// mut`'s name, `imported_variables`'s to
/// make its name (ADR 0270, amended): the names of those it exports so, the
/// module's variables. One `quiet`, whose borrows ask nothing of borrows,
/// gave the closure of each what it holds, an object's, which it shares
/// (ADR 0328), where its borrow was lowered (`Std::LocalBorrow`). And of `held_values`, its thread-locals of a value
/// and `static mut`s (`holds_a_value`), those it exports still their `{
/// value }`, which the caller refuses (ADR 0270, amended). A
/// `#[rust_js::nullable]` one of `nullable` kept as its cell, lent whole
/// where something counts its borrows, is a cell that stores `null` for
/// `None` however it's written, `$nullableCell(null)`, as its reads take it
/// (#541).
pub(in crate::lower) fn plain_statics(
    items: &mut [js::Item],
    quiet: &IndexSet<&str>,
    held_values: &IndexSet<&str>,
    nullable: &IndexSet<&str>,
) -> (Vec<(String, bool)>, Vec<String>) {
    // A closure called where it's made: its body (ADRs 0581, 0840), in a
    // function or in what the module runs when it's loaded, which no
    // `return` ends.
    applied_in_place(items);
    let cells: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c) if (!c.export || quiet.contains(c.name.as_str())) && is_box(&c.value) => {
                Some(c.name.clone())
            }
            _ => None,
        })
        .collect();
    let mut exported = Vec::new();
    for name in cells {
        let (mut reads, mut values, mut sets) = (0, 0, 0);
        js::each_item_expr_mut(items, &mut |e| match &e.kind {
            js::ExprKind::Var(v) if *v == name => reads += 1,
            js::ExprKind::Member(of, key)
                if key == js::HELD && matches!(&of.kind, js::ExprKind::Var(v) if *v == name) =>
            {
                values += 1;
            }
            _ => {}
        });
        if binds_name(items, &name) || reads != values {
            continue;
        }
        // What sets it, `X.value = ..`, its value replaced: in a function, a
        // closure too, a constant's, or in what the module runs when it's
        // loaded, `js::on_load!`'s body in its place (ADRs 0840, 0270
        // amended, #201).
        js::each_item_block_mut(items, &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Assign(target, _) = &stmt.kind
                    && matches!(&target.kind, js::ExprKind::Member(of, key)
                        if key == js::HELD && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
                {
                    sets += 1;
                }
            }
        });
        // A handle on it, lent, may set it too.
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Handle(place, _) = &e.kind
                && matches!(&place.kind, js::ExprKind::Member(of, key)
                    if key == js::HELD && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
            {
                sets += 1;
            }
        });
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Member(of, key) = &e.kind
                && key == js::HELD
                && matches!(&of.kind, js::ExprKind::Var(v) if *v == name)
            {
                *e = Expr {
                    span: e.span,
                    ..Expr::var(&name)
                };
            }
        });
        for item in items.iter_mut() {
            if let js::Item::Const(c) = item
                && c.name == name
                && let js::ExprKind::Object(props, ..) = &mut c.value.kind
                && let Some(js::Prop::Field(_, init)) = props.pop()
            {
                c.value = init;
                c.mutable = sets > 0;
                if c.export {
                    exported.push((name.clone(), c.mutable));
                }
            }
        }
    }
    // A `RefCell` of an object that's its module's variable is the object
    // (ADR 0863), written whole in place, `$assign(callbacks, [])`, its
    // `set`'s: here, its module's, the variable given the new one,
    // `callbacks = []`, `export let`, as nothing else holds the old one, its
    // borrows ending where they're made (ADR 0270, amended). Another
    // module writes the one it reads in place, as before.
    let objects: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c) if quiet.contains(c.name.as_str()) && !is_box(&c.value) => Some(c.name.clone()),
            _ => None,
        })
        .collect();
    for name in objects {
        if binds_name(items, &name) {
            continue;
        }
        let mut sets = 0;
        js::each_item_block_mut(items, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                if let StmtKind::Expr(e) = &mut stmt.kind
                    && let js::ExprKind::Call(callee, args) = &mut e.kind
                    && callee.helper_name() == Some("$assign")
                    && let [target, _] = args.as_slice()
                    && matches!(&target.kind, js::ExprKind::Var(v) if *v == name)
                {
                    let value = args.pop().expect("what it's given");
                    let target = args.pop().expect("the variable");
                    stmt.kind = StmtKind::Assign(target, value);
                    sets += 1;
                }
            }
            sets += taken_whole(stmts, &name);
        });
        if sets == 0 {
            continue;
        }
        for item in items.iter_mut() {
            if let js::Item::Const(c) = item
                && c.name == name
            {
                c.mutable = true;
                if c.export {
                    exported.push((name.clone(), true));
                }
            }
        }
    }
    // Each exported one of a value still its `{ value }`, its cell handed
    // out or its borrows asked of: a JS module importing it reads the box,
    // not what it holds, so the caller refuses it (ADR 0270, amended).
    let boxed = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c) if c.export && held_values.contains(c.name.as_str()) && is_box(&c.value) => {
                Some(c.name.clone())
            }
            _ => None,
        })
        .collect();
    for item in items.iter_mut() {
        if let js::Item::Const(c) = item
            && nullable.contains(c.name.as_str())
        {
            nullable_cell(&mut c.value);
        }
    }
    (exported, boxed)
}

/// `made`, a thread-local's cell, `{ value: init }`, as one that stores
/// `null` for `None`, `$nullableCell(init)`; of one made where it's first
/// used, `$local(() => ({ value: init }))`, what its initializer returns
/// (#541).
fn nullable_cell(made: &mut Expr) {
    match &mut made.kind {
        js::ExprKind::Object(props, ..) if is_box(&Expr::object(props.clone())) => {
            if let Some(js::Prop::Field(_, init)) = props.pop() {
                *made = Expr::call_helper("$nullableCell", vec![init]);
            }
        }
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$local") => {
            if let [arrow] = args.as_mut_slice()
                && let js::ExprKind::Arrow(_, body) = &mut arrow.kind
            {
                js::statement_lists(body, &mut |stmts| {
                    for stmt in stmts.iter_mut() {
                        if let StmtKind::Return(Some(cell)) = &mut stmt.kind {
                            nullable_cell(cell);
                        }
                    }
                });
            }
        }
        _ => {}
    }
}

/// Each `$take(name, v)` of `stmts`, a `take()` or `replace(v)` of the
/// module's variable `name`, an object, that's the first its statement
/// does: the variable given the new value, `const old = name; name = v;`,
/// the statement of `old`, as React's flushPendingUnsafeLifecycleWarnings
/// sets `pending = []` (ADR 0270, amended). In place, the old array was
/// emptied into a copy; nothing else holds it, as `$assign`'s `set` above
/// takes. A `const` of it is that `const`. How many there were.
fn taken_whole(stmts: &mut Vec<Stmt>, name: &str) -> usize {
    let taken = |e: &Expr| matches!(taken_parts(e), Some((target, _)) if matches!(&target.kind, js::ExprKind::Var(v) if v == name));
    let mut count = 0;
    let mut i = 0;
    while i < stmts.len() {
        let span = stmts[i].span;
        let slot = match &mut stmts[i].kind {
            StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) | StmtKind::Expr(e) => {
                Some(e)
            }
            StmtKind::ForOf { iterable, .. } => Some(iterable),
            StmtKind::Assign(target, e) if matches!(target.kind, js::ExprKind::Var(_)) => Some(e),
            _ => None,
        };
        let Some(slot) = slot.filter(|e| taken(e)) else {
            i += 1;
            continue;
        };
        let (variable, value) = taken_parts(slot).map(|(v, value)| (v.clone(), value)).expect("taken");
        *slot = Expr::undefined();
        let set = StmtKind::Assign(variable.clone(), value).at(span);
        count += 1;
        // What it gave unread: the variable given the new value alone.
        if let StmtKind::Expr(_) = &stmts[i].kind {
            stmts[i] = set;
            i += 1;
            continue;
        }
        // `const names = $take(..)`: `const names = pending; pending = [];`.
        if let StmtKind::Const(..) | StmtKind::Let(..) = &stmts[i].kind {
            let (StmtKind::Const(_, e) | StmtKind::Let(_, Some(e))) = &mut stmts[i].kind else {
                unreachable!("a declaration")
            };
            *e = variable;
            stmts.insert(i + 1, set);
            i += 2;
            continue;
        }
        let mut names = IndexSet::default();
        js::names_in(stmts, &mut names);
        let old = crate::lower::fresh_in(&mut names, "old");
        let slot = match &mut stmts[i].kind {
            StmtKind::Return(Some(e)) | StmtKind::Expr(e) | StmtKind::Assign(_, e) => e,
            StmtKind::ForOf { iterable, .. } => iterable,
            _ => unreachable!("a slot"),
        };
        *slot = Expr::var(&old);
        stmts.splice(i..i, [StmtKind::Const(old, variable).at(span), set]);
        i += 3;
    }
    count
}

/// Whether `made` is a thread-local's cell as its `{ value }`.
fn is_box(made: &Expr) -> bool {
    matches!(&made.kind, js::ExprKind::Object(props, ..)
        if matches!(props.as_slice(), [js::Prop::Field(key, _)] if key == js::HELD))
}

/// Whether a function of `items`, or what the module runs when it's
/// loaded, binds `name` of its own, which a read of `name` may be.
fn binds_name(items: &mut [js::Item], name: &str) -> bool {
    let mut bound = false;
    let binds = |body: &mut Vec<Stmt>, bound: &mut bool| {
        js::each_block_mut(body, &mut |stmts| {
            for stmt in stmts.iter() {
                *bound |= match &stmt.kind {
                    StmtKind::Const(n, _) | StmtKind::Let(n, _) => *n == name,
                    StmtKind::Destructure { pattern, .. } => pattern.names().contains(&name),
                    _ => false,
                };
            }
        });
        js::each_expr_mut(body, &mut |e| {
            if let js::ExprKind::Arrow(params, _) | js::ExprKind::AsyncArrow(params, _) = &e.kind {
                *bound |= params.iter().any(|p| p.names().contains(&name));
            }
        });
    };
    for item in items.iter_mut() {
        if let js::Item::Statements(stmts) = item {
            binds(stmts, &mut bound);
        }
        for function in item.functions_mut() {
            bound |= function.params.iter().any(|p| p.names().contains(&name));
            binds(&mut function.body, &mut bound);
        }
    }
    bound
}

/// A nullable thread-local's cell as it's made, `{ value: undefined }` of
/// `RefCell::new(None)`: its `None` `null`, `{ value: null }`, its module's
/// variable `let current = null` (ADR 0586, amended). One that holds the
/// `Option` itself is made so by its `init` (`null_result`).
pub(in crate::lower) fn nullable_made(made: Expr) -> Expr {
    match made.kind {
        js::ExprKind::Object(mut props, layout) if matches!(props.as_slice(), [Prop::Field(key, _)] if key == js::HELD) =>
        {
            if let Some(Prop::Field(_, value)) = props.first_mut() {
                *value = NoneForm::Null
                    .storage()
                    .written(std::mem::replace(value, Expr::undefined()), false);
            }
            Expr {
                kind: js::ExprKind::Object(props, layout),
                ..made
            }
        }
        _ => made,
    }
}

/// What a module reads of another's thread-locals that are that module's
/// variables, `export let x` (`plain_statics`), each `variable`, a
/// `Symbol` of the crate's or a library's import (ADR 0270, amended): `x`
/// of what was `x.value`, and a `with_borrow`'s closure given `x`, called
/// as a statement its body (ADRs 0581, 0840). The module only reads it,
/// as `outside_writes` holds.
pub(in crate::lower) fn imported_variables(items: &mut [js::Item], variable: &dyn Fn(&Expr) -> bool) {
    let mut changed = false;
    js::each_item_expr_mut(items, &mut |e| match &mut e.kind {
        js::ExprKind::Member(of, key) if key == js::HELD && variable(of) => {
            let read = std::mem::replace(&mut **of, Expr::undefined());
            *e = Expr {
                span: e.span,
                facts: js::Facts::of(js::Known::Live),
                ..read
            };
            changed = true;
        }
        js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$withBorrow") && matches!(args.as_slice(), [key, _] if variable(key)) =>
        {
            let (mut key, f) = (args.remove(0), args.remove(0));
            key.facts = js::Facts::of(js::Known::Live);
            *e = Expr {
                span: e.span,
                ..apply(f, vec![key])
            };
            changed = true;
        }
        _ => {}
    });
    if changed {
        applied_in_place(items);
    }
}

/// `imported_variables` of each variable `named` says of, by its name, and
/// each read through its module's namespace, `Fiber.current`, which
/// `through` says of the namespace and the member (rd114a, ADR 1018).
pub(in crate::lower) fn imported_variables_through(
    items: &mut [js::Item],
    named: &dyn Fn(&Expr) -> bool,
    through: &dyn Fn(&Expr, &str) -> bool,
) {
    imported_variables(items, &|e| match &e.kind {
        js::ExprKind::Member(namespace, export) if through(namespace, export) => true,
        _ => named(e),
    });
}

/// What each of the crate's modules reads of another's `variables`, its
/// thread-locals and `static mut`s that are its variables (ADR 0270,
/// amended): by their names, `Symbol`s of their modules, or through their
/// modules' namespaces by what those export them as, `Fiber.current`
/// (rd114a, ADR 1018).
pub(in crate::lower) fn crate_imported_variables(
    tcx: rustc_middle::ty::TyCtxt<'_>,
    fns: &IndexMap<rustc_span::def_id::DefId, crate::lower::FnInfo>,
    variables: &IndexSet<rustc_span::def_id::DefId>,
    lowered: &mut [crate::program::UnlinkedModule],
) {
    let symbol = |id: &rustc_span::def_id::DefId, export: &str| crate::lower::module_symbol(fns[id].module, export);
    let named: IndexSet<js::Symbol> = (variables.iter())
        .map(|id| symbol(id, fns[id].owner.as_ref().unwrap_or(&fns[id].name)))
        .collect();
    let namespaced: IndexSet<js::Symbol> = (variables.iter())
        .map(|id| symbol(id, &crate::lower::default_exports::exported_as(tcx, *id, &fns[id].name)))
        .collect();
    if named.is_empty() {
        return;
    }
    for unlinked in lowered {
        imported_variables_through(
            &mut unlinked.module.items,
            &|e| matches!(&e.kind, js::ExprKind::Symbol(symbol) if named.contains(symbol)),
            &|namespace, export| {
                matches!(&namespace.kind, js::ExprKind::Symbol(namespace) if namespace.export == js::NAMESPACE
                    && namespaced.contains(&js::Symbol { module: namespace.module, export: export.to_string() }))
            },
        );
    }
}

/// Each closure called where it's made, its body (ADRs 0581, 0840), in a
/// function, in what the module runs when it's loaded, which no `return`
/// ends, or in a closure a constant's value makes (ADR 0581, amended).
fn applied_in_place(items: &mut [js::Item]) {
    for item in items.iter_mut() {
        match item {
            js::Item::Statements(stmts) => js::applied_blocks(stmts, false),
            js::Item::Const(constant) => js::applied_closures(&mut constant.value),
            js::Item::Namespace(namespace) => {
                (namespace.constants.iter_mut()).for_each(|constant| js::applied_closures(&mut constant.value))
            }
            js::Item::Function(_) => {}
        }
        for function in item.functions_mut() {
            js::applied_blocks(&mut function.body, true);
            // A `try` of a read that may have thrown, `$withBorrow(..)`, now
            // a plain one, `previous = current`, that can't (#408).
            crate::lower::mir::settle_unthrown_catches(&mut function.body);
        }
    }
    // What a nullable place stores, `x ?? null`, of what's now read in
    // the parameter's place, a nullable thread-local's lent value, never
    // `undefined`: `x` (ADR 0841, amended).
    js::each_item_expr_mut(items, &mut js::Expr::store_again);
}
