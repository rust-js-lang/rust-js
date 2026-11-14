//! Tidy's handles, `{ get value() { .. }, set value(value) { .. } }` made
//! on a place (ADR 0287): one made only to read or write its value there,
//! its place; a flattened state's written whole, each of its keys set
//! (ADR 1044); one whose `value` alone is read, the place, or the value
//! where it's read at once; and each write through one, as the place
//! stores it (ADRs 0772, 0791).

use super::*;

/// Each assignment through a handle, `h.value = v`, a closure's too, whose
/// object `storage_of` says the storage of: `v` written as it stores it, a
/// nullable field's `None` `null` (ADRs 0772, 0791).
fn written_as_stored(body: &mut Vec<Stmt>, storage_of: &dyn Fn(&Expr) -> Option<js::Storage>) {
    let stored = |list: &mut Vec<Stmt>| {
        js::statement_lists(list, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                if let StmtKind::Assign(target, value) = &mut stmt.kind
                    && let ExprKind::Member(of, key) = &target.kind
                    && key == js::HELD
                    && let Some(storage) = storage_of(of)
                {
                    *value = storage.written(std::mem::replace(value, Expr::taken()), false);
                }
            }
        })
    };
    stored(body);
    js::each_expr_mut(body, &mut |e| match &mut e.kind {
        ExprKind::Arrow(_, inner) | ExprKind::AsyncArrow(_, inner) => stored(inner),
        ExprKind::Function(function) => stored(&mut function.body),
        _ => {}
    });
}

/// A handle made only to read or write its value there, `Cell::set` of
/// `&self.n`'s `({ get value() { .. }, set value(value) { .. } }).value =
/// v`: its place, `t.n = v`. Of a place that reads the same
/// alone: of one that computes, `a[f()]`, the setter would compute it after
/// `v`, where the place itself is computed before.
pub(in crate::lower::mir::tidy) fn handles_in_place(body: &mut Vec<Stmt>) {
    written_whole_in_place(body);
    // What's assigned through one, `({ get value() .., set value(v) { o.f =
    // v ?? null; } }).value = undefined`, is written as its storage stores
    // it, a nullable field's `None` `null`, before it's the place (ADR 0772,
    // amended).
    written_as_stored(body, &|of| match &of.kind {
        ExprKind::Handle(place, storage) if place.reads_same() => Some(*storage),
        _ => None,
    });
    // A handle on a property a getter may answer, a JS object's field,
    // `this._weakResponse` of a constructor's `this` cast to it (#449), is
    // `read_handles`' to make its place where it's used, as it's declared
    // (#503, #569).
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == js::HELD
            && let ExprKind::Handle(place, _) = &of.kind
            && place.reads_same()
        {
            *e = Expr {
                span: e.span,
                ..(**place).clone()
            };
        }
    });
}

/// A flattened state written whole through its handle, each of its keys
/// set on the object it's in (ADR 1044, `Storage::written_in_place`),
/// before the handle is its place: `handles_in_place`'s, and
/// `read_handles`' where it makes one of a declared handle where tidy's
/// stages run again, after `handles_in_place` (#779, #784).
fn written_whole_in_place(body: &mut Vec<Stmt>) {
    // Each a name of its own, as two such writes may be in one list (#687).
    let mut taken = IndexSet::default();
    js::names_in(body, &mut taken);
    // A closure's write too (#482).
    js::each_block_mut(body, &mut |stmts| {
        let mut out = Vec::with_capacity(stmts.len());
        for stmt in std::mem::take(stmts) {
            if let StmtKind::Assign(target, value) = &stmt.kind
                && let ExprKind::Member(of, key) = &target.kind
                && key == js::HELD
                && let ExprKind::Handle(place, storage) = &of.kind
                && let Some(sets) = storage.written_in_place(
                    place,
                    value.clone(),
                    &mut |name, value, out| {
                        let fresh = crate::names::fresh_in(&mut taken, name);
                        out.push(StmtKind::Const(fresh.clone(), value).at(stmt.span));
                        Expr::var(&fresh)
                    },
                    stmt.span,
                )
            {
                out.extend(sets);
                continue;
            }
            out.push(stmt);
        }
        *stmts = out;
    });
}

/// `const h = { get value() { return o.f; }, .. }`, a `&Cell`'s handle on a
/// field, whose `value` alone is used, the handle never given: read before
/// anything could set `o.f`, `const h = o.f`, `h` where its `value` is
/// read (ADR 0287), but where that's once (#532); else `o.f` where its `value` is, the `o` it was made
/// of, as MIR holds a reference written again before it's made, and where
/// it's written through, a closure's `h.value = 5` too (#192).
///
/// Of a flattened state's, `in_place`: one a let chain's guard is, which
/// MIR declares before its `if`, a `const` only once `declarations` made it
/// one (rd533); a field's others are their place later, in place
/// (`handles_in_place`), where what's made before them is read into them.
/// And one on a field declared only then, a guard's of a field's `RefCell`
/// no borrow of which is counted, `*tries.borrow_mut() += 1` of a pattern's
/// binding, assigned in the branch its `let` sinks to (#503).
///
/// A `let` of a shared borrow of what has no cell of its own, `frozen`'s,
/// `let chunks = store.chunks.borrow();`, is read where it's made, `const
/// chunks = store.chunks;`, whatever runs before its uses: Rust lets
/// nothing change what it's to while it's held (rd658).
pub(in crate::lower::mir::tidy) fn read_handles(
    body: &mut Vec<Stmt>,
    in_place: bool,
    lets: &crate::lower::mir::tidy::LetNames,
    rerun: bool,
) {
    let frozen = &lets.frozen;
    let mut handles: Vec<(String, Expr, bool, js::Storage)> = Vec::new();
    js::each_list(body, &mut |stmts| {
        for (k, stmt) in stmts.iter().enumerate() {
            if let StmtKind::Const(name, value) = &stmt.kind
                && let ExprKind::Handle(place, storage) = &value.kind
                && (matches!(place.kind, ExprKind::Member(..))
                    && !(in_place && matches!(storage, js::Storage::InPlace(_)))
                    || matches!(storage, js::Storage::InPlace(_)) && place.reads_same()
                    // A `&mut`'s to a place that stores `null` for `None`, a
                    // nullable parameter or an item of a nullable field's
                    // items, `v[0]` (#541).
                    || matches!(storage, js::Storage::NullForNone)
                        && (place.reads_same()
                            || matches!(&place.kind, ExprKind::Index(of, at) if of.reads_same() && at.reads_same())))
            {
                let at_once = frozen.contains(name) || read_at_once(stmts, k, name);
                handles.push((name.clone(), (**place).clone(), at_once, *storage));
            }
        }
    });
    // A `dyn`'s pair on a handle's `value`, `&mut fiber.owner as &mut dyn
    // Trait`'s: a pair on the handle's place that stores as it does, a
    // nullable field's `null` or a flattened state's keys, not one whose
    // setter assigns the place as it's given (#977).
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Pair(place, dictionary, js::Storage::AsGiven) = &e.kind
            && let ExprKind::Member(of, key) = &place.kind
            && key == js::HELD
            && let ExprKind::Var(v) = &of.kind
            && let Some((_, on, _, storage)) = handles.iter().find(|(name, ..)| name == v)
            && *storage != js::Storage::AsGiven
        {
            *e = Expr {
                span: e.span,
                ..Expr::stored_pair(on.clone(), (**dictionary).clone(), *storage)
            };
        }
    });
    // A handle on a flattened state's handle's `value`, `&mut *state` given
    // on, `Cell::from_mut(&mut *state)`'s or a call's that keeps it: that
    // handle, whose setter sets each key, not one on its place, whose
    // setter would assign the variable holding the class's object (#678).
    let states: IndexSet<String> = (handles.iter())
        .filter(|(.., storage)| matches!(storage, js::Storage::InPlace(_)))
        .map(|(name, ..)| name.clone())
        .collect();
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Handle(place, js::Storage::AsGiven) = &e.kind
            && let ExprKind::Member(of, key) = &place.kind
            && key == js::HELD
            && matches!(&of.kind, ExprKind::Var(v) if states.contains(v))
        {
            *e = Expr {
                span: e.span,
                ..(**of).clone()
            };
        }
    });
    // Written through, `h.value = 5`, a closure's too (#192): its place
    // where its `value` is, read and written there, never read at once.
    let changed = js::Writes::of(body).changed;
    for (name, _, at_once, _) in &mut handles {
        *at_once &= !changed.contains(name);
    }
    // A flattened state's written whole through it keeps it, whose setter
    // sets each key (ADR 1044), a closure's write too (#482).
    let mut written_whole = IndexSet::default();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind
                && let ExprKind::Member(of, key) = &target.kind
                && key == js::HELD
                && let ExprKind::Var(v) = &of.kind
            {
                written_whole.insert(v.clone());
            }
        }
    });
    // There, it's the handle itself where it's used, made of its place,
    // which reads the same: each whole write through it sets the object's
    // keys there (`handles_in_place`), each read is the object (#478).
    let whole: Vec<(String, Expr)> = (handles.iter())
        .filter(|(name, _, _, storage)| matches!(storage, js::Storage::InPlace(_)) && written_whole.contains(name))
        .map(|(name, place, _, storage)| (name.clone(), Expr::stored_handle(place.clone(), *storage)))
        .collect();
    if !whole.is_empty() {
        let of = |name: &str| whole.iter().find(|(n, _)| n == name).map(|(_, handle)| handle);
        js::statement_lists(body, &mut |stmts| {
            stmts.retain(|stmt| !matches!(&stmt.kind, StmtKind::Const(name, _) if of(name).is_some()));
        });
        let substitute = &mut |e: &mut Expr| {
            if let ExprKind::Var(v) = &e.kind
                && let Some(handle) = of(v)
            {
                *e = Expr {
                    span: e.span,
                    ..handle.clone()
                };
            }
        };
        js::each_expr_mut(body, substitute);
        // Where tidy's stages run again, after the `written` stage's
        // `handles in place`, which isn't: its keys set here. Before it,
        // they're set there, once what's read into them is (#779, #784).
        if rerun {
            written_whole_in_place(body);
        }
    }
    handles
        .retain(|(name, _, _, storage)| !(matches!(storage, js::Storage::InPlace(_)) && written_whole.contains(name)));
    handles.retain_mut(|(name, place, at_once, storage)| {
        // Each use of it one of its `value`: none left once those are.
        let mut rest = body.clone();
        js::each_expr_mut(&mut rest, &mut |e| {
            if let ExprKind::Member(of, key) = &e.kind
                && key == js::HELD
                && matches!(&of.kind, ExprKind::Var(v) if v == name)
            {
                *e = Expr::js_undefined();
            }
        });
        let mut root = &*place;
        while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {
            root = of;
        }
        let rereads = matches!(root.kind, ExprKind::Var(_) | ExprKind::This);
        // Read once, where it's read, as Rust's `.get()` reads it, no
        // `const` of it: `return kind.current;` of `Kind::Signal { current }
        // => current.get()`, an arm's binding (#532). Not a `let`'s the
        // program wrote, `let Fiber { tag, .. } = fiber;`, which keeps its
        // name, as `lets side by side` does (ADR 0287, amended).
        let written = lets.patterns.contains(name.as_str()) || lets.named.contains(name.as_str());
        if *at_once && rereads && !written && js::mentions_in(body, name) == 1 {
            *at_once = false;
        }
        // A `&mut`'s handle on a nullable place nothing reads, its place
        // written as a path where it's used (`nulled::unhandled`, #541): none.
        if matches!(storage, js::Storage::NullForNone) && js::mentions_in(body, name) == 0 {
            *at_once = false;
        }
        js::mentions_in(&rest, name) == 0 && (*at_once || rereads)
    });
    if handles.is_empty() {
        return;
    }
    let read = |name: &str| (handles.iter()).find(|(h, ..)| h == name);
    // What's written through one, as its storage stores it: a nullable
    // field's `None`, `null` (ADR 0791).
    written_as_stored(body, &|of| match &of.kind {
        ExprKind::Var(v) => read(v).map(|(.., storage)| *storage),
        _ => None,
    });
    js::statement_lists(body, &mut |stmts| {
        stmts.retain(
            |stmt| !matches!(&stmt.kind, StmtKind::Const(name, _) if read(name).is_some_and(|(_, _, at_once, _)| !at_once)),
        );
        for stmt in stmts.iter_mut() {
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && read(name).is_some()
                && let ExprKind::Handle(place, _) = &value.kind
            {
                *value = (**place).clone();
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == js::HELD
            && let ExprKind::Var(v) = &of.kind
            && let Some((_, place, at_once, _)) = read(v)
        {
            let read = match at_once {
                true => Expr::var(v),
                false => place.clone(),
            };
            *e = Expr { span: e.span, ..read };
        }
    });
}

/// Whether what `stmts[at]` names is last read before anything that could
/// set a field: no field set nor call made from it to there.
pub(super) fn read_at_once(stmts: &[Stmt], at: usize, name: &str) -> bool {
    let Some(last) = (at + 1..stmts.len())
        .rev()
        .find(|&k| js::mentions_in(&stmts[k..=k], name) > 0)
    else {
        return true;
    };
    let mut between = stmts[at + 1..=last].to_vec();
    let mut still = true;
    js::statement_lists(&mut between, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind {
                still &= matches!(target.kind, ExprKind::Var(_));
            }
        }
    });
    js::each_expr_mut(&mut between, &mut |e| still &= !e.has_effects());
    still
}
