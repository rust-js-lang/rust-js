//! A tree's scopes, as JS resolves its names: each scope's declarations,
//! and each read, where it is and the declaration it's of, or a name from
//! outside. The one owner of JS's scoping (#1681 change 3): reclaim asks it
//! whether a rename keeps each read its own (ADR 0357), `js::validate`
//! whether each read resolves (ADR 1143), and `alpha_eq` whether two trees
//! are the same but for what their declarations are named.

use crate::collections::IndexMap;

use crate::js::{self, Expr, ExprKind, Function, JsxTag, Known, Pattern, Prop, Stmt, StmtKind};

struct Scope {
    parent: Option<usize>,
    names: IndexMap<String, usize>,
}

/// A read: the scope it's in, and the scope of the declaration it's of,
/// `None` of a name from outside; and whether it's a global's, one
/// `Expr::global` made (`Known::Global`) or the printer writes, `String(x)`'s.
pub(crate) struct Read {
    scope: usize,
    pub(crate) of: Option<usize>,
    pub(crate) global: bool,
}

/// What the walk met, in its order: a declaration, or a read of the
/// declaration that many before it, or of a name from outside.
#[derive(PartialEq, Debug)]
enum Event {
    Declared,
    Read(Option<usize>),
    Outside(String),
}

#[derive(Default)]
pub(crate) struct Tree {
    scopes: Vec<Scope>,
    /// Each name's reads.
    pub(crate) reads: IndexMap<String, Vec<Read>>,
    /// Each name a scope declares twice, which JS refuses, in the order met.
    pub(crate) twice: Vec<String>,
    /// What the walk met, `alpha_eq`'s.
    events: Vec<Event>,
    /// The scopes declaring each name: asked of each name renamed, which
    /// a scan of every scope made a function's size times its names.
    declaring: IndexMap<String, Vec<usize>>,
    /// Whether the outermost scope's names are the module's, an `on_load!`
    /// body's, which no rename of the item's own may take (ADR 0267).
    module_scope: bool,
}

impl Tree {
    /// `item`'s scopes; of an `on_load!` body's, whose outermost names are
    /// the module's (ADR 0267), the scopes inside it, its closures' and
    /// blocks' (ADR 0952); `None` where a scope declares a name twice, which
    /// JS refuses.
    pub(crate) fn of(item: &js::Item) -> Option<Tree> {
        let mut walk = Walk::default();
        walk.enter();
        walk.item(item, true);
        walk.tree.twice.is_empty().then_some(walk.tree)
    }

    /// `module`'s scopes: its imports', its packages' and its items' names
    /// the outermost's, then each item's own.
    pub(crate) fn of_module(module: &js::Module) -> Tree {
        let mut walk = Walk::default();
        walk.enter();
        for import in &module.imports {
            import.named.iter().for_each(|(_, local)| walk.declare(local));
            import.namespace.iter().for_each(|name| walk.declare(name));
        }
        for package in &module.packages {
            package.named.iter().for_each(|(_, local)| walk.declare(local));
            package.default.iter().for_each(|name| walk.declare(name));
            package.namespace.iter().for_each(|name| walk.declare(name));
        }
        for item in &module.items {
            match item {
                js::Item::Function(function) => walk.declare(&function.name),
                js::Item::Namespace(namespace) => walk.declare(&namespace.name),
                js::Item::Const(constant) => walk.declare(&constant.name),
                js::Item::Statements(stmts) => walk.hoist(stmts),
            }
        }
        for item in &module.items {
            walk.item(item, false);
        }
        walk.tree
    }

    /// `body`'s scopes, a function's alone: its reads of its parameters and
    /// of what's outside it, from outside.
    pub(crate) fn of_body(body: &[Stmt]) -> Tree {
        let mut walk = Walk::default();
        walk.enter();
        walk.block_in_scope(body);
        walk.tree
    }

    fn declares(&self, scope: usize, name: &str) -> bool {
        self.scopes[scope].names.contains_key(name)
    }

    /// Does each read stay of what it's of, where `from` is named `to`? No
    /// scope that declares `from` declares `to` too; no read of `from`
    /// passes a declaration of `to` before its own; and no read of `to`
    /// passes one of `from` before its own.
    pub(crate) fn renames(&self, from: &str, to: &str) -> bool {
        let Some(declaring) = self.declaring.get(from).filter(|d| !d.is_empty()) else {
            return false;
        };
        // A name of the module's, an `on_load!` body's own.
        if self.module_scope && (declaring.contains(&0) || self.declares(0, to)) {
            return false;
        }
        if declaring.iter().any(|&s| self.declares(s, to)) {
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
    pub(crate) fn rename(&mut self, from: &str, to: &str) {
        let declaring = self.declaring.swap_remove(from).unwrap_or_default();
        for &scope in &declaring {
            if let Some(id) = self.scopes[scope].names.swap_remove(from) {
                self.scopes[scope].names.insert(to.to_string(), id);
            }
        }
        self.declaring.entry(to.to_string()).or_default().extend(declaring);
        if let Some(reads) = self.reads.swap_remove(from) {
            self.reads.entry(to.to_string()).or_default().extend(reads);
        }
    }
}

/// Whether `a` and `b` are the same but for what their declarations are
/// named: alike with every name left out, and each read of the declaration
/// the same number of declarations into each, or of the same name from
/// outside (#1681 change 3). `[(x) => x, x]` and `[(y) => y, y]` aren't:
/// their last reads are of different names from outside. Where the names
/// are the same too, by `==` alone.
pub(crate) fn alpha_eq(a: &Expr, b: &Expr) -> bool {
    a == b || (unnamed(a) == unnamed(b) && events(a) == events(b))
}

/// What the walk of `e` meets, in its order.
fn events(e: &Expr) -> Vec<Event> {
    let mut walk = Walk::default();
    walk.enter();
    walk.expr(e);
    walk.tree.events
}

/// `e` with each name it declares or reads left out: what `alpha_eq`
/// compares besides what each read is of. A name left in, of a kind of
/// declaration it doesn't know, makes trees unlike that differ in it: never
/// alike where they aren't.
fn unnamed(e: &Expr) -> Expr {
    let mut stmts = vec![StmtKind::Expr(e.clone()).at(js::Span::NONE)];
    let pattern = |p: &mut Pattern| unnamed_pattern(p);
    js::each_expr_mut(&mut stmts, &mut |e| match &mut e.kind {
        ExprKind::Var(name) => name.clear(),
        ExprKind::Arrow(params, _) | ExprKind::AsyncArrow(params, _) => params.iter_mut().for_each(pattern),
        ExprKind::Function(function) => {
            function.name.clear();
            function.params.iter_mut().for_each(pattern);
        }
        _ => {}
    });
    js::each_block_mut(&mut stmts, &mut |list| {
        for stmt in list.iter_mut() {
            match &mut stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => name.clear(),
                StmtKind::TryCatch(_, Some(name), _) => name.clear(),
                StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => unnamed_pattern(pattern),
                StmtKind::Function(function) => {
                    function.name.clear();
                    function.params.iter_mut().for_each(unnamed_pattern);
                }
                _ => {}
            }
        }
    });
    let [stmt] = <[Stmt; 1]>::try_from(stmts).unwrap_or_else(|_| unreachable!("one statement"));
    match stmt.kind {
        StmtKind::Expr(e) => e,
        _ => unreachable!("an expression's statement"),
    }
}

fn unnamed_pattern(pattern: &mut Pattern) {
    match pattern {
        Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => name.clear(),
        Pattern::Array(items) => items.iter_mut().flatten().for_each(unnamed_pattern),
        Pattern::Object(fields, rest) => {
            fields.iter_mut().for_each(|(_, field, _)| unnamed_pattern(field));
            if let Some(rest) = rest {
                rest.clear();
            }
        }
    }
}

#[derive(Default)]
struct Walk {
    tree: Tree,
    stack: Vec<usize>,
    declared: usize,
}

impl Walk {
    /// `item`, in the scope the walk is in; an `on_load!` body's names the
    /// module's, where `alone`, of one item's tree (ADR 0267), else hoisted
    /// with the module's (`of_module`).
    fn item(&mut self, item: &js::Item, alone: bool) {
        match item {
            js::Item::Function(function) => self.function(function, false),
            js::Item::Namespace(namespace) => namespace.methods.iter().for_each(|m| self.function(m, false)),
            js::Item::Const(constant) => self.expr(&constant.value),
            js::Item::Statements(stmts) if alone => {
                self.tree.module_scope = true;
                self.block_in_scope(stmts);
            }
            js::Item::Statements(stmts) => self.stmts(stmts),
        }
    }

    fn enter(&mut self) {
        let parent = self.stack.last().copied();
        self.tree.scopes.push(Scope {
            parent,
            names: IndexMap::default(),
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
        if self.tree.scopes[scope].names.insert(name.to_string(), id).is_some() {
            self.tree.twice.push(name.to_string());
        }
        self.tree.declaring.entry(name.to_string()).or_default().push(scope);
        self.tree.events.push(Event::Declared);
    }

    fn read(&mut self, name: &str, global: bool) {
        let scope = *self.stack.last().expect("a read is in a scope");
        let of = self.stack.iter().rev().copied().find(|&s| self.tree.declares(s, name));
        // Of the declaration it's of, by its number: what it's named apart.
        self.tree.events.push(match of {
            Some(s) => Event::Read(self.tree.scopes[s].names.get(name).copied()),
            None => Event::Outside(name.to_string()),
        });
        self.tree
            .reads
            .entry(name.to_string())
            .or_default()
            .push(Read { scope, of, global });
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
            Pattern::Name(name) | Pattern::Rest(name) => self.declare(name),
            // Its default is read where it's given none, before it's made.
            Pattern::Defaulted(name, default) => {
                self.expr(default);
                self.declare(name);
            }
            Pattern::Array(items) => items.iter().flatten().for_each(|item| self.pattern(item)),
            Pattern::Object(fields, rest) => {
                // A default is read where the field is missing, before the
                // names after it are made.
                for (_, field, default) in fields {
                    if let Some(default) = default {
                        self.expr(default);
                    }
                    self.pattern(field);
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

    /// What `stmts` declare in their block.
    fn hoist(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => self.declare(name),
                StmtKind::Destructure { pattern, .. } => self.pattern_names(pattern),
                StmtKind::Function(function) => self.declare(&function.name),
                _ => {}
            }
        }
    }

    fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    /// What a hoisted pattern declares; its defaults are read as the
    /// statement runs.
    fn pattern_names(&mut self, pattern: &Pattern) {
        pattern.names().into_iter().for_each(|name| self.declare(name));
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Const(_, value) | StmtKind::Expr(value) | StmtKind::Throw(value) => self.expr(value),
            StmtKind::Let(_, value) | StmtKind::Return(value) => value.iter().for_each(|v| self.expr(v)),
            StmtKind::Destructure { pattern, value, .. } => {
                pattern.defaults().into_iter().for_each(|d| self.expr(d));
                self.expr(value);
            }
            StmtKind::Assign(target, value) | StmtKind::Compound(_, target, value) => {
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
            StmtKind::While { cond, body, .. } | StmtKind::DoWhile { cond, body, .. } => {
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
                step,
                body,
                ..
            } => {
                self.enter();
                self.declare(name);
                self.expr(start);
                self.expr(test);
                if let Some(value) = step.value() {
                    self.expr(value);
                }
                self.block(body);
                self.leave();
            }
            StmtKind::Labeled(_, body) => self.block(body),
            // Each case a block of its own, as it's printed where it declares.
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                self.expr(discriminant);
                for js::Case { values, body, .. } in cases {
                    values.iter().for_each(|value| self.expr(value));
                    self.block(body);
                }
                if let Some(default) = default {
                    self.block(default);
                }
            }
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
            StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Directive(_) | StmtKind::Comments => {}
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

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Var(name) => self.read(name, e.facts.has(Known::Global)),
            // What it sets is resolved as a read is.
            ExprKind::Assigned(target, a) => {
                self.expr(target);
                self.expr(a);
            }
            // What the printer writes of JS's own: `String(x)`, `undefined`,
            // `NaN` and `Infinity`.
            ExprKind::Stringed(a) => {
                self.read("String", true);
                self.expr(a);
            }
            ExprKind::Undefined | ExprKind::GivenUndefined => self.read("undefined", true),
            ExprKind::Num(n) if n.is_nan() => self.read("NaN", true),
            ExprKind::Num(n) if n.is_infinite() => self.read("Infinity", true),
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Written(_)
            | ExprKind::Null
            | ExprKind::This
            | ExprKind::Super
            | ExprKind::Symbol(_)
            | ExprKind::Regex(_)
            | ExprKind::Hole(_)
            | ExprKind::Import(_)
            | ExprKind::Specifier(_) => {}
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Spread(a)
            | ExprKind::Unary(_, a)
            | ExprKind::Await(a)
            | ExprKind::Unbound(a)
            | ExprKind::DropArgument(_, _, a)
            | ExprKind::Handle(a, _)
            | ExprKind::Stepped(_, a) => self.expr(a),
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
                items.iter().for_each(|i| self.expr(i))
            }
            ExprKind::Object(props, ..) => self.props(props),
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
    use super::{Tree, alpha_eq};
    use crate::js::{self, Expr, Function, Op, Pattern, Span, Stmt, StmtKind};

    /// `(name) => body`.
    fn arrow(name: &str, body: Expr) -> Expr {
        Expr::arrow(vec![Pattern::Name(name.into())], vec![returned(body)])
    }

    // `(x) => x` and `(y) => y` are one function, named apart.
    #[test]
    fn declarations_named_apart_are_alike() {
        assert!(alpha_eq(&arrow("x", Expr::var("x")), &arrow("y", Expr::var("y"))));
        assert!(!alpha_eq(&arrow("x", Expr::var("x")), &arrow("y", Expr::var("x"))));
    }

    // `[(x) => x, x]` and `[(y) => y, y]` aren't: their last reads are of
    // different names from outside, whatever the closures are.
    #[test]
    fn reads_from_outside_are_by_their_names() {
        let of = |n: &str| Expr::array(vec![arrow(n, Expr::var(n)), Expr::var(n)]);
        assert!(!alpha_eq(&of("x"), &of("y")));
        assert!(alpha_eq(&of("x"), &of("x")));
    }

    // A read is of the innermost declaration of its name: `(x) => (x) => x`
    // is `(a) => (b) => b`, not `(a) => (b) => a`.
    #[test]
    fn a_shadowed_read_is_of_the_innermost() {
        let shadowed = arrow("x", arrow("x", Expr::var("x")));
        assert!(alpha_eq(&shadowed, &arrow("a", arrow("b", Expr::var("b")))));
        assert!(!alpha_eq(&shadowed, &arrow("a", arrow("b", Expr::var("a")))));
    }

    // A closure's read of what an outer closure declares is of that
    // declaration: `(x) => () => x` is `(y) => () => y`, and a capture of
    // a name from outside is of that name, `z` never `w`.
    #[test]
    fn a_capture_is_of_what_it_captures() {
        let capturing = |n: &str, read: &str| arrow(n, Expr::arrow(Vec::new(), vec![returned(Expr::var(read))]));
        assert!(alpha_eq(&capturing("x", "x"), &capturing("y", "y")));
        assert!(alpha_eq(&capturing("x", "z"), &capturing("y", "z")));
        assert!(!alpha_eq(&capturing("x", "z"), &capturing("y", "w")));
        assert!(!alpha_eq(&capturing("x", "x"), &capturing("y", "x")));
    }

    fn item(body: Vec<Stmt>) -> js::Item {
        js::Item::Function(Function {
            name: "f".into(),
            params: Vec::new(),
            body,
            export: false,
            is_async: false,
            noinline: false,
            span: Span::NONE,
            name_span: Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
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
