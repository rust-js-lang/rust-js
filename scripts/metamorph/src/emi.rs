//! Equivalence modulo inputs (EMI, #1086): code a case's run never reaches,
//! deleted or added, changes nothing it prints.
//!
//! - `emi-probe`: each block numbered, in the order it's written, and made
//!   to say it ran, `METAMORPH-EMI-HIT <n>` on stderr, the first time it
//!   does: what the harness runs natively to find what's never reached.
//! - `emi-prune`: each block the probe's run never reached, its numbers
//!   `METAMORPH_EMI_HITS` doesn't hold, made `unreachable!()`.
//! - `emi-insert`: code that never runs added, `if
//!   std::hint::black_box(false) { .. }`: after each `let` of a name, a
//!   borrow of it, `&mut` of one bound `mut`; and at `main`'s start, a call
//!   of each of the case's free functions that takes nothing.
//!
//! A block is a function's body, a closure's, a branch's, a loop's or a
//! match arm's, and an arm's expression; none in a `const fn`, a `const`, a
//! `static` or a binding to JS, which no run reaches.

use quote::ToTokens;
use std::collections::BTreeSet;
use syn::visit_mut::{self, VisitMut};
use syn::{Block, Expr, Item, Pat, Stmt};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Probe,
    Prune,
}

struct Blocks<'a> {
    mode: Mode,
    next: u32,
    hits: &'a BTreeSet<u32>,
    budget: &'a mut usize,
    changed: bool,
}

impl Blocks<'_> {
    fn number(&mut self) -> u32 {
        self.next += 1;
        self.next - 1
    }

    /// Whether block `n` is made `unreachable!()`: one the probe's run
    /// never reached, within the budget.
    fn pruned(&mut self, n: u32) -> bool {
        if self.mode != Mode::Prune || self.hits.contains(&n) || *self.budget == 0 {
            return false;
        }
        *self.budget -= 1;
        self.changed = true;
        true
    }
}

/// Whether `attrs` mark a binding to JS, whose body never runs natively.
fn bound(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.to_token_stream().to_string().contains("rust_js"))
}

impl VisitMut for Blocks<'_> {
    fn visit_block_mut(&mut self, block: &mut Block) {
        let n = self.number();
        if self.mode == Mode::Probe {
            block.stmts.insert(0, syn::parse_quote!(metamorph_emi_hit(#n);));
            self.changed = true;
        }
        visit_mut::visit_block_mut(self, block);
        if self.pruned(n) {
            *block = syn::parse_quote!({ unreachable!() });
        }
    }

    fn visit_arm_mut(&mut self, arm: &mut syn::Arm) {
        if let Some((_, guard)) = &mut arm.guard {
            self.visit_expr_mut(guard);
        }
        if matches!(&*arm.body, Expr::Block(b) if b.label.is_none() && b.attrs.is_empty()) {
            self.visit_expr_mut(&mut arm.body);
            return;
        }
        let n = self.number();
        self.visit_expr_mut(&mut arm.body);
        if self.mode == Mode::Probe {
            let body = &arm.body;
            *arm.body = syn::parse_quote!({ metamorph_emi_hit(#n); #body });
        } else if self.pruned(n) {
            *arm.body = syn::parse_quote!(unreachable!());
        }
    }

    fn visit_item_fn_mut(&mut self, f: &mut syn::ItemFn) {
        if f.sig.constness.is_none() && !bound(&f.attrs) {
            visit_mut::visit_item_fn_mut(self, f);
        }
    }

    fn visit_impl_item_fn_mut(&mut self, f: &mut syn::ImplItemFn) {
        if f.sig.constness.is_none() && !bound(&f.attrs) {
            visit_mut::visit_impl_item_fn_mut(self, f);
        }
    }

    fn visit_trait_item_fn_mut(&mut self, f: &mut syn::TraitItemFn) {
        if f.sig.constness.is_none() {
            visit_mut::visit_trait_item_fn_mut(self, f);
        }
    }

    // What no run reaches as it's run: a constant's, a static's, an inline
    // `const` block's, an array's length, a macro's tokens.
    fn visit_item_const_mut(&mut self, _: &mut syn::ItemConst) {}
    fn visit_item_static_mut(&mut self, _: &mut syn::ItemStatic) {}
    fn visit_impl_item_const_mut(&mut self, _: &mut syn::ImplItemConst) {}
    fn visit_trait_item_const_mut(&mut self, _: &mut syn::TraitItemConst) {}
    fn visit_expr_const_mut(&mut self, _: &mut syn::ExprConst) {}
    fn visit_type_mut(&mut self, _: &mut syn::Type) {}
    fn visit_macro_mut(&mut self, _: &mut syn::Macro) {}
}

/// The probe: each block saying it ran, the first time it does.
pub fn probe(ast: &mut syn::File) -> bool {
    let hits = BTreeSet::new();
    let mut budget = usize::MAX;
    let mut blocks = Blocks {
        mode: Mode::Probe,
        next: 0,
        hits: &hits,
        budget: &mut budget,
        changed: false,
    };
    blocks.visit_file_mut(ast);
    if !blocks.changed {
        return false;
    }
    ast.items.push(syn::parse_quote! {
        fn metamorph_emi_hit(n: u32) {
            static HIT: std::sync::Mutex<Vec<bool>> = std::sync::Mutex::new(Vec::new());
            let mut hit = HIT.lock().unwrap_or_else(|e| e.into_inner());
            if hit.len() <= n as usize {
                hit.resize(n as usize + 1, false);
            }
            if !hit[n as usize] {
                hit[n as usize] = true;
                eprintln!("METAMORPH-EMI-HIT {}", n);
            }
        }
    });
    true
}

/// Each block the probe never saw run, numbered as the probe numbers them,
/// made `unreachable!()`, of `budget` of them.
pub fn prune(ast: &mut syn::File, hits: &BTreeSet<u32>, budget: &mut usize) -> bool {
    let mut blocks = Blocks {
        mode: Mode::Prune,
        next: 0,
        hits,
        budget,
        changed: false,
    };
    blocks.visit_file_mut(ast);
    blocks.changed
}

/// The names `pat` binds by value, and whether `mut`: `x`, `mut x`, `x: T`.
fn bound_name(pat: &Pat) -> Option<(syn::Ident, bool)> {
    match pat {
        Pat::Ident(p) if p.by_ref.is_none() && p.subpat.is_none() => Some((p.ident.clone(), p.mutability.is_some())),
        Pat::Type(t) => bound_name(&t.pat),
        _ => None,
    }
}

struct Insert<'a> {
    budget: &'a mut usize,
    changed: bool,
}

impl VisitMut for Insert<'_> {
    fn visit_block_mut(&mut self, block: &mut Block) {
        visit_mut::visit_block_mut(self, block);
        let mut stmts = Vec::with_capacity(block.stmts.len());
        for stmt in std::mem::take(&mut block.stmts) {
            let borrow: Option<Stmt> = match &stmt {
                Stmt::Local(local) if local.init.is_some() && *self.budget > 0 => {
                    bound_name(&local.pat).map(|(name, mutable)| match mutable {
                        true => syn::parse_quote!(if std::hint::black_box(false) { let _emi = &mut #name; }),
                        false => syn::parse_quote!(if std::hint::black_box(false) { let _emi = &#name; }),
                    })
                }
                _ => None,
            };
            stmts.push(stmt);
            if let Some(borrow) = borrow {
                *self.budget -= 1;
                self.changed = true;
                stmts.push(borrow);
            }
        }
        block.stmts = stmts;
    }

    fn visit_item_fn_mut(&mut self, f: &mut syn::ItemFn) {
        if f.sig.constness.is_none() && !bound(&f.attrs) {
            visit_mut::visit_item_fn_mut(self, f);
        }
    }

    fn visit_impl_item_fn_mut(&mut self, f: &mut syn::ImplItemFn) {
        if f.sig.constness.is_none() && !bound(&f.attrs) {
            visit_mut::visit_impl_item_fn_mut(self, f);
        }
    }

    fn visit_item_const_mut(&mut self, _: &mut syn::ItemConst) {}
    fn visit_item_static_mut(&mut self, _: &mut syn::ItemStatic) {}
    fn visit_impl_item_const_mut(&mut self, _: &mut syn::ImplItemConst) {}
    fn visit_expr_const_mut(&mut self, _: &mut syn::ExprConst) {}
    fn visit_type_mut(&mut self, _: &mut syn::Type) {}
    fn visit_macro_mut(&mut self, _: &mut syn::Macro) {}
}

/// Code never run added, of `budget` places: a borrow of each name a `let`
/// binds, after it; a call of each free function that takes nothing, at
/// `main`'s start.
pub fn insert(ast: &mut syn::File, budget: &mut usize) -> bool {
    let callable: Vec<syn::Ident> = (ast.items.iter())
        .filter_map(|item| match item {
            Item::Fn(f)
                if f.sig.ident != "main"
                    && f.sig.inputs.is_empty()
                    && f.sig.generics.params.is_empty()
                    && f.sig.asyncness.is_none()
                    && f.sig.unsafety.is_none()
                    && f.sig.constness.is_none()
                    && !bound(&f.attrs) =>
            {
                Some(f.sig.ident.clone())
            }
            _ => None,
        })
        .collect();
    let mut insert = Insert { budget, changed: false };
    insert.visit_file_mut(ast);
    let mut changed = insert.changed;
    if let Some(Item::Fn(main)) =
        (ast.items.iter_mut()).find(|item| matches!(item, Item::Fn(f) if f.sig.ident == "main"))
    {
        for name in callable.into_iter().rev() {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            changed = true;
            main.block.stmts.insert(
                0,
                syn::parse_quote!(if std::hint::black_box(false) { let _emi = #name(); }),
            );
        }
    }
    changed
}

/// The numbers of the blocks a probe's run reached, from what it said on
/// stderr, given as `METAMORPH_EMI_HITS`: `3,5,8`.
pub fn hits_given() -> Option<BTreeSet<u32>> {
    let given = std::env::var("METAMORPH_EMI_HITS").ok()?;
    Some(
        given
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse().ok())
            .collect(),
    )
}
