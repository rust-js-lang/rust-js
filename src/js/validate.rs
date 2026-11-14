//! `js::validate`: what a tree must be before it's printed (#1681 change 3,
//! M1): each name it reads resolves, to a declaration in scope, an import,
//! the runtime's helper (`runtime::exports`) or a global (`Expr::global`,
//! `Known::Global`), and no global's read is of a name a declaration in
//! scope takes; a block declares a name once; each `break` and `continue`
//! lands on a loop, a `switch` or a label around it, within its function.
//! It keeps no list of names: what may be read undeclared is what its
//! owners made so. A tree that breaks one is rust-js's own mistake, refused,
//! never printed. Each module's tree is checked in every build, about 0.5%
//! of a compile (ADR 1143); each body between tidy's stages where
//! `stages_checked()`.

use super::{Case, Expr, ExprKind, Function, Item, JsxTag, Known, Module, Pattern, Prop, Stmt, StmtKind};
use crate::collections::IndexSet;

/// What's wrong of a tree, by its rule.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// A `break` or a `continue` that lands on nothing around it.
    Unlanded(String),
    /// A name a block declares twice, or a function's body declares again
    /// of a parameter.
    Twice(String),
    /// A read of a name nothing declares, made no global.
    Unresolved(String),
    /// A global's read of a name a declaration in scope takes, which JS
    /// reads instead.
    Shadowed(String),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::Unlanded(jump) => write!(f, "`{jump}` lands on nothing around it"),
            Problem::Twice(name) => write!(f, "`{name}` is declared twice in one block"),
            Problem::Unresolved(name) => write!(f, "`{name}` is read, and nothing declares it nor made it a global"),
            Problem::Shadowed(name) => {
                write!(
                    f,
                    "the global `{name}` is read where a declaration of `{name}` is in scope, which JS reads instead"
                )
            }
        }
    }
}

/// Whether each body is checked between tidy's stages, its contracts
/// (`mir/tidy/contracts.rs`), these rules' and its own, about 2% of a
/// compile: in a debug build, and where `RUST_JS_VALIDATE` is set, as
/// `.env.test` sets it of every process a test starts and CI's workflows of
/// theirs. A module's tree is checked in every build.
pub fn stages_checked() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| cfg!(debug_assertions) || std::env::var_os("RUST_JS_VALIDATE").is_some())
}

/// Each problem of `module`'s tree, its names' too.
pub fn module(module: &Module) -> Vec<Problem> {
    let mut cx = Cx::new(true);
    let mut top: IndexSet<String> = IndexSet::default();
    for import in &module.imports {
        top.extend(import.named.iter().map(|(_, local)| local.clone()));
        top.extend(import.namespace.iter().cloned());
    }
    for package in &module.packages {
        top.extend(package.named.iter().map(|(_, local)| local.clone()));
        top.extend(package.default.iter().cloned());
        top.extend(package.namespace.iter().cloned());
    }
    for item in &module.items {
        match item {
            Item::Namespace(namespace) => {
                top.insert(namespace.name.clone());
            }
            Item::Const(constant) => {
                top.insert(constant.name.clone());
            }
            Item::Function(function) => {
                top.insert(function.name.clone());
            }
            Item::Statements(stmts) => cx.declared_in(stmts, &mut top),
        }
    }
    cx.scopes.push(top);
    for item in &module.items {
        match item {
            Item::Namespace(namespace) => {
                namespace.constants.iter().for_each(|c| cx.expr(&c.value));
                namespace.methods.iter().for_each(|m| cx.function(m, false));
            }
            Item::Const(constant) => cx.expr(&constant.value),
            Item::Function(function) => cx.function(function, true),
            Item::Statements(stmts) => cx.stmts(stmts),
        }
    }
    cx.problems
}

/// Each problem of a function's `body` but its names', which only its
/// module resolves: of a body as tidy leaves each stage's (its contracts).
pub fn body(body: &[Stmt]) -> Vec<Problem> {
    let mut cx = Cx::new(false);
    cx.block(body, IndexSet::default());
    cx.problems
}

/// What a label is of: a loop, which `continue` may name, or a block.
#[derive(Clone, Copy, PartialEq)]
enum Target {
    Loop,
    Switch,
    Block,
}

struct Cx {
    /// Whether its reads are resolved: of a module, not of a body alone.
    names: bool,
    /// The names each scope declares, the innermost last.
    scopes: Vec<IndexSet<String>>,
    /// The loops, switches and labeled blocks around what's walked, by
    /// their label, within its function.
    labels: Vec<(Option<String>, Target)>,
    problems: Vec<Problem>,
}

impl Cx {
    fn new(names: bool) -> Cx {
        Cx {
            names,
            scopes: Vec::new(),
            labels: Vec::new(),
            problems: Vec::new(),
        }
    }

    /// The names `stmts` declare as a block, each once (a second is a
    /// problem), hoisted so a read of each anywhere in the block resolves.
    fn declared_in(&mut self, stmts: &[Stmt], into: &mut IndexSet<String>) {
        for stmt in stmts {
            let names: Vec<&str> = match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name.as_str()],
                StmtKind::Destructure { pattern, .. } => pattern.names(),
                StmtKind::Function(function) => vec![function.name.as_str()],
                _ => continue,
            };
            for name in names {
                if !into.insert(name.to_string()) {
                    self.problems.push(Problem::Twice(name.to_string()));
                }
            }
        }
    }

    fn read(&mut self, name: &str, expr: &Expr) {
        if !self.names {
            return;
        }
        let declared = self.scopes.iter().any(|scope| scope.contains(name));
        let global = expr.facts.has(Known::Global);
        match (declared, global) {
            (true, true) => self.problems.push(Problem::Shadowed(name.to_string())),
            (false, false) if !crate::runtime::exports(name) => {
                self.problems.push(Problem::Unresolved(name.to_string()))
            }
            _ => {}
        }
    }

    /// `stmts` as a block of their own.
    fn block(&mut self, stmts: &[Stmt], with: IndexSet<String>) {
        let mut scope = with;
        let params = scope.clone();
        self.declared_in(stmts, &mut scope);
        // A function's top-level declaration may not repeat a parameter.
        for name in &params {
            let again = stmts.iter().any(|stmt| match &stmt.kind {
                StmtKind::Const(n, _) | StmtKind::Let(n, _) => n == name,
                StmtKind::Destructure { pattern, .. } => pattern.names().contains(&name.as_str()),
                _ => false,
            });
            if again {
                self.problems.push(Problem::Twice(name.clone()));
            }
        }
        self.scopes.push(scope);
        self.stmts(stmts);
        self.scopes.pop();
    }

    /// `function`, named inside itself where `named`: a declaration's or a
    /// function expression's name, never a type's method's, an object's
    /// method or a function a prototype's property is given, which bind
    /// none, so a global of its name is the global inside it,
    /// `getComputedStyle`'s (#1681).
    fn function(&mut self, function: &Function, named: bool) {
        let mut params: IndexSet<String> = IndexSet::default();
        for param in &function.params {
            self.pattern_defaults(param);
            params.extend(param.names().into_iter().map(str::to_string));
        }
        let outer = std::mem::take(&mut self.labels);
        let own: IndexSet<String> = named.then(|| function.name.clone()).into_iter().collect();
        self.scopes.push(own);
        self.block(&function.body, params);
        self.scopes.pop();
        self.labels = outer;
    }

    fn pattern_defaults(&mut self, pattern: &Pattern) {
        for default in pattern.defaults() {
            self.expr(default);
        }
    }

    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    fn looped(&mut self, label: &Option<String>, body: &[Stmt], with: IndexSet<String>) {
        self.labels.push((label.clone(), Target::Loop));
        self.block(body, with);
        self.labels.pop();
    }

    fn jump(&mut self, label: &Option<String>, is_continue: bool) {
        let lands = match label {
            Some(label) => (self.labels.iter().rev())
                .any(|(l, target)| l.as_ref() == Some(label) && (!is_continue || *target == Target::Loop)),
            None => (self.labels.iter().rev()).any(|(_, target)| match is_continue {
                true => *target == Target::Loop,
                false => *target != Target::Block,
            }),
        };
        if !lands {
            let what = if is_continue { "continue" } else { "break" };
            let to = label.as_deref().map(|l| format!(" {l}")).unwrap_or_default();
            self.problems.push(Problem::Unlanded(format!("{what}{to}")));
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Const(_, value) | StmtKind::Expr(value) | StmtKind::Throw(value) => self.expr(value),
            StmtKind::Destructure { pattern, value, .. } => {
                self.pattern_defaults(pattern);
                self.expr(value);
            }
            StmtKind::Let(_, value) | StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Assign(target, value) | StmtKind::Compound(_, target, value) => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::If(test, then, els) => {
                self.expr(test);
                self.block(then, IndexSet::default());
                if let Some(els) = els {
                    self.block(els, IndexSet::default());
                }
            }
            StmtKind::While { label, cond, body } | StmtKind::DoWhile { label, body, cond } => {
                self.expr(cond);
                self.looped(label, body, IndexSet::default());
            }
            StmtKind::ForOf {
                label,
                pattern,
                iterable,
                body,
                ..
            } => {
                self.expr(iterable);
                self.pattern_defaults(pattern);
                let bound = pattern.names().into_iter().map(str::to_string).collect();
                self.looped(label, body, bound);
            }
            StmtKind::For {
                label,
                name,
                start,
                test,
                step,
                body,
            } => {
                self.expr(start);
                self.scopes.push([name.clone()].into_iter().collect());
                self.expr(test);
                if let Some(value) = step.value() {
                    self.expr(value);
                }
                self.looped(label, body, IndexSet::default());
                self.scopes.pop();
            }
            StmtKind::Labeled(label, body) => {
                self.labels.push((Some(label.clone()), Target::Block));
                self.block(body, IndexSet::default());
                self.labels.pop();
            }
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                self.expr(discriminant);
                self.labels.push((None, Target::Switch));
                // Each case a block of its own: one that declares is printed
                // in braces (`case_body`), and one that doesn't adds no name.
                for Case { values, body, .. } in cases {
                    values.iter().for_each(|value| self.expr(value));
                    self.block(body, IndexSet::default());
                }
                if let Some(default) = default {
                    self.block(default, IndexSet::default());
                }
                self.labels.pop();
            }
            StmtKind::Try(body, finally) => {
                self.block(body, IndexSet::default());
                self.block(finally, IndexSet::default());
            }
            StmtKind::TryCatch(body, param, handler) => {
                self.block(body, IndexSet::default());
                self.block(handler, param.iter().cloned().collect());
            }
            StmtKind::Function(function) => self.function(function, true),
            StmtKind::Break(label) => self.jump(label, false),
            StmtKind::Continue(label) => self.jump(label, true),
            StmtKind::Directive(_) | StmtKind::Comments => {}
        }
    }

    fn props(&mut self, props: &[Prop]) {
        for prop in props {
            match prop {
                Prop::Field(_, value)
                | Prop::Spread(value)
                | Prop::Getter(_, value)
                | Prop::Setter(_, value)
                | Prop::Method(_, value) => self.expr(value),
                Prop::Keyed(key, value) | Prop::KeyedMethod(key, value) => {
                    self.expr(key);
                    self.expr(value);
                }
            }
        }
    }

    fn expr(&mut self, expr: &Expr) {
        if expr.facts.has(super::Known::Literal) {
            return;
        }
        match &expr.kind {
            ExprKind::Var(name) => self.read(name, expr),
            ExprKind::Assigned(target, value) => {
                self.expr(target);
                self.expr(value);
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
            | ExprKind::DropArgument(_, _, a) => self.expr(a),
            ExprKind::Index(a, b)
            | ExprKind::OptionalIndex(a, b)
            | ExprKind::Binary(_, a, b)
            | ExprKind::Pair(a, b, _) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Cond(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            ExprKind::Call(f, args) | ExprKind::OptionalCall(f, args) | ExprKind::New(f, args) => {
                self.expr(f);
                args.iter().for_each(|a| self.expr(a));
            }
            ExprKind::Array(items) | ExprKind::Template(_, items, _) | ExprKind::Sequence(items) => {
                items.iter().for_each(|a| self.expr(a))
            }
            ExprKind::Object(fields, ..) => self.props(fields),
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
                let mut bound: IndexSet<String> = IndexSet::default();
                for param in params {
                    self.pattern_defaults(param);
                    bound.extend(param.names().into_iter().map(str::to_string));
                }
                let outer = std::mem::take(&mut self.labels);
                self.block(body, bound);
                self.labels = outer;
            }
            ExprKind::Function(function) => self.function(function, true),
            ExprKind::Jsx(jsx) => {
                if let JsxTag::Component(tag) = &jsx.tag {
                    self.expr(tag);
                }
                self.props(&jsx.props);
                jsx.children.iter().for_each(|c| self.expr(c));
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
            | ExprKind::Symbol(_)
            | ExprKind::ItemHole(_)
            | ExprKind::Import(_)
            | ExprKind::Specifier(_)
            | ExprKind::Regex(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::{Span, Step};

    fn stmt(kind: StmtKind) -> Stmt {
        kind.at(Span::NONE)
    }

    fn of(stmts: Vec<Stmt>) -> Vec<Problem> {
        module(&Module {
            items: vec![Item::Statements(stmts)],
            ..Module::default()
        })
    }

    fn read(e: Expr) -> Stmt {
        stmt(StmtKind::Expr(e))
    }

    // A read resolves to what's declared, a global made so, or a helper;
    // nothing else, whatever its name (#1681).
    #[test]
    fn a_read_resolves_or_is_refused() {
        assert_eq!(
            of(vec![read(Expr::var("console"))]),
            vec![Problem::Unresolved("console".into())]
        );
        assert_eq!(of(vec![read(Expr::global("console"))]), vec![]);
        assert_eq!(of(vec![read(Expr::helper("$try"))]), vec![]);
        let declared = stmt(StmtKind::Const("x".into(), Expr::num(1.0)));
        assert_eq!(of(vec![read(Expr::var("x")), declared]), vec![]);
        let arrow = Expr::arrow(vec![Pattern::Name("p".into())], vec![read(Expr::var("p"))]);
        assert_eq!(
            of(vec![read(arrow), read(Expr::var("p"))]),
            vec![Problem::Unresolved("p".into())]
        );
    }

    // A global's read where a declaration of its name is in scope reads
    // that instead: a wrong answer, refused.
    #[test]
    fn a_global_shadowed_is_refused() {
        let local = stmt(StmtKind::Let("Map".into(), None));
        let body = vec![local, read(Expr::global("Map"))];
        let f = Expr::arrow(Vec::new(), body);
        assert_eq!(of(vec![read(f)]), vec![Problem::Shadowed("Map".into())]);
        // A type's method binds no name of its own: the global of its name
        // read inside it is the global, `getComputedStyle`'s (#1681).
        let method = Function {
            name: "getComputedStyle".into(),
            params: Vec::new(),
            body: vec![read(Expr::global("getComputedStyle"))],
            export: false,
            is_async: false,
            noinline: false,
            span: Span::NONE,
            name_span: Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
        };
        let namespace = Item::Namespace(crate::js::Namespace {
            name: "Pseudo".into(),
            constants: Vec::new(),
            methods: vec![method],
            export: false,
            class: None,
        });
        let module = Module {
            items: vec![namespace],
            ..Module::default()
        };
        assert_eq!(super::module(&module), vec![]);
    }

    // A `break` lands on a `do .. while` too, and none crosses a function.
    #[test]
    fn jumps_land_within_their_function() {
        let do_while = stmt(StmtKind::DoWhile {
            label: None,
            body: vec![stmt(StmtKind::Break(None))],
            cond: Expr::bool(true),
        });
        assert_eq!(body(&[do_while]), vec![]);
        let crossing = Expr::arrow(Vec::new(), vec![stmt(StmtKind::Break(None))]);
        let loop_ = stmt(StmtKind::While {
            label: None,
            cond: Expr::bool(true),
            body: vec![read(crossing)],
        });
        assert_eq!(body(&[loop_]), vec![Problem::Unlanded("break".into())]);
        let counted = stmt(StmtKind::For {
            label: Some("o".into()),
            name: "i".into(),
            start: Expr::num(0.0),
            test: Expr::bool(true),
            step: Step::Up,
            body: vec![stmt(StmtKind::Continue(Some("o".into())))],
        });
        assert_eq!(body(&[counted]), vec![]);
        // A case's own `break`, though the printer writes the one that ends
        // it (`case_body`).
        let switch = stmt(StmtKind::Switch {
            discriminant: Expr::num(0.0),
            cases: vec![Case {
                values: vec![Expr::num(0.0)],
                body: vec![stmt(StmtKind::Break(None))],
                falls: false,
            }],
            default: None,
        });
        assert_eq!(body(&[switch]), vec![]);
    }

    // A block declares a name once, a destructuring's too; each case of a
    // `switch` is a block of its own.
    #[test]
    fn a_block_declares_a_name_once() {
        let pair = || Pattern::names_array([Some("a".into()), Some("b".into())]);
        let destructure = || {
            stmt(StmtKind::Destructure {
                pattern: pair(),
                value: Expr::var("v"),
                mutable: false,
            })
        };
        let v = stmt(StmtKind::Const("v".into(), Expr::array(Vec::new())));
        let twice = body(&[v.clone(), destructure(), destructure()]);
        assert_eq!(twice, vec![Problem::Twice("a".into()), Problem::Twice("b".into())]);
        let case = |body| Case {
            values: vec![Expr::num(0.0)],
            body,
            falls: false,
        };
        let switch = stmt(StmtKind::Switch {
            discriminant: Expr::num(0.0),
            cases: vec![case(vec![destructure()]), case(vec![destructure()])],
            default: None,
        });
        assert_eq!(body(&[v, switch]), vec![]);
    }
}
