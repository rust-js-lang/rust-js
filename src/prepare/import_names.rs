//! Each import of the crate's modules spelled apart from every name its
//! module has, as it's prepared (ADR 1150): link decides what a module
//! imports, each import a `Name` of its own, `Name::imported`, its word the
//! one its `use` gives it; here, before any other pass, each that may be
//! renamed takes the first word no binding of the module, its lowering's
//! reserved words or another import takes, `useState$1`, in the order its
//! module asked for them; then a local a runtime helper's read is in the
//! scope of gives way to it (#1975). A JS module's or a helper's import
//! keeps its word.

use crate::collections::IndexSet;
use crate::js::{self, Expr, ExprKind, Function, Item, JsxTag, Name, Pattern, Prop, Stmt, StmtKind};

/// `module`'s imports spelled, then its locals given way to helpers.
pub(super) fn spelled(module: &mut js::Module) {
    let imports: IndexSet<Name> = module.imports.iter().flat_map(js::Import::locals).cloned().collect();
    let mut taken = module.import_names.reserved.clone();
    visit(
        &module.items,
        &mut Words {
            taken: &mut taken,
            imports: &imports,
        },
    );
    for name in std::mem::take(&mut module.import_names.apart) {
        let to = crate::names::fresh_in(&mut taken, name.word());
        if to == name.word() {
            continue;
        }
        js::respelled(&mut module.items, &name, &to);
        for import in &mut module.imports {
            import
                .locals_mut()
                .filter(|local| **local == name)
                .for_each(|local| local.respell(to.clone()));
        }
    }
    // A local a helper's read is in the scope of, given apart from it, now
    // its tree's reads are all there (ADR 1150, #1975).
    let top: Vec<String> = (module.imports.iter())
        .flat_map(js::Import::locals)
        .map(|local| local.word().to_string())
        .chain(js::scopes::packaged(&module.packages).cloned())
        .collect();
    js::scopes::locals_apart_from_module_reads(&top, &mut module.items);
}

/// The words a module's bindings and reads take, but its imports' reads.
struct Words<'a> {
    taken: &'a mut IndexSet<String>,
    imports: &'a IndexSet<Name>,
}

impl Words<'_> {
    fn name(&mut self, name: &str) {
        self.taken.insert(name.to_owned());
    }
}

fn visit(items: &[Item], visitor: &mut Words<'_>) {
    for item in items {
        match item {
            Item::Function(function) => function_names(function, visitor),
            Item::Namespace(namespace) => {
                visitor.name(namespace.name.word());
                for constant in &namespace.constants {
                    expr(&constant.value, visitor);
                }
                for function in &namespace.methods {
                    function_names(function, visitor);
                }
            }
            Item::Const(constant) => {
                visitor.name(constant.name.word());
                constant.fields.names.iter().for_each(|(field, _)| visitor.name(field));
                expr(&constant.value, visitor);
            }
            Item::Statements(stmts) => block(stmts, visitor),
        }
    }
}

fn function_names(function: &Function, visitor: &mut Words<'_>) {
    visitor.name(function.name.word());
    for param in &function.params {
        pattern(param, visitor);
    }
    block(&function.body, visitor);
}

fn pattern(p: &Pattern, visitor: &mut Words<'_>) {
    // A default, `{ size = "md" }`, is a literal: it names nothing.
    p.names().into_iter().for_each(|n| visitor.name(n));
}

fn block(body: &[Stmt], visitor: &mut Words<'_>) {
    for statement in body {
        match &statement.kind {
            StmtKind::Const(n, e) => {
                visitor.name(n.word());
                expr(e, visitor);
            }
            StmtKind::Let(n, e) => {
                visitor.name(n.word());
                if let Some(e) = e {
                    expr(e, visitor);
                }
            }
            StmtKind::Destructure { pattern: p, value, .. } => {
                pattern(p, visitor);
                expr(value, visitor);
            }
            StmtKind::Assign(a, b) | StmtKind::Compound(_, a, b) => {
                expr(a, visitor);
                expr(b, visitor);
            }
            StmtKind::Expr(e) | StmtKind::Throw(e) | StmtKind::Return(Some(e)) => expr(e, visitor),
            StmtKind::If(e, yes, no) => {
                expr(e, visitor);
                block(yes, visitor);
                if let Some(no) = no {
                    block(no, visitor);
                }
            }
            StmtKind::While { cond, body, .. } | StmtKind::DoWhile { cond, body, .. } => {
                expr(cond, visitor);
                block(body, visitor);
            }
            StmtKind::ForOf {
                pattern: p,
                iterable,
                body,
                ..
            } => {
                pattern(p, visitor);
                expr(iterable, visitor);
                block(body, visitor);
            }
            StmtKind::For {
                name: n,
                start,
                test,
                step,
                body,
                ..
            } => {
                visitor.name(n.word());
                expr(start, visitor);
                expr(test, visitor);
                if let Some(value) = step.value() {
                    expr(value, visitor);
                }
                block(body, visitor);
            }
            StmtKind::Labeled(_, body) => block(body, visitor),
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                expr(discriminant, visitor);
                for crate::js::Case { values, body, .. } in cases.iter() {
                    values.iter().for_each(|value| expr(value, visitor));
                    block(body, visitor);
                }
                if let Some(default) = default {
                    block(default, visitor);
                }
            }
            StmtKind::Function(function) => function_names(function, visitor),
            StmtKind::Try(body, finally) => {
                block(body, visitor);
                block(finally, visitor);
            }
            StmtKind::TryCatch(body, error, handler) => {
                block(body, visitor);
                if let Some(error) = error {
                    visitor.name(error.word());
                }
                block(handler, visitor);
            }
            StmtKind::Return(None)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Directive(_)
            | StmtKind::Comments => {}
        }
    }
}

fn expr(e: &Expr, visitor: &mut Words<'_>) {
    match &e.kind {
        ExprKind::Symbol(_) => {}
        // Not an import's own read: what it's spelled apart from.
        ExprKind::Var(n) if !visitor.imports.contains(n) => visitor.name(n.word()),
        ExprKind::Var(_) => {}
        ExprKind::Assigned(target, a) => {
            expr(target, visitor);
            expr(a, visitor);
        }
        ExprKind::Member(a, _)
        | ExprKind::OptionalMember(a, _)
        | ExprKind::Unary(_, a)
        | ExprKind::Await(a)
        | ExprKind::Unbound(a)
        | ExprKind::Spread(a)
        | ExprKind::Stringed(a)
        | ExprKind::Handle(a, _)
        | ExprKind::Stepped(_, a)
        | ExprKind::DropArgument(_, _, a) => expr(a, visitor),
        ExprKind::Index(a, b) | ExprKind::OptionalIndex(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Pair(a, b, _) => {
            expr(a, visitor);
            expr(b, visitor);
        }
        ExprKind::Cond(a, b, c) => {
            expr(a, visitor);
            expr(b, visitor);
            expr(c, visitor);
        }
        ExprKind::Call(f, args) | ExprKind::OptionalCall(f, args) | ExprKind::New(f, args) => {
            expr(f, visitor);
            for a in args {
                expr(a, visitor);
            }
        }
        ExprKind::Array(items) | ExprKind::Template(_, items, _) | ExprKind::Sequence(items) => {
            for item in items {
                expr(item, visitor);
            }
        }
        ExprKind::Object(props, ..) => properties(props, visitor),
        ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
            for param in params {
                pattern(param, visitor);
            }
            block(body, visitor);
        }
        ExprKind::Function(function) => function_names(function, visitor),
        ExprKind::Hole(_) | ExprKind::Import(_) | ExprKind::Specifier(_) => {}
        ExprKind::Jsx(jsx) => {
            if let JsxTag::Component(e) = &jsx.tag {
                expr(e, visitor);
            }
            properties(&jsx.props, visitor);
            for child in &jsx.children {
                expr(child, visitor);
            }
        }
        ExprKind::Num(_)
        | ExprKind::BigInt(_)
        | ExprKind::BigUint(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Written(_)
        | ExprKind::Undefined
        | ExprKind::GivenUndefined
        | ExprKind::Null
        | ExprKind::This
        | ExprKind::Super
        | ExprKind::Regex(_) => {}
    }
}

fn properties(props: &[Prop], visitor: &mut Words<'_>) {
    for prop in props {
        match prop {
            Prop::Field(_, e) | Prop::Getter(_, e) | Prop::Setter(_, e) | Prop::Method(_, e) | Prop::Spread(e) => {
                expr(e, visitor)
            }
            Prop::Keyed(key, e) | Prop::KeyedMethod(key, e) => {
                expr(key, visitor);
                expr(e, visitor);
            }
        }
    }
}
