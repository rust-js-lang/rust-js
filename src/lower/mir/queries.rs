//! What a body's MIR says of a local, a place or a block, asked of the
//! body alone.

use super::*;

/// Whether `local` is assigned once each way through `body`, as a `match`'s
/// arms each assign what it gives: no assignment goes on to another, nor to
/// itself again, as one in a loop would.
pub(super) fn once_each_way(body: &mir::Body<'_>, local: Local) -> bool {
    let mut at: Vec<BasicBlock> = Vec::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind {
                let (place, rvalue) = &**assign;
                if place.local == local && place.projection.is_empty() {
                    at.push(block);
                }
                // Borrowed mutably whole, it may be assigned through the borrow.
                if let Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) = rvalue
                    && borrowed.local == local
                {
                    return false;
                }
            }
        }
        if let TerminatorKind::Call { destination, .. } = &data.terminator().kind
            && destination.local == local
            && destination.projection.is_empty()
        {
            at.push(block);
        }
    }
    let mut seen = std::collections::HashSet::new();
    for (i, &block) in at.iter().enumerate() {
        if at[i + 1..].contains(&block) {
            return false;
        }
        seen.clear();
        let mut next: Vec<BasicBlock> = body.basic_blocks[block].terminator().successors().collect();
        while let Some(b) = next.pop() {
            if !seen.insert(b) {
                continue;
            }
            if at.contains(&b) {
                return false;
            }
            next.extend(body.basic_blocks[b].terminator().successors());
        }
    }
    true
}

/// The body of the async arrow `stmts` only call and return, `return (async
/// () => { .. })()`, an `async` body made: the async function's own.
pub(super) fn called_async_body(stmts: &mut Vec<Stmt>) -> Option<Vec<Stmt>> {
    // What it captures first, `const captured = arg`, or a copy of an
    // array it's given, `arg.slice()`: made as it starts, which an async
    // function's body runs till its first `await`, and none can throw.
    let copy = |value: &Expr| {
        matches!(&value.kind, js::ExprKind::Call(callee, args)
            if args.is_empty()
                && matches!(&callee.kind, js::ExprKind::Member(of, method) if method == "slice" && of.reads_only_vars()))
    };
    let (last, captured) = stmts.split_last_mut()?;
    if !captured
        .iter()
        .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_only_vars() || copy(value)))
    {
        return None;
    }
    let Stmt {
        kind: StmtKind::Return(Some(called)),
        ..
    } = last
    else {
        return None;
    };
    let js::ExprKind::Call(callee, args) = &mut called.kind else {
        return None;
    };
    let js::ExprKind::AsyncArrow(params, body) = &mut callee.kind else {
        return None;
    };
    if !args.is_empty() || !params.is_empty() {
        return None;
    }
    let body = std::mem::take(body);
    stmts.pop();
    let mut all = std::mem::take(stmts);
    all.extend(body);
    Some(all)
}

/// Whether a value moves freely among those made around it: it does
/// nothing, and reads variables alone, none borrowed, which nothing can
/// change meanwhile.
pub(super) fn movable(state: &State<'_, '_>, value: &Value<'_>) -> bool {
    match value {
        Value::Fmt(_, _, e) if e.reads_same() => true,
        Value::Expr(e) | Value::Fmt(_, _, e) | Value::Discriminant(e, ..) | Value::Branch(e, _) | Value::Tested(e) => {
            let mut stable = true;
            e.visit_vars(&mut |name| {
                stable &= !state.naming.borrowed_names().contains(name) && state.own_names.contains(name);
            });
            e.reads_only_vars() && stable
        }
        Value::List(items) => items.iter().all(|item| movable(state, item)),
        Value::Cut(test, cut) => {
            movable(state, &Value::Expr(test.clone())) && movable(state, &Value::Expr(cut.clone()))
        }
        // A place shared, or shown, is read where it's used, as Rust reads
        // it; nothing changes it meanwhile.
        Value::Place(_) => true,
        Value::Ref(_) => false,
    }
}

/// Whether `std_values` lowers `known` of its arguments' values alone: not one
/// lowered from its places or its shape first.
pub(super) fn lowered_from_values(known: Std) -> bool {
    !matches!(
        known,
        Std::Swap
            | Std::Replace
            | Std::CloneFrom
            | Std::OptionTake
            | Std::OptionReplace
            | Std::MemTake
            | Std::Last
            | Std::Cloned
            | Std::ArrayMethod(_)
            | Std::Enumerate
            | Std::Rev
            | Std::Skip
            | Std::Take
            | Std::Fold
            | Std::Sum
            | Std::CollectString
            | Std::Collect
            | Std::Position
            | Std::Extreme(_)
            | Std::Sort
            | Std::SortByKey
            | Std::PushStr
            | Std::AssignOperator(_)
            | Std::StringEdit(_)
            | Std::StringWithCapacity
            | Std::VecMacro
            | Std::FmtNew
            | Std::AssertFailed
            | Std::Map(_)
            | Std::Range(_)
            | Std::Stream(_)
            | Std::TypeName { .. }
            | Std::Comb(_)
            | Std::IterComb(_)
            | Std::Text(_)
            | Std::Number(_)
            | Std::FromElem
            | Std::Heap(_)
            | Std::ExactLen
            | Std::SizeHint(_)
            | Std::WrappingOp(..)
            | Std::IterByRef
            | Std::NonZeroNew
            | Std::UserWrite
            | Std::UserIoWrite
            | Std::DequeRemove
            | Std::Step(_)
            | Std::Cow(_)
            | Std::Rc(_)
            | Std::Slice(_)
            | Std::OptionPlace(_)
            | Std::PtrEq
    )
}

/// What overflowing `op` says, as Rust's check does.
pub(super) fn overflow_message(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "attempt to add with overflow",
        BinOp::Sub => "attempt to subtract with overflow",
        BinOp::Mul => "attempt to multiply with overflow",
        BinOp::Div => "attempt to divide with overflow",
        BinOp::Rem => "attempt to calculate the remainder with overflow",
        BinOp::Shl => "attempt to shift left with overflow",
        _ => "attempt to shift right with overflow",
    }
}

/// The variable the program binds what `?`'s `branch`, `tried`, holds to
/// where it goes on, `f`, of `val = copy ((tried as Continue).0); f = copy
/// val;`, given nothing else, and the local its binding reads.
pub(super) fn continued_binding(state: &State<'_, '_>, tried: Local) -> Option<(Local, Local)> {
    // Each variable given just `from`, which holds what's held.
    let given = |from: &dyn Fn(&Place<'_>) -> bool| {
        let mut found = None;
        for data in state.body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let (place, Rvalue::Use(Operand::Copy(read) | Operand::Move(read), ..)) = &**assign
                    && from(read)
                    && place.projection.is_empty()
                    && state.locals.writes[place.local] == 1
                    && found.replace(place.local).is_some()
                {
                    return None;
                }
            }
        }
        found
    };
    let val = given(&|read| {
        read.local == tried
            && matches!(read.projection.as_slice(), [PlaceElem::Downcast(_, variant), PlaceElem::Field(field, _)]
                if variant.as_u32() == 0 && field.as_u32() == 0)
    })?;
    // And what it's read from, `tried` or `val`, where it's bound.
    match state.locals.user[val] {
        true => Some((val, tried)),
        false => given(&|read| read.local == val && read.projection.is_empty())
            .filter(|&l| state.locals.user[l])
            .map(|l| (l, val)),
    }
}

/// Where a function's code starts, of `whole`, its span with its body,
/// and `name`, its name's: of one a macro writes of what the program wrote
/// in it, `js::export_default! { fn f() {} }`, the line its name is on, so
/// a comment before it in the macro is its, as one before the macro is
/// (ADR 0735).
pub(in crate::lower::mir) fn function_start(tcx: ty::TyCtxt<'_>, whole: Span, name: Option<Span>) -> Span {
    match name {
        Some(name) if whole.from_expansion() && !name.from_expansion() => {
            let line = tcx.sess.source_map().span_extend_to_line(name);
            line.with_hi(whole.source_callsite().hi())
        }
        _ => whole,
    }
}

/// Labels, by block.
pub(super) fn label(block: BasicBlock) -> String {
    format!("bb{}", block.as_usize())
}

/// Whether `name` is a label `label` made, of a block, not one the program
/// writes, `'rows`.
pub(in crate::lower::mir) fn is_block_label(name: &str) -> bool {
    name.strip_prefix("bb")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Of a bounds check `msg`, the index whose element `target` reads or
/// writes first, and whether it writes it: there, `$index(xs, i)` or
/// `xs[$at(xs, i)]`, the check.
pub(super) fn deferred_check<'tcx>(
    body: &mir::Body<'tcx>,
    msg: &mir::AssertMessage<'tcx>,
    target: BasicBlock,
) -> Option<(Local, bool)> {
    let mir::AssertKind::BoundsCheck {
        index: Operand::Copy(index) | Operand::Move(index),
        ..
    } = msg
    else {
        return None;
    };
    let local = index.as_local()?;
    let first = body.basic_blocks[target].statements.iter().find(|s| {
        !matches!(
            s.kind,
            StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop
        )
    })?;
    let StatementKind::Assign(assign) = &first.kind else {
        return None;
    };
    let (written, rvalue) = &**assign;
    let indexes = |p: &Place<'tcx>| p.projection.contains(&PlaceElem::Index(local));
    match rvalue {
        Rvalue::Use(Operand::Copy(read) | Operand::Move(read), _)
        | Rvalue::CopyForDeref(read)
        | Rvalue::Ref(_, BorrowKind::Shared, read)
            if indexes(read) && !indexes(written) =>
        {
            Some((local, false))
        }
        _ if written.projection.last() == Some(&PlaceElem::Index(local)) => Some((local, true)),
        _ => None,
    }
}

/// Whether `local` is only read, by its parts or lent shared, once it's
/// assigned: never moved, copied whole, nor lent `&mut`.
pub(super) fn only_read(body: &mir::Body<'_>, local: Local) -> bool {
    struct Reads {
        local: Local,
        only: bool,
    }
    impl<'tcx> Visitor<'tcx> for Reads {
        fn visit_place(&mut self, place: &Place<'tcx>, context: PlaceContext, _: mir::Location) {
            if place.local != self.local {
                return;
            }
            self.only &= match context {
                PlaceContext::NonUse(_) | PlaceContext::MutatingUse(MutatingUseContext::Store) => {
                    place.projection.is_empty()
                }
                PlaceContext::NonMutatingUse(
                    NonMutatingUseContext::SharedBorrow
                    | NonMutatingUseContext::FakeBorrow
                    | NonMutatingUseContext::Inspect
                    | NonMutatingUseContext::PlaceMention,
                ) => true,
                PlaceContext::NonMutatingUse(NonMutatingUseContext::Copy) => !place.projection.is_empty(),
                _ => false,
            };
        }
    }
    let mut reads = Reads { local, only: true };
    reads.visit_body(body);
    reads.only
}

/// Whether `local` is read in `from`, or a block it goes on to, before it's
/// assigned again: what it holds now is read there.
pub(super) fn read_from(body: &mir::Body<'_>, from: BasicBlock, local: Local) -> bool {
    /// The first thing a statement does with `local`: reads it, `Some(true)`,
    /// or assigns it, `Some(false)`.
    struct First {
        local: Local,
        found: Option<bool>,
    }
    impl<'tcx> Visitor<'tcx> for First {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            if local != self.local || self.found.is_some() {
                return;
            }
            self.found = match context {
                PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call) => Some(false),
                PlaceContext::NonUse(_) => None,
                _ => Some(true),
            };
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut next = vec![from];
    'blocks: while let Some(block) = next.pop() {
        if !seen.insert(block) {
            continue;
        }
        let data = &body.basic_blocks[block];
        for (index, statement) in data.statements.iter().enumerate() {
            let at = mir::Location {
                block,
                statement_index: index,
            };
            let mut first = First { local, found: None };
            // What's assigned is read first, then where it goes.
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    first.visit_rvalue(&assign.1, at);
                    if first.found.is_none() && assign.0.local == local {
                        first.found = Some(!assign.0.projection.is_empty());
                    }
                }
                _ => first.visit_statement(statement, at),
            }
            match first.found {
                Some(true) => return true,
                Some(false) => continue 'blocks,
                None => {}
            }
        }
        let mut first = First { local, found: None };
        let at = mir::Location {
            block,
            statement_index: data.statements.len(),
        };
        first.visit_terminator(data.terminator(), at);
        match first.found {
            Some(true) => return true,
            Some(false) => continue,
            None => next.extend(data.terminator().successors()),
        }
    }
    false
}

/// Whether `local` is assigned in `from`, or a block it goes on to.
pub(super) fn written_from(body: &mir::Body<'_>, from: BasicBlock, local: Local) -> bool {
    let mut seen = std::collections::HashSet::new();
    let mut next = vec![from];
    while let Some(block) = next.pop() {
        if !seen.insert(block) {
            continue;
        }
        let data = &body.basic_blocks[block];
        let writes = (data.statements.iter())
            .any(|s| matches!(&s.kind, StatementKind::Assign(a) if a.0.as_local() == Some(local)))
            || matches!(&data.terminator().kind, TerminatorKind::Call { destination, .. } if destination.as_local() == Some(local));
        if writes {
            return true;
        }
        next.extend(data.terminator().successors());
    }
    false
}

/// Whether what the statement at `at` makes, a temporary MIR reads once, a
/// `&(*node).v` through `local`, a reference written again, is read by
/// `takes`, a call of what keeps nothing of it, before `local` is written,
/// every way from `at` (ADR 0853): its place, read there, is the one it
/// was, so nothing need hold what `local` is meanwhile. What's written
/// there is MIR's assignment of `local`: borrowck lets no `&mut` to `local`,
/// a closure's capture of it, write it between, as `local` is read before.
pub(super) fn read_before_written<'tcx>(
    state: &State<'_, 'tcx>,
    (at, rvalue): (mir::Location, &Rvalue<'tcx>),
    local: Local,
    takes: impl Fn(&mir::Terminator<'tcx>, Local) -> bool,
) -> bool {
    /// Whether a statement or terminator names `local`.
    struct Names {
        local: Local,
        found: bool,
    }
    impl<'tcx> Visitor<'tcx> for Names {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            self.found |= local == self.local && !matches!(context, PlaceContext::NonUse(_));
        }
    }
    let body = state.body;
    let Some(StatementKind::Assign(assign)) = (body.basic_blocks[at.block].statements)
        .get(at.statement_index)
        .map(|s| &s.kind)
    else {
        return false;
    };
    let made = assign.0;
    // Of the statement being lowered: not a cleanup's, say, lowered apart.
    if !std::ptr::addr_eq(&assign.1, rvalue)
        || !made.projection.is_empty()
        || state.locals.user[made.local]
        || state.locals.reads[made.local] != 1
    {
        return false;
    }
    let names = |visit: &dyn Fn(&mut Names)| {
        let mut names = Names {
            local: made.local,
            found: false,
        };
        visit(&mut names);
        names.found
    };
    let mut seen = HashSet::new();
    let mut next = vec![(at.block, at.statement_index + 1)];
    while let Some((block, from)) = next.pop() {
        if !seen.insert((block, from)) {
            continue;
        }
        let data = &body.basic_blocks[block];
        for (index, statement) in data.statements.iter().enumerate().skip(from) {
            let here = mir::Location {
                block,
                statement_index: index,
            };
            if names(&|n| n.visit_statement(statement, here)) {
                return false;
            }
            if let StatementKind::Assign(a) = &statement.kind
                && a.0.local == local
                && a.0.projection.is_empty()
            {
                return false;
            }
        }
        let terminator = data.terminator();
        let here = mir::Location {
            block,
            statement_index: data.statements.len(),
        };
        if names(&|n| n.visit_terminator(terminator, here)) {
            if takes(terminator, made.local) {
                continue;
            }
            return false;
        }
        if let TerminatorKind::Call { destination, .. } = &terminator.kind
            && destination.local == local
        {
            return false;
        }
        next.extend(terminator.successors().map(|b| (b, 0)));
    }
    true
}

/// Where the program wrote what `block` does first: a macro's, where it
/// called it.
pub(super) fn written_at(body: &mir::Body<'_>, block: BasicBlock) -> Option<rustc_span::BytePos> {
    written_within(body, block, 4)
}

/// `written_at`'s, of a block that only tests further, a pattern's field,
/// `Done { ok: true }`'s: where the first of what it goes to is, its arms,
/// not the `match` its test is spanned by; `depth` such tests at most.
fn written_within(body: &mir::Body<'_>, block: BasicBlock, depth: usize) -> Option<rustc_span::BytePos> {
    let data = &body.basic_blocks[block];
    let first = (data.statements.iter()).find(|s| {
        !matches!(
            s.kind,
            StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop
                | StatementKind::PlaceMention(_)
        )
    });
    if first.is_none()
        && depth > 0
        && let TerminatorKind::SwitchInt { targets, .. } = &data.terminator().kind
    {
        return (targets.all_targets().iter())
            .filter(|&&b| !is_unreachable(body, b))
            .filter_map(|&b| written_within(body, b, depth - 1))
            .min();
    }
    let first = first
        .map_or(data.terminator().source_info.span, |s| s.source_info.span)
        .source_callsite();
    (!first.is_dummy()).then(|| first.lo())
}

pub(super) fn is_unreachable(body: &mir::Body<'_>, block: BasicBlock) -> bool {
    let data = &body.basic_blocks[block];
    matches!(data.terminator().kind, TerminatorKind::Unreachable)
        && data.statements.iter().all(|s| {
            matches!(
                s.kind,
                StatementKind::FakeRead(_)
                    | StatementKind::StorageLive(_)
                    | StatementKind::StorageDead(_)
                    | StatementKind::Nop
            )
        })
}

/// The bytes of a constant that's an array of them: a template's.
pub(super) fn expr_bytes(e: &Expr) -> Option<Vec<u8>> {
    match &e.kind {
        js::ExprKind::Array(items) => items
            .iter()
            .map(|item| match item.kind {
                js::ExprKind::Num(n) if (0.0..256.0).contains(&n) && n.fract() == 0.0 => Some(n as u8),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// A check of a `%` or `/`, which the division makes itself where it can fail,
/// `$rem(a, b, min)`, and can't where it's inline, `i % 2`: none.
/// An `Assert` that isn't an `if` that throws (`assert_check`).
pub(super) enum Check {
    /// Left out: it passes, or what runs next checks it.
    Unchecked,
    /// Made where the element, at this index, is read or, `true`, written.
    Deferred(Local, bool),
}

/// What a test is wherever it runs, where its constants say: a `bool`, or
/// a comparison of two numbers, as an array's constant index below its
/// constant length, `2 < 3`, or a constant divisor against 0, `2 == 0`, of
/// a local assigned it once; `!` of one; or `&` of two, as a signed
/// division's overflow, `2 == -1 & n == MIN`, is false of one.
pub(super) fn constant_truth<'tcx>(body: &mir::Body<'tcx>, operand: &Operand<'tcx>) -> Option<bool> {
    let assigned = |operand: &Operand<'tcx>| -> Option<&Rvalue<'tcx>> {
        let local = operand.place()?.as_local()?;
        let mut assigned = (body.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter_map(|s| match &s.kind {
                StatementKind::Assign(assign) if assign.0.as_local() == Some(local) => Some(&assign.1),
                _ => None,
            });
        match (assigned.next(), assigned.next()) {
            (Some(rvalue), None) => Some(rvalue),
            _ => None,
        }
    };
    // A number, as its type reads its bits.
    let number = |operand: &Operand<'tcx>| -> Option<i128> {
        let constant = match operand {
            Operand::Constant(constant) => constant,
            _ => match assigned(operand)? {
                Rvalue::Use(Operand::Constant(constant), _) => constant,
                _ => return None,
            },
        };
        let scalar = constant.const_.try_to_scalar_int()?;
        match constant.ty().is_signed() {
            true => Some(scalar.to_int(scalar.size())),
            false => i128::try_from(scalar.to_uint(scalar.size())).ok(),
        }
    };
    if let Operand::Constant(constant) = operand {
        return constant.const_.try_to_bool();
    }
    match assigned(operand)? {
        Rvalue::Use(operand, _) => constant_truth(body, operand),
        Rvalue::UnaryOp(mir::UnOp::Not, operand) => constant_truth(body, operand).map(|t| !t),
        Rvalue::BinaryOp(BinOp::BitAnd, operands) => {
            match (constant_truth(body, &operands.0), constant_truth(body, &operands.1)) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            }
        }
        Rvalue::BinaryOp(op, operands) => {
            let (a, b) = (number(&operands.0)?, number(&operands.1)?);
            match op {
                BinOp::Lt => Some(a < b),
                BinOp::Le => Some(a <= b),
                BinOp::Gt => Some(a > b),
                BinOp::Ge => Some(a >= b),
                BinOp::Eq => Some(a == b),
                BinOp::Ne => Some(a != b),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether an `Assert`, `msg`, is the check of a division, `/` or `%`, and
/// the division runs next, from `target`: through its other checks, with no
/// call nor a write through a pointer between, so the division's helper
/// throws its panic where the `Assert` would. A call between, `v[f()] = a %
/// b`'s, runs after the check, not before it.
pub(super) fn checked_by_division(body: &mir::Body<'_>, msg: &mir::AssertMessage<'_>, target: BasicBlock) -> bool {
    let of_division = |msg: &mir::AssertMessage<'_>| {
        matches!(
            msg,
            AssertKind::DivisionByZero(_)
                | AssertKind::RemainderByZero(_)
                | AssertKind::Overflow(BinOp::Div | BinOp::Rem, ..)
        )
    };
    if !of_division(msg) {
        return false;
    }
    let mut block = target;
    // A division's checks are two at most: by zero, then overflow.
    for _ in 0..3 {
        let data = &body.basic_blocks[block];
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind {
                let (written, rvalue) = &**assign;
                if matches!(rvalue, Rvalue::BinaryOp(BinOp::Div | BinOp::Rem, _)) {
                    return true;
                }
                if written.is_indirect() {
                    return false;
                }
            }
        }
        match &data.terminator().kind {
            TerminatorKind::Assert { msg, target, .. } if of_division(msg) => block = *target,
            TerminatorKind::Goto { target } => block = *target,
            _ => return false,
        }
    }
    false
}

pub(super) fn assert_operands<'m, 'tcx>(msg: &'m AssertKind<Operand<'tcx>>) -> Vec<&'m Operand<'tcx>> {
    match msg {
        AssertKind::BoundsCheck { len, index } => vec![len, index],
        AssertKind::Overflow(_, l, r) => vec![l, r],
        AssertKind::OverflowNeg(o) | AssertKind::DivisionByZero(o) | AssertKind::RemainderByZero(o) => vec![o],
        _ => Vec::new(),
    }
}

pub(super) fn count_operand(locals: &mut Locals, operand: &Operand<'_>) {
    if let Operand::Copy(place) | Operand::Move(place) = operand {
        count_place(locals, place, false);
        if let [PlaceElem::Field(field, _)] = place.projection[..] {
            locals.fields[place.local].push(field.as_usize());
        }
    }
}

/// A place's uses: its base read, unless it's written whole, and each
/// index read.
pub(super) fn count_place(locals: &mut Locals, place: &Place<'_>, written: bool) {
    if written && place.projection.is_empty() {
        locals.writes[place.local] += 1;
    } else {
        locals.reads[place.local] += 1;
        // A part of its own written: the local is a variable. Through a
        // reference, it's what it refers to that's written.
        if written && !place.is_indirect() {
            locals.borrowed[place.local] = true;
        }
    }
    for elem in place.projection {
        if let PlaceElem::Index(index) = elem {
            locals.reads[index] += 1;
        }
    }
}

pub(super) fn count_rvalue(locals: &mut Locals, rvalue: &Rvalue<'_>, frozen: bool, itself: bool) {
    match rvalue {
        Rvalue::Use(o, _) | Rvalue::Repeat(o, _) | Rvalue::Cast(_, o, _) | Rvalue::UnaryOp(_, o) => {
            count_operand(locals, o)
        }
        Rvalue::BinaryOp(_, operands) => {
            count_operand(locals, &operands.0);
            count_operand(locals, &operands.1);
        }
        Rvalue::Aggregate(_, operands) => operands.iter().for_each(|o| count_operand(locals, o)),
        Rvalue::Ref(_, kind, place) => {
            count_place(locals, place, false);
            if let [PlaceElem::Field(field, _), PlaceElem::Deref] = place.projection[..]
                && matches!(kind, BorrowKind::Shared)
            {
                locals.fields[place.local].push(field.as_usize());
            }
            // The whole borrowed mutably may be written through the borrow: a
            // `let`, which a closure that captured it writes.
            if matches!(kind, BorrowKind::Mut { .. }) && place.projection.is_empty() && !itself {
                locals.writes[place.local] += 1;
            }
            if !place.projection.is_empty() && !place.is_indirect() && !frozen {
                locals.borrowed[place.local] = true;
            }
        }
        Rvalue::RawPtr(_, place) => {
            count_place(locals, place, false);
            // A part of its own borrowed keeps its local a variable; the
            // whole borrowed is the value itself, read once, and a reborrow,
            // `&*r`, borrows what `r` refers to.
            if !place.projection.is_empty() && !place.is_indirect() {
                locals.borrowed[place.local] = true;
            }
        }
        Rvalue::Discriminant(place) | Rvalue::CopyForDeref(place) => count_place(locals, place, false),
        _ => {}
    }
}

/// The constant a promoted body refers to, `_1 = const LOG; _0 = &_1`, if
/// that's all it is.
pub(super) fn referred_constant<'tcx>(body: &mir::Body<'tcx>) -> Option<ConstOperand<'tcx>> {
    if body.basic_blocks.len() != 1 {
        return None;
    }
    let data = &body.basic_blocks[mir::START_BLOCK];
    let mut constant = None;
    let mut referred = None;
    for statement in &data.statements {
        match &statement.kind {
            StatementKind::Assign(assign) => match &**assign {
                (place, Rvalue::Use(Operand::Constant(c), ..)) if place.projection.is_empty() => {
                    constant = Some((place.local, **c));
                }
                (place, Rvalue::Ref(_, BorrowKind::Shared, target))
                    if place.local == mir::RETURN_PLACE && target.projection.is_empty() =>
                {
                    referred = Some(target.local);
                }
                _ => return None,
            },
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
            _ => return None,
        }
    }
    let (local, c) = constant?;
    (Some(local) == referred).then_some(c)
}

/// The constant a promoted `&Some(c)` holds, `_1 = Some(const c); _0 =
/// &_1`, if that's all it is: a `Some` always, of `C` by its name where
/// it's a named one (ADR 0771).
pub(super) fn promoted_some<'tcx>(tcx: ty::TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> Option<ConstOperand<'tcx>> {
    if body.basic_blocks.len() != 1 {
        return None;
    }
    let mut some = None;
    let mut referred = None;
    for statement in &body.basic_blocks[mir::START_BLOCK].statements {
        match &statement.kind {
            StatementKind::Assign(assign) => match &**assign {
                (place, Rvalue::Aggregate(kind, operands))
                    if place.projection.is_empty()
                        && let mir::AggregateKind::Adt(did, variant, ..) = **kind
                        && tcx.is_lang_item(did, LangItem::Option)
                        && variant.as_u32() == 1
                        && let [Operand::Constant(c)] = &operands.raw[..] =>
                {
                    some = Some((place.local, **c));
                }
                (place, Rvalue::Ref(_, BorrowKind::Shared, target))
                    if place.local == mir::RETURN_PLACE && target.projection.is_empty() =>
                {
                    referred = Some(target.local);
                }
                _ => return None,
            },
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
            _ => return None,
        }
    }
    let (made, inside) = some?;
    (referred == Some(made)).then_some(inside)
}

/// Whether `local` is given one value, and only one: assigned once, or a
/// parameter never assigned, whose argument it holds throughout.
fn given_once(state: &State<'_, '_>, local: Local) -> bool {
    state.locals.writes[local] + usize::from(is_parameter(state, local)) == 1
}

/// Whether `place` is of variables each given one value, and constants, a
/// `const` read by its name too (`is_fixed`):
/// `lanes[index]` of a parameter `index` and a `const lanes`, which
/// nothing evaluated meanwhile can make another place.
pub(super) fn settled_place(state: &State<'_, '_>, place: &Expr) -> bool {
    let settled = |name: &str| {
        let named = || (state.locals.names.iter_enumerated()).filter(|&(_, n)| n == name);
        named().next().is_some() && named().all(|(local, _)| given_once(state, local))
    };
    match &place.kind {
        js::ExprKind::Var(name) => settled(name),
        js::ExprKind::Member(object, _) => settled_place(state, object),
        js::ExprKind::Index(object, index) => {
            settled_place(state, object)
                && (index.is_fixed() || matches!(&index.kind, js::ExprKind::Var(name) if settled(name)))
        }
        _ => false,
    }
}

/// Whether `local` is written again after it's first given its value, a
/// reference reassigned, so a place through it isn't the one it was. A
/// parameter's first value is its argument.
pub(super) fn written_again(state: &State<'_, '_>, local: Local) -> bool {
    state.locals.writes[local] + usize::from(is_parameter(state, local)) > 1
}

/// Whether `local` may read differently where it's read: written again
/// after its first value, a parameter's its argument; but for a variable
/// the program names assigned once each way, as a `match`'s arms each
/// assign `let variant = match ..`: not MIR's, a `vec!`'s box, say, filled
/// through it after.
pub(super) fn written_apart(state: &State<'_, '_>, local: Local) -> bool {
    written_again(state, local)
        && !(state.locals.user[local] && !is_parameter(state, local) && once_each_way(state.body, local))
}

/// Whether `local` is a parameter, whose first value its caller gives.
pub(super) fn is_parameter(state: &State<'_, '_>, local: Local) -> bool {
    state.body.args_iter().any(|a| a == local)
}

/// Whether `place` names the same place wherever it's read: variables and
/// their fields, and elements at an index that's a constant, a `const` read
/// by its name (`is_fixed`, ADR 0894, amended), or a variable
/// given one value, a parameter never assigned too.
pub(super) fn same_place(state: &State<'_, '_>, place: &Expr) -> bool {
    match &place.kind {
        js::ExprKind::Var(_) => true,
        js::ExprKind::Member(object, _) => same_place(state, object),
        js::ExprKind::Index(object, index) => {
            same_place(state, object)
                && (index.is_fixed()
                    || from_end(object, index)
                    || matches!(&index.kind, js::ExprKind::Var(name)
                        if state.locals.names.iter_enumerated().any(|(l, n)| n == name && given_once(state, l))))
        }
        _ => false,
    }
}

/// The variable a place is of, `route` of `route.title[0]`.
pub(super) fn root_var(place: &Expr) -> Option<&str> {
    match &place.kind {
        js::ExprKind::Var(name) => Some(name),
        js::ExprKind::Member(object, _) | js::ExprKind::Index(object, _) => root_var(object),
        _ => None,
    }
}

/// `if (test) { then } else { otherwise }`, written with the branch that
/// returns or throws first and the other after it, as a person writes an
/// early exit: the same, as that branch never goes on. Of two that do, the
/// program's first. Of the branches,
/// `panics` says which panic: one that does is first, as a person writes a
/// check.
pub(super) fn early_exit(
    test: Expr,
    then: Vec<Stmt>,
    otherwise: Option<Vec<Stmt>>,
    panics: [bool; 2],
    span: js::Span,
) -> Vec<Stmt> {
    // Or a `break` the program wrote, of a block it labeled: where it's
    // written, the span it has (ADR 0790).
    let exits = |stmts: &[Stmt]| match stmts.last() {
        Some(Stmt {
            kind: StmtKind::Break(Some(_)),
            span,
        }) => !span.is_none(),
        last => matches!(last.map(|s| &s.kind), Some(StmtKind::Return(_) | StmtKind::Throw(_))),
    };
    match otherwise {
        Some(otherwise) if panics == [false, true] => {
            early_exit(test.negated_test(), otherwise, Some(then), [true, false], span)
        }
        Some(otherwise) if panics == [true, false] || exits(&then) => {
            let mut out = vec![StmtKind::If(test, then, None).at(span)];
            out.extend(otherwise);
            out
        }
        Some(otherwise) if exits(&otherwise) => {
            let mut out = vec![StmtKind::If(test.negated_test(), otherwise, None).at(span)];
            out.extend(then);
            out
        }
        otherwise => vec![StmtKind::If(test, then, otherwise).at(span)],
    }
}

/// Whether `block` goes on to a call that never returns, a panic's, through
/// blocks that only go on: the message it's given made on the way.
pub(super) fn panics(body: &mir::Body<'_>, mut block: BasicBlock) -> bool {
    // A `loop {}` goes on forever.
    for _ in 0..body.basic_blocks.len() {
        match body.basic_blocks[block].terminator().kind {
            TerminatorKind::Call { target: None, .. } => return true,
            TerminatorKind::Goto { target }
            | TerminatorKind::Call {
                target: Some(target), ..
            }
            | TerminatorKind::Drop { target, .. } => block = target,
            _ => return false,
        }
    }
    false
}

/// Whether `block` only returns, through blocks that do nothing but go on.
pub(super) fn is_return(body: &mir::Body<'_>, mut block: BasicBlock) -> bool {
    // A `loop {}` goes on forever.
    for _ in 0..body.basic_blocks.len() {
        let data = &body.basic_blocks[block];
        let quiet = data.statements.iter().all(|s| {
            matches!(
                s.kind,
                StatementKind::StorageDead(_) | StatementKind::StorageLive(_) | StatementKind::Nop
            )
        });
        match data.terminator().kind {
            TerminatorKind::Return => return quiet,
            TerminatorKind::Goto { target } if quiet => block = target,
            _ => return false,
        }
    }
    false
}

/// Whether `place` is one a handle can read and write wherever it's read:
/// variables, their fields, and elements at a constant index or one a
/// variable holds, which an index is read into where it's checked.
/// Of `place`, a part of a string's `value`, `$strPart(s, a, b).value` of a
/// handle on a place `s` always is: `s`, and the part's bounds.
pub(super) fn str_part_of(place: &Expr) -> Option<(Expr, Vec<Expr>)> {
    let js::ExprKind::Member(part, field) = &place.kind else {
        return None;
    };
    let js::ExprKind::Call(f, args) = &part.kind else {
        return None;
    };
    let (js::ExprKind::Var(name), "value") = (&f.kind, field.as_str()) else {
        return None;
    };
    let (js::ExprKind::Handle(whole, _), "$strPart") = (&args.first()?.kind, name.as_str()) else {
        return None;
    };
    let bounds = args[1..].to_vec();
    (fixed_place(whole) && bounds.iter().all(|b| !b.has_effects())).then(|| ((**whole).clone(), bounds))
}

/// Whether `local` is of a function MIR put in this body, std's, not the
/// program's own.
pub(super) fn inlined(body: &mir::Body<'_>, local: Local) -> bool {
    let scope = &body.source_scopes[body.local_decls[local].source_info.scope];
    scope.inlined.is_some() || scope.inlined_parent_scope.is_some()
}

/// A place whose parts are names and constant indexes: what it is stays
/// put while what it's of is borrowed, as no variable an index reads may
/// change it meanwhile.
pub(super) fn steady_place(place: &Expr) -> bool {
    match &place.kind {
        js::ExprKind::Var(_) => true,
        js::ExprKind::Member(object, _) => steady_place(object),
        js::ExprKind::Index(object, index) => steady_place(object) && index.is_constant(),
        // An `Option`'s boxed `Some`'s value, `$someValue(o)`, read where it's used.
        js::ExprKind::Call(..) => crate::lower::std_types::option::some_value_of(place).is_some_and(steady_place),
        _ => false,
    }
}

pub(super) fn fixed_place(place: &Expr) -> bool {
    match &place.kind {
        js::ExprKind::Var(_) => true,
        js::ExprKind::Member(object, _) => fixed_place(object),
        js::ExprKind::Index(object, index) => {
            fixed_place(object)
                && (index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)) || from_end(object, index))
        }
        _ => false,
    }
}

/// Whether `index` counts back from the end of `object`, `xs.length - 1`,
/// a slice pattern's suffix: `object`'s length can't change while an item
/// of it is borrowed.
fn from_end(object: &Expr, index: &Expr) -> bool {
    matches!(&index.kind, js::ExprKind::Binary(Op::Sub, length, back)
        if back.is_constant()
            && matches!(&length.kind, js::ExprKind::Member(of, field) if field == "length" && of.written_alike(object)))
}

/// Whether a closure's `body` writes what it captured `i`th: assigns it,
/// or takes a `&mut` to it, through its environment `_1`.
pub(super) fn writes_capture(body: &mir::Body<'_>, i: usize) -> bool {
    let env = Local::from_usize(1);
    let captured = |place: &Place<'_>| {
        place.local == env
            && place
                .projection
                .iter()
                .find(|elem| !matches!(elem, PlaceElem::Deref))
                .is_some_and(|elem| matches!(elem, PlaceElem::Field(field, _) if field.as_usize() == i))
    };
    body.basic_blocks.iter().any(|data| {
        data.statements.iter().any(|statement| match &statement.kind {
            StatementKind::Assign(assign) => {
                let (place, rvalue) = &**assign;
                captured(place)
                    || matches!(rvalue, Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) if captured(borrowed))
            }
            _ => false,
        }) || matches!(&data.terminator().kind, TerminatorKind::Call { destination, .. } if captured(destination))
    })
}

/// The locals a terminator reads, as its operands.
pub(super) fn terminator_reads(terminator: &mir::Terminator<'_>) -> Vec<Local> {
    let operand = |o: &Operand<'_>| o.place().map(|p| p.local);
    match &terminator.kind {
        TerminatorKind::SwitchInt { discr, .. } => operand(discr).into_iter().collect(),
        TerminatorKind::Call { func, args, .. } => std::iter::once(func)
            .chain(args.iter().map(|a| &a.node))
            .filter_map(operand)
            .collect(),
        TerminatorKind::Assert { cond, .. } => operand(cond).into_iter().collect(),
        TerminatorKind::Drop { place, .. } => vec![place.local],
        _ => Vec::new(),
    }
}

/// Whether `local` is read only by its fields, each once, as an operand.
pub(super) fn split(locals: &Locals, local: Local) -> bool {
    let fields = &locals.fields[local];
    let mut distinct = fields.clone();
    distinct.sort();
    distinct.dedup();
    !fields.is_empty() && fields.len() == locals.reads[local] && distinct.len() == fields.len()
}

/// What `e` is a part of, through fields and constant indices, `x` of
/// `x[1].k`: itself where it's no part.
pub(super) fn call_part(e: &Expr) -> &Expr {
    match &e.kind {
        js::ExprKind::Member(object, _) => call_part(object),
        js::ExprKind::Index(object, index) if index.is_constant() => call_part(object),
        _ => e,
    }
}
