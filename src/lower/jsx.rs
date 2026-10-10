//! Lower JSX bindings into the framework-independent JS tree.

use super::recognition::Std;
use super::{FnCx, R, Shape, camel_case, fn_def, js_ident};
use crate::js;
use crate::js::{Expr, Prop, Stmt, StmtKind};
use rustc_ast::LitKind;
use rustc_hir::attrs::lang_items::LangItem;
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
    /// function, an import, or a variable nothing writes again, a `const`
    /// of `out`'s, a field of a plain Rust value of one, and a comparison or a
    /// conditional of them, which runs no code of its own: `version ===
    /// "canary"`, `!done`, `status != null`, `p.size === "S" ? 12 : 20`.
    fn reads_alike(&self, value: &Expr, out: &[Stmt]) -> bool {
        match &value.kind {
            js::ExprKind::Symbol(_) | js::ExprKind::Arrow(..) | js::ExprKind::AsyncArrow(..) => true,
            js::ExprKind::Binary(
                js::Op::Eq | js::Op::Ne | js::Op::LooseEq | js::Op::LooseNe | js::Op::And | js::Op::Or,
                a,
                b,
            ) => self.reads_alike(a, out) && self.reads_alike(b, out),
            js::ExprKind::Unary(js::UnaryOp::Not, a) => self.reads_alike(a, out),
            js::ExprKind::Cond(test, yes, no) => {
                self.reads_alike(test, out) && self.reads_alike(yes, out) && self.reads_alike(no, out)
            }
            // `p.size`: a field can't change while its value doesn't, where the
            // value is plain Rust data. A JS object's getter, `n.textContent`,
            // can, and so can what's reached through a `Cell` or a `&mut`.
            js::ExprKind::Member(object, _) => {
                matches!(&object.kind, js::ExprKind::Var(name) if self.plain_value(name))
                    && self.reads_alike(object, out)
            }
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
                    // An import's binding, which JS never lets the module change.
                    || self.krate.imports[&self.module].values().any(|alias| alias == name)
            }
            _ => value.is_constant(),
        }
    }

    /// Whether `name` is a variable nothing writes again of plain Rust data,
    /// whose fields can't change while it doesn't: no `&mut`, raw pointer or
    /// interior mutability anywhere in its type, nor a JS object, whose
    /// properties are getters.
    pub(super) fn plain_value(&self, name: &str) -> bool {
        self.locals.vars.iter().any(|(id, var)| {
            !var.mutable
                && matches!(&var.place.kind, js::ExprKind::Var(n) if n == name)
                && (self.tcx.typeck(id.0.owner.def_id).node_type(id.0).walk()).all(|arg| match arg.kind() {
                    ty::GenericArgKind::Type(t) => {
                        !matches!(t.kind(), ty::Ref(_, _, ty::Mutability::Mut) | ty::RawPtr(..))
                            && t.is_freeze(self.tcx, self.typing_env)
                            && !self.is_js_object(t)
                    }
                    _ => true,
                })
        })
    }

    /// Whether `value` reads only variables that never change, so it's the
    /// same read before or after anything else: a `useState` value, a
    /// parameter, and elements of them.
    pub(super) fn reads_unchanging(&self, value: &Expr, out: &[Stmt]) -> bool {
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
        // `{..base}` of `bump(&mut base)`. A constant is made nowhere, nor
        // is an object of them, a flattened struct none of whose fields is
        // given, and children that read only what never changes read the same after.
        if let Some(i) = fields
            .iter()
            .position(|p| matches!(p, Prop::Field(name, _) if name == "children"))
            && let Prop::Field(_, children) = &fields[i]
            && fields[i + 1..].iter().any(|p| {
                let (Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value)) = p;
                (value.has_effects() && !self.reads_unchanging(children, out))
                    || (children.has_effects() && !value.is_made_of_constants())
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
        // What gives each prop, by its name, where the struct is made here.
        let mut given = HashMap::new();
        if let ExprKind::Adt(adt) = &self.thir[super::body_queries::strip(self.thir, props)].kind
            && let Shape::Object(types) = self.shape(ty)
        {
            for field in adt.fields.iter() {
                given.insert(types[field.name.as_usize()].0.clone(), field.expr);
            }
        }
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
                    let flattened = self.flattened_attrs(field_ty, value, &own, span)?;
                    // Where its fields are written, the first: a struct made
                    // there, `AnchorHTMLAttributes { .., ..props }`, is.
                    let first = (flattened.iter())
                        .filter_map(|prop| match prop {
                            Prop::Field(key, _) | Prop::Getter(key, _) => written.get(key).copied(),
                            Prop::Spread(_) => None,
                        })
                        .min();
                    for prop in flattened {
                        // A spread is where its struct is written, before what
                        // it's updated with.
                        let at = match &prop {
                            Prop::Spread(_) => written.get(&name).copied().or(first),
                            Prop::Field(key, _) | Prop::Getter(key, _) => written.get(key).copied(),
                        };
                        attrs.push((at, prop));
                    }
                }
                Prop::Field(name, value) if name == "children" => {
                    let Shape::Object(types) = self.shape(ty) else {
                        unreachable!("a struct's fields")
                    };
                    let mut child_ty = types.into_iter().find(|(n, _)| *n == name).expect("the field").1;
                    // Shown only if a test holds, `test && <el />`, as an
                    // element's child is (ADR 0235). Given by reference, to
                    // children of any node, `&(a, b)`, what's referred to.
                    let value = match given.get(&name) {
                        Some(&child) => {
                            let child = self.referred_children(child);
                            child_ty = self.thir[child].ty;
                            self.shown_if(child, value)
                        }
                        None => value,
                    };
                    children = self.spread_children(value, child_ty, out);
                }
                // A callback of one call whose JS gives `undefined` anyway
                // returns it, `onSubmit={() => submit(false)}`: the component
                // reads what Rust's gives, `undefined` (ADR 0040).
                Prop::Field(name, mut value)
                    if js::is_handler_name(&name) && given.get(&name).is_some_and(|&e| self.calls_for_nothing(e)) =>
                {
                    js::returning_its_call(&mut value);
                    attrs.push((written.get(&name).copied(), Prop::Field(name, value)));
                }
                other => {
                    let at = match &other {
                        Prop::Field(key, _) | Prop::Getter(key, _) => written.get(key).copied(),
                        // A base's, `{..*props}`, before what it's updated
                        // with, which JSX's order says wins (ADR 0250).
                        Prop::Spread(_) => Some(BytePos(0)),
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

    /// The children a props struct's `&'a dyn ReactNode` is given, by
    /// reference and made any node, or its `Option` of one: what's referred
    /// to, `(a, b)` of `&(a, b)`, its children each a child, as JSX's are.
    fn referred_children(&self, e: ExprId) -> ExprId {
        let e = self.strip(e);
        match self.thir[e].kind {
            ExprKind::PointerCoercion { source, .. } => self.referred_children(source),
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => self.referred_children(arg),
            // `Some(&(a, b))`, of optional children.
            ExprKind::Adt(ref some) if self.option_of(self.thir[e].ty).is_some() && some.fields.len() == 1 => {
                self.referred_children(some.fields[0].expr)
            }
            _ => e,
        }
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
    /// Whether `e`, a prop's callback, boxed, counted or in `Some`, is a
    /// closure of one call whose JS gives `undefined` whatever it is: a
    /// function of the crate's that gives `()`, a closure of its own, or a
    /// binding that says so, `#[rust_js::returns_undefined]` (ADR 0040).
    fn calls_for_nothing(&self, e: ExprId) -> bool {
        let e = self.strip(e);
        let closure = match self.thir[e].kind {
            ExprKind::PointerCoercion { source, .. } => return self.calls_for_nothing(source),
            // `Box::new(f)`, `Rc::new(f)`: what recognition says is `f` itself.
            ExprKind::Call { fun, ref args, .. }
                if let [inner] = args[..]
                    && matches!(self.std_fn(fun), Some(Std::Same | Std::Leak)) =>
            {
                return self.calls_for_nothing(inner);
            }
            ExprKind::Adt(ref adt)
                if self.tcx.is_lang_item(adt.adt_def.did(), LangItem::Option) && adt.fields.len() == 1 =>
            {
                return self.calls_for_nothing(adt.fields[0].expr);
            }
            ExprKind::Closure(ref closure) => closure,
            _ => return false,
        };
        let body = self.krate.closures[&closure.closure_id];
        let thir = &body.thir;
        let mut call = super::body_queries::strip(thir, body.expr);
        if let ExprKind::Block { block } = thir[call].kind {
            call = match (&thir[block].stmts[..], thir[block].expr) {
                ([], Some(value)) => super::body_queries::strip(thir, value),
                ([stmt], None) => match thir[*stmt].kind {
                    thir::StmtKind::Expr { expr, .. } => super::body_queries::strip(thir, expr),
                    _ => return false,
                },
                _ => return false,
            };
        }
        let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
            return false;
        };
        let Some((def_id, _)) = fn_def(thir[fun].ty) else {
            return false;
        };
        if !thir[call].ty.is_unit() {
            return false;
        }
        match self.tcx.trait_of_assoc(def_id) {
            // A closure of the crate's, `next()`: Rust's, which gives nothing.
            Some(fn_trait) if self.tcx.fn_trait_kind_from_def_id(fn_trait).is_some() => {
                matches!(thir[args[0]].ty.peel_refs().kind(), ty::Closure(..))
            }
            _ if super::bindings::is_binding(self.tcx, def_id) => super::bindings::returns_undefined(self.tcx, def_id),
            _ => def_id.is_local(),
        }
    }

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
        // What an update's base gives, `..props`, where it's a props pattern's
        // rest: that rest spread, `{...props}`, before what the update gives,
        // as it holds no key its pattern named, which the props' own may be.
        let mut rest = None;
        for (i, prop) in props.into_iter().enumerate() {
            match prop {
                Prop::Field(_, value) if super::bindings::is_flatten_field(self.tcx, ty, i) => {
                    for attr in self.flattened_attrs(types[i].1, value, &shadowing, span)? {
                        match attr {
                            Prop::Spread(value) if matches!(&value.kind, js::ExprKind::Var(v) if self.locals.rests.contains_key(v)) => {
                                rest = Some(value)
                            }
                            attr => attrs.push(attr),
                        }
                    }
                }
                Prop::Field(name, value)
                    if matches!(&value.kind, js::ExprKind::Member(object, key)
                        if key == &name && matches!(&object.kind, js::ExprKind::Var(v) if self.locals.rests.contains_key(v))) =>
                {
                    let js::ExprKind::Member(object, _) = value.kind else {
                        unreachable!("matched above")
                    };
                    rest = Some(*object);
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
        if let Some(rest) = rest {
            attrs.insert(0, Prop::Spread(rest));
        }
        Ok(attrs)
    }

    pub(super) fn jsx_children(&mut self, e: ExprId, out: &mut Vec<Stmt>) -> R<Vec<Expr>> {
        let value = self.expr(e, out)?;
        let value = self.shown_if(e, value);
        Ok(self.spread_children(value, self.thir[e].ty, out))
    }

    /// A child shown only if a test holds, `test ? <b /> : undefined`, as JSX
    /// writes it, `test && <b />` (ADR 0235): the test is `false` when it
    /// fails, which renders nothing, as `undefined` does, as each test is a
    /// boolean, a Rust condition's or a pattern's (`x != null`, `!!text`),
    /// never `0` nor `""`, which render as text. An `Option` of a JS
    /// object mapped is one when there, so it's its own test:
    /// `variant.Icon && <variant.Icon />`.
    fn shown_if(&self, child: ExprId, value: Expr) -> Expr {
        // Children are a tuple, of tuples too: each its own.
        if let ExprKind::Tuple { fields } = &self.thir[self.strip(child)].kind
            && let js::ExprKind::Array(items) = &value.kind
            && fields.len() == items.len()
        {
            let items = (items.iter().cloned())
                .zip(fields.iter())
                .map(|(item, &field)| self.shown_if(field, item))
                .collect();
            return Expr {
                kind: js::ExprKind::Array(items),
                span: value.span,
            };
        }
        let js::ExprKind::Cond(test, shown, none) = &value.kind else {
            return value;
        };
        if !matches!(none.kind, js::ExprKind::Undefined) {
            return value;
        }
        let test = match &test.kind {
            // `!!error` of a value never falsy (ADR 0298): `error`, which
            // renders nothing when it isn't there.
            js::ExprKind::Unary(js::UnaryOp::Not, not)
                if let js::ExprKind::Unary(js::UnaryOp::Not, x) = &not.kind
                    && self.maps_js_object(child) =>
            {
                (**x).clone()
            }
            _ => (**test).clone(),
        };
        Expr {
            kind: js::ExprKind::Binary(js::Op::And, Box::new(test), shown.clone()),
            span: value.span,
        }
    }

    /// Is `child` a call of an `Option` of a JS object, `variant.icon.map(..)`,
    /// or of another value never falsy (ADR 0298)?
    fn maps_js_object(&self, child: ExprId) -> bool {
        matches!(self.thir[self.strip(child)].kind, ExprKind::Call { ref args, .. }
            if args.first().is_some_and(|&receiver| self
                .option_of(self.thir[receiver].ty)
                .is_some_and(|inner| self.is_js_object(inner.peel_refs()) || self.never_falsy(inner))))
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
        let value_id = value;
        let mut element = self.expr(args[0], out)?;
        if let js::ExprKind::Object(_) = element.kind {
            return self.object_field(element, name, value, out);
        }
        if !matches!(element.kind, js::ExprKind::Jsx(_)) {
            let message = "rust-js makes JSX from one expression: set an element's props in the chain that makes it";
            return Err(self.tcx.dcx().span_err(self.thir[args[0]].span, message));
        }
        // A struct's fields, or a JS value's own properties, a `Rest`'s or a
        // `js::Unknown` read from JSON say, or none, `undefined`, which JS
        // spreads as nothing (ADR 0268).
        let js_object = |ty: Ty<'tcx>| self.recognition().is_js_object(ty.peel_refs());
        // Through a reference, the struct itself, which JS spreads alike.
        let spread = self.thir[value].ty.peel_refs();
        if name == "..."
            && !matches!(self.shape(spread), Shape::Object(_))
            && !js_object(spread)
            && !self.option_of(spread).is_some_and(js_object)
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
        // A value that reads the same wherever it's read, with nothing to do
        // first, a key's `entry.url`, can't see what the children do, nor
        // they it (ADR 0254).
        let alike = first.is_empty() && lowered.as_ref().is_some_and(|v| self.reads_alike(v, out));
        let js::ExprKind::Jsx(jsx) = &mut element.kind else {
            unreachable!("checked above")
        };
        if (!first.is_empty() && !self.is_simple(value)) || (name != "children" && !jsx.children.is_empty() && !alike) {
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
                    passed_handler(arrow, self.calls_rust(value_id, true))
                }
                _ => None,
            };
            if let Some(handler) = spilled {
                out.pop();
                value = handler;
            } else if let Some(handler) = passed_handler(&value, self.calls_rust(value_id, true)) {
                value = handler;
            }
        }
        // A component's `key`, which `jsx!` gives last, goes where it was
        // written: `jsx!` reads each in that order already (ADR 0254). A
        // base's spread, written last and put first (ADR 0250), follows it.
        let at = match name.as_str() {
            "key" => {
                let written = self.js_span(self.thir[value_id].span).lo;
                jsx.props.iter().position(|prop| match prop {
                    Prop::Spread(v) if v.span.is_none() => true,
                    Prop::Field(_, v) | Prop::Getter(_, v) | Prop::Spread(v) => {
                        !v.span.is_none() && v.span.lo > written
                    }
                })
            }
            _ => None,
        };
        let prop = if name == "..." {
            Prop::Spread(value)
        } else {
            Prop::Field(name, value)
        };
        match at {
            Some(at) => jsx.props.insert(at, prop),
            None => jsx.props.push(prop),
        }
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
/// another, if there is one, with what it's given (ADR 0198). And of `() =>
/// f()`, one that calls another with nothing, which React gives the event
/// a Rust function has no parameter for.
fn passed_handler(value: &Expr, calls_rust: bool) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &value.kind else {
        return None;
    };
    if calls_rust
        && let [
            Stmt {
                kind: StmtKind::Expr(call) | StmtKind::Return(Some(call)),
                ..
            },
        ] = body.as_slice()
        && let js::ExprKind::Call(callee, args) = &call.kind
        && args.is_empty()
        && let js::ExprKind::Var(f) = &callee.kind
        && params.len() <= 1
        && !params.iter().any(|p| p.names().contains(&f.as_str()))
    {
        return Some((**callee).clone());
    }
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
