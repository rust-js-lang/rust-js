//! Expressions made: each kind's constructor, folding what's known as it's
//! made.

use super::*;

impl Expr {
    pub(super) fn new(kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: Span::NONE,
            facts: Facts::NONE,
        }
    }

    /// `this`, a class's own (ADR 0761).
    pub fn this() -> Expr {
        Expr::new(ExprKind::This)
    }

    /// `of.method(args)`, of a `str` or a slice `of` as Rust's types tell.
    pub fn read(of: Expr, method: &str, args: Vec<Expr>) -> Expr {
        Expr {
            facts: Facts::of(Known::Read),
            ..Expr::call(Expr::member(of, method), args)
        }
    }

    /// This, which lowering knows is never `undefined`, a `Some`'s value
    /// (`Known::Defined`); one it knows otherwise of, a getter's read say,
    /// as it is.
    pub fn defined(self) -> Expr {
        match self.facts.is_empty() {
            true => Expr {
                facts: Facts::of(Known::Defined),
                ..self
            },
            false => self,
        }
    }

    /// Whether `option`, an `Option` of a value never falsy, is `set`:
    /// `!!option`, or `!option` where it's not, by its truth (ADR 0298,
    /// amended), which is `option != null`'s, or `== null`'s, answer, as
    /// `null_test` reads it (`Known::Presence`).
    pub fn presence(option: Expr, set: bool) -> Expr {
        let not = Expr::unary(UnaryOp::Not, option);
        let test = if set { Expr::unary(UnaryOp::Not, not) } else { not };
        // Of the option, not a constant's `true` it folded to.
        let marked = Expr {
            facts: Facts::of(Known::Presence),
            ..test.clone()
        };
        match null_test(&marked, set) {
            Some(_) => marked,
            None => test,
        }
    }

    /// This, the property a binding's write assigns, which a setter may
    /// answer (`Known::Setter`).
    pub fn answered_by_a_setter(self) -> Expr {
        Expr {
            facts: Facts::of(Known::Setter),
            ..self
        }
    }

    /// This, a binding's read of a property, which a getter may answer
    /// (`Known::Getter`).
    pub fn answered_by_a_getter(mut self) -> Expr {
        self.facts = self.facts.with(Known::Getter);
        self
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

    /// A string as its source writes it, its pieces in order: one piece on
    /// one line is a string literal, any other a `Written` (ADR 0641).
    pub fn written(mut pieces: Vec<StrPiece>) -> Expr {
        match pieces.as_slice() {
            [] => Expr::str(""),
            [piece] if !piece.lines => Expr::str(pieces.pop().expect("a piece").text),
            _ => Expr::new(ExprKind::Written(pieces)),
        }
    }

    /// A regular expression literal, as written: `/^[0-9]$/`.
    pub fn regex(literal: &str) -> Expr {
        Expr::new(ExprKind::Regex(literal.to_string()))
    }

    pub fn template(texts: Vec<String>, values: Vec<Expr>) -> Expr {
        Expr::template_across(texts, values, Vec::new())
    }

    /// `template`, of values `across` says, in turn, are written across
    /// lines in its source, or not (`TemplateLayout::across`).
    pub fn template_across(texts: Vec<String>, values: Vec<Expr>, across: Vec<bool>) -> Expr {
        debug_assert_eq!(texts.len(), values.len() + 1);
        // `${false}` is `false`, and `${3}` is `3`: a literal the template
        // would show is written in its text, as JS shows it.
        let mut texts = texts.into_iter();
        let mut folded = vec![texts.next().expect("a text before each value")];
        let (mut kept, mut kept_across) = (Vec::new(), Vec::new());
        let across = across.into_iter().chain(std::iter::repeat(false));
        for ((value, text), across) in values.into_iter().zip(texts).zip(across) {
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
                    kept_across.push(across);
                    folded.push(text);
                }
            }
        }
        // All of it written so, it's a string.
        if kept.is_empty() {
            return Expr::str(folded.pop().expect("a text"));
        }
        let layout = TemplateLayout {
            across: Across::of(kept_across),
            ..TemplateLayout::default()
        };
        Expr::new(ExprKind::Template(folded, kept, layout))
    }

    /// A template written across `lines`: its line breaks written as they are.
    pub fn with_lines(self, lines: bool) -> Expr {
        match self.kind {
            ExprKind::Template(texts, values, layout) => Expr::new(ExprKind::Template(
                texts,
                values,
                TemplateLayout {
                    lines,
                    breaks: Vec::new(),
                    across: layout.across,
                },
            )),
            _ => self,
        }
    }

    /// A template whose format string is written as `pieces`, `concat!`'s,
    /// each its texts and values, as `template` takes them, and whether
    /// it's written across lines: one template, broken where they are
    /// (ADR 0641). Of no values, the pieces are a string's. Each piece's
    /// values are `across` lines, in turn, or not (`template_across`).
    pub fn template_pieces(pieces: Vec<TemplatePiece>) -> Expr {
        let (mut texts, mut values) = (vec![String::new()], Vec::new());
        let mut layout = TemplateLayout::default();
        let mut strings = Vec::new();
        let mut across = Vec::new();
        for (i, (piece_texts, piece_values, piece_across, lines)) in pieces.into_iter().enumerate() {
            let (piece_texts, piece_values) = match Expr::template_across(piece_texts, piece_values, piece_across).kind
            {
                ExprKind::Str(text) => (vec![text], Vec::new()),
                ExprKind::Template(piece_texts, piece_values, piece) => {
                    across.extend((0..piece_values.len()).map(|i| piece.across.at(i)));
                    (piece_texts, piece_values)
                }
                _ => unreachable!("a template is a string or a template"),
            };
            let last = texts.last_mut().expect("a text");
            match i {
                0 => layout.lines = lines,
                _ => layout.breaks.push(TemplateBreak {
                    text: values.len(),
                    at: last.len(),
                    lines,
                }),
            }
            strings.push(StrPiece {
                text: piece_texts.concat(),
                lines,
                span: Span::NONE,
            });
            let mut piece_texts = piece_texts.into_iter();
            last.push_str(&piece_texts.next().expect("a text"));
            values.extend(piece_values);
            texts.extend(piece_texts);
        }
        layout.across = Across::of(across);
        match values.is_empty() {
            true => Expr::written(strings),
            false => Expr::new(ExprKind::Template(texts, values, layout)),
        }
    }

    /// A template's value that's a template itself, a function's body put
    /// in place of its call, `${`(${a}, ${b})`}`, or a value's text in one,
    /// `${`${x}`}`: its text and values in the outer's, `(${a}, ${b})`, as
    /// a template makes each value's text the same either way. Of an inner
    /// of one piece on one line, and not of one with text into a piece
    /// written across lines, whose line breaks are as they are where the
    /// inner's are `\n`; one of values alone has none (ADR 0066).
    pub fn splice_templates(&mut self) {
        let ExprKind::Template(texts, values, layout) = &mut self.kind else {
            return;
        };
        let outer_lines = layout.lines || layout.breaks.iter().any(|at| at.lines);
        let splices = |v: &Expr| {
            matches!(&v.kind, ExprKind::Template(inner_texts, _, inner)
                if inner.is_plain() && (!outer_lines || inner_texts.iter().all(String::is_empty)))
        };
        if !values.iter().any(splices) {
            return;
        }
        // Where each of the outer's texts is now, its index and how far into
        // it: so `concat!`'s pieces break where they did (ADR 0641).
        let mut moved = vec![(0, 0)];
        let (mut spliced_texts, mut spliced_values) = (vec![texts[0].clone()], Vec::new());
        // Each value's being written across lines, an inner's its own.
        let mut spliced_across = Vec::new();
        let outer_across = std::mem::take(&mut layout.across);
        for (i, (value, after)) in std::mem::take(values).into_iter().zip(texts.iter().skip(1)).enumerate() {
            let spliced = splices(&value);
            match value.kind {
                ExprKind::Template(inner_texts, inner_values, inner_layout) if spliced => {
                    let mut inner = inner_texts.into_iter();
                    spliced_texts
                        .last_mut()
                        .expect("a text")
                        .push_str(&inner.next().unwrap_or_default());
                    for (j, (inner_value, inner_text)) in inner_values.into_iter().zip(inner).enumerate() {
                        spliced_values.push(inner_value);
                        spliced_across.push(inner_layout.across.at(j));
                        spliced_texts.push(inner_text);
                    }
                }
                _ => {
                    spliced_values.push(value);
                    spliced_across.push(outer_across.at(i));
                    spliced_texts.push(String::new());
                }
            }
            let last = spliced_texts.last_mut().expect("a text");
            moved.push((spliced_values.len(), last.len()));
            last.push_str(after);
        }
        for at in &mut layout.breaks {
            let (text, shift) = moved[at.text];
            (at.text, at.at) = (text, shift + at.at);
        }
        *texts = spliced_texts;
        *values = spliced_values;
        layout.across = Across::of(spliced_across);
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

    /// `$try(() => call)`: a JS call whose throw is an `Err`, its `Result`
    /// (ADR 0035).
    pub fn tried(call: Expr) -> Expr {
        let span = call.span;
        let thunk = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(call)).at(span)]);
        Expr::call_helper("$try", vec![thunk])
    }

    /// The call `$try(() => call)` tries, of one [`Expr::tried`] made.
    pub fn tried_call(&self) -> Option<&Expr> {
        let ExprKind::Call(callee, args) = &self.kind else {
            return None;
        };
        let (Some("$try"), [thunk]) = (callee.helper_name(), args.as_slice()) else {
            return None;
        };
        match &thunk.kind {
            ExprKind::Arrow(params, body) if params.is_empty() => match body.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Return(Some(call)),
                        ..
                    },
                ] => Some(call),
                _ => None,
            },
            _ => None,
        }
    }

    /// `new Error(message)`: a panic, of Rust's or a port's (ADR 0012), the
    /// one way rust-js makes one; `new Error()` of an empty message,
    /// `panic!("")`'s, as `message` is `""` either way (ADR 0706).
    pub fn panic(message: Expr) -> Expr {
        let args = match &message.kind {
            ExprKind::Str(text) if text.is_empty() => Vec::new(),
            _ => vec![message],
        };
        Expr::new_(Expr::var("Error"), args)
    }

    /// Of `new Error(..)`, a panic: what it's made of.
    pub fn panic_message(&self) -> Option<&[Expr]> {
        match &self.kind {
            ExprKind::New(class, args) if matches!(&class.kind, ExprKind::Var(name) if name == "Error") => Some(args),
            _ => None,
        }
    }

    /// What was thrown of an unwind's payload `pair`, `pair.error`, which
    /// its downcast to a `&JsError` finds (ADR 0710).
    pub fn thrown_error(pair: Expr) -> Expr {
        Expr::member(pair, THROWN_VALUE)
    }

    /// Of `Expr::thrown_error`'s, the payload.
    pub fn thrown_error_of(&self) -> Option<&Expr> {
        match &self.kind {
            ExprKind::Member(pair, field) if field == THROWN_VALUE => Some(pair),
            _ => None,
        }
    }

    /// `$catchUnwind(f)`: `catch_unwind(f)`'s `Result`, `f()`'s value, or
    /// what unwound, `$unwound(e)` of what was thrown (ADR 0660).
    pub fn caught_unwind(f: Expr) -> Expr {
        Expr::call_helper("$catchUnwind", vec![f])
    }

    /// What a `try` of this `Result` holds, and whether its `catch` is given
    /// what was thrown as it is, `$try(() => call)`'s (ADR 0637), or what
    /// unwound, `$unwound(e)`, `$catchUnwind(f)`'s, of `f()`: an arrow's
    /// body, `() => g(x)`'s `g(x)` (ADR 0660).
    pub fn caught(&self) -> Option<(Expr, Caught)> {
        if let Some(call) = self.tried_call() {
            return Some((call.clone(), Caught::Thrown));
        }
        let ExprKind::Call(callee, args) = &self.kind else {
            return None;
        };
        let (Some("$catchUnwind"), [f]) = (callee.helper_name(), args.as_slice()) else {
            return None;
        };
        // What makes `f` runs before it's caught, as Rust makes it before
        // `catch_unwind` is called: never in the `try`.
        if f.may_throw() {
            return None;
        }
        let run = match &f.kind {
            ExprKind::Arrow(params, body) if params.is_empty() => match body.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Return(Some(value)),
                        ..
                    },
                ] => value.clone(),
                _ => Expr::call(f.clone(), Vec::new()),
            },
            _ => Expr::call(f.clone(), Vec::new()),
        };
        Some((run, Caught::Unwound))
    }

    /// What a throwing JS call's `Result` is of, where nothing catches its
    /// throw: the call, of `$try(() => call)`, and `await p`, of `await
    /// $settle(p)` (ADRs 0035, 0643). Its throw, or its rejection, goes on
    /// as JS made it.
    pub fn uncaught(&self) -> Option<Expr> {
        if let Some(call) = self.tried_call() {
            return Some(call.clone());
        }
        let ExprKind::Await(settled) = &self.kind else {
            return None;
        };
        let ExprKind::Call(callee, args) = &settled.kind else {
            return None;
        };
        match (callee.helper_name(), args.as_slice()) {
            (Some("$settle"), [promise]) => Some(Expr {
                span: self.span,
                ..Expr::await_(promise.clone())
            }),
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
        Expr::stored_handle(place, Storage::AsGiven)
    }

    /// A handle on `place`, which stores what's written as `storage` says.
    pub fn stored_handle(place: Expr, storage: Storage) -> Expr {
        Expr::new(ExprKind::Handle(Box::new(place), storage))
    }

    pub fn pair(place: Expr, dictionary: Expr) -> Expr {
        Expr::new(ExprKind::Pair(Box::new(place), Box::new(dictionary)))
    }

    /// `(name = value)`, its value the assignment's (ADR 0860).
    pub fn assigned(name: &str, value: Expr) -> Expr {
        Expr::new(ExprKind::Assigned(name.to_string(), Box::new(value)))
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
        if let ExprKind::Object(props, ..) = &object.kind
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
            // Read or written as the place: a write's storage is its
            // writer's to keep, `cell_call`'s (ADR 0772).
            if let ExprKind::Handle(place, _) = object.kind {
                return *place;
            }
            // And of a handle through an `Option`'s box, `$someMut`, what
            // the box holds, `$someValue(o)` (ADR 0486).
            if let ExprKind::Call(callee, args) = &object.kind
                && callee.helper_name() == Some("$someMut")
                && let [
                    Expr {
                        kind: ExprKind::Handle(place, _),
                        ..
                    },
                ] = args.as_slice()
            {
                return Expr::call_helper("$someValue", vec![(**place).clone()]);
            }
            if let ExprKind::Object(props, ..) = &object.kind
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
        Expr::new(ExprKind::Object(props, ObjectLayout::default()))
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
                    facts: self.facts,
                    kind,
                    span: self.span,
                },
            ),
        }
    }

    pub fn unary(op: UnaryOp, arg: Expr) -> Expr {
        // `!false` is `true`, and so is `!null`, of any falsy constant: an
        // `Option`'s `!o` of `None` (ADR 0298).
        if let (UnaryOp::Not, ExprKind::Bool(b)) = (op, &arg.kind) {
            return Expr::bool(!b);
        }
        if op == UnaryOp::Not && arg.is_falsy_constant() {
            return Expr::bool(true);
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
                facts: Facts::NONE,
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
        // And of `undefined` or `null` both, `None == null`, a test of an
        // `Option` written out: `==` holds of them, `!=` doesn't.
        let nothing = |e: &Expr| matches!(e.kind, ExprKind::Undefined | ExprKind::GivenUndefined | ExprKind::Null);
        if nothing(&lhs) && nothing(&rhs) {
            match op {
                Op::LooseEq => return Expr::bool(true),
                Op::LooseNe => return Expr::bool(false),
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
        // And of `true` or `false` after a comparison, which is a `bool`:
        // `a != null && true` is the comparison; `a != null && false` is
        // `false`, where the comparison does nothing.
        let compared = matches!(
            lhs.kind,
            ExprKind::Binary(
                Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe | Op::Lt | Op::Le | Op::Gt | Op::Ge,
                ..
            ) | ExprKind::Unary(UnaryOp::Not, _)
        );
        if let (Op::And | Op::Or, ExprKind::Bool(second)) = (op, &rhs.kind)
            && compared
        {
            match (op, *second) {
                (Op::And, true) | (Op::Or, false) => return lhs,
                _ if !lhs.has_effects() => return rhs,
                _ => {}
            }
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
        // `a != null && Array.isArray(a)` is `Array.isArray(a)`: no array is
        // `null` or `undefined`. A chain after it, `&& ..`, is made from the
        // left, this first.
        if op == Op::And
            && let Some(tested) = null_test(&lhs, true)
            && let ExprKind::Call(callee, args) = &rhs.kind
            && let (ExprKind::Member(array, is_array), [of]) = (&callee.kind, args.as_slice())
            && matches!(&array.kind, ExprKind::Var(name) if name == "Array")
            && is_array == "isArray"
            && same_path(tested, of)
        {
            return rhs;
        }
        // `"" + x + "px"` is `x + "px"`: either makes text of `x` as `+`
        // does, then joins.
        if op == Op::Add
            && matches!(rhs.kind, ExprKind::Str(_) | ExprKind::Template(..))
            && let ExprKind::Binary(Op::Add, empty, x) = &lhs.kind
            && matches!(&empty.kind, ExprKind::Str(s) if s.is_empty())
        {
            return Expr::bin(Op::Add, (**x).clone(), rhs);
        }
        // `"#" + ("" + x)` is `"#" + x`: text joined to `x` makes text of it
        // as `"" +` does.
        if op == Op::Add
            && matches!(lhs.kind, ExprKind::Str(_) | ExprKind::Template(..))
            && let ExprKind::Binary(Op::Add, empty, x) = &rhs.kind
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
        // any `!!a`, which an empty string fails, `!s.is_empty()`'s; of an
        // `Option` never falsy's, `presence`'s, which is `a != null`
        // (`null_test`, ADR 0298, amended).
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
        // `a != null ? a[k] : undefined` is `a?.[k]`, of a key too, a
        // symbol's: `?.` skips the key where `a` is, as the test does (ADR
        // 0647).
        if let Some(tested) = null_test(&test, true)
            && matches!(els.kind, ExprKind::Undefined)
            && let ExprKind::Index(object, key) = &then.kind
            && same_path(tested, object)
        {
            let chain = Expr::optional_index((**object).clone(), (**key).clone());
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
                | ExprKind::Written(_)
                | ExprKind::Undefined
                | ExprKind::GivenUndefined
                | ExprKind::Null
        )
    }

    /// It, a `Rev`, an `Enumerate` or a `Map` made in place, so marked
    /// (`Known::InPlace`), beside what else is known of it.
    pub fn in_place(mut self) -> Expr {
        self.facts = self.facts.with(Known::InPlace);
        self
    }

    /// This, a binding's method, which may do anything (`Known::Binding`),
    /// beside what else is known of it.
    pub fn binding(mut self) -> Expr {
        self.facts = self.facts.with(Known::Binding);
        self
    }

    /// This, a dictionary the function was given, whose methods are the
    /// program's (`Known::Evidence`), beside what else is known of it.
    pub fn evidence(mut self) -> Expr {
        self.facts = self.facts.with(Known::Evidence);
        self
    }

    /// This, a reference to one of the program's functions
    /// (`Known::Item`).
    pub fn item(self) -> Expr {
        Expr {
            facts: Facts::of(Known::Item),
            ..self
        }
    }

    /// Whether this is the same value wherever and whenever it's read,
    /// whatever runs between: a constant, or one of the program's
    /// functions (`Known::Item`), which nothing assigns.
    pub fn is_fixed(&self) -> bool {
        self.is_constant() || self.facts.has(Known::Item)
    }

    /// Whether this may be `undefined`, as its form tells. A constant but
    /// `undefined`, `null` among them, and what's made here, an object, an
    /// array, a function, a `new`, a template, a regex or JSX, never is; nor
    /// what an operator gives but `&&`, `||` and `??`, which may give an
    /// operand that may be; nor what lowering knows is `Known::Defined`,
    /// nor one of the program's functions (`Known::Item`).
    /// Anything else, a variable or a call, may be. The one answer of
    /// whether a JS value may be `undefined`, which a nullable place's
    /// storage asks (`Storage::written`).
    pub fn may_be_undefined(&self) -> bool {
        match &self.kind {
            _ if self.facts.has(Known::Defined) || self.facts.has(Known::Item) => false,
            ExprKind::Undefined | ExprKind::GivenUndefined => true,
            _ if self.is_constant() => false,
            ExprKind::Array(_)
            | ExprKind::Object(..)
            | ExprKind::Arrow(..)
            | ExprKind::AsyncArrow(..)
            | ExprKind::Function(_)
            | ExprKind::New(..)
            | ExprKind::Template(..)
            | ExprKind::Regex(_)
            | ExprKind::Jsx(_)
            | ExprKind::Unary(..) => false,
            // `a ?? b` and `a || b` give `b` or what's not `undefined`; `a &&
            // b`, either.
            ExprKind::Binary(Op::Coalesce | Op::Or, _, b) => b.may_be_undefined(),
            ExprKind::Binary(Op::And, a, b) => a.may_be_undefined() || b.may_be_undefined(),
            ExprKind::Binary(..) => false,
            ExprKind::Cond(_, a, b) => a.may_be_undefined() || b.may_be_undefined(),
            _ => true,
        }
    }

    /// This again, where what it stores is known better than where it was
    /// made: a nullable place's `made ?? null` (`Storage::written`) of what's
    /// now never `undefined` (`may_be_undefined`), a `Drop` guard's field
    /// read where it's dropped as the variable it was made of, is `made`, as
    /// `Storage::written` would have stored it (ADR 0841, amended). Anything
    /// else is as it is.
    pub fn store_again(&mut self) {
        if let ExprKind::Binary(Op::Coalesce, made, null) = &mut self.kind
            && matches!(null.kind, ExprKind::Null)
            && !made.may_be_undefined()
        {
            let made = std::mem::replace(made.as_mut(), Expr::null());
            let span = self.span;
            *self = made.or_at(span);
        }
    }

    /// A constant JS tests as false: `false`, `0`, `""`, `undefined` or
    /// `null`.
    pub fn is_falsy_constant(&self) -> bool {
        match &self.kind {
            ExprKind::Bool(b) => !b,
            ExprKind::Num(n) => *n == 0.0 || n.is_nan(),
            ExprKind::BigInt(n) => *n == 0,
            ExprKind::BigUint(n) => *n == 0,
            ExprKind::Str(s) => s.is_empty(),
            ExprKind::Undefined | ExprKind::GivenUndefined | ExprKind::Null => true,
            _ => false,
        }
    }

    /// A constant, or an object made here of them, which nothing made
    /// before it or after can change.
    pub fn is_made_of_constants(&self) -> bool {
        match &self.kind {
            ExprKind::Object(props, ..) => {
                (props.iter()).all(|prop| matches!(prop, Prop::Field(_, value) if value.is_made_of_constants()))
            }
            _ => self.is_constant(),
        }
    }
}

impl Storage {
    /// What a place stored so holds, of `made`, an `Option`'s JS where it's
    /// `NullForNone`: a `None` is `null`; what's `nulled`, `null` or a
    /// value already, as MIR tells, and what's never `undefined`
    /// (`may_be_undefined`), a constant, an object made here or a `Some`'s
    /// value, as it is; a test's branches each so, `c ? f(c) : null` of `c ?
    /// f(c) : undefined`; else `made ?? null` (ADRs 0275, 0772, 0791). The
    /// one answer, where a struct is made, a `Cell` set, a nullable
    /// parameter given, and a handle's setter printed.
    pub fn written(self, made: Expr, nulled: bool) -> Expr {
        match self {
            Storage::AsGiven => made,
            Storage::NullForNone if nulled && !matches!(made.kind, ExprKind::Undefined | ExprKind::GivenUndefined) => {
                made
            }
            Storage::NullForNone => match null_for_none(&made) {
                Some(stored) => stored,
                None => Expr::bin(Op::Coalesce, made, Expr::null()),
            },
        }
    }
}

/// `made` stored with its `None` `null`, where that's what it is, or each
/// branch of its test is: `undefined` is `null`, and what's never
/// `undefined` is itself; `None` where it may be `undefined` otherwise.
fn null_for_none(made: &Expr) -> Option<Expr> {
    match &made.kind {
        ExprKind::Undefined | ExprKind::GivenUndefined => Some(Expr::null().or_at(made.span)),
        _ if !made.may_be_undefined() => Some(made.clone()),
        ExprKind::Cond(test, yes, no) => Some(Expr {
            facts: made.facts,
            kind: ExprKind::Cond(
                test.clone(),
                Box::new(null_for_none(yes)?),
                Box::new(null_for_none(no)?),
            ),
            span: made.span,
        }),
        _ => None,
    }
}
