//! Our expressions as oxc's: each kind, JSX, a property, a number, a
//! regex and a template's raw text.

use super::*;

/// The value an arrow's body only returns, `() => x`'s `x`: printed as
/// its body, `=>` its value.
pub(super) fn arrow_value(body: &[js::Stmt]) -> Option<&js::Expr> {
    match body {
        [
            js::Stmt {
                kind: StmtKind::Return(Some(value)),
                ..
            },
        ] => Some(value),
        _ => None,
    }
}

impl<'a> Cx<'a> {
    /// A conditional's branch, `c ? a : b`'s `a` or `b`. Of one with
    /// comments, before its value (ADR 0658), the JS printed first is placed
    /// where the value starts in the Rust, which the printer finds the branch
    /// by: oxc places a member, an index and an operator by what's first in
    /// them, which may be placed elsewhere, a name read in place where it's
    /// bound, `prev` of `Some(prev) => prev.types.get()`, or nowhere, and
    /// the comments were lost (ADR 0444, amended).
    fn branch(&self, e: &js::Expr) -> Expression<'a> {
        if e.span.is_none() || !self.commented_branches.contains(&e.span.lo) {
            return self.expr(e);
        }
        let mut placed = e.clone();
        let mut first = &mut placed;
        while let ExprKind::Member(inner, _)
        | ExprKind::OptionalMember(inner, _)
        | ExprKind::Index(inner, _)
        | ExprKind::OptionalIndex(inner, _)
        | ExprKind::Binary(_, inner, _)
        | ExprKind::Cond(inner, ..) = &mut first.kind
        {
            first = inner;
        }
        if !(e.span.lo <= first.span.lo && first.span.lo < e.span.hi) {
            first.span = e.span;
        }
        self.expr(&placed)
    }

    pub(super) fn expr(&self, e: &js::Expr) -> Expression<'a> {
        let b = &self.b;
        let sp = span(e.span);
        let argument = |a: &js::Expr| self.argument(a);
        // A chain with a `?.` in it, `o?.inner.v` or `a?.m(x)`, is one chain
        // expression: `(o?.inner).v` would read `.v` of `undefined`.
        if in_chain(e) {
            return Expression::new_chain_expression(sp, self.chain_element(e), b);
        }
        match &e.kind {
            ExprKind::Handle(..) | ExprKind::Pair(..) => unreachable!("written by preparation"),
            ExprKind::Assigned(target, value) => Expression::new_assignment_expression(
                sp,
                AssignmentOperator::Assign,
                self.assignment_target(target),
                self.expr(value),
                b,
            ),
            // `x++`, its value the one before (ADR 0948).
            ExprKind::Stepped(op, target) => {
                let operator = match op {
                    Op::Add => UpdateOperator::Increment,
                    _ => UpdateOperator::Decrement,
                };
                let target = self.assignment_target(target).into_simple_assignment_target();
                Expression::new_update_expression(sp, operator, false, target, b)
            }
            ExprKind::Num(n) => self.number(sp, *n),
            // `5n`, and `-5n` as `-` of it, as a number is written.
            ExprKind::BigInt(n) => {
                let digits = Expression::new_identifier(sp, self.name(&format!("{}n", n.unsigned_abs())), b);
                if *n < 0 {
                    Expression::new_unary_expression(sp, UnaryOperator::UnaryNegation, digits, b)
                } else {
                    digits
                }
            }
            ExprKind::BigUint(n) => Expression::new_identifier(sp, self.name(&format!("{n}n")), b),
            ExprKind::Bool(v) => Expression::new_boolean_literal(sp, *v, b),
            ExprKind::Str(s) => self.string_literal(sp, s),
            ExprKind::Undefined | ExprKind::GivenUndefined => Expression::new_identifier(sp, "undefined", b),
            ExprKind::Null => Expression::new_null_literal(sp, b),
            ExprKind::This => Expression::new_this_expression(sp, b),
            ExprKind::Symbol(_) => unreachable!("linking resolves every module symbol before emission"),
            ExprKind::OptionalMember(..) | ExprKind::OptionalIndex(..) | ExprKind::OptionalCall(..) => {
                unreachable!("a chain is printed whole, above")
            }
            ExprKind::Var(name) => Expression::new_identifier(sp, self.name(name), b),
            // A property that isn't a JS name, `files["worker-bundle"]`, by its
            // key: `files.worker-bundle` would be a subtraction.
            ExprKind::Member(object, property) if !member_name(property) => {
                let key = Expression::new_string_literal(SPAN, self.name(property), None, b);
                Expression::new_computed_member_expression(sp, self.expr(object), key, false, b)
            }
            ExprKind::Member(object, property) => {
                // `5.toString()` would read `5.` as a number: `(5).toString()`.
                // A number is printed as its digits (`number`), so oxc can't
                // tell it needs them.
                let object = match object.kind {
                    ExprKind::Num(n) if n.is_finite() && n >= 0.0 && !js_number(n).contains(['.', 'e']) => {
                        Expression::new_identifier(sp, self.name(&format!("({})", js_number(n))), b)
                    }
                    _ => self.expr(object),
                };
                Expression::new_static_member_expression(
                    sp,
                    object,
                    IdentifierName::new(SPAN, self.name(property), b),
                    false,
                    b,
                )
            }
            ExprKind::Index(object, index) => {
                Expression::new_computed_member_expression(sp, self.expr(object), self.expr(index), false, b)
            }
            // `String(n)` where it isn't in a template (ADR 0066), placed
            // where it is, as a call is.
            ExprKind::Stringed(value) => {
                self.expr(&js::Expr::call(js::Expr::var("String"), vec![(**value).clone()]).or_at(e.span))
            }
            ExprKind::Array(items) => {
                let items = items.iter().map(|item| match &item.kind {
                    ExprKind::Spread(all) => {
                        ArrayExpressionElement::new_spread_element(span(item.span), self.expr(all), b)
                    }
                    _ => ArrayExpressionElement::from(self.expr(item)),
                });
                Expression::new_array_expression(sp, ArenaVec::from_iter_in(items, b), b)
            }
            ExprKind::Object(props, layout) => {
                let props = props.iter().map(|prop| self.property(prop));
                let object = Expression::new_object_expression(sp, ArenaVec::from_iter_in(props, b), b);
                // Across lines as its source is, which the formatter's told.
                if let (true, Expression::ObjectExpression(made)) = (layout.lines, &object) {
                    self.across_lines.borrow_mut().insert(&**made as *const _);
                }
                object
            }
            ExprKind::Jsx(jsx) => self.jsx(sp, jsx),
            // Printed as written, as `number` prints an integer.
            ExprKind::Regex(literal) => Expression::new_identifier(sp, self.name(literal), b),
            // Its pieces joined by `+`, each a string literal or, written
            // across lines, a template literal of them (ADR 0641).
            ExprKind::Written(pieces) => {
                // Each where it's written, which a comment before it is
                // before; else where the whole is.
                let piece = |piece: &js::StrPiece| {
                    let at = match piece.span.is_none() {
                        true => sp,
                        false => span(piece.span),
                    };
                    match piece.lines {
                        true => self.template_literal(at, &[&piece.text], &[], &[], true),
                        false => self.string_literal(at, &piece.text),
                    }
                };
                let mut pieces = pieces.iter().map(piece);
                let first = pieces.next().expect("a written string's pieces");
                pieces.fold(first, |joined, next| {
                    Expression::new_binary_expression(sp, joined, BinaryOperator::Addition, next, b)
                })
            }
            // One template literal, or `concat!`'s pieces joined by `+`, each
            // a template literal where it holds a value or is written across
            // lines, and otherwise a string literal (ADR 0641).
            ExprKind::Template(texts, values, layout) if layout.breaks.is_empty() => {
                let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
                self.template_literal(sp, &texts, values, layout.across.all(), layout.lines)
            }
            ExprKind::Template(texts, values, layout) => {
                let last = texts.len() - 1;
                let starts = std::iter::once((0, 0, layout.lines))
                    .chain(layout.breaks.iter().map(|at| (at.text, at.at, at.lines)));
                let ends = (layout.breaks.iter().map(|at| (at.text, at.at))).chain([(last, texts[last].len())]);
                let mut pieces = starts.zip(ends).map(|((from, start, lines), (to, end))| {
                    let piece_texts: Vec<&str> = match from == to {
                        true => vec![&texts[from][start..end]],
                        false => std::iter::once(&texts[from][start..])
                            .chain(texts[from + 1..to].iter().map(String::as_str))
                            .chain([&texts[to][..end]])
                            .collect(),
                    };
                    match (from == to, lines) {
                        (true, false) => self.string_literal(sp, piece_texts[0]),
                        _ => {
                            let across = layout.across.all().get(from..to).unwrap_or_default();
                            self.template_literal(sp, &piece_texts, &values[from..to], across, lines)
                        }
                    }
                });
                let first = pieces.next().expect("a template's first piece");
                pieces.fold(first, |joined, next| {
                    Expression::new_binary_expression(sp, joined, BinaryOperator::Addition, next, b)
                })
            }
            ExprKind::Unary(op, arg) => {
                let op = match op {
                    UnaryOp::Neg => UnaryOperator::UnaryNegation,
                    UnaryOp::Not => UnaryOperator::LogicalNot,
                    UnaryOp::BitNot => UnaryOperator::BitwiseNot,
                    UnaryOp::Typeof => UnaryOperator::Typeof,
                    UnaryOp::Delete => UnaryOperator::Delete,
                };
                Expression::new_unary_expression(sp, op, self.expr(arg), b)
            }
            ExprKind::Binary(op, l, r) => {
                // Its right operand's comments are before it (ADR 0444,
                // amended), placed as a branch's are.
                let (l, r) = (self.expr(l), self.branch(r));
                match binary_op(*op) {
                    Ok(op) => Expression::new_binary_expression(sp, l, op, r, b),
                    Err(op) => Expression::new_logical_expression(sp, l, op, r, b),
                }
            }
            ExprKind::Cond(test, then, els) => {
                Expression::new_conditional_expression(sp, self.expr(test), self.branch(then), self.branch(els), b)
            }
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
                let is_async = matches!(e.kind, ExprKind::AsyncArrow(..));
                let params = ArenaBox::new_in(self.params(FormalParameterKind::ArrowFormalParameters, params), b);
                // `() => x` when the body only returns a value, a comment
                // before it after the `=>` (ADR 0883).
                let body = match arrow_value(body) {
                    Some(value) => ArrowFunctionBody::from(self.branch(value)),
                    None => ArrowFunctionBody::FunctionBody(ArenaBox::new_in(self.function_body(body), b)),
                };
                Expression::new_arrow_function_expression(sp, is_async, None, params, None, body, b)
            }
            // `function Label(props) { .. }`, named (ADR 0296).
            ExprKind::Function(f) => {
                let params = self.params(FormalParameterKind::FormalParameter, &f.params);
                let body = self.function_body(&f.body);
                // One of no name is anonymous, a class's method's, `function
                // (partialState) { .. }` (ADR 0761).
                let name =
                    (!f.name.is_empty()).then(|| BindingIdentifier::new(span(f.name_span), self.name(&f.name), b));
                Expression::new_function_expression(
                    span(f.span),
                    FunctionType::FunctionExpression,
                    name,
                    false, // generator
                    f.is_async,
                    false, // declare
                    None,  // type parameters
                    None,  // this param
                    ArenaBox::new_in(params, b),
                    None, // return type
                    Some(ArenaBox::new_in(body, b)),
                    b,
                )
            }
            ExprKind::ItemHole(_) => unreachable!("the pipeline puts each item where its hole is"),
            ExprKind::Import(js::ImportFrom::Specifier(from)) => {
                let source = Expression::new_string_literal(SPAN, self.allocator.alloc_str(from), None, b);
                Expression::new_import_expression(sp, source, None, None, b)
            }
            ExprKind::Import(js::ImportFrom::Module(_)) => {
                unreachable!("its output plan gives a module of the crate's its specifier")
            }
            ExprKind::DropArgument(..) => unreachable!("the pipeline keeps each drop argument or leaves it out"),
            ExprKind::Await(promise) => Expression::new_await_expression(sp, self.expr(promise), b),
            ExprKind::Spread(_) => unreachable!("`...items` is an array's item or a call's argument, which they write"),
            ExprKind::Call(callee, args) => {
                let args = args.iter().map(argument);
                Expression::new_call_expression(sp, self.expr(callee), None, ArenaVec::from_iter_in(args, b), false, b)
            }
            ExprKind::New(callee, args) if let Some(literal) = regex_literal(callee, args) => {
                Expression::new_identifier(sp, self.name(&literal), b)
            }
            ExprKind::New(callee, args) => {
                let args = without_empty_flags(callee, args).iter().map(argument);
                Expression::new_new_expression(sp, self.expr(callee), None, ArenaVec::from_iter_in(args, b), b)
            }
        }
    }

    /// `"text"`, each code point `unseen` gives, and each control, as its
    /// escape, `"\u2007\xa0"` (ADR 0732, amended): where there's one, the
    /// literal as its source, `string_raw`, each escape spelled as a
    /// template's (`escape`), its hex in lowercase, which oxc's own `\xA0`
    /// isn't, and a control one can't see escaped, which oxc writes as it is.
    fn string_literal(&self, sp: Span, text: &str) -> Expression<'a> {
        let b = &self.b;
        if !text.chars().any(|c| c.is_control() || unseen(c)) {
            return Expression::new_string_literal(sp, self.allocator.alloc_str(text), None, b);
        }
        Expression::new_identifier(sp, self.name(&string_raw(text)), b)
    }

    /// A JSX element (ADR 0040), laid out by the formatter.
    /// `` `a ${x} b` ``: the texts around `values`, its line breaks as they
    /// are where it's written across `lines` (ADR 0066).
    /// A template literal of `texts` and `values`, which of them `across`
    /// says its source writes across lines, as `TemplateLayout::across`.
    fn template_literal(
        &self,
        sp: Span,
        texts: &[&str],
        values: &[js::Expr],
        across: &[bool],
        lines: bool,
    ) -> Expression<'a> {
        let b = &self.b;
        let quasis = texts.iter().enumerate().map(|(i, text)| {
            let value = TemplateElementValue {
                raw: self.name(&template_raw(text, lines)).into(),
                cooked: Some(self.name(text).into()),
            };
            TemplateElement::new(SPAN, value, i + 1 == texts.len(), b)
        });
        let quasis = ArenaVec::from_iter_in(quasis, b);
        let values = ArenaVec::from_iter_in(values.iter().map(|v| self.expr(v)), b);
        let template = Expression::new_template_literal(sp, quasis, values, b);
        // Each value the source writes across lines, after whose `${` the
        // formatter may break the line, as Prettier does (ADR 0641, amended).
        if let (true, Expression::TemplateLiteral(made)) = (across.contains(&true), &template) {
            self.interpolations
                .borrow_mut()
                .insert(&**made as *const _, across.to_vec());
        }
        template
    }

    fn jsx(&self, sp: Span, jsx: &js::Jsx) -> Expression<'a> {
        let b = &self.b;
        let mut children = ArenaVec::new_in(b);
        // Which children are JSX text: text that JSX can write, but not
        // right before text, which JSX would read as one with it, and React
        // render as one text node, not two. So the earlier is in braces,
        // `{" of"} {total}`: a formatter makes a later `{" "}` a space, of
        // the text before it.
        let mut text = vec![false; jsx.children.len()];
        for i in (0..jsx.children.len()).rev() {
            let writable = matches!(&jsx.children[i].kind, ExprKind::Str(s) if jsx_text_safe(s) && !s.is_empty());
            text[i] = writable && !text.get(i + 1).is_some_and(|&next| next);
        }
        // Of two, the later in braces where it has a space at an edge and
        // the earlier none, as a person writes `If this is a mistake{", "}`:
        // never a blank one, which a formatter makes a space of the text.
        let plain = |s: &str| jsx_text_safe(s) && !s.is_empty() && s.trim() == s;
        let spaced = |s: &str| !s.trim().is_empty() && s.trim() != s;
        for i in 0..jsx.children.len().saturating_sub(1) {
            if let (ExprKind::Str(earlier), ExprKind::Str(later)) = (&jsx.children[i].kind, &jsx.children[i + 1].kind)
                && !text[i]
                && text[i + 1]
                && plain(earlier)
                && spaced(later)
                && (i == 0 || !text[i - 1])
            {
                text[i] = true;
                text[i + 1] = false;
            }
        }
        for (child, &text) in jsx.children.iter().zip(&text) {
            children.push(self.jsx_child(child, text));
        }
        if let JsxTag::Fragment = jsx.tag {
            let (open, close) = (JSXOpeningFragment::new(sp, b), JSXClosingFragment::new(SPAN, b));
            return Expression::new_jsx_fragment(sp, open, children, close, b);
        }
        let attrs = jsx.props.iter().filter_map(|prop| self.jsx_attribute(prop));
        let opening = JSXOpeningElement::boxed(
            sp,
            self.jsx_name(&jsx.tag, sp),
            None,
            ArenaVec::from_iter_in(attrs, b),
            b,
        );
        // `<img />` has nothing to close.
        let closing = (!children.is_empty()).then(|| JSXClosingElement::boxed(SPAN, self.jsx_name(&jsx.tag, SPAN), b));
        Expression::new_jsx_element(sp, opening, children, closing, b)
    }

    /// `div`, `Counter`, or `stats.Chart` from another module.
    fn jsx_name(&self, tag: &JsxTag, sp: Span) -> JSXElementName<'a> {
        let b = &self.b;
        let component = match tag {
            JsxTag::Intrinsic(tag) => return JSXElementName::new_identifier(sp, self.name(tag), b),
            JsxTag::Component(component) => component,
            JsxTag::Fragment => unreachable!("a fragment has no name"),
        };
        match &component.kind {
            ExprKind::Var(name) => JSXElementName::new_identifier_reference(sp, self.name(name), b),
            ExprKind::Member(object, property) => {
                let property = JSXIdentifier::new(SPAN, self.name(property), b);
                JSXElementName::new_member_expression(SPAN, self.jsx_object(object), property, b)
            }
            _ => unreachable!("lowering only makes components of names and paths"),
        }
    }

    fn jsx_object(&self, object: &js::Expr) -> JSXMemberExpressionObject<'a> {
        let b = &self.b;
        match &object.kind {
            ExprKind::Var(name) => JSXMemberExpressionObject::new_identifier_reference(SPAN, self.name(name), b),
            ExprKind::Member(inner, property) => {
                let property = JSXIdentifier::new(SPAN, self.name(property), b);
                JSXMemberExpressionObject::new_member_expression(SPAN, self.jsx_object(inner), property, b)
            }
            _ => unreachable!("lowering only makes components of names and paths"),
        }
    }

    /// `className="hero"`, `disabled` for `true`, `onClick={f}` or `{...props}`.
    /// Preparation has left out an attribute that's `undefined` (a `None`).
    fn jsx_attribute(&self, prop: &Prop) -> Option<JSXAttributeItem<'a>> {
        let b = &self.b;
        let (name, value) = match prop {
            Prop::Spread(value) => return Some(JSXAttributeItem::new_spread_attribute(SPAN, self.expr(value), b)),
            Prop::Field(name, value)
            | Prop::Getter(name, value)
            | Prop::Setter(name, value)
            | Prop::Method(name, value) => (name, value),
        };
        let sp = span(value.span);
        let value = match &value.kind {
            ExprKind::Bool(true) => None,
            ExprKind::Str(s) if jsx_text_safe(s) && !s.contains('"') => {
                Some(JSXAttributeValue::new_string_literal(sp, self.name(s), None, b))
            }
            _ => Some(JSXAttributeValue::new_expression_container(
                sp,
                JSXExpression::from(self.expr(value)),
                b,
            )),
        };
        // An inline callback may have no mapping before its first argument.
        // Anchor the attribute to its value so a newly wrapped line still maps.
        let name = JSXAttributeName::new_identifier(sp, self.name(name), b);
        Some(JSXAttributeItem::new_attribute(sp, name, value, b))
    }

    /// Text as text, `Count is `, when it's `text`; anything else in braces,
    /// `{count}`.
    fn jsx_child(&self, child: &js::Expr, text: bool) -> JSXChild<'a> {
        let b = &self.b;
        let sp = span(child.span);
        match &child.kind {
            ExprKind::Str(s) if text => JSXChild::new_text(sp, self.name(s), None, b),
            ExprKind::Jsx(jsx) => match self.jsx(sp, jsx) {
                Expression::JSXElement(e) => JSXChild::Element(e),
                Expression::JSXFragment(f) => JSXChild::Fragment(f),
                _ => unreachable!("JSX converts to JSX"),
            },
            _ => JSXChild::new_expression_container(sp, JSXExpression::from(self.expr(child)), b),
        }
    }

    /// `get name() { .. }`, `set name(value) { .. }`, or a method, `name(a)
    /// { .. }`, `async name() { .. }`.
    fn accessor(
        &self,
        kind: PropertyKind,
        name: &str,
        params: &[js::Pattern],
        body: &[js::Stmt],
        is_async: bool,
        at: js::Span,
    ) -> ObjectPropertyKind<'a> {
        let b = &self.b;
        let params = self.params(FormalParameterKind::FormalParameter, params);
        let body = self.function_body(body);
        let value = Expression::new_function_expression(
            SPAN,
            FunctionType::FunctionExpression,
            None,
            false, // generator
            is_async,
            false, // declare
            None,  // type parameters
            None,  // this param
            ArenaBox::new_in(params, b),
            None, // return type
            Some(ArenaBox::new_in(body, b)),
            b,
        );
        let method = kind == PropertyKind::Init;
        let key = if crate::names::is_identifier(name) {
            PropertyKey::new_static_identifier(SPAN, self.name(name), b)
        } else {
            PropertyKey::new_string_literal(SPAN, self.name(name), None, b)
        };
        // A method starts where its arrow did, which a comment before the
        // field is attached to, as a field's value (ADR 0708).
        let at = match method {
            true => Span::new(at.lo, at.lo),
            false => SPAN,
        };
        ObjectPropertyKind::new_object_property(at, kind, key, value, method, false, false, b)
    }

    fn property(&self, prop: &Prop) -> ObjectPropertyKind<'a> {
        let b = &self.b;
        match prop {
            Prop::Setter(name, value) => match &value.kind {
                ExprKind::Arrow(params, body) => {
                    self.accessor(PropertyKind::Set, name, params, body, false, value.span)
                }
                _ => unreachable!("a setter is an arrow of one parameter"),
            },
            Prop::Getter(name, value) => match &value.kind {
                ExprKind::Arrow(params, body) => {
                    self.accessor(PropertyKind::Get, name, params, body, false, value.span)
                }
                _ => self.accessor(
                    PropertyKind::Get,
                    name,
                    &[],
                    &[js::StmtKind::Return(Some(value.clone())).at(value.span)],
                    false,
                    value.span,
                ),
            },
            Prop::Method(name, value) => match &value.kind {
                ExprKind::Arrow(params, body) => {
                    self.accessor(PropertyKind::Init, name, params, body, false, value.span)
                }
                ExprKind::AsyncArrow(params, body) => {
                    self.accessor(PropertyKind::Init, name, params, body, true, value.span)
                }
                _ => unreachable!("a method is an arrow"),
            },
            Prop::Field(name, value) => {
                // Where its value starts, which a comment before the field
                // is attached to (ADR 0708).
                let at = Span::new(value.span.lo, value.span.lo);
                let value = self.expr(value);
                // `{ x: x }` reads better as `{ x }`.
                let shorthand = matches!(&value, Expression::Identifier(id) if id.name == name.as_str());
                // In a literal, a plain `__proto__:` key sets the prototype. Quoted
                // and computed, it's an ordinary field, like any other Rust field.
                let computed = name == "__proto__";
                // A key that isn't a JS name, like a CSS custom property's
                // `--gap`, is quoted.
                let key = if computed || !crate::names::is_identifier(name) {
                    PropertyKey::new_string_literal(SPAN, self.name(name), None, b)
                } else {
                    PropertyKey::new_static_identifier(SPAN, self.name(name), b)
                };
                ObjectPropertyKind::new_object_property(
                    at,
                    PropertyKind::Init,
                    key,
                    value,
                    false,
                    shorthand,
                    computed,
                    b,
                )
            }
            Prop::Spread(value) => ObjectPropertyKind::new_spread_property(SPAN, self.expr(value), b),
        }
    }

    /// oxc prints numbers in their shortest form, like a minifier: `1000`
    /// becomes `1e3`. Whole numbers should read as written, so print their
    /// decimal digits verbatim (as an identifier, which oxc copies as-is); a
    /// negative one gets a real unary minus, so oxc still handles spacing and
    /// parentheses. Anything else (`0.1`) is already shortest.
    fn number(&self, sp: Span, n: f64) -> Expression<'a> {
        if !n.is_finite() || (n == 0.0 && n.is_sign_negative()) {
            return Expression::new_numeric_literal(sp, n, None, NumberBase::Decimal, &self.b);
        }
        // oxc would print the shortest text, `.25`: the digits are JS's own,
        // `0.25`, as `String(n)` writes them; or the program's, its base or
        // its exponent (ADR 0627).
        let written = (self.source.get(sp.start as usize..sp.end as usize)).and_then(|text| as_written(text, n.abs()));
        let digits = written.unwrap_or_else(|| js_number(n.abs()));
        let digits = Expression::new_identifier(sp, self.name(&digits), &self.b);
        if n < 0.0 {
            Expression::new_unary_expression(sp, UnaryOperator::UnaryNegation, digits, &self.b)
        } else {
            digits
        }
    }

    /// Copy a name into oxc's arena.
    pub(super) fn name(&self, name: &str) -> &'a str {
        self.allocator.alloc_str(name)
    }

    pub(super) fn label(&self, name: &str) -> LabelIdentifier<'a> {
        LabelIdentifier::new(SPAN, self.name(name), &self.b)
    }
}

/// `new RegExp("%s", "g")` as JS writes it, `/%s/g` (ADR 0243): its pattern
/// and flags as they're written, a `/` escaped where it would end it, when
/// JS parses them as a literal. One it doesn't is made as it runs, to throw
/// then, not as the module is read. A global is never a local's name, so
/// `RegExp` is JS's.
fn regex_literal(callee: &js::Expr, args: &[js::Expr]) -> Option<String> {
    if !matches!(&callee.kind, ExprKind::Var(name) if name == "RegExp") {
        return None;
    }
    let [pattern, rest @ ..] = args else {
        return None;
    };
    let ExprKind::Str(pattern) = &pattern.kind else {
        return None;
    };
    let flags = match rest {
        [] => "",
        [flags] => match &flags.kind {
            ExprKind::Str(flags) => flags.as_str(),
            _ => return None,
        },
        _ => return None,
    };
    if pattern.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    let mut body = String::new();
    let (mut escaped, mut class) = (false, false);
    for c in pattern.chars() {
        match c {
            '/' if !escaped && !class => body.push('\\'),
            '[' if !escaped => class = true,
            ']' if !escaped => class = false,
            _ => {}
        }
        escaped = c == '\\' && !escaped;
        body.push(c);
    }
    // `//` would be a comment: an empty pattern's literal is JS's own source of it.
    if body.is_empty() {
        body.push_str("(?:)");
    }
    let allocator = Allocator::default();
    LiteralParser::new(&allocator, &body, Some(flags), Options::default())
        .parse()
        .ok()?;
    Some(format!("/{body}/{flags}"))
}

/// `new RegExp(pattern, "")`'s arguments as JS writes them, `new
/// RegExp(pattern)`: flags left out are `""` (ECMAScript's
/// RegExpInitialize), so the same expression, as isAttributeNameSafe makes
/// its pattern at run time (ADR 0243, amended). Any other's as they are.
fn without_empty_flags<'e>(callee: &js::Expr, args: &'e [js::Expr]) -> &'e [js::Expr] {
    match args {
        [pattern, flags]
            if matches!(&callee.kind, ExprKind::Var(name) if name == "RegExp")
                && matches!(&flags.kind, ExprKind::Str(flags) if flags.is_empty()) =>
        {
            std::slice::from_ref(pattern)
        }
        _ => args,
    }
}

/// Can `s` be JSX text as it is? Braces and angle brackets start JSX, `&` an
/// entity, and JSX drops whitespace at the start or end of a line.
fn jsx_text_safe(s: &str) -> bool {
    !s.contains(['{', '}', '<', '>', '&', '\n', '\r'])
}

pub(super) fn span(s: js::Span) -> Span {
    Span::new(s.lo, s.hi)
}

/// How `op` is written in JS, `+` or `&&`: what a comment before its right
/// operand is after (ADR 0444, amended).
pub(super) fn operator(op: Op) -> &'static str {
    binary_op(op).map_or_else(LogicalOperator::as_str, BinaryOperator::as_str)
}

/// JS splits binary operators in two: `&&`/`||` are "logical", the rest "binary".
fn binary_op(op: Op) -> Result<BinaryOperator, LogicalOperator> {
    Ok(match op {
        Op::And => return Err(LogicalOperator::And),
        Op::Or => return Err(LogicalOperator::Or),
        Op::Coalesce => return Err(LogicalOperator::Coalesce),
        Op::BitOr => BinaryOperator::BitwiseOR,
        Op::BitXor => BinaryOperator::BitwiseXOR,
        Op::BitAnd => BinaryOperator::BitwiseAnd,
        Op::Eq => BinaryOperator::StrictEquality,
        Op::Ne => BinaryOperator::StrictInequality,
        Op::LooseEq => BinaryOperator::Equality,
        Op::LooseNe => BinaryOperator::Inequality,
        Op::InstanceOf => BinaryOperator::Instanceof,
        Op::In => BinaryOperator::In,
        Op::Lt => BinaryOperator::LessThan,
        Op::Le => BinaryOperator::LessEqualThan,
        Op::Gt => BinaryOperator::GreaterThan,
        Op::Ge => BinaryOperator::GreaterEqualThan,
        Op::Shl => BinaryOperator::ShiftLeft,
        Op::Shr => BinaryOperator::ShiftRight,
        Op::UShr => BinaryOperator::ShiftRightZeroFill,
        Op::Add => BinaryOperator::Addition,
        Op::Sub => BinaryOperator::Subtraction,
        Op::Mul => BinaryOperator::Multiplication,
        Op::Div => BinaryOperator::Division,
        Op::Rem => BinaryOperator::Remainder,
        Op::Pow => BinaryOperator::Exponential,
    })
}

/// A positive finite number as JS's `Number.prototype.toString` writes it:
/// its shortest digits, with an exponent only past 1e21 or below 1e-6.
/// A Rust number literal, `text`, of a base or an exponent, `0xffu32` or
/// `1e4`, as JS writes it, `0xff`, `1e4`: its type's suffix, its
/// separators and a decimal's leading zeros aren't JS's. Only where JS reads it as `n`, and it's written
/// so: a literal of neither is JS's own digits (ADR 0627).
fn as_written(text: &str, n: f64) -> Option<String> {
    let text = text.trim().trim_start_matches('-').replace('_', "");
    let suffixes = [
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64",
    ];
    let based = ["0x", "0o", "0b"].iter().find(|prefix| text.starts_with(*prefix));
    let (body, value) = match based {
        Some(prefix) => {
            let digits = &text[2..];
            // `0x1f32` is hex digits: only a base's own digits and a suffix.
            let digits = suffixes.iter().find_map(|s| digits.strip_suffix(s)).unwrap_or(digits);
            let radix = match *prefix {
                "0x" => 16,
                "0o" => 8,
                _ => 2,
            };
            let value = u128::from_str_radix(digits, radix).ok()? as f64;
            (format!("{prefix}{}", digits.to_ascii_lowercase()), value)
        }
        None => {
            let body = suffixes.iter().find_map(|s| text.strip_suffix(s)).unwrap_or(&text);
            if !body.contains(['e', 'E']) {
                return None;
            }
            // A decimal's leading zeros, `00e+10`, aren't JS's: one before its
            // point or its exponent.
            let digits = body.trim_start_matches('0');
            let digits = if digits.starts_with(|c: char| c.is_ascii_digit()) {
                digits.to_string()
            } else {
                format!("0{digits}")
            };
            (digits.to_ascii_lowercase(), body.parse::<f64>().ok()?)
        }
    };
    (value == n
        && body
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')))
    .then_some(body)
}

fn js_number(n: f64) -> String {
    // `{:e}` has the shortest digits that read back as `n`: `2.5e-1`.
    let text = format!("{n:e}");
    let (mantissa, exponent) = text.split_once('e').expect("`{:e}` has an exponent");
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let k = digits.len() as i32;
    // Where the point goes: after `point` digits.
    let point = exponent.parse::<i32>().expect("an integer exponent") + 1;
    if k <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - k) as usize))
    } else if 0 < point && point <= 21 {
        format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let sign = if point - 1 < 0 { "-" } else { "+" };
        let rest = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!("{}{rest}e{sign}{}", &digits[..1], (point - 1).abs())
    }
}

/// Whether `c` is written as its escape in a string's or a template's JS,
/// `"\u2007\xA0"`, as a reader can't tell it from a space or see it at all
/// (ADR 0732): a space but `' '` and a line's end, which are written as
/// they are, and a mark that takes no room. Not one that joins or varies
/// what's seen, an emoji's U+200D or U+FE0F.
fn unseen(c: char) -> bool {
    (c.is_whitespace() && !matches!(c, ' ' | '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        || matches!(
            c,
            '\u{AD}'
                | '\u{61C}'
                | '\u{180E}'
                | '\u{200B}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}'
        )
}

/// `text` as a template literal writes it: what would end the text or start
/// a value is escaped, and so are controls, as in a string, but a line
/// break where it keeps its `lines`.
fn template_raw(text: &str, lines: bool) -> String {
    let mut raw = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => raw.push_str("\\`"),
            '\\' => raw.push_str("\\\\"),
            '$' if chars.peek() == Some(&'{') => raw.push_str("\\$"),
            '\n' if lines => raw.push('\n'),
            c => escape(c, &mut raw),
        }
    }
    raw
}

/// `text` as a string literal writes it, `"a\u200bb"`: what would end it
/// escaped, and each code point as `escape` writes it.
fn string_raw(text: &str) -> String {
    let mut raw = String::with_capacity(text.len() + 8);
    raw.push('"');
    for c in text.chars() {
        match c {
            '"' => raw.push_str("\\\""),
            '\\' => raw.push_str("\\\\"),
            c => escape(c, &mut raw),
        }
    }
    raw.push('"');
    raw
}

/// `c` as a string or a template writes it: a line's end, a control and
/// what `unseen` says as its escape, JS's own, `\t`, `\v`, or its hex in
/// lowercase, `\x` of what's below U+0100, `\xa0`, `\x1b`, as upstream
/// React writes it (ADR 0732, amended); another as it is.
fn escape(c: char, raw: &mut String) {
    match c {
        '\n' => raw.push_str("\\n"),
        '\r' => raw.push_str("\\r"),
        '\t' => raw.push_str("\\t"),
        '\u{8}' => raw.push_str("\\b"),
        '\u{B}' => raw.push_str("\\v"),
        '\u{C}' => raw.push_str("\\f"),
        '\u{2028}' => raw.push_str("\\u2028"),
        '\u{2029}' => raw.push_str("\\u2029"),
        // Each below U+00A0.
        c if c.is_control() => raw.push_str(&format!("\\x{:02x}", c as u32)),
        c if unseen(c) => raw.push_str(&match c as u32 {
            n @ ..=0xFF => format!("\\x{n:02x}"),
            n => format!("\\u{n:04x}"),
        }),
        c => raw.push(c),
    }
}

/// Replace the temporary concatenated source with the actual source files.
///
/// The map names nothing. oxc names an identifier by the Rust its span
/// covers: a parameter's declaration, `callback: impl FnOnce(A) -> T`, a
/// closure that became the function it calls, or `begin_work` for
/// `beginWork`; and a stack trace through the map (source-map-support's,
/// Node's) shows that name as the frame's. A frame is named as the JS names
/// it, as a person's JS would be (ADR 0018, amended).
pub(super) fn restore_sources<'a>(
    map: &'a SourceMap<'a>,
    sources: &'a crate::program::Sources,
    paths: &'a std::collections::HashMap<usize, String>,
    primary: Option<usize>,
) -> SourceMap<'a> {
    let mut out = SourceMapBuilder::default();
    let mut ids = std::collections::HashMap::new();
    if let Some(i) = primary {
        ids.insert(i, out.set_source_and_content(&paths[&i], &sources.files[i].text));
    }
    for token in map.get_tokens() {
        if token.get_source_id().is_none() {
            out.add_token(token.get_dst_line(), token.get_dst_col(), 0, 0, None, None);
            continue;
        }
        let Some(i) = sources.file_at(token.get_src_line()) else {
            continue;
        };
        let id = *ids
            .entry(i)
            .or_insert_with(|| out.set_source_and_content(&paths[&i], &sources.files[i].text));
        out.add_token(
            token.get_dst_line(),
            token.get_dst_col(),
            token.get_src_line() - sources.files[i].line,
            token.get_src_col(),
            Some(id),
            None,
        );
    }
    out.into_sourcemap()
}

/// Whether `.property` can be written as it is: a JS name, or a path of
/// them, `target.value`, which lowering writes as one.
pub(super) fn member_name(property: &str) -> bool {
    property.split('.').all(crate::names::is_identifier)
}

/// Where `code`, a module rust-js wrote, or its `declarations`, names another
/// by its path: each specifier of an `import` and an `export .. from`, what
/// rust-js prints of them, as the byte range of its text between its quotes,
/// and the text. A host that moves the files rewrites these and nothing
/// else (ADR 0101): found by a parse, whatever comes before them.
pub fn module_specifiers(code: &str, declarations: bool) -> Result<Vec<(usize, usize, String)>, String> {
    struct Specifiers(Vec<(usize, usize, String)>);
    impl Specifiers {
        fn add(&mut self, source: &StringLiteral) {
            let (start, end) = (source.span.start as usize + 1, source.span.end as usize - 1);
            self.0.push((start, end, source.value.to_string()));
        }
    }
    impl<'a> Visit<'a> for Specifiers {
        fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
            self.add(&it.source);
        }
        fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
            self.add(&it.source);
        }
    }
    let allocator = Allocator::default();
    let source_type = if declarations {
        SourceType::d_ts()
    } else {
        SourceType::mjs().with_jsx(true)
    };
    let parsed = Parser::new(&allocator, code, source_type).parse();
    if let Some(error) = parsed.diagnostics.errors().next() {
        // Where, in what it wrote: the line the parse stopped at.
        let at = error.labels.iter().next().map_or(0, |label| label.offset() as usize);
        let line = code[..at.min(code.len())].matches('\n').count() + 1;
        let text = code.lines().nth(line - 1).unwrap_or("").trim();
        let what = if declarations { "declarations" } else { "module" };
        return Err(format!(
            "rust-js can't read the {what} it wrote: {error}, at line {line}: `{text}`"
        ));
    }
    let mut found = Specifiers(Vec::new());
    found.visit_program(&parsed.program);
    Ok(found.0)
}

/// Is `e` of a chain with a `?.` in it: a `?.` link, or a member, an index
/// or a call of one.
pub(super) fn in_chain(e: &js::Expr) -> bool {
    match &e.kind {
        ExprKind::OptionalMember(..) | ExprKind::OptionalIndex(..) | ExprKind::OptionalCall(..) => true,
        ExprKind::Member(object, _) | ExprKind::Index(object, _) | ExprKind::Call(object, _) => in_chain(object),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::as_written;

    // A literal's base or exponent is written as JS has it, its suffix and
    // separators gone, only of the value it is (ADR 0627).
    #[test]
    fn a_literal_is_written_as_js_reads_it() {
        assert_eq!(
            as_written("0x811c_9dc5u32", 2166136261.0).as_deref(),
            Some("0x811c9dc5")
        );
        assert_eq!(as_written("1E4f64", 1e4).as_deref(), Some("1e4"));
        assert_eq!(as_written("-0b1010", 10.0).as_deref(), Some("0b1010"));
        assert_eq!(as_written("5000.0", 5000.0), None);
        // Of a value it isn't, JS's digits.
        assert_eq!(as_written("0xff", 256.0), None);
        assert_eq!(as_written("1e-1", f64::from(0.1f32)), None);
        assert_eq!(as_written("0xffu8.wrapping_add(1)", 0.0), None);
    }
}
