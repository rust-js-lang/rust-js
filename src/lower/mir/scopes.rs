//! A function's scope struct's methods, functions declared in it (ADR
//! 1043): each lowered as a body inside the function's, closing over the
//! variable its value is, its `self`, which no call gives it.

use super::*;
use std::collections::{HashMap, HashSet};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of the function being lowered, the lowering of each of its scope
    /// structs' methods it declares: the one a call of it given the
    /// literal's type arguments calls, as this lowering takes its own (ADR
    /// 0051, amended, #500). The function's own JS declares the methods'
    /// own; a copy of it, their copies its calls need. One whose call needs
    /// a copy none was made of is called nowhere it can be (`scope_called`).
    pub(in crate::lower) fn nest_scope_copies(&mut self) {
        let params = self.krate.given.js_given.params(self.tcx, self.item, self.copy);
        let recognition = self.recognition();
        let mut copies = HashMap::new();
        for scope in self.krate.scopes.of_function(self.item) {
            for &method in &scope.methods {
                if let Ok(copy) = (self.krate.given.js_given).called(&recognition, &params, (method, scope.args)) {
                    copies.insert(method, copy);
                }
            }
        }
        self.scope_copies = copies;
    }

    /// Of the body `state` lowers, its scope structs' methods, if it's a
    /// function with any (`analysis::Scopes`), its `own` body, not a
    /// closure's, each a function declared in `out` after the statements
    /// its value's `let` is.
    pub(super) fn declare_scope_functions(&mut self, state: &State<'_, 'tcx>, own: bool, out: &mut Vec<Stmt>) -> R<()> {
        let krate = self.krate;
        for scope in (krate.scopes.of_function(state.body.source.def_id())).filter(|_| own) {
            let value = state.locals.names[scope.local].clone();
            let mut functions = Vec::new();
            for &method in &scope.methods {
                functions.push(self.scope_function(method, &value)?);
            }
            let written = self.js_span(scope.written);
            let within = |s: &Stmt| !s.span.is_none() && s.span.lo >= written.lo && s.span.hi <= written.hi;
            // After the statements its `let` is, in the list they're in.
            let mut placed = false;
            js::statement_lists(out, &mut |list| {
                if !placed && let Some(last) = list.iter().rposition(within) {
                    list.splice(last + 1..last + 1, std::mem::take(&mut functions));
                    placed = true;
                }
            });
            if !placed {
                return Err(self.unsupported(scope.written, "a scope struct's `let`, lowered to no statement"));
            }
            // Its fields, made the function's variables once tidy has read
            // its body (`scope_fields`).
            self.scope_values.push((scope.function, value));
        }
        Ok(())
    }

    /// `method`, a scope struct's, a function declaration closing over
    /// `value`, its `self`.
    fn scope_function(&mut self, method: rustc_span::def_id::DefId, value: &str) -> R<Stmt> {
        let span = self.tcx.def_span(method);
        let body = (self.krate.bodies.get(&method).copied())
            .ok_or_else(|| self.unsupported(span, "a scope struct's method, without its body"))?;
        let mir =
            (body.mir.as_ref()).ok_or_else(|| self.unsupported(span, "a scope struct's method, without its MIR"))?;
        // The lowering of it its function's calls (#500), which its body
        // takes its type parameters as (`lowered_params`).
        if !self.scope_copies.contains_key(&method) {
            return Err(self.unsupported(span, "a scope struct's method, of no lowering its function calls"));
        }
        // Its own typing environment, and no copy of its function's; its
        // names apart from every one of its function's, which it may read.
        let typing_env = std::mem::replace(&mut self.typing_env, ty::TypingEnv::post_analysis(self.tcx, method));
        let copy = self.copy.take();
        self.scope_self = Some(value.to_string());
        let nested = super::super::Nested::Closure {
            names: self.names.clone(),
        };
        let mut stmts = Vec::new();
        let lowered = self.lowered_in(body, method, nested, |cx| cx.mir_signature(body, mir, &mut stmts));
        self.scope_self = None;
        self.typing_env = typing_env;
        self.copy = copy;
        let ((mut params, is_async), hoisted) = lowered?;
        stmts.splice(0..0, hoisted);
        // Its `self`, the variable it closes over.
        match params.first() {
            Some(js::Pattern::Name(name)) if name == value => {
                params.remove(0);
            }
            _ => return Err(self.unsupported(span, "a scope struct's method whose `self` isn't its value")),
        }
        let local = method.expect_local();
        let whole = self.tcx.hir_span_with_body(self.tcx.local_def_id_to_hir_id(local));
        let function = js::Function {
            name: self.scope_functions[&method].clone(),
            params,
            body: stmts,
            export: false,
            is_async,
            noinline: false,
            span: self.js_span(function_start(self.tcx, whole, self.tcx.def_ident_span(method))),
            name_span: (self.tcx.def_ident_span(method)).map_or(js::Span::NONE, |s| self.js_span(s)),
            param_spans: Vec::new(),
            set_apart: true,
        };
        Ok(StmtKind::Function(Box::new(function)).at(self.js_span(span)))
    }
}

/// Of a function's body, `out`, its scope struct's value, `value`, made the
/// function's variables, one of each field, where nothing reads it whole
/// (ADR 1043, amended, B2): `reply.nextPartId = id + 1` is `nextPartId =
/// id + 1`, as upstream's processReply has its state. Its value's `const`
/// is its fields' `let`s, in its literal's order, a `const` of a field
/// nothing writes, and one a parameter gives that nothing writes is that
/// parameter; each read of a field is its variable's. Where anything reads
/// the value but a field of it, or names its variable again, it stays one.
pub(super) fn scope_fields(value: &str, params: &[js::Pattern], out: &mut Vec<Stmt>) {
    // Every list of statements, closures' and declared functions' too.
    let mut lists: Vec<Vec<Stmt>> = Vec::new();
    js::each_block_mut(&mut out.clone(), &mut |list| lists.push(list.clone()));
    let statements = || lists.iter().flatten();
    // What holds it: its variable, and each `const` of it, a closure's.
    let mut holders = vec![value.to_string()];
    loop {
        let more: Vec<String> = (statements())
            .filter_map(|stmt| match &stmt.kind {
                StmtKind::Const(alias, e) if !holders.contains(alias) => match &e.kind {
                    js::ExprKind::Var(of) if holders.contains(of) => Some(alias.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if more.is_empty() {
            break;
        }
        holders.extend(more);
    }
    let held = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(name) if holders.contains(name));
    // Its literal, the one statement that makes it.
    let literal = statements().find_map(|stmt| match &stmt.kind {
        StmtKind::Const(name, e) | StmtKind::Let(name, Some(e)) if name == value => match &e.kind {
            js::ExprKind::Object(props, _) => Some(props.clone()),
            _ => None,
        },
        _ => None,
    });
    let Some(props) = literal else { return };
    let mut fields: Vec<(String, Option<Expr>)> = Vec::new();
    for prop in props {
        match prop {
            js::Prop::Field(key, e) => fields.push((key, Some(e))),
            _ => return,
        }
    }
    // Each read of it is a field's.
    let (mut reads, mut fields_read, mut written) = (0, 0, HashSet::new());
    let mut whole = false;
    js::each_expr(out, &mut |e| match &e.kind {
        js::ExprKind::Var(name) if holders.contains(name) => reads += 1,
        // A field's read, its literal's own: a `None` is written there,
        // `undefined`, until preparation leaves its key out.
        js::ExprKind::Member(of, key) if held(of) => {
            fields_read += 1;
            whole |= !fields.iter().any(|(f, _)| f == key);
        }
        js::ExprKind::Unary(js::UnaryOp::Delete, of) if matches!(&of.kind, js::ExprKind::Member(of, _) if held(of)) => {
            whole = true
        }
        // A handle on a field writes it.
        js::ExprKind::Handle(place, _) => place.each(&mut |e| {
            if let js::ExprKind::Member(of, key) = &e.kind
                && held(of)
            {
                written.insert(key.clone());
            }
        }),
        _ => {}
    });
    let mut aliases = 0;
    for stmt in statements() {
        match &stmt.kind {
            StmtKind::Const(alias, e) if alias != value && holders.contains(alias) && held(e) => aliases += 1,
            StmtKind::Assign(target, _) | StmtKind::Compound(_, target, _) => match &target.kind {
                js::ExprKind::Member(of, key) if held(of) => {
                    written.insert(key.clone());
                }
                js::ExprKind::Var(name) if holders.contains(name) => whole = true,
                _ => {}
            },
            _ => {}
        }
    }
    // Each holder declared once, never named again.
    let mut declared = 0;
    js::each_declared(out, &mut |name| {
        declared += usize::from(holders.iter().any(|h| h == name))
    });
    let rebound = params
        .iter()
        .flat_map(|p| p.names())
        .any(|name| holders.iter().any(|h| h == name));
    if whole || rebound || reads != fields_read + aliases || declared != holders.len() {
        return;
    }
    // A copy of a field the program names as the field, read before
    // anything can set it again: the field, so its variable keeps its name.
    js::each_block_mut(out, &mut |list| copies_read_through(list, &held));
    // Each field's variable, named apart from every name the body has.
    let mut taken = HashSet::new();
    js::names_in(out, &mut taken);
    taken.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
    // A parameter nothing assigns, nor declares again, a nested
    // function's own of its name: what it's read as everywhere in the body.
    let mut declared = HashSet::new();
    js::each_declared(out, &mut |name| {
        declared.insert(name.to_string());
    });
    let mut handled = HashSet::new();
    js::each_expr(out, &mut |e| {
        if let js::ExprKind::Handle(place, _) = &e.kind {
            place.each(&mut |e| {
                if let js::ExprKind::Var(name) = &e.kind {
                    handled.insert(name.clone());
                }
            });
        }
    });
    let assigned = |name: &str| declared.contains(name) || handled.contains(name) || js::Writes::sets(out, name);
    let param_names: Vec<&str> = params.iter().flat_map(|p| p.names()).collect();
    let mut names: HashMap<String, String> = HashMap::new();
    let mut lets = Vec::new();
    for (key, e) in fields {
        let given = e.as_ref().and_then(|e| match &e.kind {
            js::ExprKind::Var(p) if param_names.contains(&p.as_str()) && !assigned(p) && !written.contains(&key) => {
                Some(p.clone())
            }
            _ => None,
        });
        let name = match given {
            Some(p) => p,
            None => {
                let name = crate::names::fresh_in(&mut taken, &key);
                // Where its value is written, which a comment above the
                // field is before; the first where the literal's `let` is.
                let at = e.as_ref().map_or(js::Span::NONE, |e| e.span);
                lets.push(
                    match (written.contains(&key), e) {
                        (false, Some(e)) => StmtKind::Const(name.clone(), e),
                        (_, e) => StmtKind::Let(name.clone(), e),
                    }
                    .at(at),
                );
                name
            }
        };
        names.insert(key, name);
    }
    // Its reads made its variables', its `const`s and their aliases gone.
    js::each_expr_mut(out, &mut |e| {
        if let js::ExprKind::Member(of, key) = &e.kind
            && held(of)
        {
            let span = e.span;
            *e = Expr::var(&names[key]);
            e.span = span;
        }
    });
    let mut lets = Some(lets);
    js::each_block_mut(out, &mut |list| {
        let mut i = 0;
        while i < list.len() {
            match &list[i].kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, Some(_)) if name == value => {
                    let span = list[i].span;
                    let mut made = lets.take().unwrap_or_default();
                    for (k, stmt) in made.iter_mut().enumerate() {
                        if k == 0 || stmt.span.is_none() {
                            stmt.span = span;
                        }
                    }
                    let n = made.len();
                    list.splice(i..=i, made);
                    i += n;
                }
                StmtKind::Const(name, _) if holders.contains(name) => {
                    list.remove(i);
                }
                _ => i += 1,
            }
        }
    });
}

/// Of `list`, a copy of a field of the scope's value, `held` its holders,
/// the program's of the field's name: `const at = reply.at;`, an `if let
/// Some(at) = self.at.get()`'s, or `const at = e; reply.at = at;`, a
/// `let at = e; self.at.set(Some(at));`'s, each read of `at` the field's,
/// `reply.at = e;`, where every read is before anything that may set the
/// field again, a call of one of the scope's functions, or a getter's:
/// `copied_until_effects`. So its variable, named after the field, isn't
/// named apart from the copy, `at$1`, as upstream's processReply's
/// `replyLifetimeController = new AbortController();` (ADR 1043, amended).
fn copies_read_through(list: &mut Vec<Stmt>, held: &dyn Fn(&Expr) -> bool) {
    let mut i = 0;
    while i < list.len() {
        let next = list.get(i + 1).map(|s| &s.kind);
        let found = match (&list[i].kind, next) {
            (StmtKind::Const(a, e), _) if matches!(&e.kind, js::ExprKind::Member(of, key) if held(of) && key == a) => {
                Some((a.clone(), e.clone(), None))
            }
            (StmtKind::Const(a, e), Some(StmtKind::Assign(target, value)))
                if matches!(&target.kind, js::ExprKind::Member(of, key) if held(of) && key == a)
                    && matches!(&value.kind, js::ExprKind::Var(v) if v == a) =>
            {
                Some((a.clone(), target.clone(), Some(e.clone())))
            }
            _ => None,
        };
        let Some((name, field, made)) = found else {
            i += 1;
            continue;
        };
        let rest = i + 1 + usize::from(made.is_some());
        let mut declared = false;
        js::each_declared(&list[rest..], &mut |n| declared |= n == name);
        if declared || !copied_until_effects(&list[rest..], &name) {
            i += 1;
            continue;
        }
        js::each_expr_mut(&mut list[rest..], &mut |e| {
            if matches!(&e.kind, js::ExprKind::Var(v) if *v == name) {
                *e = Expr {
                    span: e.span,
                    ..field.clone()
                };
            }
        });
        let span = list[i].span;
        match made {
            None => {
                list.remove(i);
            }
            Some(made) => {
                list.splice(i..i + 2, [StmtKind::Assign(field, made).at(span)]);
                i += 1;
            }
        }
    }
}

/// Whether each read of `name` in `stmts`, as JS runs them, is before
/// anything that may run other code: a call that may have effects, a
/// getter's read, a write but a variable's; none in a closure, which runs
/// later, nor in a statement whose order isn't followed here.
fn copied_until_effects(stmts: &[Stmt], name: &str) -> bool {
    let mut order = Order {
        name,
        seen: 0,
        stopped: false,
    };
    order.stmts(stmts);
    order.seen == js::mentions_in(stmts, name)
}

/// `copied_until_effects`' walk: the reads of `name` seen, until what may
/// run other code stops it.
struct Order<'n> {
    name: &'n str,
    seen: usize,
    stopped: bool,
}

impl Order<'_> {
    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if self.stopped {
                return;
            }
            match &stmt.kind {
                StmtKind::Expr(e) | StmtKind::Return(Some(e)) | StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) => {
                    self.expr(e)
                }
                // Each branch from its test, a path of its own: what's after
                // is after what either does.
                StmtKind::If(test, then, els) => {
                    self.expr(test);
                    let (seen, stopped) = (self.seen, self.stopped);
                    self.stmts(then);
                    let then = (self.seen - seen, self.stopped);
                    (self.seen, self.stopped) = (seen, stopped);
                    self.stmts(els.as_deref().unwrap_or_default());
                    self.seen += then.0;
                    self.stopped |= then.1;
                }
                StmtKind::Assign(target, value) if matches!(target.kind, js::ExprKind::Var(_)) => {
                    self.expr(value);
                    self.stopped |= target.mentions_var(self.name);
                }
                StmtKind::Let(_, None) | StmtKind::Return(None) | StmtKind::Comments => {}
                StmtKind::Function(_) if js::mentions_in(std::slice::from_ref(stmt), self.name) == 0 => {}
                _ => self.stopped = true,
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        if self.stopped {
            return;
        }
        match &e.kind {
            js::ExprKind::Var(v) => self.seen += usize::from(v == self.name),
            js::ExprKind::Member(of, _) | js::ExprKind::OptionalMember(of, _) => {
                self.expr(of);
                self.stopped |= e.facts.has(js::Known::Getter);
            }
            js::ExprKind::Call(callee, args) => {
                self.expr(callee);
                args.iter().for_each(|arg| self.expr(arg));
                self.stopped |= e.has_effects();
            }
            js::ExprKind::Template(_, items, _) | js::ExprKind::Array(items) => items.iter().for_each(|i| self.expr(i)),
            // An object's fields, in order, `{ signal: controller.signal }`.
            js::ExprKind::Object(props, ..)
                if props
                    .iter()
                    .all(|p| matches!(p, js::Prop::Field(..) | js::Prop::Spread(_))) =>
            {
                for prop in props {
                    if let js::Prop::Field(_, value) | js::Prop::Spread(value) = prop {
                        self.expr(value);
                    }
                }
            }
            js::ExprKind::Binary(_, a, b) => {
                self.expr(a);
                self.expr(b);
            }
            js::ExprKind::Cond(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            js::ExprKind::Unary(op, a) => {
                self.expr(a);
                self.stopped |= op.writes();
            }
            // Anything else, of an order not followed here: stopped where it
            // reads `name` or may do anything, a closure made reads nothing.
            _ => self.stopped |= e.mentions_var(self.name) || e.has_effects(),
        }
    }
}
