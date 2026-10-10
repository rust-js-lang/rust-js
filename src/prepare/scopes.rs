//! What each name a function reads is, as JS's scopes resolve it: a
//! declaration of its own, by its place in the function, or a name from
//! outside it. Two functions alike but for their names read the same where
//! each read is of the same declaration (ADR 0357).

use std::collections::HashMap;

use crate::js::{self, Expr, ExprKind, Function, JsxTag, Pattern, Prop, Stmt, StmtKind};

/// What a read is of: the declaration, by the order they're declared in,
/// or a name from outside.
#[derive(PartialEq, Eq, Debug)]
pub(super) enum Of {
    Declared(usize),
    Outside(String),
}

/// Each read in `function`, in order, and what it's of, were `renamed`'s
/// first name its second; `None` where a scope declares a name twice,
/// which JS refuses.
pub(super) fn reads(function: &Function, renamed: Option<(&str, &str)>) -> Option<Vec<Of>> {
    let mut walk = Walk::new(renamed);
    walk.function(function, false);
    (!walk.twice).then_some(walk.reads)
}

/// Each read in `e`, a constant's value, as [`reads`].
pub(super) fn reads_of(e: &Expr, renamed: Option<(&str, &str)>) -> Option<Vec<Of>> {
    let mut walk = Walk::new(renamed);
    walk.expr(e);
    (!walk.twice).then_some(walk.reads)
}

struct Walk {
    renamed: Option<(String, String)>,
    scopes: Vec<HashMap<String, usize>>,
    declared: usize,
    reads: Vec<Of>,
    twice: bool,
}

impl Walk {
    fn new(renamed: Option<(&str, &str)>) -> Self {
        Walk {
            renamed: renamed.map(|(from, to)| (from.to_string(), to.to_string())),
            scopes: Vec::new(),
            declared: 0,
            reads: Vec::new(),
            twice: false,
        }
    }

    /// `name`, or what it's renamed to.
    fn named(&self, name: &str) -> String {
        match &self.renamed {
            Some((from, to)) if name == from => to.clone(),
            _ => name.to_string(),
        }
    }

    fn declare(&mut self, name: &str) {
        let name = self.named(name);
        let id = self.declared;
        self.declared += 1;
        let scope = self.scopes.last_mut().expect("a declaration is in a scope");
        self.twice |= scope.insert(name, id).is_some();
    }

    fn read(&mut self, name: &str) {
        let name = self.named(name);
        let name = name.as_str();
        let of = (self.scopes.iter().rev())
            .find_map(|scope| scope.get(name).copied())
            .map_or_else(|| Of::Outside(name.to_string()), Of::Declared);
        self.reads.push(of);
    }

    /// A function's parameters and its body's own declarations are one scope;
    /// a function expression's own name, one outside them, which only it sees.
    fn function(&mut self, function: &Function, named: bool) {
        if named {
            self.scopes.push(HashMap::new());
            self.declare(&function.name);
        }
        self.scopes.push(HashMap::new());
        for param in &function.params {
            self.pattern(param);
        }
        self.block_in_scope(&function.body);
        self.scopes.pop();
        if named {
            self.scopes.pop();
        }
    }

    fn closure(&mut self, params: &[Pattern], body: &[Stmt]) {
        self.scopes.push(HashMap::new());
        for param in params {
            self.pattern(param);
        }
        self.block_in_scope(body);
        self.scopes.pop();
    }

    fn pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Name(name) => self.declare(name),
            Pattern::Array(items) => items.iter().flatten().for_each(|name| self.declare(name)),
            Pattern::Object(fields, rest) => {
                // A default is read where the field is missing, before the
                // names after it are made.
                for (_, name, default) in fields {
                    if let Some(default) = default {
                        self.expr(default);
                    }
                    self.declare(name);
                }
                rest.iter().for_each(|name| self.declare(name));
            }
        }
    }

    fn block(&mut self, stmts: &[Stmt]) {
        self.scopes.push(HashMap::new());
        self.block_in_scope(stmts);
        self.scopes.pop();
    }

    /// A block's `const`s, `let`s and functions are its scope's from its
    /// start, as JS hoists them, read before they're made or not.
    fn block_in_scope(&mut self, stmts: &[Stmt]) {
        self.hoist(stmts);
        self.stmts(stmts);
    }

    /// What `stmts` declare in their block, an `else` after a branch that
    /// leaves too, which the printer writes after its `if` (ADR 0237).
    fn hoist(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => self.declare(name),
                StmtKind::Destructure { pattern, .. } => self.pattern_names(pattern),
                StmtKind::Function(function) => self.declare(&function.name),
                StmtKind::If(_, then, Some(els)) if js::leaves(then) => self.hoist(els),
                _ => {}
            }
        }
    }

    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::If(test, then, Some(els)) if js::leaves(then) => {
                    self.expr(test);
                    self.block(then);
                    self.stmts(els);
                }
                _ => self.stmt(stmt),
            }
        }
    }

    /// What a hoisted pattern declares; its defaults are read in order, as
    /// the statement runs.
    fn pattern_names(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Name(name) => self.declare(name),
            Pattern::Array(items) => items.iter().flatten().for_each(|name| self.declare(name)),
            Pattern::Object(fields, rest) => {
                fields.iter().for_each(|(_, name, _)| self.declare(name));
                rest.iter().for_each(|name| self.declare(name));
            }
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Const(_, value) | StmtKind::Expr(value) | StmtKind::Throw(value) => self.expr(value),
            StmtKind::Let(_, value) | StmtKind::Return(value) => value.iter().for_each(|v| self.expr(v)),
            StmtKind::Destructure { pattern, value, .. } => {
                if let Pattern::Object(fields, _) = pattern {
                    fields.iter().flat_map(|(_, _, d)| d).for_each(|d| self.expr(d));
                }
                self.expr(value);
            }
            StmtKind::Assign(target, value) => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::If(test, then, els) => {
                self.expr(test);
                self.block(then);
                if let Some(els) = els {
                    self.block(els);
                }
            }
            StmtKind::While { cond, body, .. } => {
                self.expr(cond);
                self.block(body);
            }
            // A loop's variables are its head's scope, which its iterable,
            // start and test are read in too.
            StmtKind::ForOf {
                pattern,
                iterable,
                body,
                ..
            } => {
                self.scopes.push(HashMap::new());
                self.pattern(pattern);
                self.expr(iterable);
                self.block(body);
                self.scopes.pop();
            }
            StmtKind::For {
                name,
                start,
                test,
                body,
                ..
            } => {
                self.scopes.push(HashMap::new());
                self.declare(name);
                self.expr(start);
                self.expr(test);
                self.block(body);
                self.scopes.pop();
            }
            StmtKind::Labeled(_, body) => self.block(body),
            StmtKind::Try(body, finally) => {
                self.block(body);
                self.block(finally);
            }
            // A `catch`'s parameter and its block's own declarations are one scope.
            StmtKind::TryCatch(body, error, handler) => {
                self.block(body);
                self.scopes.push(HashMap::new());
                if let Some(error) = error {
                    self.declare(error);
                }
                self.block_in_scope(handler);
                self.scopes.pop();
            }
            StmtKind::Function(function) => self.function(function, false),
            StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Directive(_) => {}
        }
    }

    fn props(&mut self, props: &[Prop]) {
        for prop in props {
            match prop {
                Prop::Field(_, value) | Prop::Spread(value) | Prop::Getter(_, value) => self.expr(value),
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Var(name) => self.read(name),
            // What the printer writes of JS's own: `String(x)`, `undefined`,
            // `NaN` and `Infinity`.
            ExprKind::Stringed(a) => {
                self.read("String");
                self.expr(a);
            }
            ExprKind::Undefined | ExprKind::GivenUndefined => self.read("undefined"),
            ExprKind::Num(n) if n.is_nan() => self.read("NaN"),
            ExprKind::Num(n) if n.is_infinite() => self.read("Infinity"),
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Lines(_)
            | ExprKind::Null
            | ExprKind::Symbol(_)
            | ExprKind::Regex(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Import(_) => {}
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Spread(a)
            | ExprKind::Unary(_, a)
            | ExprKind::Await(a)
            | ExprKind::DropArgument(_, _, a)
            | ExprKind::Handle(a) => self.expr(a),
            ExprKind::Index(a, b)
            | ExprKind::OptionalIndex(a, b)
            | ExprKind::Binary(_, a, b)
            | ExprKind::Pair(a, b) => {
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
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter().for_each(|i| self.expr(i)),
            ExprKind::Object(props) => self.props(props),
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => self.closure(params, body),
            ExprKind::Function(function) => self.function(function, true),
            ExprKind::Jsx(jsx) => {
                if let JsxTag::Component(tag) = &jsx.tag {
                    self.expr(tag);
                }
                self.props(&jsx.props);
                jsx.children.iter().for_each(|c| self.expr(c));
            }
        }
    }
}
