//! The shape of a `jsx!` body, for laying it out (#1901): its elements,
//! props and children, each piece of Rust in it by where its text is. The
//! compiler's parser (`parser.rs`) is what says a body is JSX, as each run
//! of the pass gives it the body first; this reads what it accepted, and
//! one it can't read is said so, never laid out by a guess.

use std::ops::Range;

use rustc_ast::token::{Delimiter, TokenKind};
use rustc_ast::tokenstream::{TokenStream, TokenTree};
use rustc_span::{BytePos, Span};

/// An element, or a fragment where `name` is none.
pub struct Element {
    /// Where its name is, `Ctx::Provider` or `Card::<T>`, its generics too.
    pub name: Option<Range<usize>>,
    pub attrs: Vec<Attr>,
    /// None of one that closes itself, `<br />`.
    pub children: Option<Vec<Child>>,
}

/// A prop.
pub enum Attr {
    /// `name`, `aria-label` or `on:click`, and its value, of one written
    /// with `=`.
    Named { name: Range<usize>, value: Option<Piece> },
    /// `{...props}` or `{..base}`: where it is whole, and where the Rust
    /// after its dots is.
    Spread {
        whole: Range<usize>,
        dots: usize,
        inner: Range<usize>,
    },
}

/// A child.
pub enum Child {
    Element(Element),
    Piece(Piece),
    /// `{/* .. */}`: where it is, braces and all, written as it is, where
    /// rustfmt would indent a comment's lines as a piece's.
    Comment(Range<usize>),
}

/// A piece of Rust, which rustfmt lays out.
pub enum Piece {
    /// `{expr}`: where it is whole, and where its expression is.
    Braced { whole: Range<usize>, inner: Range<usize> },
    /// A literal, `"Save"` or `3`, written as it is.
    Literal(Range<usize>),
}

/// The shape of `tokens`, a `jsx!` body's, whose file starts at `start`.
pub fn read(tokens: &TokenStream, start: BytePos) -> Result<Element, String> {
    let trees: Vec<TokenTree> = tokens.iter().cloned().collect();
    let mut reader = Reader { trees, at: 0, start };
    let element = reader.element()?;
    match reader.at == reader.trees.len() {
        true => Ok(element),
        false => Err(reader.unread("what follows the element")),
    }
}

struct Reader {
    trees: Vec<TokenTree>,
    at: usize,
    start: BytePos,
}

impl Reader {
    fn offset(&self, span: Span) -> Range<usize> {
        (span.lo() - self.start).0 as usize..(span.hi() - self.start).0 as usize
    }

    fn unread(&self, what: &str) -> String {
        let at = self.trees.get(self.at).map(|tree| self.offset(tree.span()).start);
        format!(
            "the layout can't read {what}{}",
            at.map(|at| format!(" at byte {at}")).unwrap_or_default()
        )
    }

    fn token(&self, kind: TokenKind) -> bool {
        matches!(self.trees.get(self.at), Some(TokenTree::Token(t, _)) if t.kind == kind)
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        let is = self.token(kind);
        self.at += usize::from(is);
        is
    }

    fn ident(&mut self) -> Option<Range<usize>> {
        match self.trees.get(self.at) {
            Some(TokenTree::Token(t, _)) if matches!(t.kind, TokenKind::Ident(..)) => {
                self.at += 1;
                Some(self.offset(t.span))
            }
            _ => None,
        }
    }

    /// The span of the tree last read.
    fn last(&self) -> Range<usize> {
        self.offset(self.trees[self.at - 1].span())
    }

    fn element(&mut self) -> Result<Element, String> {
        if !self.eat(TokenKind::Lt) {
            return Err(self.unread("an element, `<tag>`"));
        }
        // Its name, a path or a member, and its generics, `::<T>`.
        let name = match self.ident() {
            None => None,
            Some(first) => {
                let mut end = first.end;
                while self.token(TokenKind::PathSep) || self.token(TokenKind::Dot) {
                    self.at += 1;
                    end = self.last().end;
                    if self.token(TokenKind::Lt) {
                        let mut depth = 0i32;
                        while let Some(tree) = self.trees.get(self.at) {
                            if let TokenTree::Token(t, _) = tree {
                                match t.kind {
                                    TokenKind::Lt => depth += 1,
                                    TokenKind::Gt => depth -= 1,
                                    TokenKind::Shr => depth -= 2,
                                    _ => {}
                                }
                            }
                            self.at += 1;
                            end = self.last().end;
                            if depth <= 0 {
                                break;
                            }
                        }
                        break;
                    }
                    end = self.ident().ok_or_else(|| self.unread("a name's segment"))?.end;
                }
                Some(first.start..end)
            }
        };
        let mut attrs = Vec::new();
        while !self.token(TokenKind::Gt) && !self.token(TokenKind::Slash) {
            if let Some(TokenTree::Delimited(span, _, Delimiter::Brace, inner)) = self.trees.get(self.at)
                && let Some(TokenTree::Token(dots, _)) = inner.get(0)
                && matches!(dots.kind, TokenKind::DotDot | TokenKind::DotDotDot)
            {
                let whole = self.offset(span.entire());
                let dots = self.offset(dots.span);
                let inner = dots.end..self.offset(span.close).start;
                attrs.push(Attr::Spread {
                    whole,
                    dots: dots.end - dots.start,
                    inner,
                });
                self.at += 1;
                continue;
            }
            let first = self.ident().ok_or_else(|| self.unread("a prop's name"))?;
            let mut end = first.end;
            while self.token(TokenKind::Minus) || self.token(TokenKind::Colon) {
                self.at += 1;
                end = self.ident().ok_or_else(|| self.unread("a prop's name"))?.end;
            }
            let value = match self.eat(TokenKind::Eq) {
                true => Some(self.piece().ok_or_else(|| self.unread("a prop's value"))?),
                false => None,
            };
            attrs.push(Attr::Named {
                name: first.start..end,
                value,
            });
        }
        if self.eat(TokenKind::Slash) {
            if !self.eat(TokenKind::Gt) {
                return Err(self.unread("`/>`"));
            }
            return Ok(Element {
                name,
                attrs,
                children: None,
            });
        }
        self.at += 1; // `>`
        let mut children = Vec::new();
        loop {
            let closing = self.token(TokenKind::Lt)
                && matches!(self.trees.get(self.at + 1), Some(TokenTree::Token(t, _)) if t.kind == TokenKind::Slash);
            if closing {
                self.at += 2;
                while !self.eat(TokenKind::Gt) {
                    if self.at >= self.trees.len() {
                        return Err(self.unread("a closing tag"));
                    }
                    self.at += 1;
                }
                return Ok(Element {
                    name,
                    attrs,
                    children: Some(children),
                });
            }
            if self.token(TokenKind::Lt) {
                children.push(Child::Element(self.element()?));
            } else if let Some(TokenTree::Delimited(span, _, Delimiter::Brace, inner)) = self.trees.get(self.at)
                && inner.is_empty()
            {
                children.push(Child::Comment(self.offset(span.entire())));
                self.at += 1;
            } else if let Some(piece) = self.piece() {
                children.push(Child::Piece(piece));
            } else {
                return Err(self.unread("a child"));
            }
        }
    }

    fn piece(&mut self) -> Option<Piece> {
        match self.trees.get(self.at)? {
            TokenTree::Delimited(span, _, Delimiter::Brace, _) => {
                let piece = Piece::Braced {
                    whole: self.offset(span.entire()),
                    inner: self.offset(span.open).end..self.offset(span.close).start,
                };
                self.at += 1;
                Some(piece)
            }
            TokenTree::Token(t, _) if matches!(t.kind, TokenKind::Literal(_)) => {
                let piece = Piece::Literal(self.offset(t.span));
                self.at += 1;
                Some(piece)
            }
            _ => None,
        }
    }
}
