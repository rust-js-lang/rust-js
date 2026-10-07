//! Lower JSX bindings into the framework-independent JS tree.

use super::{FnCx, R, Shape, camel_case, js_ident};
use crate::js;
use crate::js::{Expr, Prop, Stmt, StmtKind};
use rustc_ast::LitKind;
use rustc_hir::def::DefKind;
use rustc_middle::thir::{self, ExprId, ExprKind};
use rustc_middle::ty;
use rustc_middle::ty::Ty;
use rustc_span::{BytePos, Span};
use std::collections::HashMap;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Element construction is inert (ADR 0040). Capture its inputs at the
    /// original evaluation point while leaving the element in the JSX tree.
    pub(super) fn capture_jsx(&mut self, value: &mut Expr, out: &mut Vec<Stmt>) -> bool {
        match &mut value.kind {
            js::ExprKind::Jsx(jsx) => {
                if let js::JsxTag::Component(tag) = &mut jsx.tag {
                    self.capture_jsx_input("Component", tag, out);
                }
                for prop in &mut jsx.props {
                    match prop {
                        Prop::Field(name, value) | Prop::Getter(name, value) => {
                            self.capture_jsx_input(&js_ident(name), value, out)
                        }
                        Prop::Spread(value) => {
                            // JSX spreads read properties now, not when the
                            // later element is built. Preserve getters too.
                            let old = std::mem::replace(value, Expr::undefined());
                            *value = self.spill("props", Expr::object(vec![Prop::Spread(old)]), out);
                        }
                    }
                }
                for child in &mut jsx.children {
                    self.capture_jsx_input("children", child, out);
                }
                true
            }
            js::ExprKind::Cond(test, yes, no) if jsx_tree(yes) || jsx_tree(no) => {
                self.capture_jsx_input("condition", test, out);
                let mut then = Vec::new();
                let mut otherwise = Vec::new();
                self.capture_jsx_input("children", yes, &mut then);
                self.capture_jsx_input("children", no, &mut otherwise);
                // Keep branch computations conditional. Only their result
                // bindings are visible to the deferred JSX expression.
                for statement in then.iter_mut().chain(&mut otherwise) {
                    match &mut statement.kind {
                        StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) => {
                            out.push(StmtKind::Let(name.clone(), None).at(statement.span));
                            statement.kind = StmtKind::Assign(Expr::var(name), value.clone());
                        }
                        StmtKind::Let(name, None) => {
                            out.push(StmtKind::Let(name.clone(), None).at(statement.span));
                        }
                        _ => {}
                    }
                }
                then.retain(|s| !matches!(s.kind, StmtKind::Let(_, None)));
                otherwise.retain(|s| !matches!(s.kind, StmtKind::Let(_, None)));
                if !then.is_empty() || !otherwise.is_empty() {
                    out.push(StmtKind::If(*test.clone(), then, Some(otherwise)).at(value.span));
                }
                true
            }
            // Item by item, but not of `...items`, which is no value alone.
            js::ExprKind::Array(items)
                if items.iter().any(jsx_tree) && !items.iter().any(|i| matches!(i.kind, js::ExprKind::Spread(_))) =>
            {
                for item in items {
                    self.capture_jsx_input("children", item, out);
                }
                true
            }
            _ => false,
        }
    }

    /// Whether `value` reads the same wherever it's read: a constant, a
    /// function, or a variable nothing writes again, a `const` of `out`'s,
    /// and a comparison of them, which runs no code of its own:
    /// `version === "canary"`, `!done`, `status != null`.
    fn reads_alike(&self, value: &Expr, out: &[Stmt]) -> bool {
        match &value.kind {
            js::ExprKind::Symbol(_) | js::ExprKind::Arrow(..) | js::ExprKind::AsyncArrow(..) => true,
            js::ExprKind::Binary(js::Op::Eq | js::Op::Ne | js::Op::LooseEq | js::Op::LooseNe, a, b) => {
                self.reads_alike(a, out) && self.reads_alike(b, out)
            }
            js::ExprKind::Unary(js::UnaryOp::Not, a) => self.reads_alike(a, out),
            js::ExprKind::Var(name) => {
                self.locals
                    .vars
                    .values()
                    .any(|var| !var.mutable && matches!(&var.place.kind, js::ExprKind::Var(n) if n == name))
                    || out
                        .iter()
                        .any(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == name))
                    || self.krate.fns.iter().any(|(id, f)| {
                        f.module == self.module
                            && &f.name == name
                            && matches!(self.tcx.def_kind(*id), DefKind::Fn | DefKind::AssocFn)
                    })
            }
            _ => value.is_constant(),
        }
    }

    /// Whether `value` reads only variables that never change, so it's the
    /// same read before or after anything else: a `useState` value, a
    /// parameter, and elements of them.
    fn reads_unchanging(&self, value: &Expr, out: &[Stmt]) -> bool {
        let mut vars = Vec::new();
        value.visit_vars(&mut |var| vars.push(var));
        value.reads_only_vars() && vars.into_iter().all(|var| self.reads_alike(&Expr::var(var), out))
    }

    fn capture_jsx_input(&mut self, base: &str, value: &mut Expr, out: &mut Vec<Stmt>) {
        let stable = self.reads_alike(value, out);
        if !stable && !self.capture_jsx(value, out) {
            let map = matches!(&value.kind, js::ExprKind::Call(f, _) if matches!(&f.kind, js::ExprKind::Member(_, name) if name == "map"));
            let old = std::mem::replace(value, Expr::undefined());
            *value = self.spill(if map { "items" } else { base }, old, out);
        }
    }

    // ── JSX (ADR 0040) ──────────────────────────────────────────────────

    /// A JSX element, from a binding whose `link_name` is its tag: `<div>`
    /// takes nothing, `<>` and an imported component (`<react#StrictMode>`)
    /// take their children, and `<*>` takes a component and its props.
    pub(super) fn jsx(&mut self, tag: &str, args: &[ExprId], span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        self.jsx = true;
        let (tag, props, children) = match (tag, args) {
            ("*", &[component, props]) => {
                // A JS module's component is its import, as a tag must be.
                let tag = match self.binding_component(component) {
                    Some(tag) => tag,
                    None => self.expr(component, out)?,
                };
                if !matches!(
                    tag.kind,
                    js::ExprKind::Var(_) | js::ExprKind::Symbol(_) | js::ExprKind::Member(..)
                ) {
                    return Err(self.unsupported(
                        self.thir[component].span,
                        "a JSX component other than a named function or module member",
                    ));
                }
                // JSX reads a lowercase name as a DOM element's, and Fast
                // Refresh only keeps the state of a capitalized component.
                if let Some(name) = match &tag.kind {
                    js::ExprKind::Var(name) => Some(name.as_str()),
                    js::ExprKind::Symbol(symbol) => Some(symbol.export.as_str()),
                    _ => None,
                } && !name.starts_with(|c: char| c.is_ascii_uppercase())
                {
                    let message =
                        format!("rust-js: a React component's name starts with an uppercase letter, not `{name}`");
                    return Err(self.tcx.dcx().span_err(self.thir[component].span, message));
                }
                let (props, children) = self.jsx_props(props, out)?;
                (js::JsxTag::Component(tag), props, children)
            }
            // A `react::Tag` the function names, `<Comp>`, whose value is the
            // tag, as JSX reads a capitalized variable's (ADR 0220).
            ("$", &[tag]) => match self.expr(tag, out)? {
                tag @ Expr {
                    kind: js::ExprKind::Var(_),
                    ..
                } => (js::JsxTag::Component(tag), Vec::new(), Vec::new()),
                _ => return Err(self.unsupported(self.thir[tag].span, "a JSX tag other than a variable")),
            },
            (tag, [] | [_]) if tag != "*" => {
                let children = match args {
                    &[children] => self.jsx_children(children, out)?,
                    _ => Vec::new(),
                };
                let tag = match tag {
                    "" => js::JsxTag::Fragment,
                    t if t.contains('#') => js::JsxTag::Component(self.js_ref(t)),
                    t => js::JsxTag::Intrinsic(t.to_string()),
                };
                (tag, Vec::new(), children)
            }
            _ => return Err(self.unsupported(span, "this JSX binding's signature")),
        };
        Ok(Expr::jsx(js::Jsx { tag, props, children }))
    }

    /// A component's props as attributes: a struct's fields, with its
    /// `children` as the element's; `()` for none; anything else spread,
    /// `{...props}`.
    pub(super) fn jsx_props(&mut self, props: ExprId, out: &mut Vec<Stmt>) -> R<(Vec<Prop>, Vec<Expr>)> {
        let ty = self.thir[props].ty;
        let value = self.expr(props, out)?;
        let mut fields = match value.kind {
            js::ExprKind::Undefined => return Ok((Vec::new(), Vec::new())),
            js::ExprKind::Object(fields) => fields,
            _ => return Ok((vec![Prop::Spread(value)], Vec::new())),
        };
        // As written, as JSX's are, its children last; then what a base
        // gives, in the struct's order (ADR 0203). Rust makes them in that
        // order too.
        if let ExprKind::Adt(adt) = &self.thir[super::body_queries::strip(self.thir, props)].kind
            && adt.adt_def.is_struct()
            && fields.len() == adt.adt_def.non_enum_variant().fields.len()
        {
            let mut declared: Vec<Option<Prop>> = fields.into_iter().map(Some).collect();
            let mut written: Vec<Prop> = adt
                .fields
                .iter()
                .filter_map(|f| declared[f.name.as_usize()].take())
                .collect();
            written.extend(declared.into_iter().flatten());
            fields = written;
        }
        // Read before the children, where a prop after them, a base's, is
        // made after them, as Rust makes it, and the order shows: it does
        // something, or reads what children that do something might change,
        // `{..base}` of `bump(&mut base)`. A constant is made nowhere, and
        // children that read only what never changes read the same after.
        if let Some(i) = fields
            .iter()
            .position(|p| matches!(p, Prop::Field(name, _) if name == "children"))
            && let Prop::Field(_, children) = &fields[i]
            && fields[i + 1..].iter().any(|p| {
                let (Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value)) = p;
                (value.has_effects() && !self.reads_unchanging(children, out))
                    || (children.has_effects() && !value.is_constant())
            })
        {
            for prop in &mut fields {
                self.read_first(prop, out);
            }
        }
        let rest_fields: Vec<String> = match self.shape(ty) {
            Shape::Object(types) => (types.into_iter().enumerate())
                .filter(|&(i, _)| super::bindings::is_rest_field(self.tcx, ty, i))
                .map(|(_, (name, _))| name)
                .collect(),
            _ => Vec::new(),
        };
        // Where each prop is written, by its name: its value's place in the
        // source, a flattened struct made here followed in (ADR 0213).
        let mut written = HashMap::new();
        self.written_at(props, ty, &mut written);
        let mut attrs = Vec::new();
        let mut children = Vec::new();
        for field in fields {
            match field {
                // None given, an `Element`'s default (ADR 0192).
                Prop::Field(name, value) if name == "children" && matches!(value.kind, js::ExprKind::Undefined) => {}
                // What a `Rest` holds is the element's props, `{...rest}` (ADR 0195),
                // and so is a flattened struct's: one made here, each field
                // given, an attribute of its own (ADR 0204).
                Prop::Field(name, value) if rest_fields.contains(&name) => {
                    let Shape::Object(types) = self.shape(ty) else {
                        unreachable!("a struct's fields")
                    };
                    let field_ty = types.iter().find(|(n, _)| *n == name).expect("the field").1;
                    let own: Vec<String> = (types.iter())
                        .filter(|(n, _)| !rest_fields.contains(n))
                        .map(|(n, _)| n.clone())
                        .collect();
                    let span = self.thir[props].span;
                    for prop in self.flattened_attrs(field_ty, value, &own, span)? {
                        // A spread is where its struct is written.
                        let at = match &prop {
                            Prop::Spread(_) => written.get(&name).copied(),
                            Prop::Field(key, _) | Prop::Getter(key, _) => written.get(key).copied(),
                        };
                        attrs.push((at, prop));
                    }
                }
                Prop::Field(name, value) if name == "children" => {
                    let Shape::Object(types) = self.shape(ty) else {
                        unreachable!("a struct's fields")
                    };
                    let child_ty = types.into_iter().find(|(n, _)| *n == name).expect("the field").1;
                    children = self.spread_children(value, child_ty, out);
                }
                other => {
                    let at = match &other {
                        Prop::Field(key, _) | Prop::Getter(key, _) => written.get(key).copied(),
                        Prop::Spread(_) => None,
                    };
                    attrs.push((at, other));
                }
            }
        }
        // As written, where a struct's companion made them in its order
        // (ADR 0213): what does something is made first, in Rust's order.
        let order = |attrs: &[(Option<BytePos>, Prop)]| -> Vec<usize> {
            let mut order: Vec<usize> = (0..attrs.len()).collect();
            order.sort_by_key(|&i| attrs[i].0.unwrap_or(BytePos(u32::MAX)));
            order
        };
        let sorted = order(&attrs);
        if sorted.iter().enumerate().any(|(i, &j)| i != j) {
            let effects = |(_, prop): &(Option<BytePos>, Prop)| {
                let (Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value)) = prop;
                value.has_effects()
            };
            if attrs.iter().filter(|attr| effects(attr)).count() > 1 {
                for (_, prop) in &mut attrs {
                    self.read_first(prop, out);
                }
            }
        }
        let mut attrs: Vec<Option<Prop>> = attrs.into_iter().map(|(_, prop)| Some(prop)).collect();
        let attrs = sorted.into_iter().filter_map(|i| attrs[i].take()).collect();
        Ok((attrs, children))
    }

    /// `prop`'s value read into a `const` first where it isn't read alike
    /// (ADR 0194), an object made here each of its values, so a flattened
    /// struct's stays one, taken apart where it's given (ADR 0213).
    fn read_first(&mut self, prop: &mut Prop, out: &mut Vec<Stmt>) {
        let (name, value) = match prop {
            Prop::Field(name, value) | Prop::Getter(name, value) => (name.as_str(), value),
            Prop::Spread(value) => ("props", value),
        };
        if let js::ExprKind::Object(props) = &mut value.kind {
            for prop in props {
                self.read_first(prop, out);
            }
        } else if !self.reads_alike(value, out) {
            let old = std::mem::replace(value, Expr::undefined());
            *value = self.spill(&camel_case(&js_ident(name)), old, out);
        }
    }

    /// Where each prop `props`, a struct of type `ty`, is given is written,
    /// by its name: a field's value's place, and a flattened struct's made
    /// here, each of its own (ADR 0213).
    fn written_at(&self, props: ExprId, ty: Ty<'tcx>, written: &mut HashMap<String, BytePos>) {
        let ExprKind::Adt(adt) = &self.thir[super::body_queries::strip(self.thir, props)].kind else {
            return;
        };
        let Shape::Object(types) = self.shape(ty) else { return };
        let made_here = |field: &thir::FieldExpr| {
            let i = field.name.as_usize();
            super::bindings::is_flatten_field(self.tcx, ty, i)
                && matches!(
                    self.thir[super::body_queries::strip(self.thir, field.expr)].kind,
                    ExprKind::Adt(_)
                )
        };
        // Its own names first, which a flattened struct's of the same name,
        // never given, doesn't take the place of (ADR 0205).
        for field in adt.fields.iter().filter(|field| !made_here(field)) {
            let key = &types[field.name.as_usize()].0;
            written
                .entry(key.clone())
                .or_insert(self.thir[field.expr].span.source_callsite().lo());
        }
        for field in adt.fields.iter().filter(|field| made_here(field)) {
            let mut inner = HashMap::new();
            self.written_at(field.expr, types[field.name.as_usize()].1, &mut inner);
            for (key, at) in inner {
                written.entry(key).or_insert(at);
            }
        }
    }

    /// The attributes `value`, a flattened struct of type `ty`, gives: each
    /// field of one made here, and of a flattened one in it, but one whose
    /// name the props have, `own`, which theirs is (ADR 0205); another
    /// value, `{...value}`.
    fn flattened_attrs(&mut self, ty: Ty<'tcx>, value: Expr, own: &[String], span: Span) -> R<Vec<Prop>> {
        let js::ExprKind::Object(props) = value.kind else {
            return Ok(match value.kind {
                js::ExprKind::Undefined => Vec::new(),
                _ => vec![Prop::Spread(value)],
            });
        };
        let Shape::Object(types) = self.shape(ty) else {
            unreachable!("a flattened struct's fields")
        };
        let mut shadowing = own.to_vec();
        shadowing.extend(
            (types.iter().enumerate())
                .filter(|&(i, _)| !super::bindings::is_flatten_field(self.tcx, ty, i))
                .map(|(_, (n, _))| n.clone()),
        );
        let mut attrs = Vec::new();
        for (i, prop) in props.into_iter().enumerate() {
            match prop {
                Prop::Field(_, value) if super::bindings::is_flatten_field(self.tcx, ty, i) => {
                    attrs.extend(self.flattened_attrs(types[i].1, value, &shadowing, span)?);
                }
                Prop::Field(name, value) if own.contains(&name) => {
                    if !matches!(value.kind, js::ExprKind::Undefined) {
                        return Err(self.unsupported(
                            span,
                            &format!("giving flattened props' `{name}`, which the props have too: give theirs"),
                        ));
                    }
                }
                prop => attrs.push(prop),
            }
        }
        Ok(attrs)
    }

    pub(super) fn jsx_children(&mut self, e: ExprId, out: &mut Vec<Stmt>) -> R<Vec<Expr>> {
        let value = self.expr(e, out)?;
        Ok(self.spread_children(value, self.thir[e].ty, out))
    }

    /// A tuple of children is several, `("Count is ", count)`: `Count is {count}`.
    /// Anything else is one: a `Vec` is `{items}`, which React renders item by item.
    pub(super) fn spread_children(&mut self, value: Expr, ty: Ty<'tcx>, out: &mut Vec<Stmt>) -> Vec<Expr> {
        let ty::Tuple(tys) = *ty.kind() else { return vec![value] };
        let parts = match value.kind {
            js::ExprKind::Array(items) => items,
            _ if tys.is_empty() => Vec::new(),
            _ => {
                let tuple = if value.has_effects() {
                    self.spill("children", value, out)
                } else {
                    value
                };
                (0..tys.len())
                    .map(|i| Expr::index(tuple.clone(), Expr::int(i as i128)))
                    .collect()
            }
        };
        parts
            .into_iter()
            .zip(tys.iter())
            .flat_map(|(part, t)| self.spread_children(part, t, out))
            .collect()
    }

    /// `{}` or `{__html}`: an object literal, its fields the arguments.
    pub(super) fn object_binding(
        &mut self,
        keys: &[String],
        args: &[ExprId],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if keys.len() != args.len() {
            return Err(self.unsupported(span, "an object binding whose fields don't match its arguments"));
        }
        let values = self.operands(args, out)?;
        Ok(Expr::object(
            keys.iter()
                .cloned()
                .zip(values)
                .map(|(k, v)| Prop::Field(k, v))
                .collect(),
        ))
    }

    /// `style.color("red")` on an object being built: one more field. Its
    /// earlier fields go in `const`s first when this one's value needs
    /// statements, so they're still evaluated first.
    fn object_field(&mut self, mut object: Expr, name: String, value: ExprId, out: &mut Vec<Stmt>) -> R<Expr> {
        let js::ExprKind::Object(fields) = &mut object.kind else {
            unreachable!("checked by the caller")
        };
        if !self.is_simple(value) {
            for prop in fields.iter_mut() {
                let (base, value) = match prop {
                    Prop::Field(name, value) | Prop::Getter(name, value) => (name.as_str(), value),
                    Prop::Spread(value) => ("props", value),
                };
                if !value.is_constant() {
                    let old = std::mem::replace(value, Expr::undefined());
                    *value = self.spill(&camel_case(&js_ident(base)), old, out);
                }
            }
        }
        let value = self.expr(value, out)?;
        let js::ExprKind::Object(fields) = &mut object.kind else {
            unreachable!("checked by the caller")
        };
        // `prop ...`: another object spread over what's set so far.
        fields.push(match name.as_str() {
            "..." => Prop::Spread(value),
            _ => Prop::Field(name, value),
        });
        Ok(object)
    }

    /// `element.class_name("hero")`, a binding like `#[rust_js::link_name =
    /// "prop className"]`: the attribute, on the element being built.
    pub(super) fn jsx_prop(&mut self, name: Option<&str>, args: &[ExprId], span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let (name, value) = match (name, args) {
            (Some(name), &[_, value]) => (name.to_string(), value),
            (None, &[_, name, value]) => match self.thir[self.strip_refs(name)].kind {
                ExprKind::Literal { lit, neg: false } if let LitKind::Str(s, _) = lit.node => (s.to_string(), value),
                _ => {
                    let message = "rust-js: an attribute's name is a string literal";
                    return Err(self.tcx.dcx().span_err(self.thir[name].span, message));
                }
            },
            _ => return Err(self.unsupported(span, "this JSX attribute binding's signature")),
        };
        let mut element = self.expr(args[0], out)?;
        if let js::ExprKind::Object(_) = element.kind {
            return self.object_field(element, name, value, out);
        }
        if !matches!(element.kind, js::ExprKind::Jsx(_)) {
            let message = "rust-js makes JSX from one expression: set an element's props in the chain that makes it";
            return Err(self.tcx.dcx().span_err(self.thir[args[0]].span, message));
        }
        if name == "..."
            && !matches!(self.shape(self.thir[value].ty), Shape::Object(_))
            && !super::bindings::is_rest(self.tcx, self.thir[value].ty)
        {
            return Err(self.unsupported(self.thir[value].span, "JSX props spread of a non-struct value"));
        }
        // The value, and what it needs done first, its statements, aside: JSX
        // reads its attributes in order, then its children, as Rust does, so
        // only those statements, which run before the whole element, would
        // jump ahead of what's read already, and only matter where the
        // value does more than read.
        let mut first = Vec::new();
        let (children, mut lowered) = match name.as_str() {
            "children" => (self.jsx_children(value, &mut first)?, None),
            _ => (Vec::new(), Some(self.expr(value, &mut first)?)),
        };
        // The receiver is evaluated before the argument. JSX prints attributes
        // before children; later attributes must not jump ahead of earlier
        // children. Likewise, statements introduced by an argument must not
        // jump ahead of any deferred receiver evaluations. These temporaries
        // preserve Rust evaluation order, independently of output formatting.
        let js::ExprKind::Jsx(jsx) = &mut element.kind else {
            unreachable!("checked above")
        };
        if (!first.is_empty() && !self.is_simple(value)) || (name != "children" && !jsx.children.is_empty()) {
            // One read already, a `const` of its own, is read as it is (ADR 0194).
            for prop in &mut jsx.props {
                let (base, value) = match prop {
                    Prop::Field(name, value) | Prop::Getter(name, value) => (name.as_str(), value),
                    Prop::Spread(value) => ("props", value),
                };
                if !self.reads_alike(value, out) {
                    let old = std::mem::replace(value, Expr::undefined());
                    *value = self.spill(&camel_case(&js_ident(base)), old, out);
                }
            }
            for value in &mut jsx.children {
                if !self.reads_alike(value, out) {
                    let old = std::mem::replace(value, Expr::undefined());
                    *value = self.spill("children", old, out);
                }
            }
        }
        out.extend(first);
        let js::ExprKind::Jsx(jsx) = &mut element.kind else {
            unreachable!("checked above")
        };
        let Some(mut value) = lowered.take() else {
            jsx.children.extend(children);
            return Ok(element);
        };
        // `(e) => { if (f != null) { f(e); } }` as an event's handler is
        // `f`: React ignores what a handler returns, and no handler does
        // what one that calls nothing does (ADR 0198).
        let event = name
            .strip_prefix("on")
            .is_some_and(|rest| rest.starts_with(char::is_uppercase));
        if event {
            let spilled = match (&value.kind, out.last().map(|s| &s.kind)) {
                (js::ExprKind::Var(var), Some(StmtKind::Const(declared, arrow))) if var == declared => {
                    passed_handler(arrow)
                }
                _ => None,
            };
            if let Some(handler) = spilled {
                out.pop();
                value = handler;
            } else if let Some(handler) = passed_handler(&value) {
                value = handler;
            }
        }
        jsx.props.push(if name == "..." {
            Prop::Spread(value)
        } else {
            Prop::Field(name, value)
        });
        Ok(element)
    }
}

/// JSX syntax evaluated here, rather than inside a callback passed elsewhere.
fn jsx_tree(value: &Expr) -> bool {
    match &value.kind {
        js::ExprKind::Jsx(_) => true,
        js::ExprKind::Cond(_, yes, no) => jsx_tree(yes) || jsx_tree(no),
        js::ExprKind::Array(items) => items.iter().any(jsx_tree),
        _ => false,
    }
}

/// `f` of `(e) => { if (f != null) { f(e); } }`: a handler that calls
/// another, if there is one, with what it's given (ADR 0198).
fn passed_handler(value: &Expr) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &value.kind else {
        return None;
    };
    let [js::Pattern::Name(param)] = params.as_slice() else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::If(test, then, None),
            ..
        },
    ] = body.as_slice()
    else {
        return None;
    };
    let js::ExprKind::Binary(js::Op::LooseNe, tested, null) = &test.kind else {
        return None;
    };
    let (js::ExprKind::Var(handler), js::ExprKind::Null) = (&tested.kind, &null.kind) else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Expr(call),
            ..
        },
    ] = then.as_slice()
    else {
        return None;
    };
    let js::ExprKind::Call(callee, args) = &call.kind else {
        return None;
    };
    let given = matches!(args.as_slice(), [arg] if matches!(&arg.kind, js::ExprKind::Var(a) if a == param));
    let called = matches!(&callee.kind, js::ExprKind::Var(c) if c == handler);
    (given && called && handler != param).then(|| (**tested).clone())
}
