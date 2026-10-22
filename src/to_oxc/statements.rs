//! Our statements as oxc's: a function, its parameters and body, a
//! namespace, a constant, a pattern, and each statement.

use super::*;

impl<'a> Cx<'a> {
    pub(super) fn params(&self, kind: FormalParameterKind, patterns: &[js::Pattern]) -> FormalParameters<'a> {
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
        let params = patterns.iter().map(|pattern| {
            // `location = { .. }`, where it's given none (ADR 0427).
            let (pattern, default) = match pattern {
                js::Pattern::Defaulted(name, default) => (
                    BindingPattern::new_binding_identifier(SPAN, self.name(name), b),
                    Some(ArenaBox::new_in(self.expr(default), b)),
                ),
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
        FunctionBody::new(SPAN, directives, self.stmts(&stmts), b)
    }

    /// `function f() {}`, `export` of one exported, or `export default` of
    /// one exported as its module's default alone, `default`.
    pub(super) fn function(&self, f: &js::Function, default: bool) -> Statement<'a> {
        let b = &self.b;
        let params = self.params(FormalParameterKind::FormalParameter, &f.params);
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
            let mut statement = self.stmt(s);
            // One rust-js made, a temporary, with no Rust of its own: at the
            // start of the next that has, whose comments it's printed after
            // (ADR 0444), the first of its JS.
            if s.span.is_none()
                && let Some(next) = stmts[i + 1..].iter().find(|n| !n.span.is_none())
            {
                *oxc_span::GetSpanMut::span_mut(&mut statement) = Span::new(next.span.lo, next.span.lo);
            }
            out.push(statement);
        }
    }

    fn block(&self, stmts: &[js::Stmt]) -> Statement<'a> {
        Statement::new_block_statement(SPAN, self.stmts(stmts), &self.b)
    }

    fn stmt(&self, s: &js::Stmt) -> Statement<'a> {
        let b = &self.b;
        let sp = span(s.span);
        match &s.kind {
            StmtKind::Const(name, init) => self.declare(sp, VariableDeclarationKind::Const, name, Some(init)),
            StmtKind::Function(function) => self.function(function, false),
            StmtKind::Directive(_) => unreachable!("a directive is its function's body's, taken first"),
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
            StmtKind::Compound(op, target, value) => {
                let operator = match op {
                    Op::Add => AssignmentOperator::Addition,
                    Op::Sub => AssignmentOperator::Subtraction,
                    Op::Mul => AssignmentOperator::Multiplication,
                    Op::Div => AssignmentOperator::Division,
                    Op::Rem => AssignmentOperator::Remainder,
                    Op::Coalesce => AssignmentOperator::LogicalNullish,
                    _ => unreachable!("preparation compounds these alone"),
                };
                let assign = Expression::new_assignment_expression(
                    sp,
                    operator,
                    self.assignment_target(target),
                    self.expr(value),
                    b,
                );
                Statement::new_expression_statement(sp, assign, b)
            }
            StmtKind::Expr(e) => Statement::new_expression_statement(sp, self.expr(e), b),
            StmtKind::If(cond, then, els) => {
                let els = els.as_deref().map(|els| match els {
                    // A lone `if` in the `else` prints as `else if`.
                    [
                        only @ js::Stmt {
                            kind: StmtKind::If(..), ..
                        },
                    ] => self.stmt(only),
                    _ => self.block(els),
                });
                Statement::new_if_statement(sp, self.expr(cond), self.block(then), els, b)
            }
            StmtKind::Labeled(label, body) => {
                let block = Statement::new_block_statement(sp, self.stmts(body), b);
                self.labeled(sp, Some(label), block)
            }
            StmtKind::Try(body, finally) => Statement::new_try_statement(
                sp,
                BlockStatement::boxed(SPAN, self.stmts(body), b),
                None,
                Some(BlockStatement::boxed(SPAN, self.stmts(finally), b)),
                b,
            ),
            StmtKind::TryCatch(body, error, handler) => {
                let param = error.as_ref().map(|error| {
                    let name = BindingPattern::new_binding_identifier(SPAN, self.name(error), b);
                    CatchParameter::new(SPAN, name, None, b)
                });
                let clause = CatchClause::boxed(SPAN, param, BlockStatement::boxed(SPAN, self.stmts(handler), b), b);
                Statement::new_try_statement(
                    sp,
                    BlockStatement::boxed(SPAN, self.stmts(body), b),
                    Some(clause),
                    None,
                    b,
                )
            }
            StmtKind::While { label, cond, body } => {
                let w = Statement::new_while_statement(sp, self.expr(cond), self.block(body), b);
                self.labeled(sp, label.as_deref(), w)
            }
            StmtKind::ForOf {
                label,
                pattern,
                mutable,
                iterable,
                body,
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
                let l = Statement::new_for_of_statement(sp, false, left, self.expr(iterable), self.block(body), b);
                self.labeled(sp, label.as_deref(), l)
            }
            StmtKind::For {
                label,
                name,
                start,
                test,
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
                let update = Expression::new_update_expression(SPAN, UpdateOperator::Increment, false, counter, b);
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

    fn assignment_target(&self, e: &js::Expr) -> AssignmentTarget<'a> {
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
