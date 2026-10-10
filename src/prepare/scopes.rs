//! An item's scopes, as JS resolves its names: each scope's declarations,
//! and each read, where it is and the declaration it's of, or a name from
//! outside the item. A local renamed keeps each read its own where no
//! declaration of the new name comes between a read and what it reads
//! (ADR 0357).

use std::collections::HashMap;

use crate::js::{self, Expr, ExprKind, Function, JsxTag, Pattern, Prop, Stmt, StmtKind};

struct Scope {
    parent: Option<usize>,
    names: HashMap<String, usize>,
}

/// A read: the scope it's in, and the scope of the declaration it's of,
/// `None` of a name from outside.
struct Read {
    scope: usize,
    of: Option<usize>,
}

#[derive(Default)]
pub(super) struct Tree {
    scopes: Vec<Scope>,
    /// Each name's reads.
    reads: HashMap<String, Vec<Read>>,
}

impl Tree {
    /// `item`'s scopes; `None` of an `on_load!` body's, whose names are the
    /// module's (ADR 0267), or where a scope declares a name twice, which JS
    /// refuses.
    pub(super) fn of(item: &js::Item) -> Option<Tree> {
        let mut walk = Walk::default();
        walk.enter();
        match item {
            js::Item::Function(function) => walk.function(function, false),
            js::Item::Namespace(namespace) => namespace.methods.iter().for_each(|m| walk.function(m, false)),
            js::Item::Const(constant) => walk.expr(&constant.value),
            js::Item::Statements(_) => return None,
        }
        (!walk.twice).then_some(walk.tree)
    }

    fn declares(&self, scope: usize, name: &str) -> bool {
        self.scopes[scope].names.contains_key(name)
    }

    /// Does each read stay of what it's of, where `from` is named `to`? No
    /// scope that declares `from` declares `to` too; no read of `from`
    /// passes a declaration of `to` before its own; and no read of `to`
    /// passes one of `from` before its own.
    pub(super) fn renames(&self, from: &str, to: &str) -> bool {
        let declaring: Vec<usize> = (0..self.scopes.len()).filter(|&s| self.declares(s, from)).collect();
        if declaring.is_empty() || declaring.iter().any(|&s| self.declares(s, to)) {
            return false;
        }
        let passes = |read: &Read, name: &str| {
            let mut scope = Some(read.scope);
            while scope.is_some() && scope != read.of {
                let at = scope.expect("checked");
                if self.declares(at, name) {
                    return true;
                }
                scope = self.scopes[at].parent;
            }
            false
        };
        let none = Vec::new();
        let of_from = self.reads.get(from).unwrap_or(&none);
        let of_to = self.reads.get(to).unwrap_or(&none);
        of_from.iter().all(|r| r.of.is_some() && !passes(r, to)) && of_to.iter().all(|r| !passes(r, from))
    }

    /// `from` named `to`, where [`Tree::renames`] said it keeps each read.
    pub(super) fn rename(&mut self, from: &str, to: &str) {
        for scope in &mut self.scopes {
            if let Some(id) = scope.names.remove(from) {
                scope.names.insert(to.to_string(), id);
            }
        }
        if let Some(reads) = self.reads.remove(from) {
            self.reads.entry(to.to_string()).or_default().extend(reads);
        }
    }
}

#[derive(Default)]
struct Walk {
    tree: Tree,
    stack: Vec<usize>,
    declared: usize,
    twice: bool,
}

impl Walk {
    fn enter(&mut self) {
        let parent = self.stack.last().copied();
        self.tree.scopes.push(Scope {
            parent,
            names: HashMap::new(),
        });
        self.stack.push(self.tree.scopes.len() - 1);
    }

    fn leave(&mut self) {
        self.stack.pop();
    }

    fn declare(&mut self, name: &str) {
        let id = self.declared;
        self.declared += 1;
        let scope = *self.stack.last().expect("a declaration is in a scope");
        self.twice |= self.tree.scopes[scope].names.insert(name.to_string(), id).is_some();
    }

    fn read(&mut self, name: &str) {
        let scope = *self.stack.last().expect("a read is in a scope");
        let of = self.stack.iter().rev().copied().find(|&s| self.tree.declares(s, name));
        self.tree
            .reads
            .entry(name.to_string())
            .or_default()
            .push(Read { scope, of });
    }

    /// A function's parameters and its body's own declarations are one scope;
    /// a function expression's own name, one outside them, which only it sees.
    fn function(&mut self, function: &Function, named: bool) {
        if named {
            self.enter();
            self.declare(&function.name);
        }
        self.enter();
        for param in &function.params {
            self.pattern(param);
        }
        self.block_in_scope(&function.body);
        self.leave();
        if named {
            self.leave();
        }
    }

    fn closure(&mut self, params: &[Pattern], body: &[Stmt]) {
        self.enter();
        for param in params {
            self.pattern(param);
        }
        self.block_in_scope(body);
        self.leave();
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
        self.enter();
        self.block_in_scope(stmts);
        self.leave();
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

    /// What a hoisted pattern declares; its defaults are read as the
    /// statement runs.
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
                self.enter();
                self.pattern(pattern);
                self.expr(iterable);
                self.block(body);
                self.leave();
            }
            StmtKind::For {
                name,
                start,
                test,
                body,
                ..
            } => {
                self.enter();
                self.declare(name);
                self.expr(start);
                self.expr(test);
                self.block(body);
                self.leave();
            }
            StmtKind::Labeled(_, body) => self.block(body),
            StmtKind::Try(body, finally) => {
                self.block(body);
                self.block(finally);
            }
            // A `catch`'s parameter and its block's own declarations are one scope.
            StmtKind::TryCatch(body, error, handler) => {
                self.block(body);
                self.enter();
                if let Some(error) = error {
                    self.declare(error);
                }
                self.block_in_scope(handler);
                self.leave();
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

#[cfg(test)]
mod tests {
    use super::Tree;
    use crate::js::{self, Expr, Function, Op, Span, Stmt, StmtKind};

    fn item(body: Vec<Stmt>) -> js::Item {
        js::Item::Function(Function {
            name: "f".into(),
            params: Vec::new(),
            body,
            export: false,
            is_async: false,
            span: Span::NONE,
            name_span: Span::NONE,
        })
    }

    fn constant(name: &str, value: Expr) -> Stmt {
        StmtKind::Const(name.into(), value).at(Span::NONE)
    }

    fn returned(value: Expr) -> Stmt {
        StmtKind::Return(Some(value)).at(Span::NONE)
    }

    fn when(body: Vec<Stmt>) -> Stmt {
        StmtKind::If(Expr::var("a"), body, None).at(Span::NONE)
    }

    // `if (a) { const n$1 = 2; if (a) { const n = 3; return n; } }`: the
    // inner `n`'s read is its own, past which `n$1` may be `n` too.
    #[test]
    fn a_read_is_of_its_innermost_declaration() {
        let tree = Tree::of(&item(vec![
            constant("n", Expr::int(1)),
            when(vec![
                constant("n$1", Expr::int(2)),
                when(vec![constant("n", Expr::int(3)), returned(Expr::var("n"))]),
                returned(Expr::var("n$1")),
            ]),
            returned(Expr::var("n")),
        ]))
        .expect("each scope declares a name once");
        assert!(tree.renames("n$1", "n"));
    }

    // `const n$1 = 1; if (a) { const n = 2; return n + n$1; }`: the block's
    // read of `n$1` would be of its own `n`.
    #[test]
    fn a_read_passing_a_declaration_of_the_new_name_keeps_its_own() {
        let tree = Tree::of(&item(vec![
            constant("n$1", Expr::int(1)),
            when(vec![
                constant("n", Expr::int(2)),
                returned(Expr::bin(Op::Add, Expr::var("n"), Expr::var("n$1"))),
            ]),
            returned(Expr::var("n$1")),
        ]))
        .expect("each scope declares a name once");
        assert!(!tree.renames("n$1", "n"));
    }

    // `if (a) { const x$1 = 1; } return x$1;`: the last a name from outside,
    // an import's, which a rename would rename too.
    #[test]
    fn a_name_also_read_from_outside_stays() {
        let tree = Tree::of(&item(vec![
            when(vec![constant("x$1", Expr::int(1)), returned(Expr::var("x$1"))]),
            returned(Expr::var("x$1")),
        ]))
        .expect("each scope declares a name once");
        assert!(!tree.renames("x$1", "x"));
    }
}
