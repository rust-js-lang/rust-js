//! Our statements as oxc's: a function, its parameters and body, a
//! namespace, a constant, a pattern, and each statement.

use super::*;

impl<'a> Cx<'a> {
    pub(super) fn params(&self, kind: FormalParameterKind, patterns: &[js::Pattern]) -> FormalParameters<'a> {
        self.params_at(kind, patterns, &[])
    }

    /// `params`, each mapped to where it's written, of `spans`, one each
    /// or none: a comment before one is before it (ADR 0708, amended).
    pub(super) fn params_at(
        &self,
        kind: FormalParameterKind,
        patterns: &[js::Pattern],
        spans: &[js::Span],
    ) -> FormalParameters<'a> {
        let b = &self.b;
        // The last, `...args`, gathers the rest (ADR 0428).
        let (rest, patterns) = match patterns.split_last() {
            Some((js::Pattern::Rest(name), before)) => (Some(name), before),
            _ => (None, patterns),
        };
        let rest = rest.map(|name| {
            let binding = BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
            ArenaBox::new_in(
                FormalParameterRest::new(
                    SPAN,
                    ArenaVec::new_in(b),
                    BindingRestElement::new(SPAN, binding, b),
                    None,
                    b,
                ),
                b,
            )
        });
        let params = patterns.iter().enumerate().map(|(i, pattern)| {
            let at = spans.get(i).map_or(SPAN, |&s| span(s));
            // `location = { .. }`, where it's given none (ADR 0427).
            let (pattern, default) = match pattern {
                js::Pattern::Defaulted(name, default) => (
                    BindingPattern::new_binding_identifier(at, self.name(name), b),
                    Some(ArenaBox::new_in(self.expr(default), b)),
                ),
                js::Pattern::Name(name) => (BindingPattern::new_binding_identifier(at, self.name(name), b), None),
                pattern => (self.pattern(pattern), None),
            };
            FormalParameter::new(
                SPAN,
                ArenaVec::new_in(b),
                pattern,
                None,
                default,
                false,
                None,
                false,
                false,
                b,
            )
        });
        FormalParameters::new(SPAN, kind, ArenaVec::from_iter_in(params, b), rest, b)
    }

    /// A function's or an arrow's body: its directives, `"use cache";`, in
    /// its prologue (ADR 0349), then its statements.
    pub(super) fn function_body(&self, body: &[js::Stmt]) -> FunctionBody<'a> {
        let b = &self.b;
        let directives = body.iter().filter_map(|s| match &s.kind {
            StmtKind::Directive(directive) => {
                let directive = self.name(directive);
                Some(Directive::new(
                    SPAN,
                    StringLiteral::new(SPAN, directive, None, b),
                    directive,
                    b,
                ))
            }
            _ => None,
        });
        let directives = ArenaVec::from_iter_in(directives, b);
        let stmts: Vec<js::Stmt> = (body.iter())
            .filter(|s| !matches!(s.kind, StmtKind::Directive(_)))
            .cloned()
            .collect();
        FunctionBody::new(commented_end(&stmts), directives, self.stmts(&stmts), b)
    }

    /// `function f() {}`, `export` of one exported, or `export default` of
    /// one exported as its module's default alone, `default`.
    /// A `pub use`'s `export { a, b as c } from "./m.js";`, or of a module
    /// by a name, `export * as ns from "./m.js";` (ADR 0240, amended).
    /// Printed at `start`, its comments' place (`comments_of`).
    pub(super) fn reexport(&self, reexport: &js::Reexport, start: u32) -> Statement<'a> {
        let b = &self.b;
        let sp = Span::new(start, reexport.span.hi.max(start));
        let source = StringLiteral::new(SPAN, self.allocator.alloc_str(&reexport.from), None, b);
        let name = |name: &str| ModuleExportName::new_identifier_name(SPAN, self.allocator.alloc_str(name), b);
        if let [(namespace, alias)] = reexport.named.as_slice()
            && namespace == "*"
        {
            return Statement::new_export_all_declaration(
                sp,
                Some(name(alias)),
                source,
                None,
                ImportOrExportKind::Value,
                b,
            );
        }
        let specifiers = (reexport.named.iter())
            .map(|(export, alias)| ExportSpecifier::new(SPAN, name(export), name(alias), ImportOrExportKind::Value, b));
        Statement::new_export_from_declaration(
            sp,
            ArenaVec::from_iter_in(specifiers, b),
            source,
            ImportOrExportKind::Value,
            None,
            b,
        )
    }

    pub(super) fn function(&self, f: &js::Function, default: bool) -> Statement<'a> {
        let b = &self.b;
        let params = self.params_at(FormalParameterKind::FormalParameter, &f.params, &f.param_spans);
        let body = self.function_body(&f.body);
        let decl = Declaration::new_function_declaration(
            span(f.span),
            FunctionType::FunctionDeclaration,
            Some(BindingIdentifier::new(span(f.name_span), self.name(&f.name), b)),
            false, // generator
            f.is_async,
            false, // declare
            None,  // type parameters
            None,  // this param
            ArenaBox::new_in(params, b),
            None, // return type
            Some(ArenaBox::new_in(body, b)),
            b,
        );
        match (default, decl) {
            (true, Declaration::FunctionDeclaration(function)) => Statement::new_export_default_declaration(
                span(f.span),
                oxc_ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(function),
                b,
            ),
            (_, decl) if f.export => Statement::new_export_declaration(span(f.span), decl, b),
            (_, decl) => decl.into(),
        }
    }

    /// A JS class (ADR 0761), as JS wrote one before `class`: its
    /// constructor function, `export function Component(props) { this.props
    /// = props; }`; then its prototype's constants and methods, each a
    /// `function` reading `this`, `Component.prototype.setState = function
    /// (partialState) { .. };`, a getter `Object.defineProperty(
    /// QueenPosition.prototype, "leftDiagonal", { get() { .. } });`; and its
    /// statics, `Component.from = function (..) { .. };`.
    pub(super) fn class(&self, n: &js::Namespace, class: &js::Class) -> Vec<Statement<'a>> {
        let mut out = Vec::new();
        if let Some(constructor) = n.methods.iter().find(|f| f.name == class.constructor) {
            let mut constructor = constructor.clone();
            constructor.name = n.name.clone();
            constructor.export = n.export;
            out.push(self.function(&constructor, false));
        }
        let prototype = || js::Expr::member(js::Expr::var(&n.name), "prototype");
        let mut assigned = Vec::new();
        for c in &n.constants {
            let to = js::Expr::member(prototype(), c.name.clone());
            assigned.push(js::StmtKind::Assign(to, c.value.clone()).at(c.span));
        }
        for f in n.methods.iter().filter(|f| f.name != class.constructor) {
            if class.getters.contains(&f.name) {
                let at = |kind| js::Expr {
                    kind,
                    span: f.span,
                    facts: js::Facts::NONE,
                };
                let get = js::Prop::Method("get".to_owned(), at(js::ExprKind::Arrow(Vec::new(), f.body.clone())));
                let descriptor = at(js::ExprKind::Object(vec![get], js::ObjectLayout::default()));
                let define = js::Expr::member(js::Expr::var("Object"), "defineProperty");
                let call = js::Expr::call(
                    define,
                    vec![prototype(), at(js::ExprKind::Str(f.name.clone())), descriptor],
                );
                assigned.push(js::StmtKind::Expr(call).at(f.span));
                continue;
            }
            let of = match class.prototype.contains(&f.name) {
                true => prototype(),
                false => js::Expr::var(&n.name),
            };
            let mut function = f.clone();
            function.name = String::new();
            function.export = false;
            let value = js::Expr {
                kind: js::ExprKind::Function(Box::new(function)),
                span: f.span,
                facts: js::Facts::NONE,
            };
            assigned.push(js::StmtKind::Assign(js::Expr::member(of, f.name.clone()), value).at(f.span));
        }
        out.extend(self.stmts(&assigned));
        out
    }

    /// `export const Counter = { new(step) { .. }, .. };`: each method in
    /// shorthand, as a hand-written object of functions has them.
    pub(super) fn namespace(&self, n: &js::Namespace) -> Statement<'a> {
        let b = &self.b;
        let constants = n.constants.iter().map(|c| {
            let key = PropertyKey::new_static_identifier(span(c.span), self.name(&c.name), b);
            ObjectPropertyKind::new_object_property(
                span(c.span),
                PropertyKind::Init,
                key,
                self.expr(&c.value),
                false,
                false,
                false,
                b,
            )
        });
        let methods = {
            ArenaVec::from_iter_in(
                constants.chain(n.methods.iter().map(|f| {
                    let params = self.params(FormalParameterKind::FormalParameter, &f.params);
                    let body = self.function_body(&f.body);
                    let value = Expression::new_function_expression(
                        span(f.span),
                        FunctionType::FunctionExpression,
                        None,
                        false, // generator
                        f.is_async,
                        false, // declare
                        None,  // type parameters
                        None,  // this param
                        ArenaBox::new_in(params, b),
                        None, // return type
                        Some(ArenaBox::new_in(body, b)),
                        b,
                    );
                    let key = PropertyKey::new_static_identifier(span(f.name_span), self.name(&f.name), b);
                    ObjectPropertyKind::new_object_property(
                        span(f.span),
                        PropertyKind::Init,
                        key,
                        value,
                        true,
                        false,
                        false,
                        b,
                    )
                })),
                b,
            )
        };
        let id = BindingPattern::new_binding_identifier(SPAN, self.name(&n.name), b);
        let object = Expression::new_object_expression(SPAN, methods, b);
        let declarator = VariableDeclarator::new(SPAN, id, None, Some(object), false, b);
        let decl = Declaration::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            ArenaVec::from_iter_in([declarator], b),
            false,
            b,
        );
        if n.export {
            Statement::new_export_declaration(SPAN, decl, b)
        } else {
            decl.into()
        }
    }

    /// `const { Intro, p: P } = MDXComponents;` of constants each reading a
    /// property of the object, `read_together`'s.
    pub(super) fn constant(&self, c: &js::Const) -> Statement<'a> {
        let b = &self.b;
        let sp = span(c.span);
        let id = BindingPattern::new_binding_identifier(SPAN, self.name(&c.name), b);
        // `let cached;`: a variable set later starts `undefined` by itself.
        let init = match (&c.value.kind, c.mutable) {
            (ExprKind::Undefined, true) => None,
            _ => Some(self.expr(&c.value)),
        };
        let declarator = VariableDeclarator::new(sp, id, None, init, false, b);
        let kind = if c.mutable {
            VariableDeclarationKind::Let
        } else {
            VariableDeclarationKind::Const
        };
        let decl = Declaration::new_variable_declaration(sp, kind, ArenaVec::from_iter_in([declarator], b), false, b);
        if c.export {
            Statement::new_export_declaration(sp, decl, b)
        } else {
            decl.into()
        }
    }

    /// `export default "19.3.0";`, a constant declared as the default,
    /// which JS has no declaration of (ADR 0625).
    pub(super) fn default_value(&self, c: &js::Const) -> Statement<'a> {
        let value = oxc_ast::ast::ExportDefaultDeclarationKind::from(self.expr(&c.value));
        Statement::new_export_default_declaration(span(c.span), value, &self.b)
    }

    pub(super) fn stmts(&self, stmts: &[js::Stmt]) -> ArenaVec<'a, Statement<'a>> {
        let mut out = ArenaVec::new_in(&self.b);
        self.push_stmts(stmts, &mut out);
        out
    }

    /// `stmts`, each pushed to `out`.
    fn push_stmts(&self, stmts: &[js::Stmt], out: &mut ArenaVec<'a, Statement<'a>>) {
        for (i, s) in stmts.iter().enumerate() {
            // Comments where they're written, which oxc prints before what
            // follows them, or the block's end (ADR 0753): no statement.
            if let StmtKind::Comments = s.kind {
                continue;
            }
            let mut statement = self.stmt(s);
            // One rust-js made right after a block's comments, a `break;`
            // tidy ends a case or a branch with: where they end, so they're
            // before it, in its block, not past it (ADR 0753, amended).
            let after_comments = (i.checked_sub(1).map(|at| &stmts[at]))
                .filter(|before| matches!(before.kind, StmtKind::Comments) && !before.span.is_none());
            if s.span.is_none()
                && let Some(comments) = after_comments
            {
                *oxc_span::GetSpanMut::span_mut(&mut statement) = Span::new(comments.span.lo, comments.span.lo);
            // One rust-js made, a temporary, with no Rust of its own: at the
            // start of the next that has, whose comments it's printed after
            // (ADR 0444), the first of its JS.
            } else if s.span.is_none()
                && let Some(next) = stmts[i + 1..].iter().find(|n| !n.span.is_none())
            {
                *oxc_span::GetSpanMut::span_mut(&mut statement) = Span::new(next.span.lo, next.span.lo);
            }
            out.push(statement);
        }
    }

    /// A `switch` case's statements: `break;` after them where they'd go
    /// on, as a case falls through to the next where nothing ends it, and in
    /// a block of their own, `case 1: { const s = ..; }`, where they
    /// declare, which the cases' one scope would share (ADR 0629).
    /// A case's statements, a `break` last where it goes on to another,
    /// not where it falls through to the next, `falls`, nor in the last
    /// clause, `last`, which JS falls out of: there, not tidy's either,
    /// `default: level = 0;` (ADR 0905, amended).
    fn case_body(&self, body: &[js::Stmt], falls: bool, last: bool) -> ArenaVec<'a, Statement<'a>> {
        let falls_out = falls || last;
        let body = match body.split_last() {
            Some((end, before)) if last && matches!(end.kind, StmtKind::Break(None)) => before,
            _ => body,
        };
        let mut stmts = self.stmts(body);
        if !falls_out && !body.last().is_some_and(js::ends) {
            // After the case's comments, where they end, so they're before
            // it (ADR 0753, amended): `case 4: // TODO .. break;`.
            let at = match body.last() {
                Some(end) if matches!(end.kind, StmtKind::Comments) && !end.span.is_none() => {
                    Span::new(end.span.lo, end.span.lo)
                }
                _ => SPAN,
            };
            stmts.push(Statement::new_break_statement(at, None, &self.b));
        }
        let declares = body.iter().any(|s| {
            matches!(
                s.kind,
                StmtKind::Const(..) | StmtKind::Let(..) | StmtKind::Destructure { .. } | StmtKind::Function(_)
            )
        });
        match declares {
            true => ArenaVec::from_iter_in([Statement::new_block_statement(SPAN, stmts, &self.b)], &self.b),
            false => stmts,
        }
    }

    /// A block of `stmts`. Of one that ends with comments of its own
    /// (`StmtKind::Comments`), to where they end, which oxc prints them
    /// before, inside the block (ADR 0753).
    fn block(&self, stmts: &[js::Stmt]) -> Statement<'a> {
        Statement::new_block_statement(commented_end(stmts), self.stmts(stmts), &self.b)
    }

    /// A `try`'s, a `catch`'s or a `finally`'s block of `stmts`, to where
    /// the comments it ends with end, as any block's (ADRs 0753, 0862): a
    /// `catch_unwind` closure's last comment is the `try`'s, before its
    /// `}`, not the `catch`'s first. It starts there too, as oxc prints
    /// what's where a `catch`'s block starts before its `{`.
    fn clause_block(&self, stmts: &[js::Stmt]) -> ArenaBox<'a, BlockStatement<'a>> {
        let end = commented_end(stmts).end;
        BlockStatement::boxed(Span::new(end, end), self.stmts(stmts), &self.b)
    }

    /// `catch (error) { handler }`, or `catch { handler }` of no binding.
    fn catch_clause(&self, error: Option<&str>, handler: &[js::Stmt]) -> ArenaBox<'a, CatchClause<'a>> {
        let b = &self.b;
        let param = error.map(|error| {
            let name = BindingPattern::new_binding_identifier(SPAN, self.name(error), b);
            CatchParameter::new(SPAN, name, None, b)
        });
        CatchClause::boxed(SPAN, param, self.clause_block(handler), b)
    }

    fn stmt(&self, s: &js::Stmt) -> Statement<'a> {
        let b = &self.b;
        let sp = span(s.span);
        match &s.kind {
            StmtKind::Const(name, init) => self.declare(sp, VariableDeclarationKind::Const, name, Some(init)),
            StmtKind::Function(function) => self.function(function, false),
            StmtKind::Directive(_) => unreachable!("a directive is its function's body's, taken first"),
            StmtKind::Comments => unreachable!("comments are printed before what follows them, `push_stmts`"),
            StmtKind::Let(name, init) => self.declare(sp, VariableDeclarationKind::Let, name, init.as_ref()),
            StmtKind::Destructure {
                pattern,
                value,
                mutable,
            } => {
                let kind = if *mutable {
                    VariableDeclarationKind::Let
                } else {
                    VariableDeclarationKind::Const
                };
                let declarator =
                    VariableDeclarator::new(sp, self.pattern(pattern), None, Some(self.expr(value)), false, b);
                Statement::new_variable_declaration(sp, kind, ArenaVec::from_iter_in([declarator], b), false, b)
            }
            StmtKind::Assign(target, value) => {
                let assign = Expression::new_assignment_expression(
                    sp,
                    AssignmentOperator::Assign,
                    self.assignment_target(target),
                    self.expr(value),
                    b,
                );
                Statement::new_expression_statement(sp, assign, b)
            }
            // A step of one, `x++` and `x--`, as JS writes a counter's: the
            // number literal 1 is only a number's, a BigInt's is `1n`, a
            // string's text (ADR 0631).
            StmtKind::Compound(
                op @ (Op::Add | Op::Sub),
                target,
                js::Expr {
                    kind: ExprKind::Num(1.0),
                    ..
                },
            ) => {
                let operator = match op {
                    Op::Add => UpdateOperator::Increment,
                    _ => UpdateOperator::Decrement,
                };
                let target = self.assignment_target(target).into_simple_assignment_target();
                let update = Expression::new_update_expression(sp, operator, false, target, b);
                Statement::new_expression_statement(sp, update, b)
            }
            StmtKind::Compound(op, target, value) => {
                let assign = Expression::new_assignment_expression(
                    sp,
                    compound_operator(*op),
                    self.assignment_target(target),
                    self.expr(value),
                    b,
                );
                Statement::new_expression_statement(sp, assign, b)
            }
            StmtKind::Expr(e) => Statement::new_expression_statement(sp, self.expr(e), b),
            StmtKind::If(cond, then, els) => {
                let els = els.as_deref().map(|els| match els {
                    // A lone `if` in the `else` prints as `else if`; not one
                    // comments are before, `else { // Nothing has been
                    // pinged.\n if (..) }`, as it's written (ADR 0444, amended).
                    [
                        only @ js::Stmt {
                            kind: StmtKind::If(..), ..
                        },
                    ] if only.span.is_none() || !self.commented.contains(&only.span.lo) => self.stmt(only),
                    _ => self.block(els),
                });
                Statement::new_if_statement(sp, self.expr(cond), self.block(then), els, b)
            }
            // A label of a `switch` alone, the `switch`'s, `getTag: switch
            // (type) { .. }`, as JS writes it (ADR 0944).
            StmtKind::Labeled(label, body) => {
                let labeled = match &body[..] {
                    [
                        only @ js::Stmt {
                            kind: StmtKind::Switch { .. },
                            ..
                        },
                    ] => self.stmt(only),
                    _ => Statement::new_block_statement(sp, self.stmts(body), b),
                };
                self.labeled(sp, Some(label), labeled)
            }
            // Each case's values stacked, `case 1: case 2:`, the last its
            // statements (ADR 0629).
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                let mut clauses = ArenaVec::new_in(b);
                // One that does nothing, `_ => {}`'s, tidy's `switches` left
                // out (#268).
                let default = default.as_ref();
                // The last clause, which JS falls out of, needs no `break`
                // (ADR 0629, amended).
                let last_case = cases.iter().rposition(|case| !case.values.is_empty());
                for (k, case) in cases.iter().enumerate() {
                    let Some((last, stacked)) = case.values.split_last() else {
                        continue;
                    };
                    // Where a case falls through to it, where its comment
                    // says so is (ADR 0905).
                    let mut at = super::fallen_to(cases, k).map_or(SPAN, |lo| Span::new(lo, lo));
                    for value in stacked {
                        clauses.push(SwitchCase::new(at, Some(self.expr(value)), ArenaVec::new_in(b), b));
                        at = SPAN;
                    }
                    // One that falls through to the next case goes on to it,
                    // no `break` (ADR 0905).
                    let ends = default.is_none() && Some(k) == last_case;
                    clauses.push(SwitchCase::new(
                        if ends { commented_clause(at, &case.body) } else { at },
                        Some(self.expr(last)),
                        self.case_body(&case.body, case.falls, ends),
                        b,
                    ));
                }
                if let Some(default) = default {
                    // Where the last case falls through to it, where its
                    // comment says so is (ADR 0636, amended).
                    let at = super::default_fallen_to(cases, Some(default)).map_or(SPAN, |lo| Span::new(lo, lo));
                    let at = commented_clause(at, default);
                    clauses.push(SwitchCase::new(at, None, self.case_body(default, false, true), b));
                }
                Statement::new_switch_statement(sp, self.expr(discriminant), clauses, b)
            }
            // A `finally` whose `try` holds a `try` and `catch` alone is one
            // `try`, `catch` and `finally`, as JS writes it: the same
            // statement, the `finally` run after the `catch` either way
            // (ADR 0098, amended).
            StmtKind::Try(body, finally) => {
                let (body, clause) = match body.as_slice() {
                    [
                        js::Stmt {
                            kind: StmtKind::TryCatch(inner, error, handler),
                            ..
                        },
                    ] => (inner, Some(self.catch_clause(error.as_deref(), handler))),
                    _ => (body, None),
                };
                Statement::new_try_statement(sp, self.clause_block(body), clause, Some(self.clause_block(finally)), b)
            }
            StmtKind::TryCatch(body, error, handler) => {
                let clause = self.catch_clause(error.as_deref(), handler);
                Statement::new_try_statement(sp, self.clause_block(body), Some(clause), None, b)
            }
            StmtKind::While { label, cond, body } => {
                let w = Statement::new_while_statement(sp, self.expr(cond), self.block(body), b);
                self.labeled(sp, label.as_deref(), w)
            }
            StmtKind::DoWhile { label, body, cond } => {
                let w = Statement::new_do_while_statement(sp, self.block(body), self.expr(cond), b);
                self.labeled(sp, label.as_deref(), w)
            }
            StmtKind::ForOf {
                label,
                pattern,
                mutable,
                iterable,
                body,
                keys,
            } => {
                let id = self.pattern(pattern);
                let declarator = VariableDeclarator::new(SPAN, id, None, None, false, b);
                let kind = if *mutable {
                    VariableDeclarationKind::Let
                } else {
                    VariableDeclarationKind::Const
                };
                let left = ForStatementLeft::new_variable_declaration(
                    SPAN,
                    kind,
                    ArenaVec::from_iter_in([declarator], b),
                    false,
                    b,
                );
                let l = if *keys {
                    Statement::new_for_in_statement(sp, left, self.expr(iterable), self.block(body), b)
                } else {
                    Statement::new_for_of_statement(sp, false, left, self.expr(iterable), self.block(body), b)
                };
                self.labeled(sp, label.as_deref(), l)
            }
            StmtKind::For {
                label,
                name,
                start,
                test,
                step,
                body,
            } => {
                let id = BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
                let declarator = VariableDeclarator::new(SPAN, id, None, Some(self.expr(start)), false, b);
                let init = ForStatementInit::new_variable_declaration(
                    SPAN,
                    VariableDeclarationKind::Let,
                    ArenaVec::from_iter_in([declarator], b),
                    false,
                    b,
                );
                let counter = SimpleAssignmentTarget::new_assignment_target_identifier(SPAN, self.name(name), b);
                let update = match step {
                    js::Step::Up => {
                        Expression::new_update_expression(SPAN, UpdateOperator::Increment, false, counter, b)
                    }
                    js::Step::Down => {
                        Expression::new_update_expression(SPAN, UpdateOperator::Decrement, false, counter, b)
                    }
                    js::Step::By(op, value) => {
                        let target = AssignmentTarget::from(counter);
                        Expression::new_assignment_expression(SPAN, compound_operator(*op), target, self.expr(value), b)
                    }
                };
                let l = Statement::new_for_statement(
                    sp,
                    Some(init),
                    Some(self.expr(test)),
                    Some(update),
                    self.block(body),
                    b,
                );
                self.labeled(sp, label.as_deref(), l)
            }
            StmtKind::Break(label) => Statement::new_break_statement(sp, label.as_deref().map(|l| self.label(l)), b),
            StmtKind::Continue(label) => {
                Statement::new_continue_statement(sp, label.as_deref().map(|l| self.label(l)), b)
            }
            StmtKind::Throw(value) => Statement::new_throw_statement(sp, self.expr(value), b),
            StmtKind::Return(value) => Statement::new_return_statement(sp, value.as_ref().map(|v| self.expr(v)), b),
        }
    }

    fn pattern(&self, pattern: &js::Pattern) -> BindingPattern<'a> {
        let b = &self.b;
        let name = |name: &str| BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
        match pattern {
            js::Pattern::Name(n) => name(n),
            js::Pattern::Rest(_) | js::Pattern::Defaulted(..) => {
                unreachable!("a rest or a default is a function's parameter, `params`'")
            }
            js::Pattern::Array(items) => {
                let items = items.iter().map(|item| item.as_ref().map(|item| self.pattern(item)));
                BindingPattern::new_array_pattern(SPAN, ArenaVec::from_iter_in(items, b), None, b)
            }
            js::Pattern::Object(fields, rest) => {
                let fields = fields.iter().map(|(field, var, default)| {
                    // A field that isn't a JS name, `data-platform`, is quoted.
                    let key = if crate::names::is_identifier(field) {
                        PropertyKey::new_static_identifier(SPAN, self.name(field), b)
                    } else {
                        PropertyKey::new_string_literal(SPAN, self.name(field), None, b)
                    };
                    // `{ size = "md" }`, where it's missing (ADR 0212).
                    let value = match default {
                        Some(default) => {
                            BindingPattern::new_assignment_pattern(SPAN, self.pattern(var), self.expr(default), b)
                        }
                        None => self.pattern(var),
                    };
                    // `{ x }` for `{ x: x }`.
                    BindingProperty::new(SPAN, key, value, var.name() == Some(field.as_str()), false, b)
                });
                let rest = rest
                    .as_deref()
                    .map(|rest| ArenaBox::new_in(BindingRestElement::new(SPAN, name(rest), b), b));
                BindingPattern::new_object_pattern(SPAN, ArenaVec::from_iter_in(fields, b), rest, b)
            }
        }
    }

    pub(super) fn assignment_target(&self, e: &js::Expr) -> AssignmentTarget<'a> {
        let b = &self.b;
        let sp = span(e.span);
        match &e.kind {
            ExprKind::Var(name) => AssignmentTarget::new_assignment_target_identifier(sp, self.name(name), b),
            // A property that isn't a JS name, `files["2d"]`, by its key.
            ExprKind::Member(object, property) if !member_name(property) => {
                let key = Expression::new_string_literal(SPAN, self.name(property), None, b);
                AssignmentTarget::new_computed_member_expression(sp, self.expr(object), key, false, b)
            }
            ExprKind::Member(object, property) => AssignmentTarget::new_static_member_expression(
                sp,
                self.expr(object),
                IdentifierName::new(SPAN, self.name(property), b),
                false,
                b,
            ),
            ExprKind::Index(object, index) => {
                AssignmentTarget::new_computed_member_expression(sp, self.expr(object), self.expr(index), false, b)
            }
            // Each at once, a swap's, `[a[i], a[j]] = [a[j], a[i]]`.
            ExprKind::Array(items) => {
                let items = items
                    .iter()
                    .map(|item| Some(AssignmentTargetMaybeDefault::from(self.assignment_target(item))));
                AssignmentTarget::new_array_assignment_target(sp, ArenaVec::from_iter_in(items, b), None, b)
            }
            _ => unreachable!("lowering only assigns to variables, fields and arrays of them"),
        }
    }

    fn labeled(&self, sp: Span, label: Option<&str>, s: Statement<'a>) -> Statement<'a> {
        match label {
            Some(l) => Statement::new_labeled_statement(sp, self.label(l), s, &self.b),
            None => s,
        }
    }

    fn declare(&self, sp: Span, kind: VariableDeclarationKind, name: &str, init: Option<&js::Expr>) -> Statement<'a> {
        let b = &self.b;
        let id = BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
        let declarator = VariableDeclarator::new(sp, id, None, init.map(|e| self.expr(e)), false, b);
        Statement::new_variable_declaration(sp, kind, ArenaVec::from_iter_in([declarator], b), false, b)
    }

    /// A call's argument, `...items` among them (ADR 0221).
    pub(super) fn argument(&self, a: &js::Expr) -> Argument<'a> {
        match &a.kind {
            ExprKind::Spread(all) => Argument::new_spread_element(span(a.span), self.expr(all), &self.b),
            _ => Argument::from(self.expr(a)),
        }
    }

    /// A link of a chain with a `?.` in it, `o?.inner` of `o?.inner.v`: its
    /// object a link too, inside the one chain expression, so each `?.`
    /// ends all of it where its object is `undefined`.
    pub(super) fn chain_element(&self, e: &js::Expr) -> ChainElement<'a> {
        let b = &self.b;
        let sp = span(e.span);
        match &e.kind {
            ExprKind::Member(object, property) | ExprKind::OptionalMember(object, property) => {
                let optional = matches!(e.kind, ExprKind::OptionalMember(..));
                let object = self.chain_object(object);
                if member_name(property) {
                    let name = IdentifierName::new(SPAN, self.name(property), b);
                    ChainElement::StaticMemberExpression(StaticMemberExpression::boxed(sp, object, name, optional, b))
                } else {
                    // A property that isn't a JS name, by its key.
                    let key = Expression::new_string_literal(SPAN, self.name(property), None, b);
                    ChainElement::ComputedMemberExpression(ComputedMemberExpression::boxed(
                        sp, object, key, optional, b,
                    ))
                }
            }
            ExprKind::Index(object, index) | ExprKind::OptionalIndex(object, index) => {
                ChainElement::ComputedMemberExpression(ComputedMemberExpression::boxed(
                    sp,
                    self.chain_object(object),
                    self.expr(index),
                    matches!(e.kind, ExprKind::OptionalIndex(..)),
                    b,
                ))
            }
            ExprKind::Call(callee, args) | ExprKind::OptionalCall(callee, args) => {
                let optional = matches!(e.kind, ExprKind::OptionalCall(..));
                let args = ArenaVec::from_iter_in(args.iter().map(|a| self.argument(a)), b);
                ChainElement::CallExpression(CallExpression::boxed(
                    sp,
                    self.chain_object(callee),
                    None,
                    args,
                    optional,
                    b,
                ))
            }
            _ => unreachable!("a chain is of members, indexes and calls"),
        }
    }

    /// The object of a chain's link: a link itself, where the chain goes on
    /// through it, else the expression it is.
    fn chain_object(&self, object: &js::Expr) -> Expression<'a> {
        if !in_chain(object) {
            return self.expr(object);
        }
        match self.chain_element(object) {
            ChainElement::StaticMemberExpression(member) => Expression::StaticMemberExpression(member),
            ChainElement::ComputedMemberExpression(member) => Expression::ComputedMemberExpression(member),
            ChainElement::CallExpression(call) => Expression::CallExpression(call),
            _ => unreachable!("a chain's links are members and calls"),
        }
    }
}

/// The span of a block, or a function's body, of `stmts`: to where the
/// comments of its own it ends with end (`StmtKind::Comments`), which oxc
/// prints them before, inside it, before its `}` (ADRs 0753, 0862).
fn commented_end(stmts: &[js::Stmt]) -> Span {
    let end = (stmts.iter())
        .filter(|s| matches!(s.kind, StmtKind::Comments) && !s.span.is_none())
        .map(|s| s.span.lo)
        .max();
    end.map_or(SPAN, |end| Span::new(end, end + 1))
}

/// `a op= b`'s operator of `op`, a compound assignment preparation makes.
/// The last clause's place, `at`, of one ending with comments of its own,
/// `default: // ..` (`StmtKind::Comments`): where nothing follows them, no
/// `break` nor a next clause, it ends past them, so oxc prints them in it,
/// before its end, as a block's (ADR 0753, amended). Empty, so nothing is
/// mapped there.
fn commented_clause(at: Span, body: &[js::Stmt]) -> Span {
    match body.last() {
        Some(end) if at == SPAN && matches!(end.kind, StmtKind::Comments) && !end.span.is_none() => {
            Span::new(end.span.lo + 1, end.span.lo + 1)
        }
        _ => at,
    }
}

fn compound_operator(op: Op) -> AssignmentOperator {
    match op {
        Op::Add => AssignmentOperator::Addition,
        Op::Sub => AssignmentOperator::Subtraction,
        Op::Mul => AssignmentOperator::Multiplication,
        Op::Div => AssignmentOperator::Division,
        Op::Rem => AssignmentOperator::Remainder,
        Op::Coalesce => AssignmentOperator::LogicalNullish,
        Op::BitAnd => AssignmentOperator::BitwiseAnd,
        Op::BitOr => AssignmentOperator::BitwiseOR,
        Op::BitXor => AssignmentOperator::BitwiseXOR,
        Op::Shl => AssignmentOperator::ShiftLeft,
        Op::Shr => AssignmentOperator::ShiftRight,
        Op::UShr => AssignmentOperator::ShiftRightZeroFill,
        _ => unreachable!("preparation compounds these alone"),
    }
}
