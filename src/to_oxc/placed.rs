//! Where a statement copied into a function from past it is printed: a
//! `Drop` impl's body, written where its value's dropped (`drops/inlined.rs`),
//! keeps the impl's spans, its map's and its comments' places. oxc prints a
//! comment whose node is gone, a block's of only comments (ADR 0753), before
//! the first statement past it by where each starts, and at a block's end
//! those before it: a statement copied from past its function, or a block
//! that ends there, is past the comments of what's printed after the
//! function, which would print in it (rd328). Each is given a place of its
//! own in its function, right past the last printed before it, where no
//! comment is, and its comments are there. A statement's place maps to
//! nothing: what it holds keeps its spans, and its map. One copied from
//! before its function is before every comment printed after it.

use std::collections::{HashMap, HashSet};

use oxc_ast::AstKind;
use oxc_ast::ast::{BlockStatement, Function, FunctionBody, Statement, SwitchCase};
use oxc_ast_visit::{Visit, VisitMut};
use oxc_ast_visit::{walk, walk_mut};
use oxc_span::{GetSpan, GetSpanMut, Span};
use oxc_syntax::scope::ScopeFlags;

/// Of `body`, a module's statements, each statement of a function placed
/// in it, as they're printed, past the function's end, and each block
/// ending there: at the first place past the last printed before it that's
/// none of `keys`, where comments are (`comments_of`), nor where a node
/// starts, nor another's. Where each was written, the place of the first
/// printed there, which its comments are at.
pub(super) fn placed(body: &mut [Statement<'_>], keys: HashSet<u32>) -> HashMap<u32, u32> {
    let mut starts = Starts {
        keys,
        function: None,
        copied: false,
    };
    body.iter().for_each(|statement| starts.visit_statement(statement));
    let mut placing = Placing {
        function: None,
        after: 0,
        keys: starts.keys,
        here: HashMap::new(),
        moved: HashMap::new(),
    };
    if starts.copied {
        body.iter_mut().for_each(|statement| placing.visit_statement(statement));
    }
    placing.moved
}

/// Where comments are and nodes start, whose comments oxc prints there;
/// and whether a function has a statement written past it.
struct Starts {
    keys: HashSet<u32>,
    function: Option<Span>,
    copied: bool,
}

impl<'a> Visit<'a> for Starts {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.keys.insert(kind.span().start);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.function.is_some() || it.span.is_empty() {
            return walk::walk_function(self, it, flags);
        }
        self.function = Some(it.span);
        walk::walk_function(self, it, flags);
        self.function = None;
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        let start = it.span().start;
        self.copied |= !matches!(it, Statement::BlockStatement(_)) && past(self.function, start);
        walk::walk_statement(self, it);
    }
}

/// Whether `at` is past the end of `function`, a module's.
fn past(function: Option<Span>, at: u32) -> bool {
    function.is_some_and(|function| at != 0 && at >= function.end)
}

/// The function whose statements are placed, the last place printed in
/// it, where comments are or a place is, the places of what's written
/// where in it, and in the module, of the first.
struct Placing {
    function: Option<Span>,
    after: u32,
    keys: HashSet<u32>,
    here: HashMap<u32, u32>,
    moved: HashMap<u32, u32>,
}

impl Placing {
    /// The place in the function of what's written at `at`: the one
    /// given before, or the next.
    fn place(&mut self, at: u32) -> u32 {
        if let Some(&place) = self.here.get(&at) {
            return place;
        }
        let mut place = self.after + 1;
        while self.keys.contains(&place) {
            place += 1;
        }
        self.keys.insert(place);
        self.after = place;
        self.here.insert(at, place);
        self.moved.entry(at).or_insert(place);
        place
    }

    /// A block's `span`, which ends where its comments end (ADR 0753): one
    /// ending past the function, at its place, once what's in it is.
    fn ended(&mut self, span: &mut Span) {
        if past(self.function, span.end) && (span.start == span.end || past(self.function, span.start)) {
            // Its comments' place, before its end; and its start, there or
            // where it ends.
            let place = self.place(span.end - 1);
            *span = Span::new(place + span.start.saturating_sub(span.end - 1), place + 1);
        }
    }
}

impl<'a> VisitMut<'a> for Placing {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        if self.function.is_some() || it.span.is_empty() {
            return walk_mut::walk_function(self, it, flags);
        }
        (self.function, self.after) = (Some(it.span), it.span.start);
        self.here.clear();
        walk_mut::walk_function(self, it, flags);
        self.function = None;
    }

    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        let start = it.span().start;
        // A block's span is where its comments end, `ended`'s.
        let block = matches!(it, Statement::BlockStatement(_));
        if !block && past(self.function, start) {
            let place = self.place(start);
            *it.span_mut() = Span::new(place, place);
        } else if !block && self.function.is_some() && start != 0 {
            self.after = self.after.max(start);
        }
        walk_mut::walk_statement(self, it);
    }

    fn visit_block_statement(&mut self, it: &mut BlockStatement<'a>) {
        walk_mut::walk_block_statement(self, it);
        self.ended(&mut it.span);
    }

    fn visit_function_body(&mut self, it: &mut FunctionBody<'a>) {
        walk_mut::walk_function_body(self, it);
        self.ended(&mut it.span);
    }

    fn visit_switch_case(&mut self, it: &mut SwitchCase<'a>) {
        walk_mut::walk_switch_case(self, it);
        self.ended(&mut it.span);
    }
}
