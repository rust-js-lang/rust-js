//! Our expressions as oxc's: each kind, JSX, a property, a number, a
//! regex and a template's raw text.

use super::*;

impl<'a> Cx<'a> {
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
            ExprKind::Handle(_) | ExprKind::Pair(..) => unreachable!("written by preparation"),
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
            ExprKind::Str(s) => Expression::new_string_literal(sp, self.allocator.alloc_str(s), None, b),
            ExprKind::Undefined | ExprKind::GivenUndefined => Expression::new_identifier(sp, "undefined", b),
            ExprKind::Null => Expression::new_null_literal(sp, b),
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
            // `String(n)` where it isn't in a template (ADR 0066).
            ExprKind::Stringed(value) => self.expr(&js::Expr::call(js::Expr::var("String"), vec![(**value).clone()])),
            ExprKind::Array(items) => {
                let items = items.iter().map(|item| match &item.kind {
                    ExprKind::Spread(all) => {
                        ArrayExpressionElement::new_spread_element(span(item.span), self.expr(all), b)
                    }
                    _ => ArrayExpressionElement::from(self.expr(item)),
                });
                Expression::new_array_expression(sp, ArenaVec::from_iter_in(items, b), b)
            }
            ExprKind::Object(props) => {
                let props = props.iter().map(|prop| self.property(prop));
                Expression::new_object_expression(sp, ArenaVec::from_iter_in(props, b), b)
            }
            ExprKind::Jsx(jsx) => self.jsx(sp, jsx),
            // Printed as written, as `number` prints an integer.
            ExprKind::Regex(literal) => Expression::new_identifier(sp, self.name(literal), b),
            ExprKind::Lines(text) => {
                let value = TemplateElementValue {
                    raw: self.name(&template_raw(text, true)).into(),
                    cooked: Some(self.name(text).into()),
                };
                let quasis = ArenaVec::from_iter_in([TemplateElement::new(SPAN, value, true, b)], b);
                Expression::new_template_literal(sp, quasis, ArenaVec::new_in(b), b)
            }
            ExprKind::Template(texts, values, lines) => {
                let quasis = texts.iter().enumerate().map(|(i, text)| {
                    let value = TemplateElementValue {
                        raw: self.name(&template_raw(text, *lines)).into(),
                        cooked: Some(self.name(text).into()),
                    };
                    TemplateElement::new(SPAN, value, i + 1 == texts.len(), b)
                });
                let quasis = ArenaVec::from_iter_in(quasis, b);
                let values = ArenaVec::from_iter_in(values.iter().map(|v| self.expr(v)), b);
                Expression::new_template_literal(sp, quasis, values, b)
            }
            ExprKind::Unary(op, arg) => {
                let op = match op {
                    UnaryOp::Neg => UnaryOperator::UnaryNegation,
                    UnaryOp::Not => UnaryOperator::LogicalNot,
                    UnaryOp::BitNot => UnaryOperator::BitwiseNot,
                    UnaryOp::Typeof => UnaryOperator::Typeof,
                };
                Expression::new_unary_expression(sp, op, self.expr(arg), b)
            }
            ExprKind::Binary(op, l, r) => {
                let (l, r) = (self.expr(l), self.expr(r));
                match binary_op(*op) {
                    Ok(op) => Expression::new_binary_expression(sp, l, op, r, b),
                    Err(op) => Expression::new_logical_expression(sp, l, op, r, b),
                }
            }
            ExprKind::Cond(test, then, els) => {
                Expression::new_conditional_expression(sp, self.expr(test), self.expr(then), self.expr(els), b)
            }
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
                let is_async = matches!(e.kind, ExprKind::AsyncArrow(..));
                let params = ArenaBox::new_in(self.params(FormalParameterKind::ArrowFormalParameters, params), b);
                // `() => x` when the body only returns a value, and no
                // comment is printed before it: a statement's.
                let body = match body.as_slice() {
                    [
                        js::Stmt {
                            kind: StmtKind::Return(Some(value)),
                            span,
                        },
                    ] if span.is_none() || !self.commented.contains(&span.lo) => {
                        ArrowFunctionBody::from(self.expr(value))
                    }
                    _ => ArrowFunctionBody::FunctionBody(ArenaBox::new_in(self.function_body(body), b)),
                };
                Expression::new_arrow_function_expression(sp, is_async, None, params, None, body, b)
            }
            // `function Label(props) { .. }`, named (ADR 0296).
            ExprKind::Function(f) => {
                let params = self.params(FormalParameterKind::FormalParameter, &f.params);
                let body = self.function_body(&f.body);
                Expression::new_function_expression(
                    span(f.span),
                    FunctionType::FunctionExpression,
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
                )
            }
            ExprKind::FunctionHole(_) => unreachable!("the pipeline puts each function where its hole is"),
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
                let args = args.iter().map(argument);
                Expression::new_new_expression(sp, self.expr(callee), None, ArenaVec::from_iter_in(args, b), b)
            }
        }
    }

    /// A JSX element (ADR 0040), laid out by the formatter.
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
            Prop::Field(name, value) | Prop::Getter(name, value) | Prop::Setter(name, value) => (name, value),
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

    /// `get name() { .. }` or `set name(value) { .. }`.
    fn accessor(
        &self,
        kind: PropertyKind,
        name: &str,
        params: &[js::Pattern],
        body: &[js::Stmt],
    ) -> ObjectPropertyKind<'a> {
        let b = &self.b;
        let params = self.params(FormalParameterKind::FormalParameter, params);
        let body = self.function_body(body);
        let value = Expression::new_function_expression(
            SPAN,
            FunctionType::FunctionExpression,
            None,
            false, // generator
            false, // async
            false, // declare
            None,  // type parameters
            None,  // this param
            ArenaBox::new_in(params, b),
            None, // return type
            Some(ArenaBox::new_in(body, b)),
            b,
        );
        let key = PropertyKey::new_static_identifier(SPAN, self.name(name), b);
        ObjectPropertyKind::new_object_property(SPAN, kind, key, value, false, false, false, b)
    }

    fn property(&self, prop: &Prop) -> ObjectPropertyKind<'a> {
        let b = &self.b;
        match prop {
            Prop::Setter(name, value) => match &value.kind {
                ExprKind::Arrow(params, body) => self.accessor(PropertyKind::Set, name, params, body),
                _ => unreachable!("a setter is an arrow of one parameter"),
            },
            Prop::Getter(name, value) => match &value.kind {
                ExprKind::Arrow(params, body) => self.accessor(PropertyKind::Get, name, params, body),
                _ => self.accessor(
                    PropertyKind::Get,
                    name,
                    &[],
                    &[js::StmtKind::Return(Some(value.clone())).at(value.span)],
                ),
            },
            Prop::Field(name, value) => {
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
                    SPAN,
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
        // `0.25`, as `String(n)` writes them.
        let digits = Expression::new_identifier(sp, self.name(&js_number(n.abs())), &self.b);
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

/// Can `s` be JSX text as it is? Braces and angle brackets start JSX, `&` an
/// entity, and JSX drops whitespace at the start or end of a line.
fn jsx_text_safe(s: &str) -> bool {
    !s.contains(['{', '}', '<', '>', '&', '\n', '\r'])
}

pub(super) fn span(s: js::Span) -> Span {
    Span::new(s.lo, s.hi)
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
            '\n' => raw.push_str("\\n"),
            '\r' => raw.push_str("\\r"),
            '\t' => raw.push_str("\\t"),
            '\u{2028}' => raw.push_str("\\u2028"),
            '\u{2029}' => raw.push_str("\\u2029"),
            c if c.is_control() => raw.push_str(&format!("\\u{:04x}", c as u32)),
            c => raw.push(c),
        }
    }
    raw
}

/// Replace the temporary concatenated source with the actual source files.
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
    let names: Vec<_> = map.get_names().map(|name| out.add_name(name)).collect();
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
            token.get_name_id().map(|id| names[id as usize]),
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
