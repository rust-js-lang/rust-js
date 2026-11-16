//! A place a tree reads, as a key: a variable's, or `this`'s, and the
//! fields read of it, `element.editable`; what a call of it gives,
//! `isItem()`; or an array's items, `...items`. The one way to key a place
//! (#1681 change 3, ADR 1150): its root is a variable by its id, never a
//! name's word spelled with its fields, and each field is a property's key,
//! a word, which is what it is.

use super::{Expr, ExprKind, Name};

/// What a path starts from.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum PathRoot {
    /// A variable, by its id.
    Var(Name),
    /// A class's method's `self` (`classes::receiver_as_this`).
    This,
}

/// What a path names of its root and fields.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PathEnd {
    /// The place itself, `element.editable`.
    Place,
    /// What a call of it gives, `isItem()` of `isItem(item)`.
    Called,
    /// An array's items, `...items`, which only a `&mut` changes.
    Items,
}

/// A place a tree reads, as a key (the module's doc).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Path {
    pub root: PathRoot,
    pub fields: Vec<String>,
    pub end: PathEnd,
}

impl Path {
    /// A variable itself.
    pub fn var(name: &Name) -> Path {
        Path {
            root: PathRoot::Var(name.clone()),
            fields: Vec::new(),
            end: PathEnd::Place,
        }
    }

    /// `this` itself.
    pub fn this() -> Path {
        Path {
            root: PathRoot::This,
            fields: Vec::new(),
            end: PathEnd::Place,
        }
    }

    /// Its field `field`, `q.title` of `q`.
    pub fn field(mut self, field: &str) -> Path {
        self.fields.push(field.to_string());
        self
    }

    /// The items of the array `name` is, `...items`.
    pub fn items(name: &Name) -> Path {
        Path {
            end: PathEnd::Items,
            ..Path::var(name)
        }
    }
}

impl Expr {
    /// The place this reads as a key: a member chain of a variable or of
    /// `this`, `element.editable`. `None` of anything else, a variable
    /// alone, or a chain with a computed member, an index or a call in it,
    /// `a[i].b`, `f().b`, whose place isn't known by its keys.
    pub fn path(&self) -> Option<Path> {
        let ExprKind::Member(of, field) = &self.kind else {
            return None;
        };
        let path = match &of.kind {
            ExprKind::Var(name) => Path::var(name),
            ExprKind::This => Path::this(),
            _ => of.path()?,
        };
        Some(path.field(field))
    }

    /// What this call calls, as a key of what it gives: `isItem()` of
    /// `isItem(item)`, `globalThis.isItem()`, a variable or a place its
    /// callee is; `None` of another call, and of what isn't a call.
    pub fn called_path(&self) -> Option<Path> {
        let ExprKind::Call(callee, _) = &self.kind else {
            return None;
        };
        let path = match &callee.kind {
            ExprKind::Var(name) => Path::var(name),
            _ => callee.path()?,
        };
        Some(Path {
            end: PathEnd::Called,
            ..path
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A member chain of a variable or of `this` is a path, its fields its
    // keys; a variable alone, a computed member, an index and a call in the
    // chain are none, as their place isn't known by its keys.
    #[test]
    fn a_place_is_a_path_of_a_member_chain_alone() {
        let q = Name::declared("q".to_string());
        let read = Expr::member(Expr::member(Expr::var(&q), "title"), "length");
        assert_eq!(read.path(), Some(Path::var(&q).field("title").field("length")));
        assert_eq!(
            Expr::member(Expr::this(), "editable").path(),
            Some(Path::this().field("editable"))
        );
        assert_eq!(Expr::var(&q).path(), None);
        let indexed = Expr::member(Expr::index(Expr::var(&q), Expr::int(0)), "b");
        assert_eq!(indexed.path(), None);
        let called = Expr::member(Expr::call(Expr::var(&q), Vec::new()), "b");
        assert_eq!(called.path(), None);
    }

    // Two variables of one word are two paths: a path's root is its id.
    #[test]
    fn a_paths_root_is_its_variable_not_its_word() {
        let (a, b) = (Name::declared("q".to_string()), Name::declared("q".to_string()));
        assert_ne!(Path::var(&a).field("title"), Path::var(&b).field("title"));
    }
}
