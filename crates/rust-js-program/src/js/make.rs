//! Expressions made: each kind's constructor, folding what's known as it's
//! made.

use super::storage::null_for_none;
use super::*;

mod operators;
mod unwinding;
pub use unwinding::Payload;

impl Expr {
    pub(super) fn new(kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: Span::NONE,
            facts: Facts::NONE,
            held_as: Default::default(),
        }
    }

    /// `this`, a class's own (ADR 0761).
    pub fn this() -> Expr {
        Expr::new(ExprKind::This)
    }

    /// `super`, a class's that extends one, called `super(props)` (rd331).
    pub fn super_() -> Expr {
        Expr::new(ExprKind::Super)
    }

    /// `of.method(args)`, of a `str` or a slice `of` as Rust's types tell.
    pub fn read(of: Expr, method: &str, args: Vec<Expr>) -> Expr {
        Expr {
            facts: Facts::of(Known::Read),
            ..Expr::call(Expr::member(of, method), args)
        }
    }

    /// `of.method(args)`, a call of one of JS's own methods, of text, an
    /// array, a number or a global, `parts.join(", ")`, `Math.max(a, b)`,
    /// where lowering knows `of` is one (`Known::BuiltIn`, #1674).
    pub fn builtin(of: Expr, method: &str, args: Vec<Expr>) -> Expr {
        Expr::call(Expr::member(of, method), args).built_in()
    }

    /// `name`, one of JS's own globals, `Math` or `console`, which no call
    /// changes (`Known::BuiltIn`, #1674).
    pub fn js_global(name: &str) -> Expr {
        Expr::global(name).built_in()
    }

    /// A read of the global `name`, JS's `Array` or a binding's `document`,
    /// which the module declares nowhere (`Name::global`). One of JS's own,
    /// which no call changes, is `js_global`'s.
    pub fn global(name: &str) -> Expr {
        Expr::var(Name::global(name))
    }

    /// A read of the module's `name`, one it declares at its top: its
    /// function's, its constant's or its import's (`Name::module`).
    pub fn module(name: &str) -> Expr {
        Expr::var(Name::module(name))
    }

    /// `Number(n)` or `BigInt(n)` of `n`, a Rust number or `bool`, never
    /// an object whose `valueOf` it would call: it runs nothing and throws
    /// nothing (`ExprKind::Converted`, #1468).
    pub fn converted(conversion: Conversion, n: Expr) -> Expr {
        Expr::new(ExprKind::Converted(conversion, Box::new(n)))
    }

    /// This, a call of one of JS's own functions or methods
    /// (`Known::BuiltIn`), `String(n)`, beside what else is known of it.
    pub fn built_in(mut self) -> Expr {
        self.facts = self.facts.with(Known::BuiltIn);
        self
    }

    /// This, `js::shown`'s value of text, a number or a `bool`, which a
    /// template shows running nothing (`Known::Plain`, #1676).
    pub fn plain(mut self) -> Expr {
        self.facts = self.facts.with(Known::Plain);
        self
    }

    /// A null explicitly supplied by a fallback stays an argument.
    pub fn has_null_fallback(&self) -> bool {
        matches!(&self.kind, ExprKind::Binary(Op::Coalesce, _, fallback) if matches!(fallback.kind, ExprKind::Null))
    }

    /// Whether `option`, an `Option`, is `set`: `option != null`, or `==
    /// null` where it's not, whatever it holds (ADR 0298, amended by the
    /// user, 2026-10-28: one form, never its truth). The one test of an
    /// `Option`'s being there, which `null_test` reads. Of one whose `None`
    /// MIR says is only `undefined` or only `null` (`given`, `place_none`),
    /// the test says so, `option !== undefined` or
    /// `option !== null`, its own operator (`Op::NoneNe`, #2112 C S5; ADR
    /// 0298, amended by the user, 2026-10-28: by what its `None` can be).
    /// `Some(value)` where `test` holds, else `None`, `test ? value :
    /// undefined`, of a `value` that's there: a `SomeWhere`, where `Expr::cond`
    /// makes a test of it, else what it makes, an optional chain.
    pub fn some_where(test: Expr, value: Expr) -> Expr {
        match Expr::cond(test, value, Expr::js_undefined()) {
            Expr {
                kind: ExprKind::Cond(test, value, _),
                span,
                ..
            } => Expr {
                facts: Facts::NONE,
                kind: ExprKind::SomeWhere(test, value),
                span,
                held_as: Default::default(),
            },
            chain => chain,
        }
    }

    /// Of `Some(value)` where `test` holds, else `None`, its test, its
    /// value and whether that's there, a `SomeWhere`'s; of a test made
    /// otherwise, `test ? value : undefined`, the same, its value not known
    /// there. The one matcher of the form, which passes that rewrite it ask.
    pub fn some_where_parts(&self) -> Option<(&Expr, &Expr, bool)> {
        match &self.kind {
            ExprKind::SomeWhere(test, value) => Some((test, value, true)),
            ExprKind::Cond(test, value, none) if matches!(none.kind, ExprKind::Undefined) => Some((test, value, false)),
            _ => None,
        }
    }

    /// Whether this is a conditional, `a ? b : c`, as it's printed: a test
    /// made of three, or a `SomeWhere`, `a ? b : undefined`.
    pub fn is_conditional(&self) -> bool {
        self.conditional_parts().is_some()
    }

    /// Of a conditional as it's printed, `a ? b : c`, its three parts: a
    /// `SomeWhere`'s `c` the `undefined` it prints.
    pub fn conditional_parts(&self) -> Option<(&Expr, &Expr, &Expr)> {
        thread_local! {
            static UNDEFINED: &'static Expr = Box::leak(Box::new(Expr::js_undefined()));
        }
        match &self.kind {
            ExprKind::Cond(a, b, c) => Some((a, b, c)),
            ExprKind::SomeWhere(a, b) => Some((a, b, UNDEFINED.with(|undefined| *undefined))),
            _ => None,
        }
    }

    #[track_caller]
    pub fn presence(option: Expr, set: bool, given: Option<super::OnlyNone>) -> Expr {
        // Of one made where it's tested, `Some(v)` where `t` holds, a
        // `SomeWhere`, the test it's made by, `t` (ADR 0910). Not of a test
        // made otherwise, `t ? v : undefined` of a `v` that's only never
        // `undefined`, an `Option` that may be `null` (#539).
        if let ExprKind::SomeWhere(test, _) = &option.kind {
            let test = (**test).clone();
            return if set { test } else { test.negated_test() };
        }
        // Of one whose `None` is only one of `undefined` and `null`, the test
        // of that one, `x !== undefined` (`Op::NoneNe`, #2112 C S5).
        let op = match (given, set) {
            (Some(none), true) => Op::NoneNe(none),
            (Some(none), false) => Op::NoneEq(none),
            (None, true) => Op::LooseNe,
            (None, false) => Op::LooseEq,
        };
        Expr::bin(op, option, Expr::null())
    }

    /// This, the property a binding's write assigns, which a setter may
    /// answer (`Known::Setter`).
    pub fn answered_by_a_setter(self) -> Expr {
        Expr {
            facts: Facts::of(Known::Setter),
            ..self
        }
    }

    /// This, a field of a struct only the program makes (`Known::Own`).
    pub fn own_field(mut self) -> Expr {
        self.facts = self.facts.with(Known::Own);
        self
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

    /// JS's `undefined`, a value: what's absent, `None`'s, a unit's.
    pub fn js_undefined() -> Expr {
        Expr::new(ExprKind::Undefined)
    }

    /// What a value is taken out of, by `std::mem::replace`, till what's
    /// made of it is put back: never printed (`Hole::Taken`).
    #[track_caller]
    pub fn taken() -> Expr {
        Expr::new(ExprKind::Hole(Hole::Taken(std::panic::Location::caller())))
    }

    /// A hole, `hole`, which no tree is printed with (`Hole`).
    pub fn hole(hole: Hole) -> Expr {
        Expr::new(ExprKind::Hole(hole))
    }

    /// `list` and then each of `optional` given, `undefined` for one absent
    /// before a given one, and none past the last given: a helper's trailing
    /// arguments, its drops.
    pub fn optional_args(mut list: Vec<Expr>, mut optional: Vec<Option<Expr>>) -> Vec<Expr> {
        while matches!(optional.last(), Some(None)) {
            optional.pop();
        }
        list.extend(optional.into_iter().map(|arg| arg.unwrap_or_else(Expr::js_undefined)));
        list
    }

    pub fn null() -> Expr {
        Expr::new(ExprKind::Null)
    }

    pub fn var(name: impl Into<Name>) -> Expr {
        Expr::new(ExprKind::Var(name.into()))
    }

    /// The runtime's helper `name` exports, `$cmp`: asked for by being
    /// written, as the linker asks for each its module's tree reads. A name
    /// the runtime doesn't export is a bug of rust-js's. No other variable
    /// is named so: Rust's names have no `$`, and the compiler's own are
    /// named apart from the runtime's (`names::dictionary_cache`).
    pub fn helper(name: &'static str) -> Expr {
        assert!(crate::runtime::exports(name), "the runtime exports no `{name}`");
        Expr::module(name)
    }

    /// The runtime's helper this is, by the name it exports (ADR 0103).
    pub fn helper_name(&self) -> Option<&str> {
        match &self.kind {
            ExprKind::Var(name) => name.helper(),
            _ => None,
        }
    }

    /// The items an iterator gives, collected: `Array.from(it)`, as JS
    /// writes it, an iterable's or an iterator's alike (the user's decision,
    /// bw8/cm7, ADR 0055 amended), never ES2025's `it.toArray()`.
    pub fn collected(it: Expr) -> Expr {
        // Of what JS's iteration of its receiver steps, `set.values()`'s,
        // the receiver itself, `Array.from(set)` (`Known::ReceiverIterated`).
        let it = match it.kind {
            ExprKind::Call(callee, args) if it.facts.has(Known::ReceiverIterated) && args.is_empty() => {
                match callee.kind {
                    // A string's own iterator too, `s[Symbol.iterator]()`.
                    ExprKind::Member(receiver, _) | ExprKind::Index(receiver, _) => *receiver,
                    kind => Expr {
                        kind: ExprKind::Call(Box::new(Expr { kind, ..*callee }), args),
                        ..it
                    },
                }
            }
            kind => Expr { kind, ..it },
        };
        Expr::call(Expr::member(Expr::global("Array"), "from"), vec![it])
    }

    /// Of `Array.from(it)`, `collected`'s, `it`.
    pub fn collected_of(&self) -> Option<&Expr> {
        match &self.kind {
            ExprKind::Call(callee, args) => match (&callee.kind, args.as_slice()) {
                (ExprKind::Member(array, from), [it])
                    if from == "from" && matches!(&array.kind, ExprKind::Var(name) if name.is("Array")) =>
                {
                    Some(it)
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// A call of the runtime's helper `name`, `$cmp(a, b)`.
    /// `self` as an item JS's own method is given, `push(a, undefined)`:
    /// an `undefined` a value, `GivenUndefined`, which no rule leaves out,
    /// as a method that counts what it's given, `push`, `splice`, would
    /// count one fewer (ADR 0330, amended).
    pub fn given_item(self) -> Expr {
        match self.kind {
            ExprKind::Undefined => Expr {
                kind: ExprKind::GivenUndefined,
                ..self
            },
            _ => self,
        }
    }

    /// `f()`, every helper call it makes that takes where a panic it raises
    /// was raised given `at`: the location of the statement or terminator of
    /// MIR the lowering is at, `caller_location`'s (#2292, ADR 0427 amended).
    /// What makes a helper call outside, tidy or preparation, gives it none,
    /// which `js::validate` refuses.
    pub fn raised_at<T>(at: Expr, f: impl FnOnce() -> T) -> T {
        let outer = RAISED_AT.replace(Some(Expr::new(ExprKind::RaisedAt(Box::new(at)))));
        let made = f();
        RAISED_AT.set(outer);
        made
    }

    /// The location `raised_at` gives what's made now, kept for a call made
    /// later, where the lowering is elsewhere: `&mut v[1..]`'s view, made
    /// where it's indexed and called where it's used, raises its panic at
    /// the index, as Rust's does (#2292). Given back to `raised_at` there.
    pub fn raising_at() -> Option<Expr> {
        RAISED_AT.with_borrow(|at| match at.as_ref().map(|at| &at.kind) {
            Some(ExprKind::RaisedAt(at)) => Some((**at).clone()),
            _ => None,
        })
    }

    /// Where std raises one of its panics, the `at`th site of `name`,
    /// `$stdSites.name[at]`: the runtime's constant of the toolchain, which
    /// `bun run std` writes from std's MIR, whatever calls it (#2292).
    pub fn std_site(name: &str, at: usize) -> Expr {
        Expr::index(Expr::member(Expr::helper("$stdSites"), name), Expr::int(at as i128))
    }

    pub fn call_helper(name: &'static str, args: Vec<Expr>) -> Expr {
        Expr::call(Expr::helper(name), args)
    }

    pub fn drop_argument(item: (u32, u32), index: u32, drop: Expr) -> Expr {
        Expr::new(ExprKind::DropArgument(item, index, Box::new(drop)))
    }

    pub fn import(from: ImportFrom) -> Expr {
        Expr::new(ExprKind::Import(from))
    }

    /// `require("x")`, of the module `from` names (ADR 0305).
    pub fn require(from: ImportFrom) -> Expr {
        Expr::call(Expr::global("require"), vec![Expr::new(ExprKind::Specifier(from))])
    }

    pub fn handle(place: Expr) -> Expr {
        Expr::stored_handle(place, Storage::AsGiven)
    }

    /// A handle on `place`, which stores what's written as `storage` says.
    pub fn stored_handle(place: Expr, storage: Storage) -> Expr {
        Expr::new(ExprKind::Handle(Box::new(place), storage))
    }

    pub fn pair(place: Expr, dictionary: Expr) -> Expr {
        Expr::stored_pair(place, dictionary, Storage::AsGiven)
    }

    /// A `dyn`'s pair on `place`, which stores what's written as `storage`
    /// says (#968).
    pub fn stored_pair(place: Expr, dictionary: Expr, storage: Storage) -> Expr {
        Expr::new(ExprKind::Pair(Box::new(place), Box::new(dictionary), storage))
    }

    /// `target++` or `target--`, its value the one before (ADR 0948).
    pub fn stepped(op: Op, target: Expr) -> Expr {
        Expr::new(ExprKind::Stepped(op, Box::new(target)))
    }

    /// `(target = value)`, its value the assignment's (ADR 0860).
    pub fn assigned(target: Expr, value: Expr) -> Expr {
        Expr::new(ExprKind::Assigned(Box::new(target), Box::new(value)))
    }

    /// `(a, b)`, each in turn, its value the last's (#523).
    pub fn sequence(items: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::Sequence(items))
    }

    /// What a box, a cell or a handle holds: `b.value` (`HELD`).
    pub fn held(of: Expr) -> Expr {
        Expr::member(of, HELD)
    }

    /// What holds what this reads, `b` of `b.value`, where it's that read
    /// (`HELD`).
    pub fn held_by(&self) -> Option<&Expr> {
        match &self.kind {
            ExprKind::Member(of, property) if property == HELD => Some(of),
            _ => None,
        }
    }

    pub fn member(object: Expr, property: impl Into<String>) -> Expr {
        let property = property.into();
        // A field of an object made here whose other fields do nothing is
        // what it's made of: a cell's, `{ value: x }.value`, `x`; `{ x: 0,
        // y: 0 }.y`, a constant's field
        // (`V::ZERO.y`), is `0`; a `dyn`'s pair made of what does nothing,
        // `{ value: f.last, impl: f.impl }.impl`, a shared reference nothing
        // assigns through (ADR 0409), `f.impl`; and a `try_borrow`'s `Ok`,
        // `{ TAG: "Ok", _0: state }.TAG`, `"Ok"` (#687).
        if let ExprKind::Object(props, ..) = &object.kind
            && (props.iter())
                .all(|p| matches!(p, Prop::Field(name, value) if *name == property || !value.has_effects()))
            && let Some(Prop::Field(_, value)) = props
                .iter()
                .find(|p| matches!(p, Prop::Field(name, _) if *name == property))
        {
            return value.clone();
        }
        // A handle's `value` is the place it's on (ADR 0099); a cell made
        // here, `{ value: x }`, holds `x`, a field of an object made here
        // above: a cell in a field is the property itself (ADR 0288).
        if property == HELD {
            // Read or written as the place: a write's storage is its
            // writer's to keep, `cell_call`'s (ADR 0772).
            if let ExprKind::Handle(place, _) = object.kind {
                return *place;
            }
            // A struct's `RefCell` field's one cell, `$fieldCell(o, "f")`,
            // holds `o.f`, which its `value` reads and writes; not a
            // nullable one's, `$fieldCell(o, "f", true)`, which writes its
            // `None` `null` (ADR 0863, amended).
            if let ExprKind::Call(callee, args) = &object.kind
                && callee.helper_name() == Some("$fieldCell")
                && let [holder, key] = args.as_slice()
                && let ExprKind::Str(key) = &key.kind
            {
                return Expr::member(holder.clone(), key.clone());
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
        if let (ExprKind::Array(items, ..), Some(k)) = (&object.kind, index.as_int())
            && let Some(item) = usize::try_from(k).ok().and_then(|k| items.get(k))
            && !items.iter().any(|i| matches!(i.kind, ExprKind::Spread(_)))
            && items.iter().all(|i| std::ptr::eq(i, item) || !i.has_effects())
        {
            return item.clone();
        }
        Expr::new(ExprKind::Index(Box::new(object), Box::new(index)))
    }

    /// `items[index]`, the item Rust's index reads, `v[i]`: an index, never
    /// a part (`Known::Indexed`).
    pub fn indexed(items: Expr, index: Expr) -> Expr {
        let read = Expr::index(items, index);
        match read.kind {
            ExprKind::Index(..) => Expr {
                facts: read.facts.with(Known::Indexed),
                ..read
            },
            _ => read,
        }
    }

    pub fn array(items: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::Array(items, Default::default()))
    }

    /// `[1, 2, 3]` of a constant's items, known to be literals alone,
    /// `Known::Literal`, where each item is: a number, a string, a boolean,
    /// `null`, `undefined` or such an array (#1096).
    pub fn literal_array(items: Vec<Expr>) -> Expr {
        let literal = items.iter().all(|item| {
            matches!(
                item.kind,
                ExprKind::Num(_)
                    | ExprKind::BigInt(_)
                    | ExprKind::Bool(_)
                    | ExprKind::Str(_)
                    | ExprKind::Null
                    | ExprKind::Undefined
            ) || item.facts.has(Known::Literal)
        });
        let mut array = Expr::array(items);
        if literal {
            array.facts = array.facts.with(Known::Literal);
        }
        array
    }

    /// `...items`, as an array's item or a call's argument.
    pub fn spread(items: Expr) -> Expr {
        Expr::new(ExprKind::Spread(Box::new(items)))
    }

    pub fn object(props: Vec<Prop>) -> Expr {
        Expr::new(ExprKind::Object(props, ObjectLayout::default()))
    }

    /// `text + part` of a `text` that's text whatever it's of, a `String`
    /// written to or added to: `s + n` of `s + ("" + n)`, `bin`'s rule of
    /// text joined to `"" +` (ADR 0639, amended).
    pub fn text_joined(mut text: Expr, part: Expr) -> Expr {
        text.facts = text.facts.with(Known::Text);
        let mut joined = Expr::bin(Op::Add, text, part);
        joined.facts = joined.facts.with(Known::Text);
        joined
    }

    /// Of text joined to a number's text, `s + String(n)`, the number,
    /// which `+` makes text the same way: `prefix + id` (rd537). Prepared
    /// once tidy's done, whose `f += s; return f` is `return s` of what's
    /// text; not of `""`, whose `"" + n` reads worse than `String(n)`. What
    /// `text_joined` made is text whatever its text's been inlined to.
    pub fn number_joined(&mut self) {
        let joined = self.facts.has(Known::Text);
        if let ExprKind::Binary(Op::Add, text, part) = &mut self.kind
            && (joined || is_text(text))
            && !matches!(&text.kind, ExprKind::Str(s) if s.is_empty())
            && let ExprKind::Stringed(number) = &mut part.kind
        {
            let number = std::mem::replace(&mut **number, Expr::taken());
            **part = Expr {
                span: part.span,
                ..number
            };
        }
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
        // A strict test of being set folds as a loose one (`null_test`, #2112 C S5).
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
        // A strict test of being set folds as a loose one (`null_test`, #2112 C S5).
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
        // A strict test of being set folds as a loose one (`null_test`, #2112 C S5).
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

    pub fn async_arrow(params: Vec<Pattern>, body: Vec<Stmt>) -> Expr {
        Expr::new(ExprKind::AsyncArrow(params, body))
    }

    /// `(0, f)`: `f` read unbound (`ExprKind::Unbound`).
    pub fn unbound(f: Expr) -> Expr {
        Expr::new(ExprKind::Unbound(Box::new(f)))
    }

    /// What's read unbound that JS gives no `this` of, `(0, f)` of a
    /// variable, a field's function MIR held, or of a call's value: itself,
    /// `f`, whose call gives none in a module; but `eval`'s, an indirect
    /// eval, which `eval(..)` isn't (ADR 0972, amended, rd579). A member's
    /// or an item's read stays unbound, `(0, s.f)`. What preparation does
    /// of each expression.
    pub fn unbound_variable_read(e: &mut Expr) {
        if let ExprKind::Unbound(f) = &e.kind
            && !matches!(
                &f.kind,
                ExprKind::Member(..) | ExprKind::OptionalMember(..) | ExprKind::Index(..)
            )
            && !matches!(&f.kind, ExprKind::Var(name) if name.is("eval"))
        {
            *e = (**f).clone();
        }
    }

    pub fn await_(promise: Expr) -> Expr {
        Expr::new(ExprKind::Await(Box::new(promise)))
    }

    pub fn jsx(jsx: Jsx) -> Expr {
        Expr::new(ExprKind::Jsx(Box::new(jsx)))
    }

    pub fn call(callee: Expr, args: Vec<Expr>) -> Expr {
        // A call of an entry of a `&mut`'s dictionary through its box: the
        // entry of what it's to (`through_box::called`, #1470).
        let mut args = args;
        // A helper that takes where a panic it raises was raised is given
        // the location the lowering is at, as it's made (`raised_at`, #2292).
        if let Some(helper) = callee.helper_name()
            && let Some(at) = crate::runtime::location_at(helper)
            && args.len() >= at
            && !args.iter().any(|arg| matches!(arg.kind, ExprKind::RaisedAt(_)))
            && let Some(location) = RAISED_AT.with_borrow(Clone::clone)
        {
            args.insert(at, location);
        }
        if let Some(folded) = super::through_box::called(&callee, &mut args) {
            return folded;
        }
        Expr::new(ExprKind::Call(Box::new(callee), args))
    }

    /// `Expr::call` of a helper whose location is given only where its call
    /// asks (`runtime::location_where_asked`), `asked` whether this one does:
    /// the location the lowering is at, where `raisedAt` is, an optional
    /// parameter before it not given `undefined`, `$count(it, undefined,
    /// "src/lib.rs:4:9")`; of one that doesn't, none (#2324).
    pub fn call_located(callee: Expr, args: Vec<Expr>, asked: bool) -> Expr {
        let mut args = args;
        if asked
            && let Some(helper) = callee.helper_name()
            && let Some(at) = crate::runtime::location_where_asked(helper)
            && let Some(location) = RAISED_AT.with_borrow(Clone::clone)
        {
            while args.len() < at {
                let pad = Expr::js_undefined();
                args.push(Expr {
                    facts: pad.facts.with(Known::Padded),
                    ..pad
                });
            }
            args.insert(at, location);
        }
        Expr::call(callee, args)
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

    /// A literal JS's `== null` holds of: `undefined`, a given `undefined`
    /// or `null`, the one answer of which constant is nullish (#2182).
    pub fn is_nullish(&self) -> bool {
        matches!(
            self.kind,
            ExprKind::Undefined | ExprKind::GivenUndefined | ExprKind::Null
        )
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

    /// It, a collect that closes what it steps, so marked
    /// (`Known::ClosesWhatItSteps`), beside what else is known of it.
    pub fn closes_what_it_steps(mut self) -> Expr {
        self.facts = self.facts.with(Known::ClosesWhatItSteps);
        self
    }

    /// It, a `Rev`, an `Enumerate` or a `Map` made in place, so marked
    /// (`Known::InPlace`), beside what else is known of it.
    pub fn in_place(mut self) -> Expr {
        self.facts = self.facts.with(Known::InPlace);
        self
    }

    /// This, a dictionary the function was given, whose methods are the
    /// program's (`Known::Evidence`), beside what else is known of it.
    pub fn evidence(mut self) -> Expr {
        self.facts = self.facts.with(Known::Evidence);
        self
    }

    /// This, a call that throws only where what it's given does
    /// (`Known::NeverThrows`), beside what else is known of it.
    pub fn never_throws(mut self) -> Expr {
        self.facts = self.facts.with(Known::NeverThrows);
        self
    }

    /// This, a binding's call that gives what JS's iteration of its
    /// receiver steps (`Known::ReceiverIterated`).
    pub fn receiver_iterated(mut self) -> Expr {
        self.facts = self.facts.with(Known::ReceiverIterated);
        self
    }

    /// This, a call of a method read of the `this` it's given
    /// (`Known::OwnMethodCall`), beside what else is known of it.
    pub fn own_method(mut self) -> Expr {
        self.facts = self.facts.with(Known::OwnMethodCall);
        self
    }

    /// This, a member a key names of a value of any shape, read only as
    /// its value's method (`Known::KeyedMethod`), beside what else is
    /// known of it.
    pub fn keyed_method(mut self) -> Expr {
        self.facts = self.facts.with(Known::KeyedMethod);
        self
    }

    /// `value`, the variable, of `value.f.call(value, ..args)`, a call of
    /// a method read in place of the variable it's given as its `this`
    /// (`Known::OwnMethodCall`): what `own_method_call` writes as the
    /// method's call. Of `f` held in a variable, or read of another value,
    /// none.
    fn own_method_receiver(&self) -> Option<&Name> {
        if !self.facts.has(Known::OwnMethodCall) {
            return None;
        }
        let ExprKind::Call(callee, args) = &self.kind else {
            return None;
        };
        let ExprKind::Member(method, name) = &callee.kind else {
            return None;
        };
        let (ExprKind::Member(of, _) | ExprKind::Index(of, _)) = &method.kind else {
            return None;
        };
        match (&of.kind, args.first().map(|this| &this.kind)) {
            (ExprKind::Var(value), Some(ExprKind::Var(this))) if name == "call" && value == this => Some(value),
            _ => None,
        }
    }

    /// `value` of `value.f.call(value, ..args)` where `f` is a member a
    /// key names (`Known::KeyedMethod`), `js::method((value, key))`'s: the
    /// variable alone the receiver of a call that only rd334's form, which
    /// names it twice, writes (rd226b).
    pub fn own_keyed_method_receiver(&self) -> Option<&Name> {
        let ExprKind::Call(callee, _) = &self.kind else {
            return None;
        };
        let ExprKind::Member(method, _) = &callee.kind else {
            return None;
        };
        self.own_method_receiver()
            .filter(|_| method.facts.has(Known::KeyedMethod))
    }

    /// `value.f.call(value, ..args)` of a method read in place of the
    /// variable it's given as its `this` (`Known::OwnMethodCall`): the
    /// method's call, `value.f(..args)`, whose `this` JS gives as `call`
    /// gives it, the read first, before its arguments, either way (rd334).
    /// Of `f` held in a variable, which a call of gives no `this`, or read
    /// of another value, as it is. What preparation does of each
    /// expression.
    pub fn own_method_call(e: &mut Expr) {
        e.own_method_called(None);
    }

    /// `own_method_call`, its receiver `receiver` where given, in place of
    /// the variable: `props.fill.applyFill(instance)` of `fill.applyFill.call(
    /// fill, instance)`, the value read once, first, as JS reads a method
    /// call's receiver (rd226b). Whether it was such a call.
    pub fn own_method_called(&mut self, receiver: Option<Expr>) -> bool {
        if self.own_method_receiver().is_none() {
            return false;
        }
        let ExprKind::Call(callee, args) = &mut self.kind else {
            return false;
        };
        let ExprKind::Member(method, _) = &mut callee.kind else {
            return false;
        };
        let mut method = std::mem::replace(&mut **method, Expr::taken());
        if let (Some(receiver), ExprKind::Member(of, _) | ExprKind::Index(of, _)) = (receiver, &mut method.kind) {
            **of = receiver;
        }
        let mut args = std::mem::take(args);
        args.remove(0);
        self.kind = ExprKind::Call(Box::new(method), args);
        // A method's call now, which is no `call`'s to write again.
        self.facts = self.facts.without(Known::OwnMethodCall);
        true
    }

    /// This, a binding's call that runs none of the program's code
    /// (`Known::RunsNothing`), beside what else is known of it.
    pub fn runs_nothing(mut self) -> Expr {
        self.facts = self.facts.with(Known::RunsNothing);
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

    /// Whether this names what it reads by a name alone, as a function
    /// called or given is named: a variable, another module's item, or one
    /// read through its module's namespace, `Plugin.extractEvents` (rd114,
    /// ADR 1018), the crate's module's or a library's, which is that
    /// module's binding of it, as live as an import of it: the one member
    /// read that's an item (`Known::Item`).
    pub fn is_named(&self) -> bool {
        match &self.kind {
            ExprKind::Var(_) | ExprKind::Symbol(_) => true,
            ExprKind::Member(of, _) => {
                self.facts.has(Known::Item) && matches!(&of.kind, ExprKind::Var(_) | ExprKind::Symbol(_))
            }
            _ => false,
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
    /// operand that may be. What MIR proves where it's read is `presence`'s
    /// (#2112 C S4), which its uses ask beside this. An item's read,
    /// `Known::Item`, may be: a `const`'s or a thread-local's `None` is
    /// `undefined`, `const sigil = undefined;` (#1957).
    /// Anything else, a variable or a call, may be. The one answer of
    /// whether a JS value may be `undefined`, which a nullable place's
    /// storage asks (`Storage::written`), as `given` says, by its form
    /// alone. What MIR says is `null` or a value, `Undefined::Never`,
    /// `written` answers itself.
    pub fn may_be_undefined(&self) -> bool {
        match &self.kind {
            ExprKind::Undefined | ExprKind::GivenUndefined => true,
            _ if self.is_constant() => false,
            ExprKind::Array(..)
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
    /// read where it's dropped as the variable it was made of, is `made`; and
    /// of a test now read in place whose branches are each so, or
    /// `undefined`, the test with `null` for `undefined`, `(__DEV__ ?
    /// fiber._debugTask : undefined) ?? null` `__DEV__ ? fiber._debugTask :
    /// null`, as `Storage::written` would have stored it (ADR 0841,
    /// amended). Anything else is as it is. What's never `undefined` as
    /// `given` says (`Undefined`).
    pub fn store_again(&mut self, given: super::Undefined) {
        if let ExprKind::Binary(Op::Coalesce, made, null) = &self.kind
            && matches!(null.kind, ExprKind::Null)
            && let Some(stored) = null_for_none(made, given)
        {
            let span = self.span;
            *self = stored.or_at(span);
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
            _ => self.is_nullish(),
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

/// Whether `test`, or the last of an `&&` chain of tests, is `typeof place
/// === "<type>"` of a type no `null` or `undefined` is, `"function"`,
/// `"string"` and the rest, not `"object"` or `"undefined"`.
fn typed_present(test: &Expr, place: &Expr) -> bool {
    match &test.kind {
        ExprKind::Binary(Op::And, _, last) => typed_present(last, place),
        ExprKind::Binary(Op::Eq, of, kind) => {
            matches!(&of.kind, ExprKind::Unary(UnaryOp::Typeof, typed) if typed.kind == place.kind)
                && matches!(&kind.kind, ExprKind::Str(s) if s != "object" && s != "undefined")
        }
        _ => false,
    }
}

/// Whether `e` is text whatever its parts are: a string, a template, or
/// what's known to be (`Known::Text`), as a `String` added to is
/// (`Expr::text_joined`), `";" + name + ":"`.
fn is_text(e: &Expr) -> bool {
    e.facts.has(Known::Text)
        || matches!(
            &e.kind,
            ExprKind::Str(_) | ExprKind::Written(_) | ExprKind::Template(..)
        )
}

/// `x === null` of `x !== undefined && x == null`, `x !== null` of `x ===
/// undefined || x != null`, and `x != null` of `x !== undefined && x !=
/// null`, `x == null` of `x === undefined || x == null`, of a place `x`
/// (`Expr::bin`).
fn null_apart(op: Op, lhs: &Expr, rhs: &Expr) -> Option<Expr> {
    use super::stmt_walk::{null_test, same_path, taken_place};
    let present = match op {
        Op::And => false,
        Op::Or => true,
        _ => return None,
    };
    // `x !== undefined` of `And`, `x === undefined` of `Or`: written so, or
    // of an `Option` whose `None` is only `undefined`.
    let strict = if present { Op::Eq } else { Op::Ne };
    let x = match &lhs.kind {
        ExprKind::Binary(o, x, u) if *o == strict && matches!(u.kind, ExprKind::Undefined) => &**x,
        _ => null_test(lhs, !present).filter(|_| {
            matches!(&lhs.kind, ExprKind::Binary(op, ..) if matches!(op, Op::NoneEq(super::OnlyNone::Undefined) | Op::NoneNe(super::OnlyNone::Undefined)))
        })?,
    };
    if !taken_place(x) {
        return None;
    }
    // `x !== undefined && x != null` is `x != null`, and `x === undefined ||
    // x == null` is `x == null`: the second says the first, `if let
    // Some(Some(v)) = last`'s test (ADR 1017, amended).
    if let Some(inner) = null_test(rhs, !present)
        && same_path(x, inner)
    {
        return Some(rhs.clone());
    }
    let inner = null_test(rhs, present)?;
    if !same_path(x, inner) {
        return None;
    }
    let strict = if present { Op::Ne } else { Op::Eq };
    Some(Expr::new(ExprKind::Binary(
        strict,
        Box::new(inner.clone()),
        Box::new(Expr::null()),
    )))
}

thread_local! {
    /// The location helper calls made now are given (`Expr::raised_at`):
    /// the lowering's, scoped to one statement or terminator of MIR.
    static RAISED_AT: std::cell::RefCell<Option<Expr>> = const { std::cell::RefCell::new(None) };
}
