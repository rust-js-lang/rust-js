//! `if-let`: a `match` of two arms, one of which binds nothing, made the
//! `if let` that means the same, `match e { Some(v) => a, None => b }` →
//! `if let Some(v) = e { a } else { b }`: rustc held the `match` to cover
//! every value, so what the other arm's pattern doesn't match the one that
//! binds nothing does. And a `let` of such a `match` whose arm that binds
//! gives what it binds made a `let else`, `let x = match e { Some(v) => v,
//! None => return };` → `let Some(x) = e else { return };`. Where the other
//! arm doesn't diverge, rustc refuses the rewrite: it's no variant.

use proc_macro2::Ident;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Arm, Expr, Pat, Stmt};

pub fn if_let(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut made = IfLet { budget, changed: false };
    made.visit_file_mut(ast);
    made.changed
}

struct IfLet<'b> {
    budget: &'b mut usize,
    changed: bool,
}

/// The two arms of `m` an `if let` can be, of no guards: the one it
/// matches, and the other, which binds nothing, its `else`.
fn two_arms(m: &syn::ExprMatch) -> Option<(&Arm, &Arm)> {
    let [first, second] = &m.arms[..] else { return None };
    let plain = |arm: &Arm| arm.guard.is_none() && arm.attrs.is_empty();
    if !plain(first) || !plain(second) {
        return None;
    }
    match (binds_nothing(&first.pat), binds_nothing(&second.pat)) {
        (_, true) if !matches!(first.pat, Pat::Wild(_)) => Some((first, second)),
        (true, false) => Some((second, first)),
        _ => None,
    }
}

/// Whether `pat` binds no name: each identifier in it a constant's or a
/// variant's, `None`, as the case's names are written.
fn binds_nothing(pat: &Pat) -> bool {
    let mut binds = Binds {
        names: Vec::new(),
        other: false,
    };
    binds.visit_pat(pat);
    (binds.names.iter()).all(|name| name.to_string().starts_with(|c: char| c.is_ascii_uppercase()))
}

fn block_of(e: &Expr) -> syn::Block {
    match e {
        Expr::Block(b) if b.label.is_none() && b.attrs.is_empty() => b.block.clone(),
        other => syn::parse_quote!({ #other }),
    }
}

/// The one name `pat` binds, by itself, if it binds one so.
struct Binds {
    names: Vec<Ident>,
    other: bool,
}

impl<'a> Visit<'a> for Binds {
    fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
        if p.by_ref.is_some() || p.mutability.is_some() || p.subpat.is_some() {
            self.other = true;
        }
        self.names.push(p.ident.clone());
        visit::visit_pat_ident(self, p);
    }
}

/// `pat` with its binding `from` named `to`.
struct Rename<'n> {
    from: &'n Ident,
    to: &'n syn::PatIdent,
}

impl VisitMut for Rename<'_> {
    fn visit_pat_mut(&mut self, p: &mut Pat) {
        if let Pat::Ident(i) = p
            && &i.ident == self.from
        {
            *p = Pat::Ident(self.to.clone());
            return;
        }
        visit_mut::visit_pat_mut(self, p);
    }
}

impl IfLet<'_> {
    /// `let x = match e { P(v) => v, _ => diverges };` made a `let else`.
    fn let_else(&mut self, s: &mut Stmt) {
        let Stmt::Local(local) = s else { return };
        let Some(init) = &local.init else { return };
        if *self.budget == 0 || init.diverge.is_some() {
            return;
        }
        let Pat::Ident(to) = &local.pat else { return };
        if to.by_ref.is_some() || to.subpat.is_some() {
            return;
        }
        let Expr::Match(m) = &*init.expr else { return };
        let Some((first, second)) = two_arms(m) else { return };
        let Expr::Path(given) = &*first.body else { return };
        let Some(from) = given.path.get_ident() else { return };
        let mut binds = Binds {
            names: Vec::new(),
            other: false,
        };
        binds.visit_pat(&first.pat);
        if binds.other || binds.names.len() != 1 || &binds.names[0] != from {
            return;
        }
        let mut pat = first.pat.clone();
        Rename { from, to }.visit_pat_mut(&mut pat);
        let (scrutinee, otherwise) = (&m.expr, block_of(&second.body));
        let attrs = &local.attrs;
        *s = syn::parse_quote!(#(#attrs)* let #pat = #scrutinee else #otherwise;);
        *self.budget -= 1;
        self.changed = true;
    }
}

impl VisitMut for IfLet<'_> {
    fn visit_stmt_mut(&mut self, s: &mut Stmt) {
        self.let_else(s);
        visit_mut::visit_stmt_mut(self, s);
    }

    fn visit_expr_mut(&mut self, e: &mut Expr) {
        visit_mut::visit_expr_mut(self, e);
        let Expr::Match(m) = e else { return };
        if *self.budget == 0 || !m.attrs.is_empty() {
            return;
        }
        let Some((first, second)) = two_arms(m) else { return };
        let (pat, scrutinee) = (&first.pat, &m.expr);
        let (then, otherwise) = (block_of(&first.body), block_of(&second.body));
        *e = syn::parse_quote!(if let #pat = (#scrutinee) #then else #otherwise);
        *self.budget -= 1;
        self.changed = true;
    }
}
