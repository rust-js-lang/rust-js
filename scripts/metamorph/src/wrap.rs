//! A local moved into a value that holds it: `boxed`, `option`,
//! `rc-refcell` and `field`. Each `let x = e;` of a name bound once in its
//! function holds `e` in the wrapper, and each use of `x` reads it there,
//! a place as `x` was.

use proc_macro2::{Group, Ident, Punct, Spacing, TokenStream, TokenTree};
use quote::quote;
use syn::visit::Visit;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Item, Pat};

use crate::{Bindings, each_function};

/// What a local is moved into.
#[derive(Clone, Copy, PartialEq)]
pub enum Wrap {
    /// `Box::new(e)`, read `(*x)`.
    Boxed,
    /// `Some(e)`, read `(*x.as_ref().unwrap())`, or `as_mut` of a `mut` one.
    Option,
    /// `Rc::new(RefCell::new(e))`, read `(*x.borrow())`, or `borrow_mut`
    /// of a `mut` one.
    RcRefCell,
    /// `MetamorphField(e)`, a one-field struct, read `(x.0)`.
    Field,
}

impl Wrap {
    fn value(self, e: &Expr) -> Expr {
        match self {
            Wrap::Boxed => syn::parse_quote!(Box::new(#e)),
            Wrap::Option => syn::parse_quote!(Some(#e)),
            Wrap::RcRefCell => syn::parse_quote!(::std::rc::Rc::new(::std::cell::RefCell::new(#e))),
            Wrap::Field => syn::parse_quote!(MetamorphField(#e)),
        }
    }

    fn ty(self, t: &syn::Type) -> syn::Type {
        match self {
            Wrap::Boxed => syn::parse_quote!(Box<#t>),
            Wrap::Option => syn::parse_quote!(Option<#t>),
            Wrap::RcRefCell => syn::parse_quote!(::std::rc::Rc<::std::cell::RefCell<#t>>),
            Wrap::Field => syn::parse_quote!(MetamorphField<#t>),
        }
    }

    /// `x` read where it's held, a place.
    fn read(self, x: &Ident, mutable: bool) -> TokenStream {
        match (self, mutable) {
            (Wrap::Boxed, _) => quote!((*#x)),
            (Wrap::Option, false) => quote!((*#x.as_ref().unwrap())),
            (Wrap::Option, true) => quote!((*#x.as_mut().unwrap())),
            (Wrap::RcRefCell, false) => quote!((*#x.borrow())),
            (Wrap::RcRefCell, true) => quote!((*#x.borrow_mut())),
            (Wrap::Field, _) => quote!((#x.0)),
        }
    }
}

pub fn wrap(ast: &mut syn::File, how: Wrap, budget: &mut usize) -> bool {
    let mut changed = false;
    each_function(ast, &mut |sig, block| {
        let mut bound = Bindings::default();
        for input in &sig.inputs {
            bound.visit_fn_arg(input);
        }
        bound.visit_block(block);
        let once = |name: &Ident| bound.0.get(&name.to_string()) == Some(&1);
        let mut names = Vec::new();
        Lets {
            how,
            once: &once,
            names: &mut names,
            budget,
        }
        .visit_block_mut(block);
        for (name, mutable) in names {
            Through {
                how,
                name: &name,
                mutable,
            }
            .visit_block_mut(block);
            changed = true;
        }
    });
    if changed && how == Wrap::Field {
        ast.items.push(syn::parse_quote!(
            #[derive(Clone, Copy)]
            struct MetamorphField<T>(T);
        ));
    }
    changed
}

/// Each `let x = e;` of a name bound once moved into the wrapper.
struct Lets<'n, 'o> {
    how: Wrap,
    once: &'o dyn Fn(&Ident) -> bool,
    names: &'n mut Vec<(Ident, bool)>,
    budget: &'n mut usize,
}

impl VisitMut for Lets<'_, '_> {
    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        visit_mut::visit_local_mut(self, local);
        let Some(init) = &mut local.init else { return };
        if init.diverge.is_some() || *self.budget == 0 {
            return;
        }
        let (ident, mutable, ty) = match &mut local.pat {
            Pat::Ident(p) if p.by_ref.is_none() && p.subpat.is_none() => {
                (p.ident.clone(), p.mutability.is_some(), None)
            }
            Pat::Type(t) => match &*t.pat {
                Pat::Ident(p) if p.by_ref.is_none() && p.subpat.is_none() => {
                    (p.ident.clone(), p.mutability.is_some(), Some(&mut t.ty))
                }
                _ => return,
            },
            _ => return,
        };
        if !(self.once)(&ident) {
            return;
        }
        *init.expr = self.how.value(&init.expr);
        if let Some(ty) = ty {
            **ty = self.how.ty(ty);
        }
        *self.budget -= 1;
        self.names.push((ident, mutable));
    }
    fn visit_item_mut(&mut self, _: &mut Item) {}
}

/// Each use of `name` read where it's held: a path's, a struct literal's
/// shorthand field's, and a macro's tokens', a format string's own
/// `{name}` given as a named argument, `name = <read>`.
struct Through<'n> {
    how: Wrap,
    name: &'n Ident,
    mutable: bool,
}

/// Whether `text`, a format string's, names `name` itself: `{name}`,
/// `{name:?}` or a width or precision, `name$`.
fn names_in_format(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    let ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut from = 0;
    while let Some(at) = text[from..].find(name) {
        let start = from + at;
        let end = start + name.len();
        from = end;
        let before = start.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(end).copied();
        if before.is_some_and(ident) || after.is_some_and(ident) {
            continue;
        }
        match (before, after) {
            (Some(b'{'), Some(b'}' | b':')) => {
                // Not an escaped `{{name}`.
                if start < 2 || bytes[start - 2] != b'{' {
                    return true;
                }
            }
            (_, Some(b'$')) => return true,
            _ => {}
        }
    }
    false
}

impl Through<'_> {
    fn is_it(&self, e: &Expr) -> bool {
        matches!(e, Expr::Path(p) if p.qself.is_none() && p.path.is_ident(self.name))
    }

    fn read(&self) -> TokenStream {
        self.how.read(self.name, self.mutable)
    }

    fn tokens(&self, stream: TokenStream) -> TokenStream {
        let trees: Vec<TokenTree> = stream.into_iter().collect();
        let mut out = Vec::new();
        for (i, tree) in trees.iter().enumerate() {
            let punct = |at: Option<&TokenTree>, c: char| matches!(at, Some(TokenTree::Punct(p)) if p.as_char() == c);
            let before = i.checked_sub(1).and_then(|i| trees.get(i));
            match tree {
                TokenTree::Ident(id) if id == self.name => {
                    let after = trees.get(i + 1);
                    // A field, `a.x`; a path's segment, `m::x` or `x::y`; a
                    // field named in a literal, `S { x: 1 }`; a macro, `x!`.
                    if punct(before, '.') || punct(before, ':') || punct(after, ':') || punct(after, '!') {
                        out.push(tree.clone());
                    } else {
                        out.extend(self.read());
                    }
                }
                TokenTree::Group(g) => {
                    // A macro's own tokens, `format!(..)` in another's.
                    let inner = if self.how != Wrap::Boxed && punct(before, '!') {
                        self.macro_tokens(g.stream())
                    } else {
                        self.tokens(g.stream())
                    };
                    let mut group = Group::new(g.delimiter(), inner);
                    group.set_span(g.span());
                    out.push(TokenTree::Group(group));
                }
                _ => out.push(tree.clone()),
            }
        }
        out.into_iter().collect()
    }

    /// A macro's tokens, and its format string's `{name}` given as a named
    /// argument: what it formats is the value, not what holds it.
    fn macro_tokens(&self, stream: TokenStream) -> TokenStream {
        let named = stream.clone().into_iter().any(|t| match t {
            TokenTree::Literal(l) => names_in_format(&l.to_string(), &self.name.to_string()),
            _ => false,
        });
        let mut out: Vec<TokenTree> = self.tokens(stream).into_iter().collect();
        if named {
            if !matches!(out.last(), Some(TokenTree::Punct(p)) if p.as_char() == ',') {
                out.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
            }
            out.push(TokenTree::Ident(self.name.clone()));
            out.push(TokenTree::Punct(Punct::new('=', Spacing::Alone)));
            out.extend(self.read());
        }
        out.into_iter().collect()
    }
}

impl VisitMut for Through<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        if self.is_it(e) {
            let read = self.read();
            *e = syn::parse2(read).expect("a read");
            return;
        }
        visit_mut::visit_expr_mut(self, e);
    }
    fn visit_field_value_mut(&mut self, field: &mut syn::FieldValue) {
        if field.colon_token.is_none() && matches!(&field.member, syn::Member::Named(m) if m == self.name) {
            field.colon_token = Some(Default::default());
        }
        visit_mut::visit_field_value_mut(self, field);
    }
    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        // Its own `let`'s value, the wrapper, read as it was.
        if let Some(init) = &mut local.init {
            self.visit_expr_mut(&mut init.expr);
            if let Some((_, diverge)) = &mut init.diverge {
                self.visit_expr_mut(diverge);
            }
        }
    }
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        let tokens = std::mem::take(&mut m.tokens);
        m.tokens = match self.how {
            Wrap::Boxed => self.tokens(tokens),
            _ => self.macro_tokens(tokens),
        };
    }
    fn visit_item_mut(&mut self, _: &mut Item) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_string_names_its_own_arguments() {
        assert!(names_in_format("\"{x}\"", "x"));
        assert!(names_in_format("\"{x:?} {}\"", "x"));
        assert!(names_in_format("\"{:>x$}\"", "x"));
        assert!(!names_in_format("\"{{x}}\"", "x"));
        assert!(!names_in_format("\"{xs}\"", "x"));
        assert!(!names_in_format("\"x\"", "x"));
    }
}
