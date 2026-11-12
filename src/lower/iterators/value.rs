//! An iterator's value as the lowering makes it: what it's made of, and the
//! facts a step past its maker asks of it, written once, by `emit`, as its
//! JS and the `Known` facts that JS carries (ADR 1092's owner (b), #730).
//! A reader asks this module, never the JS's shape: whether a value is
//! closed, lent or a range's numbers is what its maker said, not what a
//! helper's name looks like.

use crate::js::{self, Expr, Known};
use crate::lower::std_types::range::RangeKind;

/// What a JS iterator is made as, each its maker's.
pub(in crate::lower) enum Form {
    /// `$iterator(value, next, ..)`: `value`, the crate's own iterator or
    /// what a dictionary's `next_back` steps, stepped by `next`, and what
    /// else it's given, each in its place (`stepped_ends`).
    Own { value: Expr, next: Expr, given: Vec<Expr> },
    /// A range's numbers, `$iterator(range, step)`: the range, its bounds,
    /// stepped by its kind's step (`std_types/range.rs`'s `range_stepper`).
    Range { range: Expr, step: Expr },
    /// `$given(value, dictionary, ..)`: a type parameter's value given as it
    /// is, stepped by its dictionaries (`given_js_iterator`, ADR 0411).
    Given {
        value: Expr,
        dictionary: Expr,
        given: Vec<Expr>,
    },
    /// `$knowing(it, back, len, ..)`: std's compiled struct, given its back
    /// and its length by a use site (`knowing_held`, ADR 0561).
    Knowing { it: Expr, given: Vec<Expr> },
    /// Any other JS iterator, as its maker wrote it.
    Js(Expr),
}

/// What closes a value to drop what it hasn't given, `$closing(it, drop)`
/// (`closed_by`, ADRs 0422, 0465): `drop`, and whether it drops each item
/// of an array's as it's passed, not as one slice (#234).
struct Closing {
    drop: Expr,
    passes_each: bool,
}

/// An iterator's value: its form, and whether it's closed to drop what it
/// holds, or lent, `$lent(it)`, which nothing past it closes (ADR 0071).
pub(in crate::lower) struct IterValue {
    form: Form,
    closes: Option<Closing>,
    lent: bool,
}

impl IterValue {
    /// A value of `form`, neither closed nor lent.
    pub(in crate::lower) fn of(form: Form) -> IterValue {
        IterValue {
            form,
            closes: None,
            lent: false,
        }
    }

    /// `it`, a JS iterator its maker wrote.
    pub(in crate::lower) fn js(it: Expr) -> IterValue {
        IterValue::of(Form::Js(it))
    }

    /// It, closed by `drop`, `passes_each` or not (`closed_by`). Of one
    /// closed or lent already, that closed again around it.
    pub(in crate::lower) fn closed(self, drop: Expr, passes_each: bool) -> IterValue {
        let mut value = match self.closes.is_some() || self.lent {
            true => IterValue::js(self.emit()),
            false => self,
        };
        value.closes = Some(Closing { drop, passes_each });
        value
    }

    /// It, lent (`lent_iterator`).
    pub(in crate::lower) fn lent(mut self) -> IterValue {
        self.lent = true;
        self
    }

    /// Its JS, each part marked with what it is: the one writer of
    /// `$iterator`, `$given`, `$knowing`, `$closing` and `$lent`, and of
    /// the facts each carries.
    pub(in crate::lower) fn emit(self) -> Expr {
        let made = |name: &'static str, args: Vec<Expr>, known: &[Known]| {
            let mut e = Expr::call_helper(name, args);
            for &fact in known {
                e.facts = e.facts.with(fact);
            }
            e
        };
        let mut e = match self.form {
            Form::Own { value, next, given } => {
                let mut args = vec![value, next];
                args.extend(given);
                made("$iterator", args, &[Known::IterStepped, Known::IterOwn])
            }
            Form::Range { range, step } => {
                made("$iterator", vec![range, step], &[Known::IterStepped, Known::IterRange])
            }
            Form::Given {
                value,
                dictionary,
                given,
            } => {
                let mut args = vec![value, dictionary];
                args.extend(given);
                made("$given", args, &[Known::IterStepped])
            }
            Form::Knowing { it, given } => {
                let mut args = vec![it];
                args.extend(given);
                made("$knowing", args, &[Known::IterKnowing])
            }
            Form::Js(e) => e,
        };
        if let Some(Closing { drop, passes_each }) = self.closes {
            let mut args = vec![e, drop];
            if passes_each {
                args.push(Expr::bool(false));
            }
            e = made("$closing", args, &[Known::IterClosing]);
        }
        if self.lent {
            // What it lends, never `Iterator.from(x)` of it, which calls a
            // `$Steps`' `[Symbol.iterator]()`, arming it to close itself at
            // its end, as a consumer that takes it by value does: `$lent(x)`,
            // which makes a JS iterator of what's no iterator itself (#1508).
            if let Some(x) = super::iterator_from(&e) {
                e = x.clone();
            }
            e = made("$lent", vec![e], &[Known::IterLent]);
        }
        e
    }
}

/// Whether `e` is a string's iterator made here: JS's own,
/// `s[Symbol.iterator]()`, or the runtime's, `$chars(s)` or
/// `$charIndices(s)`, none of which has a `return()`.
fn made_string_iterator(e: &Expr) -> bool {
    super::string_iterated(e).is_some()
        || matches!(&e.kind, js::ExprKind::Call(callee, _) if matches!(callee.helper_name(), Some("$chars" | "$charIndices")))
}

/// Of the runtime's `chars()` made here, `$chars(s)` as `e` is, the
/// string, whose JS iteration steps the same `char`s (#1186).
pub(in crate::lower) fn chars_made_of(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$chars") && args.len() == 1 => args.first(),
        _ => None,
    }
}

/// Of a string's iterator made here lent, `$lent(s[Symbol.iterator]())`
/// as `e` is, the iterator: which has no `return()` a lending keeps from
/// being called, and which nothing else holds, so it's lent as it is
/// (#1186).
pub(in crate::lower) fn lent_string_iterator(e: &Expr) -> Option<&Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let [it] = &args[..] else { return None };
    (callee.helper_name() == Some("$lent") && made_string_iterator(it)).then_some(it)
}

/// Each fact `emit` marks a value's JS with.
const VALUE_FACTS: [Known; 12] = [
    Known::IterStepped,
    Known::IterRange,
    Known::IterOwn,
    Known::IterKnowing,
    Known::IterClosing,
    Known::IterLent,
    Known::IterRev,
    Known::IterStepBy,
    Known::IterRepeatingN,
    Known::IterKnowsBack,
    Known::IterKnowsLength,
    Known::IterEnumerate,
];

/// Of `facts`, those `emit` marked a value's JS with: what a value made
/// again of the same call keeps, `for (x of e)`'s `e` without what else was
/// known of it (`iterated`).
pub(in crate::lower) fn its_facts(facts: js::Facts) -> js::Facts {
    (VALUE_FACTS.into_iter())
        .filter(|&fact| facts.has(fact))
        .fold(js::Facts::NONE, js::Facts::with)
}

/// What `e` steps, of a JS iterator of a value its maker wrote
/// (`Known::IterStepped`), `$iterator(value, next, ..)` or `$given(value,
/// ..)`: `value`, the crate's iterator, a range or a type parameter's value.
/// `None` of another, or of one a variable holds, whose value isn't there.
pub(in crate::lower) fn steps(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(_, args) if e.facts.has(Known::IterStepped) => args.first(),
        _ => None,
    }
}

/// What closes a value `emit()` closed to drop what it hasn't given,
/// `$closing(it, drop)` (`Known::IterClosing`): `it`, `drop`, and whether
/// it drops each of an array's items as it's passed, not as one slice
/// (#234).
pub(in crate::lower) struct Closed<'e> {
    pub(in crate::lower) it: &'e Expr,
    pub(in crate::lower) drop: &'e Expr,
    pub(in crate::lower) passes_each: bool,
}

/// `e`'s closing, of a value `emit()` closed; `None` of another.
pub(in crate::lower) fn closed(e: &Expr) -> Option<Closed<'_>> {
    let js::ExprKind::Call(_, args) = &e.kind else {
        return None;
    };
    match (args.as_slice(), e.facts.has(Known::IterClosing)) {
        ([it, drop], true) => Some(Closed {
            it,
            drop,
            passes_each: false,
        }),
        ([it, drop, _], true) => Some(Closed {
            it,
            drop,
            passes_each: true,
        }),
        _ => None,
    }
}

/// What `e`, closed, is made of, its `it` and `drop` (`closed`).
pub(in crate::lower) fn closed_parts(e: &Expr) -> Option<(&Expr, &Expr)> {
    closed(e).map(|closed| (closed.it, closed.drop))
}

/// What `e`, a value `emit()` lent (`Known::IterLent`), `$lent(it)`, lends:
/// `it`, made for this call alone, which no one else steps; or `e` itself.
pub(in crate::lower) fn unlent(e: &Expr) -> &Expr {
    match &e.kind {
        js::ExprKind::Call(_, of) if e.facts.has(Known::IterLent) && of.len() == 1 => &of[0],
        _ => e,
    }
}

/// Whether `e` is lent, a JS iterator nothing closes (`unlent`).
pub(in crate::lower) fn is_lent(e: &Expr) -> bool {
    !std::ptr::eq(unlent(e), e)
}

/// What `e` steps (`steps`), held: a map's source made a JS iterator
/// before it's asked its back, `$iterator(range, $rangeNext)`, is stepped
/// again from what it steps, not from the JS iterator (ADR 0535). Not of
/// one made in place (`made_in_place`): a `rev()` of the crate's iterator,
/// `$iterator(it, nextBack)`, is its own form, which only `steps` reads into
/// (#257).
pub(in crate::lower) fn stepped_value(e: &Expr) -> Option<Expr> {
    if super::made_in_place(e) {
        return None;
    }
    steps(e).cloned()
}

/// `e`, an adapter MIR made of a JS iterator `method_source` made one,
/// `it.map(f)`: lazy, a JS iterator itself, said so (`Known::Iterator`), so
/// one nothing reads is no statement (tidy's `unread_maps`, #805).
pub(in crate::lower) fn lazily_made(mut e: Expr) -> Expr {
    e.facts = e.facts.with(Known::Iterator);
    e
}

/// `it.enumerate()` made in place, each item and its index, `src.map((x, i)
/// => [i, x])`, in place of which a callback that takes the pair apart is
/// given each item and its index, of an array (`eager_chains`).
pub(in crate::lower) fn enumerated(it: Expr) -> Expr {
    Expr::call(Expr::member(it, "map"), vec![enumerating()])
}

/// `(x, i) => [i, x]`, what `enumerated` maps each item by.
fn enumerating() -> Expr {
    let pair = Expr::array(vec![Expr::var("i"), Expr::var("x")]);
    Expr::arrow(
        vec!["x".into(), "i".into()],
        vec![js::StmtKind::Return(Some(pair)).at(js::Span::NONE)],
    )
}

/// `$enumerate(it)`: an enumeration of `it`, made knowing its back and its
/// length, whose index from the back is where it is (ADR 0528), marked so.
pub(in crate::lower) fn enumerated_knowing(it: Expr) -> Expr {
    let mut e = Expr::call_helper("$enumerate", vec![it]);
    e.facts = e.facts.with(Known::IterEnumerate);
    e
}

/// How an enumeration made here is written (`enumerated_of`): in place,
/// `src.map((x, i) => [i, x])` (`enumerated`), or knowing its back and its
/// length, `$enumerate(src)` (`enumerated_knowing`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum EnumeratedForm {
    Mapped,
    Made,
}

/// An enumeration made here: what it enumerates, and how it's written.
pub(in crate::lower) struct Enumerated<'e> {
    pub(in crate::lower) source: &'e Expr,
    pub(in crate::lower) form: EnumeratedForm,
}

/// What `e`, an enumeration made here, `enumerated`'s or
/// `enumerated_knowing`'s, enumerates, and in which form: the one reader of
/// either (#764).
pub(in crate::lower) fn enumerated_of(e: &Expr) -> Option<Enumerated<'_>> {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if let js::ExprKind::Member(source, method) = &callee.kind
                && method == "map"
                && args.len() == 1
                && args[0] == enumerating() =>
        {
            Some(Enumerated {
                source,
                form: EnumeratedForm::Mapped,
            })
        }
        js::ExprKind::Call(_, args) if e.facts.has(Known::IterEnumerate) && args.len() == 1 => Some(Enumerated {
            source: &args[0],
            form: EnumeratedForm::Made,
        }),
        _ => None,
    }
}

/// `$rev(it)`: `it` stepped from its back (ADR 0516), marked so.
pub(in crate::lower) fn reversed_by(it: Expr) -> Expr {
    let mut e = Expr::call_helper("$rev", vec![it]);
    e.facts = e.facts.with(Known::IterRev);
    e
}

/// What `e`, a `reversed_by` one, steps from its back: `it`.
pub(in crate::lower) fn reversed_of(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(_, args) if e.facts.has(Known::IterRev) && args.len() == 1 => args.first(),
        _ => None,
    }
}

/// Whether `e` is a `reversed_by` one.
pub(in crate::lower) fn is_reversed(e: &Expr) -> bool {
    reversed_of(e).is_some()
}

/// What `e`, a zip made here, is given (`zipped`'s): `$lazyZip(a, b, ..)`'s
/// sides, and what drops each of `a`'s items `b` runs out at (ADR 0465);
/// or `$trustedZip(a, b)`'s, std's read by index (ADR 0973), `by_index`.
pub(in crate::lower) struct ZipParts<'e> {
    pub(in crate::lower) a: &'e Expr,
    pub(in crate::lower) b: &'e Expr,
    pub(in crate::lower) drop_a: Option<&'e Expr>,
    pub(in crate::lower) by_index: bool,
}

pub(in crate::lower) fn zip_parts(e: &Expr) -> Option<ZipParts<'_>> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let by_index = match callee.helper_name() {
        Some("$lazyZip") => false,
        Some("$trustedZip") => true,
        _ => return None,
    };
    let (a, b, drop_a) = match args.as_slice() {
        [a, b] => (a, b, None),
        [a, b, drop_a] => (a, b, Some(drop_a)),
        _ => return None,
    };
    Some(ZipParts { a, b, drop_a, by_index })
}

/// No more items than `e` gives, read of its form alone where it's sure: a
/// range's of literal bounds, a number's, a `u64`'s or a `char`'s, made
/// there (`stepped_range`, `$lazyRange(a, b)`); through what gives no more
/// than its source, a `rev()`, an `enumerate()`, a `map(f)`, a lent one, a
/// `take(n)` of no more than `n` too; a chain's, the sum; a zip's, the
/// fewer. `None` of another. Whether a collect's room surely fits, so it
/// needn't be checked (#1441).
pub(in crate::lower) fn count_bound(e: &Expr) -> Option<i128> {
    let literal = |e: &Expr| -> Option<i128> {
        match &e.kind {
            js::ExprKind::Str(s) if s.chars().count() == 1 => s.chars().next().map(|c| i128::from(u32::from(c))),
            _ => e.as_int().or_else(|| e.as_bigint()),
        }
    };
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    // A range's numbers its maker wrote, stepped by its kind's step, from
    // the front or the back, made knowing its back too.
    if e.facts.has(Known::IterRange)
        && let [range, step, ..] = &args[..]
        && let (js::ExprKind::Object(bounds, ..), Some(next)) = (&range.kind, step.helper_name())
        && let [js::Prop::Field(s, start), js::Prop::Field(f, end)] = &bounds[..]
        && s == "start"
        && f == "end"
    {
        let inclusive = next.contains("Inclusive");
        return Some((literal(end)? - literal(start)? + i128::from(inclusive)).max(0));
    }
    // Of `$lazyRange(a, b, s)`, no more than `b - a`, of any step.
    if let Some((start, Some(end), _)) = crate::lower::std_types::range::lazy_range_parts(e) {
        return Some((literal(&end)? - literal(&start)?).max(0));
    }
    match (callee.helper_name(), &args[..]) {
        (Some("$taking"), [source, n, ..]) => {
            return match (literal(n), count_bound(source)) {
                (Some(n), Some(bound)) => Some(n.min(bound)),
                (n, bound) => n.or(bound),
            };
        }
        (Some("$rev" | "$knowing" | "$enumerate" | "$lent" | "$mapEnds"), [source, ..]) => return count_bound(source),
        (Some("$lazyChain"), [a, b, ..]) => return Some(count_bound(a)? + count_bound(b)?),
        (Some("$lazyZip" | "$trustedZip" | "$zipEnds" | "$trustedZipEnds"), [a, b, ..]) => {
            return match (count_bound(a), count_bound(b)) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
        }
        _ => {}
    }
    let js::ExprKind::Member(source, name) = &callee.kind else {
        return None;
    };
    match (name.as_str(), &args[..]) {
        ("map", [_]) => count_bound(source),
        ("take", [n]) => match (literal(n), count_bound(source)) {
            (Some(n), Some(bound)) => Some(n.min(bound)),
            (n, bound) => n.or(bound),
        },
        _ => None,
    }
}

/// A chain made here, `$lazyChain(a, b)`: its two halves.
pub(in crate::lower) fn chain_parts(e: &Expr) -> Option<(&Expr, &Expr)> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    match (callee.helper_name(), args.as_slice()) {
        (Some("$lazyChain"), [a, b, ..]) => Some((a, b)),
        _ => None,
    }
}

/// `$zipEnds(a, b, dropA, dropB)`, or of a zip `by_index`
/// `$trustedZipEnds`: a zip of sides made knowing their backs and lengths,
/// which knows its own, as std's `Zip` is double-ended and exact-sized
/// where both its sides are; what its back trims of each dropped by its
/// drop (#740). One std folds to the shorter's count of pairs, `SpecFold`'s
/// `TrustedLen` impl, `to_shorter`, so: `$zipEnds(a, b, dropA, dropB,
/// true)` (ADR 0997, #812).
pub(in crate::lower) fn zip_ends(
    by_index: bool,
    a: Expr,
    b: Expr,
    drops: (Option<Expr>, Option<Expr>),
    to_shorter: bool,
) -> Expr {
    let mut args = vec![a, b];
    match drops {
        _ if to_shorter && !by_index => {
            let (drop_a, drop_b) = drops;
            args.extend([
                drop_a.unwrap_or_else(Expr::undefined),
                drop_b.unwrap_or_else(Expr::undefined),
                Expr::bool(true),
            ]);
        }
        (drop_a, Some(drop_b)) => args.extend([drop_a.unwrap_or_else(Expr::undefined), drop_b]),
        (drop_a, None) => args.extend(drop_a),
    }
    Expr::call_helper(if by_index { "$trustedZipEnds" } else { "$zipEnds" }, args)
}

/// `$stepBy(it, n, ..)`: `it` stepped every `n`th item, given `args`, `it`
/// and `n` first (#231), marked so.
pub(in crate::lower) fn stepped_every(args: Vec<Expr>) -> Expr {
    let mut e = Expr::call_helper("$stepBy", args);
    e.facts = e.facts.with(Known::IterStepBy);
    e
}

/// What a `stepped_every` one is given, `it` and `n` first.
pub(in crate::lower) fn stepped_every_parts(e: &Expr) -> Option<&[Expr]> {
    match &e.kind {
        js::ExprKind::Call(_, args) if e.facts.has(Known::IterStepBy) => Some(args.as_slice()),
        _ => None,
    }
}

/// `$stepByEnds(it, n)`: `it` stepped every `n`th item from either end,
/// knowing its back and its length, as std's `StepBy` of an exact-sized
/// source does (#718), marked made knowing its back.
/// Of items with a destructor, given `drop`, by which it drops each it
/// passes over where its source drops none itself (#823).
pub(in crate::lower) fn stepped_by_ends(it: Expr, step: Expr, drop: Option<Expr>) -> Expr {
    let mut args = vec![it, step];
    args.extend(drop);
    made_knowing_its_back(Expr::call_helper("$stepByEnds", args))
}

/// What a `stepped_by_ends` one steps, and by how many.
pub(in crate::lower) fn stepped_by_ends_parts(e: &Expr) -> Option<(&Expr, &Expr)> {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if was_made_knowing_its_back(e) && callee.helper_name() == Some("$stepByEnds") =>
        {
            match args.as_slice() {
                [it, step, ..] => Some((it, step)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `$repeatingN(x, n, ..)`: `repeat_n(x, n)`'s items (ADR 0513), given
/// `args`, `x` and `n` first, marked so.
pub(in crate::lower) fn repeating_n(args: Vec<Expr>) -> Expr {
    let mut e = Expr::call_helper("$repeatingN", args);
    e.facts = e.facts.with(Known::IterRepeatingN);
    e
}

/// What a `repeating_n` one is given, `x` and `n` first.
pub(in crate::lower) fn repeating_n_parts(e: &Expr) -> Option<&[Expr]> {
    match &e.kind {
        js::ExprKind::Call(_, args) if e.facts.has(Known::IterRepeatingN) => Some(args.as_slice()),
        _ => None,
    }
}

/// What `e`, an `$iterator` `emit()` wrote (`Form::Own` or `Form::Range`),
/// is given, its value and its step first: what's asked of its own steps,
/// `nth` or `size_hint` (`behind_a_dyn`, tidy's `stepped_in_place`), and of
/// a range's step, from either end. Not `$given`'s.
pub(in crate::lower) fn iterator_parts(e: &Expr) -> Option<&[Expr]> {
    match &e.kind {
        js::ExprKind::Call(_, args) if e.facts.has(Known::IterOwn) || e.facts.has(Known::IterRange) => {
            Some(args.as_slice())
        }
        _ => None,
    }
}

/// Whether `e` is a range's numbers, an `$iterator` stepped by a range's
/// step, however it's stepped and whatever it's given beside: one std's
/// steps give what its defaults do, which a `Rev` hides nothing of.
pub(in crate::lower) fn is_range_stepped(e: &Expr) -> bool {
    iterator_parts(e)
        .and_then(|args| args.get(1))
        .and_then(Expr::helper_name)
        .is_some_and(|next| next.starts_with("$range"))
}

/// `source`, the value a map whose function gives each item as it is,
/// `map(|x| Some(x))`'s, is as it was (`mir/iter.rs`), said so, so what
/// makes the map knowing its back or its length makes its source so
/// (`is_mapped_as_it_is`, #1342).
pub(in crate::lower) fn mapped_as_it_is(mut source: Expr) -> Expr {
    source.facts = source.facts.with(Known::IterMappedAsItIs);
    source
}

/// Of `e`, a map's source as it was (`mapped_as_it_is`), that source, as
/// the map was given it: not the marks its maker gave the map, made in
/// place (`Known::InPlace`), which a map's own call holds where it's
/// written, `src.map(f)`.
pub(in crate::lower) fn source_mapped_as_it_is(e: &Expr) -> Option<Expr> {
    let mut source = e.clone();
    source.facts = (source.facts.without(Known::IterMappedAsItIs)).without(Known::InPlace);
    e.facts.has(Known::IterMappedAsItIs).then_some(source)
}

/// Whether `e` is std's compiled struct given what a use site knows,
/// `$knowing(it, ..)` (`Form::Knowing`).
pub(in crate::lower) fn is_knowing(e: &Expr) -> bool {
    e.facts.has(Known::IterKnowing)
}

/// Whether `e`, `$knowing(it, back, len, ..)`, is given its length, `len`:
/// std's struct, given its compiled `len` where it's asked (#814).
pub(in crate::lower) fn knows_its_given_length(e: &Expr) -> bool {
    is_knowing(e)
        && matches!(&e.kind, js::ExprKind::Call(_, args) if args.get(2).is_some_and(|len| *len != Expr::undefined()))
}

/// `e`, a `rev()` its maker made knowing its back (`reversed_knowing`),
/// marked so: what's asked of it past its maker reads the mark, not the
/// type's set (back_stepped, ADR 0545).
pub(in crate::lower) fn made_knowing_its_back(mut e: Expr) -> Expr {
    e.facts = e.facts.with(Known::IterKnowsBack);
    e
}

/// `$held(e)`, an iterator its owner drops held (#206): the same iterator,
/// so what's known of its value, not of its JS's form, is kept on it, made
/// knowing its back (`made_knowing_its_back`) or its length
/// (`made_knowing_its_length`) and made in place (`made_in_place`).
pub(in crate::lower) fn held_by_its_owner(e: Expr) -> Expr {
    let (knows_back, in_place) = (was_made_knowing_its_back(&e), super::made_in_place(&e));
    let knows_length = was_made_knowing_its_length(&e);
    let held = Expr::call_helper("$held", vec![e]);
    let held = match knows_back {
        true => made_knowing_its_back(held),
        false => held,
    };
    let held = match knows_length {
        true => made_knowing_its_length(held),
        false => held,
    };
    match in_place {
        true => held.in_place(),
        false => held,
    }
}

/// What `e`, `held_by_its_owner`'s, holds: `it` of `$held(it)`.
pub(in crate::lower) fn held_of(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, of) if callee.helper_name() == Some("$held") && of.len() == 1 => Some(&of[0]),
        _ => None,
    }
}

/// Whether `e` was made knowing its back (`made_knowing_its_back`).
pub(in crate::lower) fn was_made_knowing_its_back(e: &Expr) -> bool {
    e.facts.has(Known::IterKnowsBack)
}

/// `e`, a map its maker made knowing its length, of what knows it, marked
/// so: what's asked of it past its maker, its `len()` of a variable that
/// holds it, reads the mark, not the type (#851).
pub(in crate::lower) fn made_knowing_its_length(mut e: Expr) -> Expr {
    e.facts = e.facts.with(Known::IterKnowsLength);
    e
}

/// Whether `e` was made knowing its length (`made_knowing_its_length`).
pub(in crate::lower) fn was_made_knowing_its_length(e: &Expr) -> bool {
    e.facts.has(Known::IterKnowsLength)
}

/// Whether `e`'s JS steps from its back, by its form (#851): what's marked
/// so, `$knowing`'s, an array's `$iter`, a reversal's, a zip's or a
/// flatten's ends; an adapter of a source that does, a map's, a filter's,
/// an enumeration's, a `skip(n)`'s or a `take(n)`'s, and what's lent,
/// closed or held of one; a chain of sides that do; the crate's own, or a
/// type parameter's value, given its `next_back`. The one answer of a value's form, which what asks it
/// reads where its type says it may (`has_a_back`): never its type alone,
/// which another value of it need not share.
pub(in crate::lower) fn knows_its_back_by_form(e: &Expr) -> bool {
    // An array's, `$iter(v)`, its owner's to say (`fresh_iter_source`).
    if was_made_knowing_its_back(e) || is_knowing(e) || super::fresh_iter_source(e).is_some() {
        return true;
    }
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return false;
    };
    match callee.helper_name() {
        Some("$rev" | "$zipEnds" | "$trustedZipEnds" | "$flattenEnds" | "$chars" | "$charIndices") => true,
        Some(
            "$mapEnds" | "$filterEnds" | "$enumerate" | "$lent" | "$closing" | "$held" | "$taking" | "$skipping"
            | "$stepByEnds",
        ) => args.first().is_some_and(knows_its_back_by_form),
        Some("$lazyChain") => matches!(&args[..], [a, b, ..] if knows_its_back_by_form(a) && knows_its_back_by_form(b)),
        Some("$iterator" | "$given") => args.get(3).is_some_and(|back| *back != Expr::undefined()),
        _ => false,
    }
}

/// What `e` steps, where its back is its source's: of what holds or lends
/// it as it is, `$held`'s, `$lent`'s and `$closing`'s, the
/// value of its own type (`true`); of an adapter, `$mapEnds`'s,
/// `$filterEnds`'s, `$enumerate`'s, `$taking`'s, `$skipping`'s or
/// `$stepByEnds`', its source's (`false`) (#851).
pub(in crate::lower) fn back_source(e: &Expr) -> Option<(&Expr, bool)> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let same = match callee.helper_name()? {
        "$held" | "$lent" | "$closing" => true,
        "$mapEnds" | "$filterEnds" | "$enumerate" | "$taking" | "$skipping" | "$stepByEnds" => false,
        _ => return None,
    };
    args.first().map(|source| (source, same))
}

/// Whether `e`'s form says it has no back (#851): an adapter of one whose
/// back is its source's, of what lacks one. (JS's own iterator helper,
/// `.map(f)`, never reaches what asks it of a form: what's stepped from its
/// back is made knowing it where it's made, #863.) (Std's
/// compiled struct not given its compiled back is the lowering's question
/// of its type, `compiled_struct`.) Not what its form doesn't say, a variable's,
/// a field's, a function's result: what its maker made it, which the type
/// says (`has_a_back`).
pub(in crate::lower) fn form_lacks_a_back(e: &Expr) -> bool {
    if knows_its_back_by_form(e) {
        return false;
    }
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return false;
    };
    match callee.helper_name() {
        Some("$mapEnds" | "$filterEnds" | "$enumerate" | "$lent" | "$closing" | "$held" | "$taking" | "$skipping") => {
            args.first().is_some_and(form_lacks_a_back)
        }
        _ => false,
    }
}

/// A range's numbers as its maker wrote them (`Form::Range`), `$iterator({
/// start: a, end: b }, $rangeNext)`: its bounds, made where it's stepped,
/// whether its end is in it, `a..=b`'s, whether it's stepped from its end,
/// and its step.
pub(in crate::lower) struct SteppedRange<'e> {
    /// The range itself, `{ start: a, end: b }`.
    pub(in crate::lower) range: &'e Expr,
    pub(in crate::lower) start: &'e Expr,
    pub(in crate::lower) end: &'e Expr,
    pub(in crate::lower) step: &'e Expr,
    pub(in crate::lower) inclusive: bool,
    pub(in crate::lower) back: bool,
}

/// `e`'s range, where it's a range's numbers its maker wrote
/// (`Known::IterRange`), of an `a..b` or an `a..=b` made there, stepped from
/// either end by its kind's step (`RangeKind::step`). `None` of another, or
/// of a range a variable holds.
pub(in crate::lower) fn stepped_range(e: &Expr) -> Option<SteppedRange<'_>> {
    if !e.facts.has(Known::IterRange) {
        return None;
    }
    let js::ExprKind::Call(_, args) = &e.kind else {
        return None;
    };
    let [range, step] = &args[..] else {
        return None;
    };
    let (js::ExprKind::Object(bounds, ..), Some(next)) = (&range.kind, step.helper_name()) else {
        return None;
    };
    let [js::Prop::Field(s, start), js::Prop::Field(f, end)] = &bounds[..] else {
        return None;
    };
    let (inclusive, back) = [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .find(|&(inclusive, back)| {
            let kind = if inclusive {
                RangeKind::Inclusive
            } else {
                RangeKind::Exclusive
            };
            kind.step(back) == Some(next)
        })?;
    ((s.as_str(), f.as_str()) == ("start", "end")).then_some(SteppedRange {
        range,
        start,
        end,
        step,
        inclusive,
        back,
    })
}

/// What `it`, a range's numbers, steps, its bounds, as the runtime's
/// iterator holds them, `it.of` (ADR 0545's #721 note): what a range's own
/// `nth`, `last` and `count` are of, of one stepped from its end.
pub(in crate::lower) fn bounds_of(it: Expr) -> Expr {
    Expr::member(it, "of")
}

/// The range a `bounds_of` reads, of a range's numbers made there: what a
/// read of its bounds is, the bounds themselves.
pub(in crate::lower) fn bounds_read(e: &Expr) -> Option<SteppedRange<'_>> {
    match &e.kind {
        js::ExprKind::Member(made, field) if field == "of" => stepped_range(made),
        _ => None,
    }
}

/// Of `(s..).step_by(n)`, made where it's stepped, `$stepBy($iterator({
/// start: s }, $rangeFromStep(..)), n)`: `s` and `n`, a whole number from
/// 1 up, which every step adds (rd260); of `s..` itself, `$iterator({
/// start: s }, $rangeFromStep(..))`, `s` and 1.
pub(in crate::lower) fn stepped_from(e: &Expr) -> Option<(&Expr, i128)> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let (made, n) = match (callee.helper_name(), &args[..]) {
        (Some("$stepBy"), [made, n]) => (made, n.as_int().filter(|&n| n >= 1)?),
        (_, [_, _]) if e.facts.has(Known::IterRange) => (e, 1),
        _ => return None,
    };
    let js::ExprKind::Call(_, parts) = &made.kind else {
        return None;
    };
    let ([range, next], true) = (&parts[..], made.facts.has(Known::IterRange)) else {
        return None;
    };
    let js::ExprKind::Call(stepper, _) = &next.kind else {
        return None;
    };
    let (js::ExprKind::Object(bounds, ..), Some("$rangeFromStep")) = (&range.kind, stepper.helper_name()) else {
        return None;
    };
    let [js::Prop::Field(s, start)] = &bounds[..] else {
        return None;
    };
    (s == "start").then_some((start, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str) -> Expr {
        Expr::var(name)
    }

    #[test]
    fn each_form_is_written_and_marked() {
        let range = IterValue::of(Form::Range {
            range: var("r"),
            step: Expr::helper("$rangeNext"),
        })
        .emit();
        assert!(matches!(&range.kind, js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$iterator") && args.len() == 2));
        assert!(range.facts.has(Known::IterStepped) && range.facts.has(Known::IterRange));
        let closed = IterValue::js(var("it")).closed(var("drop"), true).lent().emit();
        assert!(closed.facts.has(Known::IterLent) && !closed.facts.has(Known::IterClosing));
        let js::ExprKind::Call(_, inner) = &closed.kind else {
            panic!("a call")
        };
        assert!(inner[0].facts.has(Known::IterClosing));
        let js::ExprKind::Call(_, parts) = &inner[0].kind else {
            panic!("a call")
        };
        assert_eq!(parts.len(), 3);
    }
}
