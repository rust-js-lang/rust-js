//! Parse only JSX tokens. Rust inside braces stays a Rust token stream,
//! with its original spans, for rustc to parse and type-check.

use rustc_ast::token::{Delimiter, TokenKind};
use rustc_ast::tokenstream::{DelimSpacing, DelimSpan, Spacing, TokenStream, TokenTree};
use rustc_ast::{self as ast, FnRetTy, ItemKind, TyKind};
use rustc_parse::parser::{AllowConstBlockItems, ForceCollect, Parser};
use rustc_session::Session;
use rustc_span::{ErrorGuaranteed, Ident, Span};
use std::collections::HashSet;

use super::formatting::Layout;
use super::{rust_expression, template};

type R<T> = Result<T, ErrorGuaranteed>;

/// `tags`: the capitalized parameters and `let`s of the function the JSX is
/// in, each a `react::Tag`, `<Comp>` of `let Comp = As::H1`.
pub(super) fn jsx(sess: &Session, tokens: TokenStream, span: Span, tags: &HashSet<String>) -> R<TokenStream> {
    parse(sess, tokens, span, 0, None, tags)
}

pub(super) fn formatted(sess: &Session, tokens: TokenStream, span: Span, indent: usize, layout: &mut Layout) -> R<()> {
    let tags = layout.tags.clone();
    parse(sess, tokens, span, indent, Some(layout), &tags).map(|_| ())
}

fn parse(
    sess: &Session,
    tokens: TokenStream,
    span: Span,
    indent: usize,
    layout: Option<&mut Layout>,
    tags: &HashSet<String>,
) -> R<TokenStream> {
    let mut p = Jsx {
        sess,
        tokens: tokens.iter().cloned().collect(),
        at: 0,
        span,
        layout,
        tags,
    };
    let element = p.element(0, indent)?;
    if p.at != p.tokens.len() {
        return Err(p.error("wrap adjacent JSX elements in <>...</>"));
    }
    Ok(element)
}

struct Jsx<'a> {
    sess: &'a Session,
    tokens: Vec<TokenTree>,
    at: usize,
    span: Span,
    layout: Option<&'a mut Layout>,
    tags: &'a HashSet<String>,
}

impl Jsx<'_> {
    fn mark(&mut self, indent: usize) -> usize {
        let span = self
            .tokens
            .get(self.at)
            .map_or(self.span.shrink_to_hi(), TokenTree::span);
        self.layout.as_mut().map_or(indent, |layout| layout.mark(span, indent))
    }
    fn error(&self, message: &str) -> ErrorGuaranteed {
        self.sess.dcx().span_err(
            self.tokens
                .get(self.at)
                .map_or(self.span.shrink_to_hi(), TokenTree::span),
            format!("jsx: {message}"),
        )
    }

    fn is(&self, kind: TokenKind) -> bool {
        matches!(self.tokens.get(self.at), Some(TokenTree::Token(t, _)) if t.kind == kind)
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.is(kind) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn need(&mut self, kind: TokenKind, message: &str) -> R<()> {
        if self.eat(kind) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn ident(&mut self) -> R<String> {
        match self.tokens.get(self.at) {
            Some(TokenTree::Token(t, _)) if let TokenKind::Ident(name, _) = t.kind => {
                self.at += 1;
                Ok(name.to_string())
            }
            _ => Err(self.error("expected a name")),
        }
    }

    fn value(&mut self, indent: usize) -> R<TokenStream> {
        self.mark(indent);
        match self.tokens.get(self.at).cloned() {
            Some(TokenTree::Delimited(span, spacing, Delimiter::Brace, value)) => {
                if let Some(layout) = &mut self.layout {
                    layout.value(self.sess, span, &value, indent)?;
                }
                self.at += 1;
                // JSX braces delimit an expression; they are not necessarily a
                // Rust block. Keep a block only when it actually has statements.
                let block = TokenStream::new(vec![TokenTree::Delimited(
                    span,
                    spacing,
                    Delimiter::Brace,
                    value.clone(),
                )]);
                let mut parser = Parser::new(&self.sess.psess, block.clone(), Some("JSX expression"));
                let parsed = parser.parse_block().map_err(|e| e.emit())?;
                let expression =
                    matches!(parsed.stmts.as_slice(), [stmt] if matches!(stmt.kind, ast::StmtKind::Expr(_)));
                rust_expression(self.sess, if expression { value } else { block }, self.tags)
            }
            Some(TokenTree::Token(ref token, _)) if matches!(token.kind, TokenKind::Literal(_)) => {
                let value = self.tokens[self.at].clone();
                self.at += 1;
                Ok(TokenStream::new(vec![value]))
            }
            _ => Err(self.error("expected a literal or a Rust expression in {braces}")),
        }
    }

    // Rust turbofish on a JSX component: <Card::<T> ... />. Preserve the
    // original type tokens, including lifetimes and nested generic arguments.
    fn type_args(&mut self) -> R<TokenStream> {
        self.need(TokenKind::Lt, "expected generic arguments")?;
        let mut depth = 1;
        let mut args = Vec::new();
        while let Some(tree) = self.tokens.get(self.at).cloned() {
            if let TokenTree::Token(mut token, spacing) = tree {
                match token.kind {
                    TokenKind::Lt => depth += 1,
                    TokenKind::Gt => depth -= 1,
                    TokenKind::Shr if depth <= 2 => {
                        token.kind = TokenKind::Gt;
                        if depth == 1 {
                            token.span = token.span.with_lo(token.span.lo() + rustc_span::BytePos(1));
                            self.tokens[self.at] = TokenTree::Token(token, spacing);
                        } else {
                            token.span = token.span.with_hi(token.span.lo() + rustc_span::BytePos(1));
                            args.push(TokenTree::Token(token, spacing));
                            self.at += 1;
                        }
                        return Ok(TokenStream::new(args));
                    }
                    TokenKind::Shr => depth -= 2,
                    _ => {}
                }
            }
            self.at += 1;
            if depth == 0 {
                return Ok(TokenStream::new(args));
            }
            args.push(tree);
        }
        Err(self.error("missing > after generic arguments"))
    }

    fn element(&mut self, depth: usize, indent: usize) -> R<TokenStream> {
        if depth > 128 {
            return Err(self.error("JSX nesting exceeds 128 elements"));
        }
        let span = self.tokens.get(self.at).map_or(self.span, TokenTree::span);
        let indent = self.mark(indent);
        self.need(TokenKind::Lt, "expected <tag> or <>fragment</>")?;
        let mut name = String::new();
        let mut types = None;
        let mut provider = false;
        if !self.is(TokenKind::Gt) {
            name = self.ident()?;
            while self.is(TokenKind::PathSep) || self.is(TokenKind::Dot) {
                let member = self.eat(TokenKind::Dot);
                if !member {
                    self.at += 1;
                }
                if self.is(TokenKind::Lt) {
                    types = Some(self.type_args()?);
                    break;
                }
                name.push_str("::");
                let segment = self.ident()?;
                provider = member && segment == "Provider";
                name.push_str(&segment);
            }
        }
        let intrinsic = !name.is_empty() && !name.contains("::") && name.starts_with(char::is_lowercase);
        // A capitalized local of the function, `<Comp>`: a `react::Tag`'s
        // element, or a component given its props whole, as before.
        let local = self.tags.contains(&name);
        let builtin = match name.as_str() {
            "Fragment" => Some("keyed_fragment"),
            "StrictMode" => Some("strict_mode"),
            "Suspense" => Some("suspense"),
            "Activity" => Some("activity"),
            "Profiler" => Some("profiler"),
            "ViewTransition" => Some("view_transition"),
            _ => None,
        };
        if types.is_some() && (intrinsic || local || builtin.is_some() || provider) {
            return Err(self.error("generic arguments belong on a function component"));
        }
        let mut attrs: Vec<(String, TokenStream, Span)> = Vec::new();
        let mut spread = None;
        // How many attributes come before the spread: an element's may come
        // after it too, as JSX writes them, in their order.
        let mut spread_at = 0;
        // `{..base}`, Rust's struct update, where `{...base}` is JSX's spread.
        let mut struct_update = false;
        while !self.is(TokenKind::Gt) && !self.is(TokenKind::Slash) {
            self.mark(indent + 4);
            let attr_span = self.tokens.get(self.at).map_or(span, TokenTree::span);
            if let Some(TokenTree::Delimited(_, _, Delimiter::Brace, tokens)) = self.tokens.get(self.at)
                && matches!(tokens.get(0), Some(TokenTree::Token(t, _)) if matches!(t.kind, TokenKind::DotDot | TokenKind::DotDotDot))
            {
                if spread.is_some() {
                    return Err(self.error("only one props spread is supported"));
                }
                struct_update = matches!(tokens.get(0), Some(TokenTree::Token(t, _)) if t.kind == TokenKind::DotDot);
                if let Some(layout) = &mut self.layout
                    && let Some(TokenTree::Delimited(span, _, _, tokens)) = self.tokens.get(self.at)
                {
                    layout.value(self.sess, *span, tokens, indent + 4)?;
                }
                spread = Some(rust_expression(
                    self.sess,
                    TokenStream::new(tokens.iter().skip(1).cloned().collect()),
                    self.tags,
                )?);
                self.at += 1;
                spread_at = attrs.len();
                if !(intrinsic || local) && !self.is(TokenKind::Gt) && !self.is(TokenKind::Slash) {
                    return Err(self.error("put the props spread last"));
                }
                continue;
            }
            let mut attr = self.ident()?;
            while self.eat(TokenKind::Minus) {
                attr.push('-');
                attr.push_str(&self.ident()?);
            }
            if attrs.iter().any(|(n, _, _)| n == &attr) {
                return Err(self.error("duplicate attribute"));
            }
            let value = if self.eat(TokenKind::Eq) {
                self.value(indent + 4)?
            } else {
                template(self.sess, "true".into(), attr_span)
            };
            attrs.push((attr, value, attr_span));
        }
        self.mark(indent);
        let closed = self.eat(TokenKind::Slash);
        self.need(TokenKind::Gt, "expected > or />")?;
        let mut children = Vec::new();
        if !closed {
            loop {
                if self.is(TokenKind::Lt)
                    && matches!(self.tokens.get(self.at + 1), Some(TokenTree::Token(t, _)) if t.kind == TokenKind::Slash)
                {
                    self.mark(indent);
                    self.at += 2;
                    let mut closing = String::new();
                    if !self.is(TokenKind::Gt) {
                        closing = self.ident()?;
                        while self.eat(TokenKind::PathSep) || self.eat(TokenKind::Dot) {
                            closing.push_str("::");
                            closing.push_str(&self.ident()?);
                        }
                    }
                    if closing != name {
                        return Err(self.error(&format!("expected </{name}>, found </{closing}>")));
                    }
                    self.need(TokenKind::Gt, "expected > after closing tag")?;
                    break;
                }
                if self.at == self.tokens.len() {
                    return Err(self.error(&format!("missing closing tag </{name}>")));
                }
                if self.is(TokenKind::Lt) {
                    children.push(self.element(depth + 1, indent + 4)?);
                } else if matches!(self.tokens.get(self.at), Some(TokenTree::Delimited(_, _, Delimiter::Brace, ts)) if ts.is_empty())
                {
                    self.mark(indent + 4);
                    self.at += 1; // JSX comment: {/* ... */}
                } else {
                    children.push(self.value(indent + 4)?);
                }
            }
        }
        let has_children = !children.is_empty();
        let tag = local && !(attrs.is_empty() && !has_children && spread.is_some());
        let children = tuple(children, span);
        if name.is_empty() {
            if !attrs.is_empty() || spread.is_some() || closed {
                return Err(self.error("a fragment is <>children</>"));
            }
            return Ok(call(
                self.sess,
                template(self.sess, "::react::fragment".into(), span),
                vec![children],
                span,
            ));
        }
        if intrinsic || tag || builtin.is_some() {
            let function = match builtin {
                Some(n) => format!("::react::{n}"),
                None if tag => "::react::tag".to_string(),
                None => format!("::react::html::r#{}", snake(&name)),
            };
            let mut expr = call(
                self.sess,
                template(self.sess, function, span),
                if name == "StrictMode" {
                    vec![arguments(vec![], span)]
                } else if tag {
                    vec![template(self.sess, name.clone(), span)]
                } else {
                    vec![]
                },
                span,
            );
            let mut spread = spread;
            for (i, (attr, value, at)) in attrs.into_iter().enumerate() {
                if i == spread_at
                    && let Some(props) = spread.take()
                {
                    expr = method_call(self.sess, expr, "props", vec![props], span);
                }
                let (method, args) = if attr.contains('-') {
                    ("attr".into(), vec![template(self.sess, format!("{attr:?}"), at), value])
                } else {
                    (snake(&attr), vec![value])
                };
                expr = method_call(self.sess, expr, &method, args, at);
            }
            if let Some(props) = spread {
                expr = method_call(self.sess, expr, "props", vec![props], span);
            }
            if has_children || (builtin.is_some() && name != "StrictMode") {
                expr = method_call(self.sess, expr, "children", vec![children], span);
            }
            return Ok(expr);
        }
        // A props literal already evaluates its fields in source order.
        // Capture only when the separate key or spread would change that
        // order. Ordinary components must stay ordinary JSX expressions.
        // `{..base}` is Rust's struct update, evaluated last, as Rust's is.
        let capture = attrs
            .iter()
            .position(|(name, _, _)| name == "key")
            .is_some_and(|at| at + 1 != attrs.len() || spread.is_some() || has_children)
            || (spread.is_some() && has_children && !struct_update)
            || attrs.iter().any(|(name, _, _)| name == "ref");
        // Evaluate attributes in written order, including `key`, before
        // constructing props. The match bindings cannot capture user names:
        // every user expression is in the scrutinee, outside their scope.
        let mut values = Vec::new();
        let mut bindings = Vec::new();
        let mut bind = |value: TokenStream, at: Span| {
            if !capture {
                return value;
            }
            let name = template(self.sess, format!("__jsx{}", values.len()), at);
            values.push(value);
            bindings.push(name.clone());
            name
        };
        for (_, value, at) in &mut attrs {
            *value = bind(value.clone(), *at);
        }
        if let Some(value) = &mut spread {
            *value = bind(value.clone(), span);
        }
        let children = if has_children { bind(children, span) } else { children };
        let mut key = None;
        attrs.retain(|(name, value, at)| {
            if name == "key" {
                key = Some((value.clone(), *at));
                false
            } else {
                true
            }
        });
        if spread.is_some() && !attrs.is_empty() && !struct_update {
            return Err(self.error("a component's named props take the rest from `{..base}`, Rust's struct update, not JSX's `{...base}`, which would override them"));
        }
        let mut reference_prop = None;
        attrs.retain(|(name, value, _)| {
            if name == "ref" {
                reference_prop = Some(value.clone());
                false
            } else {
                true
            }
        });
        let component = if provider {
            name.strip_suffix("::Provider").unwrap()
        } else {
            &name
        };
        let target = if provider {
            format!("::react::provider(&{component})")
        } else {
            format!("&{name}")
        };
        let mut reference = template(self.sess, target, span);
        if let Some(types) = &types {
            reference = TokenStream::new(
                reference
                    .iter()
                    .chain(template(self.sess, "::<".into(), span).iter())
                    .chain(types.iter())
                    .chain(template(self.sess, ">".into(), span).iter())
                    .cloned()
                    .collect(),
            );
        }
        let mut expr = if attrs.is_empty()
            && !has_children
            && let Some(props) = spread.clone()
        {
            call(
                self.sess,
                template(self.sess, "::react::component".into(), span),
                vec![reference, props],
                span,
            )
        } else {
            if has_children {
                if attrs.iter().any(|(n, _, _)| n == "children") {
                    return Err(self.error("children were provided twice"));
                }
                attrs.push(("children".into(), children, span));
            }
            let mut fields = Vec::new();
            for (attr, value, at) in attrs {
                // `aria-label` is a field a Rust struct can have, `aria_label`,
                // which `rust_js::name` gives JS's name again (ADR 0200).
                let field = snake(&attr).replace('-', "_");
                fields.extend(template(self.sess, format!("r#{field}:"), at).iter().cloned());
                fields.extend(value.iter().cloned());
                fields.extend(template(self.sess, ",".into(), at).iter().cloned());
            }
            if let Some(base) = spread {
                fields.extend(template(self.sess, "..".into(), span).iter().cloned());
                fields.extend(base.iter().cloned());
            }
            if let Some(reference) = reference_prop {
                let mut prefix: Vec<_> = template(self.sess, "@ref".into(), span).iter().cloned().collect();
                prefix.push(group(Delimiter::Parenthesis, reference, span));
                fields.splice(0..0, prefix);
            }
            if let Some(types) = types {
                let mut prefix: Vec<_> = template(self.sess, "@types".into(), span).iter().cloned().collect();
                prefix.push(group(Delimiter::Parenthesis, types, span));
                fields.splice(0..0, prefix);
            }
            if provider {
                fields.splice(0..0, template(self.sess, "@provider ".into(), span).iter().cloned());
            }
            let mut tokens: Vec<_> = template(self.sess, format!("{component}!"), span)
                .iter()
                .cloned()
                .collect();
            tokens.push(group(Delimiter::Brace, TokenStream::new(fields), span));
            TokenStream::new(tokens)
        };
        if let Some((key, at)) = key {
            expr = method_call(self.sess, expr, "key", vec![key], at);
        }
        if values.is_empty() {
            return Ok(expr);
        }
        let mut tokens: Vec<_> = template(self.sess, "match".into(), span).iter().cloned().collect();
        tokens.extend(arguments(values, span).iter().cloned());
        let mut arm: Vec<_> = arguments(bindings, span).iter().cloned().collect();
        arm.extend(template(self.sess, "=>".into(), span).iter().cloned());
        arm.extend(expr.iter().cloned());
        tokens.push(group(Delimiter::Brace, TokenStream::new(arm), span));
        Ok(TokenStream::new(tokens))
    }
}

fn snake(name: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            // Match the bindings generator: HTML stays one word, while
            // innerHTML and HTMLInput become inner_html and html_input.
            let previous = i.checked_sub(1).map(|i| chars[i]);
            let next = chars.get(i + 1);
            if previous.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                || (previous.is_some_and(|c| c.is_ascii_uppercase()) && next.is_some_and(|c| c.is_ascii_lowercase()))
            {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn group(delimiter: Delimiter, tokens: TokenStream, span: Span) -> TokenTree {
    TokenTree::Delimited(
        DelimSpan::from_single(span),
        DelimSpacing::new(Spacing::Alone, Spacing::Alone),
        delimiter,
        tokens,
    )
}

fn tuple(mut parts: Vec<TokenStream>, span: Span) -> TokenStream {
    // Pair tuples have no fixed sibling limit in the Node trait, and JSX
    // lowering flattens them without allocating arrays in the output.
    let mut tail = parts
        .pop()
        .unwrap_or_else(|| TokenStream::new(vec![group(Delimiter::Parenthesis, TokenStream::default(), span)]));
    while let Some(head) = parts.pop() {
        tail = arguments(vec![head, tail], span);
    }
    tail
}

fn arguments(parts: Vec<TokenStream>, span: Span) -> TokenStream {
    let mut tokens = Vec::new();
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            tokens.push(TokenTree::token_alone(TokenKind::Comma, span));
        }
        tokens.extend(part.iter().cloned());
    }
    // A one-value match input/pattern must be a tuple, not redundant
    // parentheses (which would trigger unused_parens in the user's crate).
    // A trailing comma is also valid for ordinary function arguments.
    if !tokens.is_empty() {
        tokens.push(TokenTree::token_alone(TokenKind::Comma, span));
    }
    TokenStream::new(vec![group(Delimiter::Parenthesis, TokenStream::new(tokens), span)])
}

fn call(sess: &Session, function: TokenStream, args: Vec<TokenStream>, span: Span) -> TokenStream {
    let marker = template(sess, "#[rust_js::jsx]".into(), span);
    TokenStream::new(
        marker
            .iter()
            .chain(function.iter())
            .chain(arguments(args, span).iter())
            .cloned()
            .collect(),
    )
}

fn method_call(sess: &Session, receiver: TokenStream, method: &str, args: Vec<TokenStream>, span: Span) -> TokenStream {
    let method = template(sess, format!(".r#{method}"), span);
    let receiver = TokenStream::new(vec![group(Delimiter::Parenthesis, receiver, span)]);
    let function = TokenStream::new(receiver.iter().chain(method.iter()).cloned().collect());
    call(sess, function, args, span)
}

/// A component's name, what it's called by, its props' type and, of a
/// `ForwardRef`, its handle's: `None` of what's no component.
fn signature(sess: &Session, item: &ast::Item) -> Option<(Ident, String, String, Option<String>)> {
    super::configured_attrs(sess, &item.attrs)?;
    let signature = match &item.kind {
        ItemKind::Fn(f) => {
            let FnRetTy::Ty(ret) = &f.sig.decl.output else {
                return None;
            };
            let TyKind::Path(_, path) = &ret.kind else { return None };
            if path.segments.last()?.ident.as_str() != "Element" || f.sig.decl.inputs.len() > 1 {
                return None;
            }
            let props = match f.sig.decl.inputs.first() {
                Some(param) => props_path(&param.ty)?,
                None => String::new(),
            };
            (f.ident, f.ident.to_string(), props, None)
        }
        ItemKind::Static(s) => {
            let TyKind::Path(_, path) = &s.ty.kind else { return None };
            let segment = path.segments.last()?;
            let ast::GenericArgs::AngleBracketed(args) = segment.args.as_deref()? else {
                return None;
            };
            let Some(ast::AngleBracketedArg::Arg(ast::GenericArg::Type(ty))) = args.args.first() else {
                return None;
            };
            let props = match segment.ident.as_str() {
                "Context" => "::react::Provider".to_string(),
                "Memo" | "Lazy" | "ForwardRef" => props_path(ty)?,
                _ => return None,
            };
            {
                let handle = if segment.ident.as_str() == "ForwardRef" {
                    let Some(ast::AngleBracketedArg::Arg(ast::GenericArg::Type(handle))) = args.args.get(1) else {
                        return None;
                    };
                    Some(sess.source_map().span_to_snippet(handle.span).ok()?)
                } else {
                    None
                };
                (s.ident, format!("&{}", s.ident), props, handle)
            }
        }
        _ => return None,
    };
    signature
        .0
        .as_str()
        .starts_with(char::is_uppercase)
        .then_some(signature)
}

/// The props types the components among `item` take, by their names: a
/// function's, a `static`'s or a `thread_local!`'s, `ButtonLinkProps`.
pub(super) fn props_names(sess: &Session, item: &ast::Item) -> Vec<String> {
    let declarations = thread_local_declarations(sess, item).unwrap_or_else(|| vec![Box::new(item.clone())]);
    declarations
        .iter()
        .filter_map(|declaration| signature(sess, declaration))
        .filter_map(|(_, _, props, _)| props.rsplit("::").next().map(str::to_string))
        .filter(|name| !name.is_empty())
        .collect()
}

/// A hygienic props constructor beside a component. Rust resolves its props
/// type in the definition's module, including aliases and private imports.
/// Calling it through an imported/renamed component uses Rust's macro
/// namespace. Its props are built by their type's companion where it has
/// one, `built` (ADR 0213), else they're a struct literal.
pub(super) fn component(sess: &Session, item: &ast::Item, built: &HashSet<String>) -> Option<Box<ast::Item>> {
    let (ident, target, props, handle) = signature(sess, item)?;
    let companion = props.rsplit("::").next().is_some_and(|name| built.contains(name));
    let pattern = if props.is_empty() {
        ""
    } else {
        "$($field:ident: $value:expr,)* $(..$base:expr)?"
    };
    let value = if props.is_empty() {
        "()".to_string()
    } else if companion {
        format!("{props}!(@given [$($field: $value,)*] [$($base)?])")
    } else {
        format!("{props} {{ $($field: $value,)* $(..$base)? }}")
    };
    let call = |target: &str, value: &str| format!("#[rust_js::jsx] ::react::component({target}, {value})");
    let arm = |prefix: &str, body: &str| format!("({prefix} {pattern}) => {{ {body} }}");
    let mut arms = vec![arm("", &call(&target, &value))];
    let ref_value = match companion {
        true => format!("{props}!(@given [r#ref: $reference, $($field: $value,)*] [$($base)?])"),
        false => format!("{props} {{ r#ref: $reference, $($field: $value,)* $(..$base)? }}"),
    };
    if let Some(handle) = handle {
        let body = format!(
            "#[rust_js::jsx] ({}).r#ref(::react::checked_ref::<{handle}, _, _>($reference))",
            call(&target, &value)
        );
        arms.push(arm("@ref ($reference:expr)", &body));
    } else if !props.is_empty() {
        arms.push(arm("@ref ($reference:expr)", &call(&target, &ref_value)));
    }
    if props == "::react::Provider" {
        arms.push(arm("@provider", &call(&format!("::react::provider({target})"), &value)));
    } else if matches!(&item.kind, ItemKind::Fn(f) if !f.generics.params.is_empty()) {
        let target = format!("{target}::<$($types)*>");
        arms.push(arm("@types ($($types:tt)*)", &call(&target, &value)));
        if !props.is_empty() {
            arms.push(arm(
                "@types ($($types:tt)*) @ref ($reference:expr)",
                &call(&target, &ref_value),
            ));
        }
    }
    let body = format!("{{ {} }}", arms.join(","));
    // Its expansion writes `#[rust_js::jsx]` on an expression too (ADR 0110).
    let tokens = template(
        sess,
        format!("#[allow_internal_unstable(stmt_expr_attributes)] macro {ident} {body}"),
        item.span,
    );
    let mut parser = Parser::new(&sess.psess, tokens, Some("JSX component props"));
    match parser.parse_item(ForceCollect::No, AllowConstBlockItems::No) {
        Ok(Some(mut companion)) => {
            companion.vis = item.vis.clone();
            Some(companion)
        }
        Ok(None) => None,
        Err(e) => {
            e.emit();
            None
        }
    }
}

fn props_path(ty: &ast::Ty) -> Option<String> {
    match &ty.kind {
        TyKind::Tup(parts) if parts.is_empty() => Some(String::new()),
        // Rust infers generic arguments from the fields and component call.
        TyKind::Path(None, path) => Some(
            path.segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        ),
        _ => None,
    }
}

// rustc stores module items in boxes.
#[allow(clippy::vec_box)]
pub(super) fn thread_local_components(
    sess: &Session,
    item: &ast::Item,
    built: &HashSet<String>,
) -> Vec<Box<ast::Item>> {
    (thread_local_declarations(sess, item).into_iter().flatten())
        .filter_map(|declaration| component(sess, &declaration, built))
        .collect()
}

/// A `thread_local!`'s declarations, each a `static`; `None` of another item.
#[allow(clippy::vec_box)]
fn thread_local_declarations(sess: &Session, item: &ast::Item) -> Option<Vec<Box<ast::Item>>> {
    let ItemKind::MacCall(mac) = &item.kind else {
        return None;
    };
    if mac
        .path
        .segments
        .last()
        .is_none_or(|s| s.ident.as_str() != "thread_local")
    {
        return None;
    }
    let mut parser = Parser::new(&sess.psess, mac.args.tokens.clone(), Some("JSX component declarations"));
    let mut declarations = Vec::new();
    while parser.token.kind != TokenKind::Eof {
        match parser.parse_item(ForceCollect::No, AllowConstBlockItems::No) {
            Ok(Some(declaration)) => declarations.push(declaration),
            Ok(None) => break,
            // Not an item, as `thread_local!`'s last declaration, which needs
            // no `;`, isn't: the macro's own expansion says what's wrong, if
            // anything is.
            Err(e) => {
                e.cancel();
                break;
            }
        }
    }
    Some(declarations)
}

/// A props struct's companion (ADR 0213): a macro of its name beside it,
/// which builds it from what JSX gives, `S!(@given [r#href: "/a",] [])`,
/// each of its own fields in its slot, and every other name its flattened
/// field's, by that struct's companion. One not given is left out,
/// `__omitted()`, of an `Option`, a `Rest` or a field with a default; its
/// children are their type's `Default`; and one that's required is said
/// missing. With a base, `[base]`, it's the struct literal with it, as
/// before. A `Default` struct of only `Option`s and the like, none
/// flattened, is its literal with its `Default`.
///
/// It's made for the crate's components' props structs, `props`, and for
/// every `Default` struct, as a flattened one is.
pub(super) fn props_companion(sess: &Session, item: &ast::Item, props: &HashSet<String>) -> Option<Box<ast::Item>> {
    let ItemKind::Struct(ident, _, ast::VariantData::Struct { fields, .. }) = &item.kind else {
        return None;
    };
    let attrs = super::configured_attrs(sess, &item.attrs)?;
    let derives_default = attrs.iter().any(|attr| {
        attr.has_name(rustc_span::sym::derive)
            && attr
                .meta_item_list()
                .is_some_and(|list| list.iter().any(|m| m.ident().is_some_and(|i| i.as_str() == "Default")))
    });
    let name = ident.as_str().to_string();
    if !derives_default && !props.contains(&name) {
        return None;
    }
    let tool = |attrs: &ast::AttrVec, wanted: &str| {
        attrs.iter().any(|attr| match &attr.kind {
            ast::AttrKind::Normal(normal) => matches!(&normal.item.path.segments[..],
                [tool, item] if tool.ident.as_str() == "rust_js" && item.ident.as_str() == wanted),
            _ => false,
        })
    };
    // Each own field, how a slot of it left empty is filled, and whether it
    // has a default; and the flattened one, its name and its type's path.
    let mut own = Vec::new();
    let mut flatten = None;
    for field in fields {
        let field_name = field.ident?.as_str().to_string();
        let attrs = super::configured_attrs(sess, &field.attrs)?;
        if tool(&attrs, "flatten") {
            flatten = Some((field_name, props_path(&field.ty)?));
            continue;
        }
        let last = match &field.ty.kind {
            TyKind::Path(_, path) => path.segments.last().map(|s| s.ident.as_str().to_string()),
            _ => None,
        };
        let defaulted = tool(&attrs, "default");
        let empty = if defaulted || matches!(last.as_deref(), Some("Option" | "Rest")) {
            "omitted"
        } else if field_name == "children" {
            "children"
        } else {
            "required"
        };
        own.push((field_name, empty, defaulted));
    }
    let given = "(@given [$($pairs:tt)*] [$base:expr]) => { NAME { $($pairs)* ..$base } }".to_string();
    let simple = derives_default
        && flatten.is_none()
        && own
            .iter()
            .all(|&(_, empty, defaulted)| !defaulted && empty != "required");
    let arms = if simple {
        vec![
            given,
            "(@given [$($name:ident: $value:expr,)*] []) => { NAME { $($name: $value,)* ..::core::default::Default::default() } }"
                .to_string(),
        ]
    } else {
        let n = own.len();
        let patterns = (0..n).map(|i| format!("$s{i}:tt")).collect::<Vec<_>>().join(" ");
        let filled = |slot: usize| {
            (0..n)
                .map(|i| {
                    if i == slot {
                        "[$v]".to_string()
                    } else {
                        format!("$s{i}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        // Each own field's slot; the flattened field's, given whole,
        // `props={props}`, a component's own passed on; and the names its
        // flattened struct has.
        let mut arms = vec![
            given,
            format!(
                "(@given [$($name:ident: $value:expr,)*] []) => {{ NAME!(@slots [{}] [] [] $($name: $value,)*) }}",
                vec!["[]"; n].join(" ")
            ),
        ];
        for (i, (field, _, _)) in own.iter().enumerate() {
            arms.push(format!(
                "(@slots [{patterns}] $flat:tt [$($rest:tt)*] r#{field}: $v:expr, $($tail:tt)*) => {{ NAME!(@slots [{}] $flat [$($rest)*] $($tail)*) }}",
                filled(i)
            ));
        }
        if let Some((field, _)) = &flatten {
            arms.push(format!(
                "(@slots [$($s:tt)*] [] [$($rest:tt)*] r#{field}: $v:expr, $($tail:tt)*) => {{ NAME!(@slots [$($s)*] [$v] [$($rest)*] $($tail)*) }}"
            ));
        }
        let other = match flatten {
            Some(_) => "NAME!(@slots [$($s)*] $flat [$($rest)* $name: $v,] $($tail)*)",
            // rustc's own error, at the name: `NAME` has no field `nope`.
            None => "NAME { $name: $v, ..::core::panic!() }",
        };
        arms.push(format!(
            "(@slots [$($s:tt)*] $flat:tt [$($rest:tt)*] $name:ident: $v:expr, $($tail:tt)*) => {{ {other} }}"
        ));
        let values: Vec<String> = (own.iter().enumerate())
            .map(|(i, (field, empty, _))| format!("r#{field}: NAME!(@slot {empty} \"{field}\" $s{i}), "))
            .collect();
        let values = values.concat();
        match &flatten {
            Some((field, path)) => {
                arms.push(format!(
                    "(@slots [{patterns}] [$whole:expr] []) => {{ NAME {{ {values}r#{field}: $whole }} }}"
                ));
                arms.push(format!(
                    "(@slots [{patterns}] [] [$($rest:tt)*]) => {{ NAME {{ {values}r#{field}: {path}!(@given [$($rest)*] []) }} }}"
                ));
                arms.push(format!(
                    "(@slots [$($s:tt)*] [$whole:expr] [$($rest:tt)+]) => {{ ::core::compile_error!(\"give `{field}` or the props of it, not both\") }}"
                ));
            }
            None => arms.push(format!("(@slots [{patterns}] [] []) => {{ NAME {{ {values} }} }}")),
        }
        arms.push("(@slot $empty:ident $name:literal [$v:expr]) => { $v }".to_string());
        arms.push(
            "(@slot required $name:literal []) => { ::core::compile_error!(concat!(\"missing prop `\", $name, \"` of `NAME`\")) }"
                .to_string(),
        );
        arms.push("(@slot omitted $name:literal []) => { ::react::__omitted() }".to_string());
        arms.push("(@slot children $name:literal []) => { ::core::default::Default::default() }".to_string());
        arms
    };
    let source = format!("macro {name} {{ {} }}", arms.join(", ")).replace("NAME", &name);
    let mut parser = Parser::new(&sess.psess, template(sess, source, item.span), Some("JSX props"));
    match parser.parse_item(ForceCollect::No, AllowConstBlockItems::No) {
        Ok(Some(mut companion)) => {
            companion.vis = item.vis.clone();
            Some(companion)
        }
        Ok(None) => None,
        Err(e) => {
            e.emit();
            None
        }
    }
}
