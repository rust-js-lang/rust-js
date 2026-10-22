//! Expressions made: each kind's constructor, folding what's known as it's
//! made.

use super::*;

impl Expr {
    pub(super) fn new(kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: Span::NONE,
            reads: false,
        }
    }

    /// `of.method(args)`, of a `str` or a slice `of` as Rust's types tell.
    pub fn read(of: Expr, method: &str, args: Vec<Expr>) -> Expr {
        Expr {
            reads: true,
            ..Expr::call(Expr::member(of, method), args)
        }
    }

    pub fn num(n: impl Into<f64>) -> Expr {
        Expr::new(ExprKind::Num(n.into()))
    }

    pub fn int(n: i128) -> Expr {
        // A number-represented integer fits in 32 bits, so this is exact.
        Expr::num(n as f64)
    }

    pub fn bigint(n: i128) -> Expr {
        Expr::new(ExprKind::BigInt(n))
    }

    /// A `u128`'s value, `BigInt` where it fits.
    pub fn biguint(n: u128) -> Expr {
        match i128::try_from(n) {
            Ok(n) => Expr::bigint(n),
            Err(_) => Expr::new(ExprKind::BigUint(n)),
        }
    }

    pub fn bool(b: bool) -> Expr {
        Expr::new(ExprKind::Bool(b))
    }

    pub fn str(s: impl Into<String>) -> Expr {
        Expr::new(ExprKind::Str(s.into()))
    }

    pub fn lines(s: impl Into<String>) -> Expr {
        Expr::new(ExprKind::Lines(s.into()))
    }

    /// A regular expression literal, as written: `/^[0-9]$/`.
    pub fn regex(literal: &str) -> Expr {
        Expr::new(ExprKind::Regex(literal.to_string()))
    }

    pub fn template(texts: Vec<String>, values: Vec<Expr>) -> Expr {
        debug_assert_eq!(texts.len(), values.len() + 1);
        // `${false}` is `false`, and `${3}` is `3`: a literal the template
        // would show is written in its text, as JS shows it.
        let mut texts = texts.into_iter();
        let mut folded = vec![texts.next().expect("a text before each value")];
        let mut kept = Vec::new();
        for (value, text) in values.into_iter().zip(texts) {
            let shown = match value.kind {
                ExprKind::Bool(b) => Some(b.to_string()),
                ExprKind::Num(n) if n.fract() == 0.0 && n.abs() < 1e21 => Some((n as i128).to_string()),
                _ => None,
            };
            match shown {
                Some(shown) => {
                    let last = folded.last_mut().expect("a text");
                    last.push_str(&shown);
                    last.push_str(&text);
                }
                None => {
                    kept.push(value);
                    folded.push(text);
                }
            }
        }
        // All of it written so, it's a string.
        if kept.is_empty() {
            return Expr::str(folded.pop().expect("a text"));
        }
        Expr::new(ExprKind::Template(folded, kept, false))
    }

    /// A template written across `lines`: its line breaks written as they are.
    pub fn with_lines(self, lines: bool) -> Expr {
        match self.kind {
            ExprKind::Template(texts, values, _) => Expr::new(ExprKind::Template(texts, values, lines)),
            _ => self,
        }
    }

    pub fn undefined() -> Expr {
        Expr::new(ExprKind::Undefined)
    }

    /// `list` and then each of `optional` given, `undefined` for one absent
    /// before a given one, and none past the last given: a helper's trailing
    /// arguments, its drops.
    pub fn optional_args(mut list: Vec<Expr>, mut optional: Vec<Option<Expr>>) -> Vec<Expr> {
        while matches!(optional.last(), Some(None)) {
            optional.pop();
        }
        list.extend(optional.into_iter().map(|arg| arg.unwrap_or_else(Expr::undefined)));
        list
    }

    pub fn null() -> Expr {
        Expr::new(ExprKind::Null)
    }

    pub fn var(name: &str) -> Expr {
        Expr::new(ExprKind::Var(name.to_string()))
    }

    /// The runtime's helper `name` exports, `$cmp`: asked for by being
    /// written, as the linker asks for each its module's tree reads. A name
    /// the runtime doesn't export is a bug of rust-js's. No other variable
    /// is named so: Rust's names have no `$`, and the compiler's own are
    /// named apart from the runtime's (`names::dictionary_cache`).
    pub fn helper(name: &'static str) -> Expr {
        assert!(crate::runtime::exports(name), "the runtime exports no `{name}`");
        Expr::var(name)
    }

    /// The runtime's helper this is, by the name it exports (ADR 0103).
    pub fn helper_name(&self) -> Option<&str> {
        match &self.kind {
            ExprKind::Var(name) if name.starts_with('$') && crate::runtime::exports(name) => Some(name),
            _ => None,
        }
    }

    /// A call of the runtime's helper `name`, `$cmp(a, b)`.
    pub fn call_helper(name: &'static str, args: Vec<Expr>) -> Expr {
        Expr::call(Expr::helper(name), args)
    }

    pub fn drop_argument(item: u32, index: u32, drop: Expr) -> Expr {
        Expr::new(ExprKind::DropArgument(item, index, Box::new(drop)))
    }

    pub fn import(from: ImportFrom) -> Expr {
        Expr::new(ExprKind::Import(from))
    }

    pub fn handle(place: Expr) -> Expr {
        Expr::new(ExprKind::Handle(Box::new(place)))
    }

    pub fn pair(place: Expr, dictionary: Expr) -> Expr {
        Expr::new(ExprKind::Pair(Box::new(place), Box::new(dictionary)))
    }

    /// What a box, a cell or a handle holds: `b.value` (`HELD`).
    pub fn held(of: Expr) -> Expr {
        Expr::member(of, HELD)
    }

    pub fn member(object: Expr, property: impl Into<String>) -> Expr {
        let property = property.into();
        // `{ x: 0, y: 0 }.y`, a constant's field (`V::ZERO.y`), is `0`; and a
        // `dyn`'s pair made of what does nothing, `{ value: f.last, impl:
        // f.impl }.impl`, a shared reference nothing assigns through (ADR 0409).
        if let ExprKind::Object(props) = &object.kind
            && (props
                .iter()
                .all(|p| matches!(p, Prop::Field(_, value) if value.is_constant()))
                || is_read_pair(props))
            && let Some(Prop::Field(_, value)) = props
                .iter()
                .find(|p| matches!(p, Prop::Field(name, _) if *name == property))
        {
            return value.clone();
        }
        // A handle's `value` is the place it's on (ADR 0099), and a cell
        // made here, `{ value: x }`, holds `x`: a cell in a field is the
        // property itself (ADR 0288).
        if property == HELD {
            if let ExprKind::Handle(place) = object.kind {
                return *place;
            }
            // And of a handle through an `Option`'s box, `$someMut`, what
            // the box holds, `$someValue(o)` (ADR 0486).
            if let ExprKind::Call(callee, args) = &object.kind
                && callee.helper_name() == Some("$someMut")
                && let [
                    Expr {
                        kind: ExprKind::Handle(place),
                        ..
                    },
                ] = args.as_slice()
            {
                return Expr::call_helper("$someValue", vec![(**place).clone()]);
            }
            if let ExprKind::Object(props) = &object.kind
                && let [Prop::Field(name, value)] = props.as_slice()
                && name == HELD
            {
                return value.clone();
            }
        }
        Expr::new(ExprKind::Member(Box::new(object), property))
    }

    /// `object?.property`.
    pub fn optional_member(object: Expr, property: impl Into<String>) -> Expr {
        Expr::new(ExprKind::OptionalMember(Box::new(object), property.into()))
    }

    /// `String(x)` of a Rust number, `x` in a template (ADR 0066).
    pub fn stringed(value: Expr) -> Expr {
        Expr::new(ExprKind::Stringed(Box::new(value)))
    }

    pub fn optional_index(object: Expr, index: Expr) -> Expr {
        Expr::new(ExprKind::OptionalIndex(Box::new(object), Box::new(index)))
    }

    pub fn index(object: Expr, index: Expr) -> Expr {
        // `[a, b][1]` is `b`, where none is `...items`, and what's left
        // unread does nothing.
        if let (ExprKind::Array(items), Some(k)) = (&object.kind, index.as_int())
            && let Some(item) = usize::try_from(k).ok().and_then(|k| items.get(k))
            && !items.iter().any(|i| matches!(i.kind, ExprKind::Spread(_)))
            && items.iter().all(|i| std::ptr::eq(i, item) || !i.has_effects())
        {
            return item.clone();
        }
        Expr::new(ExprKind::Index(Box::new(object), Box::new(index)))
    }

    pub fn array(items: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::Array(items))
    }

    /// `...items`, as an array's item or a call's argument.
    pub fn spread(items: Expr) -> Expr {
        Expr::new(ExprKind::Spread(Box::new(items)))
    }

    pub fn object(props: Vec<Prop>) -> Expr {
        Expr::new(ExprKind::Object(props))
    }

    /// The test that holds where this one doesn't, read only for its truth:
    /// `!a` of `a`, `a` of `!a`, `a !== b` of `a === b`, and of a chain, its
    /// parts', `!a || b !== c` of `a && b === c`. The one negation of a test.
    pub fn negated_test(self) -> Expr {
        match self.kind {
            ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {
                let flipped = if op == Op::And { Op::Or } else { Op::And };
                Expr::bin(flipped, a.negated_test(), b.negated_test())
            }
            ExprKind::Unary(UnaryOp::Not, inner) => *inner,
            kind => Expr::unary(
                UnaryOp::Not,
                Expr {
                    reads: false,
                    kind,
                    span: self.span,
                },
            ),
        }
    }

    pub fn unary(op: UnaryOp, arg: Expr) -> Expr {
        // `!false` is `true`.
        if let (UnaryOp::Not, ExprKind::Bool(b)) = (op, &arg.kind) {
            return Expr::bool(!b);
        }
        // `!!!a` is `!a`, for every `a`: the inner two make a `bool` of it.
        if let (UnaryOp::Not, ExprKind::Unary(UnaryOp::Not, inner)) = (op, &arg.kind)
            && let ExprKind::Unary(UnaryOp::Not, value) = &inner.kind
        {
            return Expr::unary(UnaryOp::Not, (**value).clone());
        }
        // `!(a === b)` is `a !== b`, for every `a` and `b`.
        if let (UnaryOp::Not, ExprKind::Binary(eq @ (Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe), a, b)) =
            (op, &arg.kind)
        {
            let ne = match eq {
                Op::Eq => Op::Ne,
                Op::Ne => Op::Eq,
                Op::LooseEq => Op::LooseNe,
                _ => Op::LooseEq,
            };
            return Expr {
                reads: false,
                kind: ExprKind::Binary(ne, a.clone(), b.clone()),
                span: arg.span,
            };
        }
        Expr::new(ExprKind::Unary(op, Box::new(arg)))
    }

    pub fn bin(op: Op, lhs: Expr, rhs: Expr) -> Expr {
        // `0 < 0` is `false`: a comparison of two numbers written out is its
        // answer, as a person writes it.
        if let (ExprKind::Num(a), ExprKind::Num(b)) = (&lhs.kind, &rhs.kind) {
            let answer = match op {
                Op::Eq | Op::LooseEq => Some(a == b),
                Op::Ne | Op::LooseNe => Some(a != b),
                Op::Lt => Some(a < b),
                Op::Le => Some(a <= b),
                Op::Gt => Some(a > b),
                Op::Ge => Some(a >= b),
                _ => None,
            };
            if let Some(answer) = answer {
                return Expr::bool(answer);
            }
        }
        // And of two strings, a tag's test of a variant made here, `"Ok" ===
        // "Ok"`.
        if let (ExprKind::Str(a), ExprKind::Str(b)) = (&lhs.kind, &rhs.kind) {
            match op {
                Op::Eq | Op::LooseEq => return Expr::bool(a == b),
                Op::Ne | Op::LooseNe => return Expr::bool(a != b),
                _ => {}
            }
        }
        // `0 || x` is `x`: `||` gives what's after a falsy number, an
        // ordering's `Equal` before the next part's. And of `true` or
        // `false` first, `&&` and `||` are what JS gives, never running
        // what's after where it wouldn't.
        if op == Op::Or && matches!(lhs.kind, ExprKind::Num(n) if n == 0.0) {
            return rhs;
        }
        if let (Op::And | Op::Or, ExprKind::Bool(first)) = (op, &lhs.kind) {
            return match (op, *first) {
                (Op::And, true) | (Op::Or, false) => rhs,
                _ => lhs,
            };
        }
        // `a != null && !!a` is `!!a`: neither is truthy.
        if op == Op::And
            && let Some(tested) = null_test(&lhs, true)
            && let ExprKind::Unary(UnaryOp::Not, inner) = &rhs.kind
            && let ExprKind::Unary(UnaryOp::Not, of) = &inner.kind
            && same_path(tested, of)
        {
            return rhs;
        }
        // `a != null && a === "index"` is `a === "index"`: no literal is
        // `null` or `undefined`.
        if op == Op::And
            && let Some(tested) = null_test(&lhs, true)
            && let ExprKind::Binary(Op::Eq, a, b) = &rhs.kind
            && [(a, b), (b, a)].iter().any(|(x, literal)| {
                same_path(tested, x) && matches!(literal.kind, ExprKind::Str(_) | ExprKind::Bool(_) | ExprKind::Num(_))
            })
        {
            return rhs;
        }
        // `a != null && Array.isArray(a)` is `Array.isArray(a)`: no array is
        // `null` or `undefined`; and of `Array.isArray(a) && ..`, that.
        let mut first = &rhs;
        while let ExprKind::Binary(Op::And, a, _) = &first.kind {
            first = a;
        }
        if op == Op::And
            && let Some(tested) = presence_of(&lhs)
            && let ExprKind::Call(callee, args) = &first.kind
            && let (ExprKind::Member(array, is_array), [of]) = (&callee.kind, args.as_slice())
            && matches!(&array.kind, ExprKind::Var(name) if name == "Array")
            && is_array == "isArray"
            && same_path(tested, of)
        {
            return rhs;
        }
        // `x + "" + "px"` is `x + "px"`: either makes text of `x` as `+`
        // does, then joins.
        if op == Op::Add
            && matches!(rhs.kind, ExprKind::Str(_) | ExprKind::Template(..))
            && let ExprKind::Binary(Op::Add, x, empty) = &lhs.kind
            && matches!(&empty.kind, ExprKind::Str(s) if s.is_empty())
        {
            return Expr::bin(Op::Add, (**x).clone(), rhs);
        }
        // `"#" + (x + "")` is `"#" + x`: text joined to `x` makes text of it
        // as `+ ""` does.
        if op == Op::Add
            && matches!(lhs.kind, ExprKind::Str(_) | ExprKind::Template(..))
            && let ExprKind::Binary(Op::Add, x, empty) = &rhs.kind
            && matches!(&empty.kind, ExprKind::Str(s) if s.is_empty())
        {
            return Expr::bin(Op::Add, lhs, (**x).clone());
        }
        Expr::new(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)))
    }

    pub fn cond(test: Expr, then: Expr, els: Expr) -> Expr {
        // `true ? a : b` is `a`: the branch that runs.
        if let ExprKind::Bool(b) = test.kind {
            return if b { then } else { els };
        }
        // `a != null ? a.b : undefined` is `a?.b`, as a person writes it. One
        // property only: `a?.b.c` would end the chain at `.c` too. Not of
        // `!!a`, which an empty string fails, `!s.is_empty()`'s: an
        // `Option` never falsy's is lowered as one of `a != null` (ADR 0298).
        if let Some(tested) = null_test(&test, true)
            && matches!(els.kind, ExprKind::Undefined)
            && let ExprKind::Member(object, property) = &then.kind
            && same_path(tested, object)
        {
            let chain = Expr::optional_member((**object).clone(), property.clone());
            return Expr {
                span: then.span,
                ..chain
            };
        }
        // `a != null ? a.m(x) : undefined` is `a?.m(x)`: `?.` skips the call,
        // its arguments too, where `a` is, as the test does.
        if let Some(tested) = null_test(&test, true)
            && matches!(els.kind, ExprKind::Undefined)
            && let ExprKind::Call(callee, args) = &then.kind
            && let ExprKind::Member(object, property) = &callee.kind
            && same_path(tested, object)
        {
            let chain = Expr::call(
                Expr::optional_member((**object).clone(), property.clone()),
                args.clone(),
            );
            return Expr {
                span: then.span,
                ..chain
            };
        }
        Expr::new(ExprKind::Cond(Box::new(test), Box::new(then), Box::new(els)))
    }

    pub fn new_(callee: Expr, args: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::New(Box::new(callee), args))
    }

    pub fn arrow(params: Vec<Pattern>, body: Vec<Stmt>) -> Expr {
        Expr::new(ExprKind::Arrow(params, body))
    }

    /// This function, where it's called with one argument alone: of `(x)
    /// => f(x)`, by what doesn't read `x`, the function called, `f`. Not
    /// where it's given more, as a JS array's `map` gives an index too.
    pub fn called_alone(self) -> Expr {
        if let ExprKind::Arrow(params, body) = &self.kind
            && let [Pattern::Name(param)] = params.as_slice()
            && let [
                Stmt {
                    kind: StmtKind::Return(Some(value)),
                    ..
                },
            ] = body.as_slice()
            && let ExprKind::Call(callee, args) = &value.kind
            && let [
                Expr {
                    kind: ExprKind::Var(arg),
                    ..
                },
            ] = args.as_slice()
            && arg == param
        {
            let mut reads = false;
            callee.visit_vars(&mut |var| reads |= var == param);
            if !reads {
                return (**callee).clone();
            }
        }
        self
    }

    pub fn async_arrow(params: Vec<Pattern>, body: Vec<Stmt>) -> Expr {
        Expr::new(ExprKind::AsyncArrow(params, body))
    }

    pub fn await_(promise: Expr) -> Expr {
        Expr::new(ExprKind::Await(Box::new(promise)))
    }

    pub fn jsx(jsx: Jsx) -> Expr {
        Expr::new(ExprKind::Jsx(Box::new(jsx)))
    }

    pub fn call(callee: Expr, args: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::Call(Box::new(callee), args))
    }

    /// Give this node a span, unless it already has one. Lowering calls
    /// this on every result, so the outermost node made for a Rust
    /// expression gets that expression's span.
    pub fn or_at(mut self, span: Span) -> Expr {
        if self.span.is_none() {
            self.span = span;
        }
        self
    }

    /// The integer value, if this is an integer literal.
    pub fn as_int(&self) -> Option<i128> {
        match self.kind {
            ExprKind::Num(n) if n.fract() == 0.0 => Some(n as i128),
            _ => None,
        }
    }

    pub fn as_bigint(&self) -> Option<i128> {
        match self.kind {
            ExprKind::BigInt(n) => Some(n),
            _ => None,
        }
    }

    /// Literals can be evaluated at any time, so they never need a temporary.
    pub fn is_constant(&self) -> bool {
        matches!(
            self.kind,
            ExprKind::Num(_)
                | ExprKind::BigInt(_)
                | ExprKind::BigUint(_)
                | ExprKind::Bool(_)
                | ExprKind::Str(_)
                | ExprKind::Lines(_)
                | ExprKind::Undefined
                | ExprKind::GivenUndefined
                | ExprKind::Null
        )
    }

    /// A function, an object, an array or a string made here: never `null`
    /// nor `undefined`, as an `Option`'s `Some` of it.
    pub fn is_made_here(&self) -> bool {
        matches!(
            self.kind,
            ExprKind::Arrow(..)
                | ExprKind::AsyncArrow(..)
                | ExprKind::Object(_)
                | ExprKind::Array(_)
                | ExprKind::Template(..)
        )
    }

    /// A constant, or an object made here of them, which nothing made
    /// before it or after can change.
    pub fn is_made_of_constants(&self) -> bool {
        match &self.kind {
            ExprKind::Object(props) => {
                (props.iter()).all(|prop| matches!(prop, Prop::Field(_, value) if value.is_made_of_constants()))
            }
            _ => self.is_constant(),
        }
    }
}
