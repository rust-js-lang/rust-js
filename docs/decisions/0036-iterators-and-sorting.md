# 0036. An iterator is a JS array; `Ordering` is -1, 0 or 1

Status: Accepted; a chain whose stages do what can be seen is lazy ([0139](0139-lazy-chains.md)). Extends [0025](0025-vec-loops-refcell-mut.md), where a
`Vec` is a JS array and `for` loops over one. Amended: a fallible `collect()` of any collection (below); a range whose end may panic, beside a drop, is a `for` (below). Amended by
[0986](0986-len-of-an-iterator-made-in-place.md): `len()` of a chain whose
closures do what can be seen is its source's, counted through its
adapters, which runs none of them.

Case: B, A, C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust code works on collections through iterator chains, such as
`v.iter().filter(..).map(..).collect()`, and sorts with `sort`, `sort_by` and
`sort_by_key`. rust-js had only `for` loops and a few `Vec` methods. The
playground's file trees are sorted lists, built and walked with these.

A Rust iterator is lazy: nothing happens until something consumes it, one
element at a time, through the whole chain. JS arrays have the same
vocabulary (`map`, `filter`, `some`, `every`, `find`, `reduce`), but each step
runs over the whole array at once. ReScript's `Array` module wraps JS's array
methods, and Scala.js's collections are Scala's own.

## Decision

**An iterator is a JS array, and its adapters are the array's methods:**

| Rust | JS |
|---|---|
| `v.iter()`, `v.into_iter()`, `.copied()`, `.cloned()` | `v` |
| `(a..b)` as an iterator | `$range(a, b)` |
| `s.chars()` | `Array.from(s)` (by code point) |
| `.map(f)`, `.filter(p)`, `.find(p)`, `.for_each(f)` | the same methods |
| `.any(p)`, `.all(p)` | `.some(p)`, `.every(p)` |
| `.enumerate()` | `.map((x, i) => [i, x])` |
| `.rev()`, `.skip(n)`, `.take(n)` | `.toReversed()`, `.slice(n)`, `.slice(0, n)` |
| `.fold(init, f)`, `.sum()` | `.reduce(f, init)`, `.reduce((a, b) => a + b, 0)`, wrapped (ADR 0011) |
| `.count()`, `.last()` | `.length`, `.at(-1)` |
| `.len()` of an `ExactSizeIterator` | `.length`, or `it.items.length - it.at` of one stepped through (ADR 0055): what it has left. It takes no item, so runs no closure: one of a chain whose closures do what can be seen, and a `Peekable`'s, are errors. |
| `.position(p)`, `.max()`, `.min()` | `$position`, `$max`, `$min`: options (ADR 0030) |
| `.collect::<Vec<_>>()`, `.collect::<String>()` | a new array (an adapter's result already is one; otherwise `.slice()`), `.join("")` |
| `.collect::<Result<Vec<_>, _>>()`, `.collect::<Option<Vec<_>>>()` | `$collectResults(items)`, `$collectOptions(items)`: the first `Err` or `None`, where it stops, or the values. A chain whose stages do what can be seen runs lazily up to it (ADR 0139). Into a `Result` or an `Option` of anything but an array is an error. (Amended: it was the array, unwrapped.) |

**Sorting sorts in place, with a comparator**, and JS's sort is stable, as
Rust's `sort` is:

| Rust | JS |
|---|---|
| `v.sort()` on numbers | `v.sort((a, b) => a - b)`, since JS's own `sort()` compares as strings |
| `v.sort()` on strings or `bool`s | `v.sort()` |
| `v.sort_by(f)` | `v.sort(f)` |
| `v.sort_by_key(k)` | `v.sort((a, b) => $cmp(key(a), key(b)))` |
| `v.reverse()`, `v.to_vec()` | `v.reverse()`, `v.slice()` |
| an array's `a.map(f)`, a new array (amended) | `a.map(f)`, of a copy of `a` where its items are changed in place (ADR 0020) |

**`Ordering` is -1, 0 or 1**, its discriminant (`#[repr(i8)]`), not a string
as other fieldless enums are (ADR 0013). That's what a JS comparator returns,
so `sort_by(|a, b| b.cmp(a))` is `sort((a, b) => $cmp(b, a))`. `a.cmp(&b)` of
numbers, strings, `char`s and `bool`s is `$cmp(a, b)`. `then(o)` and
`then_with(f)` are `||`, since `Equal` is 0, the one that's falsy, and
`reverse()` is `-o`. `a.max(b)` and `a.min(b)` of numbers are `Math.max` and
`Math.min`.

Two things came with it:
- **An operator on references to numbers** (`x % 10` with `x: &i32`, which
  rustc writes as a call of `Rem::rem`) is the operator.
- **A closure's `|&x|` or `|&&x|` parameter is just `x`**, and a closure's own
  names are free again after it, so sibling closures can each have an `x`.

## Why

- **The JS reads as JS programmers write it**: `v.filter((x) => x % 2 === 0)`.
- **A chain that `collect`s, `count`s or `sum`s**, which is what programs do
  with iterators, gives the same answer either way.
- **Sorting numbers right** is where JS surprises people: `[10, 9].sort()` is
  `[10, 9]`. The comparator is always there for numbers.

## Alternatives

- **JS iterators** (`Iterator.prototype.map` and friends): lazy, like Rust's,
  but newer than the array methods, and a chain would end in `.toArray()`.
- **Generators and a helper per adapter**: faithful, but every chain would be
  helper calls.
- **`Ordering` as `"Less"`, `"Equal"`, `"Greater"`**, like other enums: then
  every comparator would need a helper to turn the name into a number.

## Consequences

- **Every step runs over the whole array before the next.** A chain whose
  closures have effects runs them in a different order than Rust would: all
  of `map`'s calls, then all of `filter`'s. So does a chain that `find`s:
  Rust stops at the first match, and JS has run earlier steps on every
  element. Infinite iterators (`0..`) don't work.
- `$cmp` on strings compares code points, as Rust does (ADR 0183). (Amended:
  it was UTF-16 units, as `<`.)
- `a..=b` includes its end: `for (let i = a; i <= b; i++)`, and
  `$range(a, b + 1)` as an iterator. A search that takes its iterator by `&mut`,
  `(0..n).all(f)` or `any`, `find` or `position`, is of the range it
  borrows. Found in a rustc test: it had been called on `{ start, end }`.
- Sorting by `Ord` came with ADR 0057, and an iterator of the crate's own,
  lazy and possibly endless, with ADR 0055.
- `filter_map`, `flat_map`, `zip`, `chain` and more came with ADR 0062.
- Not yet: `peekable`, `next()` on a held iterator, ranges in variables,
  and `binary_search`.

## Since

- `for c in text.chars()` is `for (const c of text)`: JS steps through a
  string by code point, as `Array.from` does, and a string can't change
  while the loop runs, so the copy shows nothing (case A). A `chars()`
  passed on or collected is still `Array.from(text)`. Found porting
  react.dev's `createFileMap`.
- `items.iter().any(|t| t == value)` is `items.includes(value)`, of
  strings, integers, `bool`s and fieldless enums, where `===` and
  `includes` agree (case A). Not of floats: `includes` finds NaN, which
  `==` never does (case C, `some` stays). The value is read once, so it
  must be a variable, a property or a constant. Found porting react.dev's
  `createFileMap`, `tokens.includes('hidden')`.

## Since

- **An unstable sort is JS's `sort` only where its ties can't be told
  apart** (ADR 0342): of numbers, text, `bool`s and `char`s, and tuples
  and arrays of them. Elsewhere, and for `select_nth_unstable`, it's std's
  own algorithm, as its order of equal items is std's.

## Amendment: a fallible `collect()` of any collection

`collect()` into a `Result` or an `Option` was of an array alone, so
`Result<BTreeMap<_, _>, _>`, the way a parser of `k=v` pairs collects
them, was refused. The values, an array, are made into the collection
the way a plain `collect()` makes one, `collected_into`, one method for
both: `$collectResults(items, (values) => new Map(values))`, `values.join("")`
of text, a heap's or a set's. A collection of the crate's own, its
`FromIterator`, is made by its own `from_iter`, the function rust-js
compiled, given the values, `$collectResults(items, bagFromIter)`; one
of std's rust-js doesn't know, an `Rc<[T]>`'s, is refused.
`collect_fallible_into` runs each against native Rust.

## Amendment: `sum()` and `product()` into an `Option` or a `Result`

`sum()` and `product()` into an `Option` or a `Result` of numbers were
refused, std's `Sum` impls having no dictionary here (#117). They're
`collect()` into one, the first `None` or `Err` where stepping stops, of
which the numbers' sum is what's made of them:
`$collectOptions(it, false, (values) => values.reduce(..))`. Held by
`test/corpus/sum_fallible.rs`.

## Amendment: `is_sorted()`

`is_sorted()` of an iterator was refused (#117). It's a slice's, one
answer, `is_sorted_call`: `$isSortedBy(items, (a, b) => a <= b)` of
numbers, JS's `<=` being `false` of a NaN as `partial_cmp`'s is, else their
`partial_cmp` at most `Equal`. `$isSortedBy` steps any iterable, stopping
at the first pair out of order, so an iterator's later items aren't made.
(A first cut gave slices a second form, `$isSorted`, beside the one they
had; it's gone.) Held by `test/corpus/is_sorted_items.rs` and
`test/corpus/slice_methods.rs`.

## Amendment: comparing two iterators

`cmp`, `partial_cmp`, `lt`, `le`, `gt` and `ge` of two iterators were
refused (#117). They're `$iterCmp(a, b, cmp)`, which steps both in turn as
std's `cmp_by` does, `a` before `b`, to the first `Ordering` of two that
isn't `Equal`, or which ends first; given `partial_cmp`, `undefined` where
two items have none, `None`. `lt` and the rest are it against 0,
`$iterCmp(..) < 0`, which `undefined` fails, as std's are `false` of it.
Items with a destructor, which it doesn't drop as it compares them, are
refused. Held by `test/corpus/iter_compare.rs`.

## Amendment: `std::iter::zip`

`std::iter::zip(a, b)`, `a.into_iter().zip(b)`, was refused (#117). It's
`$lazyZip` of each's items as `zip`'s argument's are made (ADR 0528): an
array's as it is, a B-tree's in order, a range's stepped, an `Option`'s
`[]` or `[x]`. Of items with a destructor, which `zip` drops where one side
runs out, it's refused. Held by `test/corpus/zip_function.rs`.

## Amendment: `collect()` into `()` (#322)

`collect()` into `()` was refused: TheAlgorithms/rust's autocomplete trie,
`.map(|s| sub_result.push(c.to_string() + s)).collect()` in a branch of
`()`. std's `impl FromIterator<()> for ()` is `iter.for_each(|()| {})`, so
**it steps each item, `it.forEach(() => {})`**, and of a `map(g)`, whose
items it does nothing with, tidy writes `forEach(g)`: `rest.forEach((s) =>
{ out.push(prefix + s); })`. Held by the corpus case `collect_into_unit`
and the mutations `collect-into-unit-refused` and
`map-for-each-nothing-kept`.


## Amendment: an inclusive range's loop counts (#332)

`for i in a..=n` of a variable `n` had become `while ((i =
$rangeInclusiveNext(iter)) != null)` once ranges were `{ start, end }`
values, where this decision has `for (let i = a; i <= n; i++)`: tidy's
`for_of` counted `$rangeNext`'s loop alone. Found porting
trekhleb/javascript-algorithms' `sieveOfEratosthenes`. **It counts again,
`i <= end`**, under the rules `a..n`'s loop has: a number start (a `char`'s
range steps by its code point), and an end the body can't change, or held
in a `const end` first. No observable difference (ADR 0262): a JS number
doesn't wrap, so the count passes `n`, even a type's largest, `255` or
`4294967295`, only at the test that ends the loop, as the helper's own
`range.start++` does. Held by the corpus case `inclusive_range_loops`,
`lowering-loops.test.ts`, and the mutations
`mir-tidy-for-inclusive-range-kept` and
`mir-tidy-for-inclusive-range-exclusive`.

## Amendment: a range from arithmetic counts (#368)

`for j in i + 1..m.len()`, every pairwise algorithm's inner loop, stayed
`const iter = { start: i + 1, end: m.length }; while ((j =
$rangeNext(iter)) != null)`: tidy counted a range only from a literal or a
numeric variable, lest `++` step a `char`'s range, which is text. A start
of arithmetic is an integer's, as a `char` has none, so it counts: `for
(let j = i + 1; j < m.length; j++)`. Where the end is held first, `const
end = m.length`, read before a start Rust reads first, the start reads
only variables and the end has no effects, which leaves them as they are
(defensive: lowering already spills an end of effects, `bump(&mut k)`,
before the loop). Held by `lowering-written-loops.test.ts`'s "a range
from arithmetic is a counting for" and the mutations
`range-of-arithmetic-uncounted` and `held-end-of-computed-start-refused`.

## Amendment: a range whose end may panic, beside a drop

`for k in 0..id(n)` where a value is held whose drop runs if `id(n)`
panics, a value reassigned in the loop say, stayed `let iter; try { iter =
{ start: 0, end: id(n) }; } catch (error) { drop; throw error; }` then
`while ((k = $rangeNext(iter)) != null)`: MIR makes the range in the
`try` of the call's cleanup, which `loops` didn't read as a range made.
Now its end is held first in that `try`, `let end; try { end = id(n); }
catch (error) { drop; throw error; }`, and the range of it is counted, `for
(let k = 0; k < end; k++)`: the same calls, in the same order, the same
drop where the end panics. Of a constant start only, which reads the same
after the end as before. The held end's name is one the function doesn't
use, so a program's own `end` keeps its own. An inclusive range's step,
`$rangeInclusiveNext` and `$rangeInclusiveNextBack`, never throws, as
`$rangeNext` doesn't (ADR 0901's `NEVER_THROW`), so its loop's step needs
no `try` of its own either. No observable difference (ADR 0262, case A).
Held by the corpus case `drop_reassigned_in_inner_loop`, run against
native Rust, and the mutations `held-range-end-kept`,
`held-range-end-named-end` and `inclusive-range-step-may-throw`.
