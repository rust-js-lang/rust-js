//! `for-each`: a `for` loop whose body leaves it only at its end made the
//! iterator chain that means the same, `for x in e { body }` →
//! `(e).into_iter().for_each(|x| { body });`. And `for-loop`, back: a
//! statement `e.for_each(|x| body)` made `for x in e { body }`.

use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Item, Pat, Stmt};

/// What a body does that a closure would change the meaning of: leaving
/// its loop or function, `break`, `continue`, `return` or `?`, there and
/// not in a loop, closure or item of its own; or an `.await`, or a label.
#[derive(Default)]
struct Leaves {
    loops: usize,
    closures: usize,
    leaves: bool,
}

impl<'a> Visit<'a> for Leaves {
    fn visit_expr(&mut self, e: &'a Expr) {
        match e {
            Expr::Break(b) if b.label.is_some() || self.loops == 0 => self.leaves = true,
            Expr::Continue(c) if c.label.is_some() || self.loops == 0 => self.leaves = true,
            Expr::Return(_) | Expr::Try(_) if self.closures == 0 => self.leaves = true,
            Expr::Await(_) => self.leaves = true,
            Expr::ForLoop(_) | Expr::While(_) | Expr::Loop(_) => {
                self.loops += 1;
                visit::visit_expr(self, e);
                self.loops -= 1;
                return;
            }
            Expr::Closure(_) | Expr::Async(_) => {
                let (loops, closures) = (self.loops, self.closures);
                // A closure's own `break` can't be the loop's: it'd be refused.
                self.loops = usize::MAX / 2;
                self.closures += 1;
                visit::visit_expr(self, e);
                (self.loops, self.closures) = (loops, closures);
                return;
            }
            _ => {}
        }
        visit::visit_expr(self, e);
    }
    fn visit_label(&mut self, _: &'a syn::Label) {
        self.leaves = true;
    }
    fn visit_macro(&mut self, m: &'a syn::Macro) {
        // A macro's tokens may hold any of them: `?` or `return` in `vec![..]`.
        let said = m.tokens.to_string();
        if ["return", "break", "continue", "?", "await"]
            .iter()
            .any(|w| said.contains(w))
        {
            self.leaves = true;
        }
    }
    fn visit_item(&mut self, _: &'a Item) {}
}

fn leaves(block: &syn::Block) -> bool {
    let mut seen = Leaves::default();
    seen.visit_block(block);
    seen.leaves
}

pub fn for_each(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut made = ForEach { budget, changed: false };
    made.visit_file_mut(ast);
    made.changed
}

struct ForEach<'b> {
    budget: &'b mut usize,
    changed: bool,
}

impl VisitMut for ForEach<'_> {
    fn visit_stmt_mut(&mut self, s: &mut Stmt) {
        visit_mut::visit_stmt_mut(self, s);
        let Stmt::Expr(Expr::ForLoop(f), semi) = s else { return };
        if *self.budget == 0 || f.label.is_some() || !f.attrs.is_empty() || leaves(&f.body) {
            return;
        }
        let (pat, expr, body) = (&f.pat, &f.expr, &f.body);
        let chain: Expr = syn::parse_quote!((#expr).into_iter().for_each(|#pat| #body));
        *s = Stmt::Expr(chain, Some(semi.unwrap_or_default()));
        *self.budget -= 1;
        self.changed = true;
    }
}

pub fn for_loop(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut made = ForLoop { budget, changed: false };
    made.visit_file_mut(ast);
    made.changed
}

struct ForLoop<'b> {
    budget: &'b mut usize,
    changed: bool,
}

impl VisitMut for ForLoop<'_> {
    fn visit_stmt_mut(&mut self, s: &mut Stmt) {
        visit_mut::visit_stmt_mut(self, s);
        let Stmt::Expr(Expr::MethodCall(call), Some(_)) = s else {
            return;
        };
        if *self.budget == 0 || call.method != "for_each" || call.turbofish.is_some() || call.args.len() != 1 {
            return;
        }
        let Expr::Closure(closure) = &call.args[0] else { return };
        if closure.capture.is_some() || closure.asyncness.is_some() || closure.inputs.len() != 1 {
            return;
        }
        // A pattern of no type: a `for`'s can't say one.
        let pat = &closure.inputs[0];
        if matches!(pat, Pat::Type(_)) {
            return;
        }
        let body: syn::Block = match &*closure.body {
            Expr::Block(b) if b.label.is_none() && b.attrs.is_empty() => b.block.clone(),
            other => syn::parse_quote!({ #other; }),
        };
        // Its `return` ends one item's run: a `for`'s would end the function.
        let mut seen = Leaves {
            loops: 1,
            ..Leaves::default()
        };
        seen.visit_block(&body);
        if seen.leaves {
            return;
        }
        let iter = &call.receiver;
        *s = Stmt::Expr(syn::parse_quote!(for #pat in #iter #body), None);
        *self.budget -= 1;
        self.changed = true;
    }
}
