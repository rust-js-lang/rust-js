//! What a tree writes of its variables, each kind of write apart (ADR 0670):
//! a variable's binding replaced, `x = ..`; the object it holds changed,
//! `x.f = ..`; a handle made on it, which what it's given may set; and each
//! of those done in a closure, which a call of it does later. A rewrite
//! asks these of the tree as it is when it runs, and puts its own safety
//! condition together from them; none is kept across a rewrite.

use super::*;

/// The writes of a tree's variables, `Writes::of(stmts)`.
#[derive(Default)]
pub struct Writes {
    /// `x = ..` and `x += ..`: each variable whose binding is replaced, in a
    /// closure too.
    pub replaced: HashSet<String>,
    /// Those a closure, or a function declared in the tree, replaces.
    pub replaced_in_closures: HashSet<String>,
    /// `x.f = ..` and `x[0] += ..`: each variable whose object a write
    /// changes, the one at the root of what's written, in a closure too.
    pub changed: HashSet<String>,
    /// Those a closure, or a function declared in the tree, changes.
    pub changed_in_closures: HashSet<String>,
    /// `&mut x`'s handle, a getter and a setter of `x`: each variable one is
    /// made on, which what it's given may replace (ADR 0099).
    pub handled: HashSet<String>,
}

impl Writes {
    pub fn of(stmts: &[Stmt]) -> Writes {
        let mut writes = Writes::default();
        writes.walk(stmts, false);
        writes
    }

    /// What a closure may write when it's called: each variable whose
    /// binding or object one writes.
    pub fn by_closures(&self) -> HashSet<String> {
        self.replaced_in_closures
            .union(&self.changed_in_closures)
            .cloned()
            .collect()
    }

    /// Whether `name`'s binding may be replaced: assigned, or a handle made
    /// on it.
    pub fn may_replace(&self, name: &str) -> bool {
        self.replaced.contains(name) || self.handled.contains(name)
    }

    fn walk(&mut self, stmts: &[Stmt], in_closure: bool) {
        // Each expression once, however deep: each closure's and each
        // declared function's own statements are walked where it's found,
        // never what's nested in them, which this finds in turn.
        self.lists(stmts, in_closure);
        each_expr(stmts, &mut |e| match &e.kind {
            ExprKind::Handle(place, _) | ExprKind::Pair(place, _) => {
                if let ExprKind::Var(name) = &place.kind {
                    self.handled.insert(name.clone());
                }
            }
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => self.lists(body, true),
            ExprKind::Function(function) => self.lists(&function.body, true),
            _ => {}
        });
    }

    /// What `stmts`' own lists write, not their closures', and their
    /// declared functions' as a closure's.
    fn lists(&mut self, stmts: &[Stmt], in_closure: bool) {
        each_list(stmts, &mut |list| {
            for stmt in list {
                match &stmt.kind {
                    StmtKind::Assign(target, _) | StmtKind::Compound(_, target, _) => self.target(target, in_closure),
                    StmtKind::Function(function) => self.lists(&function.body, true),
                    _ => {}
                }
            }
        });
    }

    /// What writing `target` writes: its variable's binding, or the object
    /// at its root.
    fn target(&mut self, target: &Expr, in_closure: bool) {
        let mut root = target;
        while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {
            root = of;
        }
        let ExprKind::Var(name) = &root.kind else { return };
        let (all, closures) = match std::ptr::eq(root, target) {
            true => (&mut self.replaced, &mut self.replaced_in_closures),
            false => (&mut self.changed, &mut self.changed_in_closures),
        };
        all.insert(name.clone());
        if in_closure {
            closures.insert(name.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stmt(kind: StmtKind) -> Stmt {
        kind.at(Span::NONE)
    }

    fn assign(target: Expr) -> Stmt {
        stmt(StmtKind::Assign(target, Expr::int(1)))
    }

    fn names(set: &HashSet<String>) -> Vec<&str> {
        let mut names: Vec<&str> = set.iter().map(String::as_str).collect();
        names.sort();
        names
    }

    #[test]
    fn a_binding_replaced_is_not_its_object_changed() {
        let writes = Writes::of(&[
            assign(Expr::var("x")),
            assign(Expr::member(Expr::var("o"), "field")),
            stmt(StmtKind::Compound(
                Op::Add,
                Expr::index(Expr::var("a"), Expr::int(0)),
                Expr::int(1),
            )),
            stmt(StmtKind::Compound(Op::Add, Expr::var("n"), Expr::int(1))),
        ]);
        assert_eq!(names(&writes.replaced), ["n", "x"]);
        assert_eq!(names(&writes.changed), ["a", "o"]);
        assert!(writes.by_closures().is_empty());
        assert!(writes.may_replace("n") && !writes.may_replace("o"));
    }

    #[test]
    fn a_closure_writes_when_its_called() {
        let closure = Expr::arrow(
            Vec::new(),
            vec![assign(Expr::var("x")), assign(Expr::member(Expr::var("o"), "f"))],
        );
        let writes = Writes::of(&[stmt(StmtKind::Const("f".into(), closure))]);
        assert_eq!(names(&writes.replaced_in_closures), ["x"]);
        assert_eq!(names(&writes.changed_in_closures), ["o"]);
        assert_eq!(names(&writes.by_closures()), ["o", "x"]);
        assert!(writes.may_replace("x"));
    }

    /// Closures nested 64 deep, the innermost assigning: each closure's
    /// body is walked once, not once for each closure around it, which
    /// would be 2^64 walks of the innermost.
    #[test]
    fn nested_closures_are_walked_once() {
        let mut body = vec![assign(Expr::var("x"))];
        for _ in 0..64 {
            body = vec![stmt(StmtKind::Expr(Expr::arrow(Vec::new(), body)))];
        }
        let writes = Writes::of(&body);
        assert_eq!(names(&writes.replaced_in_closures), ["x"]);
    }

    #[test]
    fn a_handle_may_replace_what_its_on() {
        let handle = Expr {
            kind: ExprKind::Handle(Box::new(Expr::var("h")), None),
            ..Expr::var("h")
        };
        let writes = Writes::of(&[stmt(StmtKind::Expr(Expr::call(Expr::var("g"), vec![handle])))]);
        assert_eq!(names(&writes.handled), ["h"]);
        assert!(writes.may_replace("h") && writes.replaced.is_empty());
    }
}
