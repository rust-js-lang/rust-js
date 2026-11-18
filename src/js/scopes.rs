//! A tree's scopes, as JS resolves its names: each scope's declarations,
//! and each read, where it is and the declaration it's of, or a name from
//! outside. The one owner of JS's scoping (#1681 change 3): reclaim asks it
//! whether a rename keeps each read its own (ADR 0357), `js::validate`
//! whether each read resolves (ADR 1143), and `alpha_eq` whether two trees
//! are the same but for what their declarations are named.

use crate::collections::IndexMap;

use crate::js::{self, Expr, ExprKind, Function, JsxTag, Name, Pattern, Prop, Stmt, StmtKind};

struct Scope {
    parent: Option<usize>,
    names: IndexMap<String, usize>,
    /// The variable of each word a function's tree declares here, its
    /// `Name` and so its id: the module's top's words alone have none.
    bound: IndexMap<String, Name>,
}

/// A read: the scope it's in, and the scope of the declaration it's of,
/// `None` of a name from outside; and whose name it reads.
pub(crate) struct Read {
    scope: usize,
    pub(crate) of: Option<usize>,
    pub(crate) whose: Whose,
    /// The variable it reads, a variable's read's `Name`.
    name: Option<Name>,
}

/// Whose name a read reads, of its name's form.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Whose {
    /// A name in scope: a local's, or one the module declares.
    Local,
    /// A global's, one `Expr::global` made (`Name::global`) or the printer
    /// writes, `String(x)`'s: declared nowhere in the module.
    Global,
    /// One the module declares at its top (`Name::module`), its function's,
    /// its import's or the runtime's helper's.
    Module,
}

impl Whose {
    /// Of a read of `name`, by its form.
    fn of(name: &js::Name) -> Whose {
        match () {
            _ if name.is_global() => Whose::Global,
            _ if name.is_module() => Whose::Module,
            _ => Whose::Local,
        }
    }
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

    /// `item`'s scopes inside its module's: what the module declares at its
    /// top besides its items, `top` (`module_top`), its imports and
    /// packages, the scope around the item's, which a name the item declares
    /// shadows (`shadows`); `of`'s otherwise.
    pub(crate) fn of_in(item: &js::Item, top: &[String]) -> Option<Tree> {
        let mut walk = Walk::default();
        walk.enter();
        top.iter().for_each(|name| walk.declare(name));
        walk.item(item, true);
        walk.tree.twice.is_empty().then_some(walk.tree)
    }

    /// What `module` declares at its top besides its items: its imports'
    /// and its packages' names.
    pub(crate) fn module_top(module: &js::Module) -> Vec<String> {
        let imported = (module.imports.iter())
            .flat_map(|import| import.named.iter().map(|(_, local)| local).chain(&import.namespace))
            .map(|local| local.word().to_string());
        imported.chain(packaged(&module.packages).cloned()).collect()
    }

    /// Each name declared or read, in the order first met.
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        let declared = self.declaring.keys().filter(|name| !self.reads.contains_key(*name));
        self.reads.keys().chain(declared).map(String::as_str)
    }

    /// `module`'s scopes: its imports', its packages' and its items' names
    /// the outermost's, then each item's own.
    pub(crate) fn of_module(module: &js::Module) -> Tree {
        Tree::of_top(Tree::module_top(module).iter(), &module.items)
    }

    /// A module's scopes, of what its top declares besides its items,
    /// `top`, and its items.
    fn of_top<'t>(top: impl Iterator<Item = &'t String>, items: &[js::Item]) -> Tree {
        let mut walk = Walk::default();
        walk.enter();
        top.for_each(|name| walk.declare(name));
        for item in items {
            match item {
                js::Item::Function(function) => walk.declare(function.name.word()),
                js::Item::Namespace(namespace) => walk.declare(namespace.name.word()),
                js::Item::Const(constant) => walk.declare(constant.name.word()),
                js::Item::Statements(stmts) => walk.hoist(stmts),
            }
        }
        for item in items {
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

    /// Would `from` named `to` shadow a variable, or be shadowed by one:
    /// does a scope around one that declares `from` declare `to`, or one
    /// inside it? JS lets it where no read crosses (`renames`); a loop's
    /// counter a person writes doesn't: `j` inside a loop of `i`, and `j`
    /// of a loop that holds one of `i` (ADR 1042, `prepare::counters`).
    pub(crate) fn shadows(&self, from: &str, to: &str) -> bool {
        let ups = |scope: usize| std::iter::successors(self.scopes[scope].parent, |&up| self.scopes[up].parent);
        let declaring = self.declaring.get(from).map(Vec::as_slice).unwrap_or_default();
        let around = declaring
            .iter()
            .any(|&scope| ups(scope).any(|up| self.declares(up, to)));
        let inside =
            (self.declaring.get(to).into_iter().flatten()).any(|&inner| ups(inner).any(|up| declaring.contains(&up)));
        around || inside
    }

    /// Whether a variable spelled already that's given apart from a root,
    /// as `apart` says, `next$7` of `next`, is where reclaiming it as that
    /// root is what `from` named it would stop: declared in a scope that declares `from`;
    /// or inside one, `from` read inside its scope; or around one, it read
    /// inside `from`'s. So a made name the program's would then give way to
    /// doesn't take it (ADR 0357).
    pub(crate) fn reclaims_to(&self, from: &str, apart: &dyn Fn(&Name) -> bool) -> bool {
        let ups = |scope: usize| std::iter::successors(Some(scope), |&up| self.scopes[up].parent);
        let declaring = self.declaring.get(from).map(Vec::as_slice).unwrap_or_default();
        let read_in = |word: &str, scope: usize| {
            (self.reads.get(word).into_iter().flatten()).any(|read| ups(read.scope).any(|up| up == scope))
        };
        (self.scopes.iter().enumerate()).any(|(at, scope)| {
            (scope.bound.iter()).any(|(word, name)| {
                apart(name)
                    && declaring.iter().any(|&d| {
                        d == at
                            || (ups(at).skip(1).any(|up| up == d) && read_in(from, at))
                            || (ups(d).skip(1).any(|up| up == at) && read_in(word, d))
                    })
            })
        })
    }

    /// Does each read stay of what it's of, where `from` is named `to`? No
    /// scope that declares `from` declares `to` too; no read of `from`
    /// passes a declaration of `to` before its own; and no read of `to`
    /// passes one of `from` before its own.
    pub(crate) fn renames(&self, from: &str, to: &str) -> bool {
        let Some(declaring) = self.declaring.get(from).filter(|d| !d.is_empty()) else {
            return false;
        };
        // A name of the module's, an `on_load!` body's own; an inner one may
        // shadow one, as JS's scopes resolve its reads (below).
        if self.module_scope && declaring.contains(&0) {
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

    /// Each variable a function declares in whose scope a read of a name
    /// of the module's is, a helper's the module imports by its word: the
    /// variable JS reads there instead (`Problem::ModuleShadowed`).
    pub(crate) fn taking_module_reads(&self) -> Vec<Name> {
        let mut taking: Vec<Name> = Vec::new();
        for (word, reads) in &self.reads {
            for read in reads {
                let Some(of) = read.of.filter(|&of| of != 0 && read.whose == Whose::Module) else {
                    continue;
                };
                if let Some(local) = self.scopes[of].bound.get(word)
                    && !taking.contains(local)
                {
                    taking.push(local.clone());
                }
            }
        }
        taking
    }

    /// Each word read where JS's scopes resolve it to another variable
    /// than the one it is, by their ids: a declaration spelled as it is
    /// nearer than its own (`Problem::Captured`). Of variables told apart
    /// from any other of their spelling, by an id or as a body's item
    /// (`Name::is_identified`): the compiler makes no local of its word
    /// alone (#2034).
    pub(crate) fn captured(&self) -> Vec<String> {
        let mut captured = Vec::new();
        for (word, reads) in &self.reads {
            for read in reads {
                if let (Some(of), Some(name)) = (read.of, &read.name)
                    && let Some(declared) = self.scopes[of].bound.get(word)
                    && name.is_identified()
                    && declared.is_identified()
                    && declared != name
                    && !captured.contains(word)
                {
                    captured.push(word.clone());
                }
            }
        }
        captured
    }

    /// Whether each of `names` is in reach where `at`, read once, is read:
    /// declared, by its id, in that read's scope or one around it, so JS
    /// reads that variable there once preparation spells each apart (ADR
    /// 1150). A body's parameters, outside its tree, aren't any scope's.
    pub(crate) fn reach(&self, at: &Name, names: &[&Name]) -> bool {
        let mut reads = (self.reads.get(at.word()).into_iter().flatten()).filter(|read| read.name.as_ref() == Some(at));
        let (Some(read), None) = (reads.next(), reads.next()) else {
            return false;
        };
        names.iter().all(|name| {
            let mut scope = Some(read.scope);
            while let Some(at) = scope {
                if self.scopes[at].bound.get(name.word()) == Some(*name) {
                    return true;
                }
                scope = self.scopes[at].parent;
            }
            false
        })
    }

    /// Of `names`, each a read of which JS's scopes resolve to another
    /// variable than itself, by their ids: one declared in the tree read
    /// where a nearer declaration is spelled as it is, or where it's out of
    /// reach; one declared outside it, a function's parameter, read where
    /// a declaration in it is spelled so. What a pass that puts a variable's
    /// reads in new places asks before keeping them (`drops::inlined`).
    pub(crate) fn not_read_as_themselves(&self, names: &[&Name]) -> Vec<Name> {
        let declared = |name: &Name| {
            self.scopes
                .iter()
                .any(|scope| scope.bound.get(name.word()) == Some(name))
        };
        let mut not = Vec::new();
        for name in names {
            let inside = declared(name);
            let reads = self.reads.get(name.word()).into_iter().flatten();
            let own = |read: &Read| match read.of {
                Some(of) => inside && self.scopes[of].bound.get(name.word()) == Some(*name),
                None => !inside,
            };
            if reads
                .filter(|read| read.name.as_ref() == Some(*name))
                .any(|read| !own(read))
                && !not.contains(*name)
            {
                not.push((*name).clone());
            }
        }
        not
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

/// A module's `items`, beside what its top declares besides them, `top`,
/// with each local in whose scope they read a name of the module's spelled
/// apart from every name it has, `copiedClone$1`: a runtime helper's read,
/// which lowering's locals can't be named apart from as they are from the
/// module's own names (ADR 1150), what's imported decided only by the reads
/// its linked tree has. A helper is imported by its word, so the local
/// gives way, as a local does to a function of the module's.
pub(crate) fn locals_apart_from_module_reads(top: &[String], items: &mut [js::Item]) {
    let tree = Tree::of_top(top.iter(), items);
    let taking = tree.taking_module_reads();
    if taking.is_empty() {
        return;
    }
    let mut taken: crate::collections::IndexSet<String> = tree.names().map(str::to_owned).collect();
    for local in taking {
        let to = crate::names::fresh_in(&mut taken, local.word());
        js::respelled(items, &local, &to);
    }
}

/// The names a module's packages' imports declare at its top.
pub(crate) fn packaged(packages: &[js::Package]) -> impl Iterator<Item = &String> {
    (packages.iter()).flat_map(|package| {
        (package.named.iter().map(|(_, local)| local))
            .chain(&package.default)
            .chain(&package.namespace)
    })
}

/// Each word `stmts` declare or read, their closures' and functions'
/// too: the spelling question JS asks where a name is spelled into them, a
/// word none of these may be, as JS would read it as one of them.
pub(crate) fn spelled(stmts: &[Stmt]) -> crate::collections::IndexSet<String> {
    Tree::of_body(stmts).names().map(str::to_owned).collect()
}

/// Each word `stmts` declare, their closures' and functions' too: the
/// spelling question JS asks of a name spelled as one of them, which a
/// reader, and JS inside that scope, takes for the declared one.
pub(crate) fn declared_spelled(stmts: &[Stmt]) -> crate::collections::IndexSet<String> {
    Tree::of_body(stmts).declaring.keys().cloned().collect()
}

/// Whether each of `names` is in reach where `at`, read once in `body`, a
/// function's, is read, by their ids ([`Tree::reach`]): asked by ids, no
/// word, where a pass moves a closure's body to its call.
pub(crate) fn reach(body: &[Stmt], at: &Name, names: &[&Name]) -> bool {
    Tree::of_body(body).reach(at, names)
}

/// Of `names`, each a read of which in `body`, a function's, JS's scopes
/// resolve to another variable ([`Tree::not_read_as_themselves`]): asked
/// by ids, no word, where a pass puts a variable's reads in new places.
pub(crate) fn not_read_as_themselves(body: &[Stmt], names: &[&Name]) -> Vec<Name> {
    Tree::of_body(body).not_read_as_themselves(names)
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

/// `alpha_eq` of two functions, made apart: a copy's and its function's,
/// or two classes' methods (#1681 change 3).
pub(crate) fn alpha_eq_functions(a: &js::Function, b: &js::Function) -> bool {
    let of = |f: &js::Function| Expr::new(ExprKind::Function(Box::new(f.clone())));
    alpha_eq(&of(a), &of(b))
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
        ExprKind::Var(name) => *name = Name::default(),
        ExprKind::Arrow(params, _) | ExprKind::AsyncArrow(params, _) => params.iter_mut().for_each(pattern),
        ExprKind::Function(function) => {
            function.name = Name::default();
            function.params.iter_mut().for_each(pattern);
        }
        _ => {}
    });
    js::each_block_mut(&mut stmts, &mut |list| {
        for stmt in list.iter_mut() {
            match &mut stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => {
                    *name = Name::default()
                }
                StmtKind::TryCatch(_, Some(name), _) => *name = Name::default(),
                StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => unnamed_pattern(pattern),
                StmtKind::Function(function) => {
                    function.name = Name::default();
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
        Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => *name = Name::default(),
        Pattern::Array(items) => items.iter_mut().flatten().for_each(unnamed_pattern),
        Pattern::Object(fields, rest) => {
            fields.iter_mut().for_each(|(_, field, _)| unnamed_pattern(field));
            if let Some(rest) = rest {
                *rest = Name::default();
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
            js::Item::Namespace(namespace) => {
                namespace.constants.iter().for_each(|c| self.expr(&c.value));
                namespace.methods.iter().for_each(|m| self.function(m, false));
            }
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
            bound: IndexMap::default(),
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

    /// `name`, a variable a function's tree declares: declared, and kept as
    /// the variable its scope's word is.
    fn bind(&mut self, name: &Name) {
        self.declare(name.word());
        let scope = *self.stack.last().expect("a declaration is in a scope");
        self.tree.scopes[scope]
            .bound
            .insert(name.word().to_string(), name.clone());
    }

    fn read(&mut self, name: &str, whose: Whose) {
        let scope = *self.stack.last().expect("a read is in a scope");
        let of = self.stack.iter().rev().copied().find(|&s| self.tree.declares(s, name));
        // Of the declaration it's of, by its number: what it's named apart.
        self.tree.events.push(match of {
            Some(s) => Event::Read(self.tree.scopes[s].names.get(name).copied()),
            None => Event::Outside(name.to_string()),
        });
        self.tree.reads.entry(name.to_string()).or_default().push(Read {
            scope,
            of,
            whose,
            name: None,
        });
    }

    /// A read of the variable `name`, by its word, which knows its id.
    fn read_name(&mut self, name: &Name) {
        self.read(name.word(), Whose::of(name));
        if let Some(read) = self.tree.reads.get_mut(name.word()).and_then(|reads| reads.last_mut()) {
            read.name = Some(name.clone());
        }
    }

    /// A function's parameters and its body's own declarations are one scope;
    /// a function expression's own name, one outside them, which only it sees.
    fn function(&mut self, function: &Function, named: bool) {
        if named {
            self.enter();
            self.bind(&function.name);
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
            Pattern::Name(name) | Pattern::Rest(name) => self.bind(name),
            // Its default is read where it's given none, before it's made.
            Pattern::Defaulted(name, default) => {
                self.expr(default);
                self.bind(name);
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
                rest.iter().for_each(|name| self.bind(name));
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
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => self.bind(name),
                StmtKind::Destructure { pattern, .. } => self.pattern_names(pattern),
                StmtKind::Function(function) => self.bind(&function.name),
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
        pattern.bound().into_iter().for_each(|name| self.bind(name));
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
                self.bind(name);
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
                    self.bind(error);
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
            ExprKind::Var(name) => self.read_name(name),
            // What it sets is resolved as a read is.
            ExprKind::Assigned(target, a) => {
                self.expr(target);
                self.expr(a);
            }
            // What the printer writes of JS's own: `String(x)`, `undefined`,
            // `NaN` and `Infinity`.
            ExprKind::Stringed(a) => {
                self.read("String", Whose::Global);
                self.expr(a);
            }
            ExprKind::Undefined | ExprKind::GivenUndefined => self.read("undefined", Whose::Global),
            ExprKind::Num(n) if n.is_nan() => self.read("NaN", Whose::Global),
            ExprKind::Num(n) if n.is_infinite() => self.read("Infinity", Whose::Global),
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
