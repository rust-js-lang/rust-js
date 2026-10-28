//! Resolve symbolic module references after lowering and reachability.
//! Locals keep their names; import aliases avoid every binding that could
//! shadow them, including parameters of nested arrows and copied defaults.

use crate::js::{Expr, ExprKind, Function, Item, JsxTag, Pattern, Prop, Stmt, StmtKind};
use crate::names::fresh_in;
use crate::program::{ImportRequest, Lowered, LoweredImport, LoweredModule, Unlinked};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Proof that import resolution and runtime dependency closure completed.
/// The private field prevents other phases from constructing this directly.
pub(crate) struct Linked(Lowered);

impl Linked {
    pub(crate) fn into_program(self) -> Lowered {
        self.0
    }
}

/// Consume symbolic output; printing only receives the completed linked crate.
pub(crate) fn link(unlinked: Unlinked) -> Linked {
    let mut modules: Vec<LoweredModule> = unlinked
        .modules
        .into_iter()
        .map(|mut item| {
            resolve(&mut item.module, &item.imports, item.reserved_names);
            // Each helper its tree reads, asked for by being written.
            let mut written = Vec::new();
            crate::js::each_item_expr_mut(&mut item.module.items, &mut |e| {
                if let Some(name) = e.helper_name() {
                    written.extend(crate::runtime::helper_of(name));
                }
            });
            written.sort();
            written.dedup();
            item.module.runtime = written;
            item.module
        })
        .collect();
    keep_imported(&mut modules);
    Linked(Lowered {
        library: unlinked.library,
        sources: unlinked.sources,
        modules,
        tests: unlinked.tests,
    })
}

/// A module of types alone that another module's JS imports, or
/// re-exports, `export * as bar from "./foo/bar.js"`, keeps its JS, though
/// nothing is in it: only a declaration's `import type` may name a module
/// whose JS isn't written (ADR 1005, amended).
fn keep_imported(modules: &mut [LoweredModule]) {
    let imported: HashSet<Vec<String>> = (modules.iter())
        .flat_map(|module| {
            let reexported = module.reexports.iter().filter_map(|reexport| match &reexport.from {
                crate::js::ImportFrom::Module(path) => Some(path.clone()),
                crate::js::ImportFrom::Specifier(_) => None,
            });
            module
                .imports
                .iter()
                .map(|import| import.path.clone())
                .chain(reexported)
        })
        .collect();
    for module in modules.iter_mut().filter(|module| imported.contains(&module.path)) {
        module.types_only = false;
    }
}

fn resolve(module: &mut LoweredModule, imports: &[ImportRequest], mut names: HashSet<String>) {
    let mut read = HashSet::new();
    visit(
        module,
        &mut Visitor {
            names: &mut names,
            replacements: None,
            read: &mut read,
        },
    );
    let mut grouped: BTreeMap<Vec<String>, Vec<(String, String)>> = BTreeMap::new();
    // An import of what the module reads: one its lowering asked for whose
    // read tidy took out, a test of a numbered enum's variant an
    // exhaustive `match` needn't write, `Dark` (ADR 0722), isn't written.
    let replacements: HashMap<_, _> = imports
        .iter()
        .filter(|request| read.contains(&request.symbol))
        .map(|request| {
            let alias = fresh_in(&mut names, &request.local);
            grouped
                .entry(request.path.clone())
                .or_default()
                .push((request.export.clone(), alias.clone()));
            (request.symbol.clone(), alias)
        })
        .collect();
    module.imports = grouped
        .into_iter()
        .map(|(path, named)| LoweredImport { path, named })
        .collect();
    visit(
        module,
        &mut Visitor {
            names: &mut names,
            replacements: Some(&replacements),
            read: &mut HashSet::new(),
        },
    );
}

struct Visitor<'a> {
    names: &'a mut HashSet<String>,
    replacements: Option<&'a HashMap<crate::js::Symbol, String>>,
    /// The other modules' items the module reads, by their symbols.
    read: &'a mut HashSet<crate::js::Symbol>,
}

impl Visitor<'_> {
    fn name(&mut self, name: &str) {
        if self.replacements.is_none() {
            self.names.insert(name.to_owned());
        }
    }
}

fn visit(module: &mut LoweredModule, visitor: &mut Visitor<'_>) {
    for item in &mut module.items {
        match item {
            Item::Function(function) => function_names(function, visitor),
            Item::Namespace(namespace) => {
                visitor.name(&namespace.name);
                for constant in &mut namespace.constants {
                    expr(&mut constant.value, visitor);
                }
                for function in &mut namespace.methods {
                    function_names(function, visitor);
                }
            }
            Item::Const(constant) => {
                visitor.name(&constant.name);
                expr(&mut constant.value, visitor);
            }
            Item::Statements(stmts) => block(stmts, visitor),
        }
    }
}

fn function_names(function: &mut Function, visitor: &mut Visitor<'_>) {
    visitor.name(&function.name);
    for param in &mut function.params {
        pattern(param, visitor);
    }
    block(&mut function.body, visitor);
}

fn pattern(p: &mut Pattern, visitor: &mut Visitor<'_>) {
    // A default, `{ size = "md" }`, is a literal: it names nothing.
    p.names_mut().into_iter().for_each(|n| visitor.name(n));
}

fn block(body: &mut [Stmt], visitor: &mut Visitor<'_>) {
    for statement in body {
        match &mut statement.kind {
            StmtKind::Const(n, e) => {
                visitor.name(n);
                expr(e, visitor);
            }
            StmtKind::Let(n, e) => {
                visitor.name(n);
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
                visitor.name(n);
                expr(start, visitor);
                expr(test, visitor);
                if let Some(value) = step.value_mut() {
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
                for crate::js::Case { values, body, .. } in cases.iter_mut() {
                    values.iter_mut().for_each(|value| expr(value, visitor));
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
                    visitor.name(error);
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

fn expr(e: &mut Expr, visitor: &mut Visitor<'_>) {
    if let ExprKind::Symbol(symbol) = &e.kind
        && let Some(replacements) = visitor.replacements
    {
        e.kind = ExprKind::Var(
            replacements
                .get(symbol)
                .expect("every symbolic module reference has a dependency")
                .clone(),
        );
    }
    match &mut e.kind {
        ExprKind::Symbol(symbol) => {
            visitor.read.insert(symbol.clone());
        }
        ExprKind::Var(n) => visitor.name(n),
        ExprKind::Assigned(target, a) => {
            expr(target, visitor);
            expr(a, visitor);
        }
        ExprKind::Member(a, _)
        | ExprKind::OptionalMember(a, _)
        | ExprKind::Unary(_, a)
        | ExprKind::Await(a)
        | ExprKind::Spread(a)
        | ExprKind::Stringed(a)
        | ExprKind::Handle(a, _)
        | ExprKind::Stepped(_, a)
        | ExprKind::DropArgument(_, _, a) => expr(a, visitor),
        ExprKind::Index(a, b) | ExprKind::OptionalIndex(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Pair(a, b) => {
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
        ExprKind::Array(items) | ExprKind::Template(_, items, _) => {
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
        ExprKind::ItemHole(_) | ExprKind::Import(_) => {}
        ExprKind::Jsx(jsx) => {
            if let JsxTag::Component(e) = &mut jsx.tag {
                expr(e, visitor);
            }
            properties(&mut jsx.props, visitor);
            for child in &mut jsx.children {
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
        | ExprKind::Regex(_) => {}
    }
}

fn properties(props: &mut [Prop], visitor: &mut Visitor<'_>) {
    for prop in props {
        match prop {
            Prop::Field(_, e) | Prop::Getter(_, e) | Prop::Setter(_, e) | Prop::Method(_, e) | Prop::Spread(e) => {
                expr(e, visitor)
            }
        }
    }
}
