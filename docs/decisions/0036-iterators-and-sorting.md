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

## Amendment: a held end beside a variable start

`for k in s..id(n)` where a value is held whose drop runs if `id(n)`
panics kept MIR's `try { iter = { start: s, end: id(n) }; } catch ..`
and `while ((k = $rangeNext(iter)) != null)`, as the held end above was of
a constant start only. A variable's reads the same after the end as
before where nothing the end runs can set it: one no closure, function or
`&mut`'s handle of the body names (`tidy::bindings::captured`), JS's only
ways for a call to set a variable of the caller's. Such a start is held
beside the end too, `let end; try { end = id(n); } catch ..` then `for
(let k = s; k < end; k++)`; one a closure may set, `let mut bump = || s
+= 1;`, keeps MIR's range, read where Rust reads it. No observable
difference (ADR 0262, case N). Held by the corpus case
`drop_reassigned_in_inner_loop` (`from_start`, an empty range too;
`from_bumped`) and the mutations `held-end-of-captured-start` and
`held-end-of-variable-start-unmade` (`mir/tidy/loops.ts`).

## Amendment: a loop that returns in a branch that returns after it (rd602, rd603)

`if ok { for step in path { if lazy(value) { return wait(*step); } .. }
return value; }`, React's Flight client's getOutlinedModel walking its
path, `for i in 1..path_len` with a `panic!` in it too, kept MIR's loop,
`while ((step = iter.next().value) != null)` and `const iter = { start:
1, end: pathLen }; .. $rangeNext(iter)`: when `loops` runs, each way out
of the loop is `result = ..; break bb95;`, a block's `break` to a
`return result;` past the `if` the loop is in, as no `return` is made
yet of a value given in a branch; `for_of` takes a loop only where its
end leaves, which a `break` to a block outside it doesn't show.
**`loops` runs again right after `returned in branches`** ("returned
loops", `stages::for_loops`), which makes those `break`s the `return`s
they go to: then the loop's end leaves by its `return`, and it's `for
(let i = 0; i < path.length; i++)`, `for (let i = 1; i < pathLen; i++)`,
as upstream. Nothing else changes: the same steps, in the same order, a
panic thrown where Rust's is, with its message, and what's held dropped
as it was (case N, ADR 0262). Held by the corpus's
`for_returning_in_an_if`, run against native Rust, and the mutation
`returned-loops-off` (`mir/tidy/stages.ts`).

## Amendment: an iterator's `is_sorted_by` and `is_sorted_by_key` (#763)

A slice's `is_sorted_by` and `is_sorted_by_key` were compiled; an
iterator's were refused, "calling `std::iter::Iterator::is_sorted_by`".
They're std iterator methods from MIR now, beside `is_sorted`:
`is_sorted_by(f)` is `$isSortedBy(it, f)`, `f` of each two in a row up to
the first out of order, as std's steps them; `is_sorted_by_key(f)` is
std's `self.map(f).is_sorted()`, `$isSortedBy(it.map(f), ..)`, each item's
key made once and compared by its `PartialOrd`. A key with a destructor
is refused, "`is_sorted_by_key` of a key with a destructor": valid Rust,
whose keys std drops as each is passed, which `$isSortedBy` of a JS
iterator's `map` doesn't; left to do. A function that holds what has a destructor drops where std's
does: `is_sorted_by`'s before the iterator, as its `all` drops it, given
to `$isSortedBy` as what drops with it (`consumed_function`, ADR 0470);
`is_sorted_by_key`'s after, as the `Map` it asks drops its fields.
`test/corpus/iterator_is_sorted_by.rs` checks them against native Rust in
both builds; mutations `is-sorted-by-ends-lost`, `is-sorted-by-unconsumed`,
`is-sorted-by-key-unmapped` and `is-sorted-by-ends-unrun`.

## Amendment: a key with a destructor drops where std drops it (2026-11-05, #776, #783)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. For an iterator the key was a refusal (#776). A
slice dropped no key at all, a wrong answer (#783, P0).

An iterator's `is_sorted_by_key(f)` of a key with a destructor was refused.
A slice's, `[1u32, 3, 2].is_sorted_by_key(|x| K(*x))`, compiled and dropped
no key. Std's `is_sorted_by` (`core/src/iter/traits/iterator.rs`) keeps
the item before as `last`, and its `check` closure ends each step with
`*last = curr`, which drops the key it replaces. Where two are out of
order it returns `false`, dropping `curr`. Then `all` drops its closure,
`is_sorted_by` drops `last` and then `self`, what's left of the `Map`. A
slice's is `self.iter().is_sorted_by_key(f)`, the same steps.

`$isSortedBy(items, f, ends, drop)` takes the items' drop last, so no call
without one changes. Given one, it steps by hand rather than by `for..of`,
whose `return` closes the iterator before a `finally` could drop `last`:
- each key replaced is dropped as `*last = curr` drops it. If that drop
  panics, `last` is already `curr`, so it is the one dropped as it unwinds;
- where two are out of order, `curr` is dropped, then `ends`, then `last`,
  then what's left of `items` is closed, by `$inTurn`;
- where `f` or a step panics, it goes the same way, `curr` first where `f`
  was given one.

`is_sorted_call` gives it the key's drop, `std_drop_function`'s, from both
the iterator's `sorted_called` and the slice's `SliceOp::IsSortedByKey`. A
slice's `$isSortedByKey(v, f, cmp, drop)` given a drop is `$isSortedBy` of
its keys, `v.values().map(f)`. `check_leaves_none` counts `is_sorted_by_key`
among the methods that move each item on to their function, as `map` does,
so a `Vec`'s items with a destructor of their own compile too.

`test/corpus/keys_with_a_destructor_drop_where_std_drops_them.rs` checks
this against native Rust in both builds:
- the crate's iterator, a slice's iterator, a `Vec`'s items with a
  destructor, and slices;
- sorted, unsorted and empty;
- a key's function, the keys' comparison, and a replaced key's drop, each
  panicking part way.

Mutations: `sorted-dropping-replaced-kept`, `sorted-dropping-curr-kept`,
`sorted-dropping-replaced-before-set`, `slice-sorted-by-key-undropped`
(`runtime/slice_ops.ts`), `is-sorted-by-key-drop-unpassed`
(`lower/mir/iter/collect.ts`), `slice-is-sorted-by-key-drop-unpassed`
(`lower/std_types/slice.ts`) and `is-sorted-by-key-leaves-none`
(`lower/mir/iter/closing.ts`). The guide's limitations no longer list it.
Kind: missing case, both. Area: iterators for #776, drops-unwind for #783.

## Amendment: a stable sort whose calls can be seen is std's driftsort (2026-11-08, #925, #907)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept where they can; JS's `sort` stays where nothing
can tell the two apart (case "none").

`sort`, `sort_by` and `sort_by_key` were JS's `sort`, which is stable as
Rust's is, but asks its comparator another sequence of calls. A comparator,
an `Ord` or a key function that prints, counts or panics saw them:
`[5, 3, 8, 1, 9, 2, 7].sort_by(..)` called `cmp 3 5, cmp 8 5, cmp 1 8 ..`
natively and `cmp 3 5, cmp 8 3, cmp 8 5 ..` as JS; and a panic left
another order, whose drops followed it (#907's 66 unwind-table rows).

**Where a sort's calls can be seen, it's `$sortStable(v, isLess, kind,
size)`, std's driftsort step for step** (`core::slice::sort::stable`,
rustc 1.99): insertion sort up to 20; else runs found or made (`create_run`,
`min_good_run_len`), merged lazily as powersort's tree has them
(`merge_tree_depth`), each unsorted one quicksorted stably into scratch
(`stable_partition`, std's pivot and its left ancestor), eager driftsort
where the pivots are bad too often, and its small sort, `"general8"` or
`"general"` (`small_sort_general` of a `Freeze` item of at most 16 bytes,
or more) or `"fallback"` (insertion, of one that isn't `Freeze`, which also
passes no ancestor pivot on). `size` sizes std's scratch, `max(len - len /
2, min(len, 8 MB / size), 48)`, which decides whether two unsorted runs are
merged lazily; a zero-sized item's sort asks nothing. `is_less` is std's:
`T::lt`, `compare(a, b) == Less`, `f(a).lt(&f(b))`, as `sort_is_less` writes them for the unstable sort. Its guards are std's:
`merge`'s `MergeState` writes what's left of its copy back as a comparison
panics, a partition leaves the slice as it was, and the insertion and small
sorts' (ADR 0342's #908) are shared with the unstable sort.

```rust
people.sort_by(|a, b| a.name.cmp(&b.name)); // JS's sort: nothing seen
loud.sort(); // `Ord` that prints
```

```js
people.sort((a, b) => $cmp(a.name, b.name));
$sortStable(loud, (a, b) => loudOrd_cmp(a, b) < 0, "general8", 8);
```

**Whether a sort's calls can be seen has one owner each**:
- `order_seen(T)` (src/lower/ordering.rs), `order_may_panic`'s shape with
  "seen" for "may panic" (`order_may`, now the one walk of both): an impl
  of the crate's or a library's any of whose methods isn't quiet, a derived
  one as its fields', std's types as what they hold, a type parameter's, a
  `dyn`'s or an opaque type's always.
- `call_seen(F)`, the quiet functions (`analysis/unwinding.rs`'s `quiet`,
  built on ADR 0902's `steps_on`): the crate's functions and closures that
  never unwind, print nothing, write nothing but their own locals (no place
  through a reference: a captured `&mut`, a `static mut`), and call only
  quiet ones or std's `std_quiet` ones: comparisons and `==` of unseen
  orders and equalities (`eq_may`, the same walk of `PartialEq`),
  `Ordering`'s `then`, `reverse` and `then_with` of a quiet function,
  arithmetic of numbers (a division or remainder only by a constant
  neither 0 nor -1), a number's `total_cmp`, text's and a collection's
  `len` and views, text's `chars()` and cases and a std iterator of them
  counted or compared, `format!` of what can't fail to show, an
  `Option`'s tests and `map` of a quiet function, a clone of text, and
  JS's `localeCompare` (a builtins binding that reads only). An exhaustive
  `match`'s impossible branch, which never runs, is no way to be seen.
- `stable_sort_seen` (src/lower/std_types/slice.rs) asks them: of `sort()`,
  the items' order; of `sort_by`, the comparator, unless it orders by UTF-16
  units, JS's own string order (ADR 0945); of `sort_by_key`, the key's
  function, its order and its drop, which runs after each comparison; of a
  type parameter's items, always, as its caller's type may be seen.

A type parameter's items are sorted as its caller's type picks: `kind` and
`size` are its caller's, `TStableSort` (a new type fact) and `TSize`, as
the unstable sort's `TSort` is (ADR 0471). A type whose layout is known
though it holds one behind a reference, `(&T, usize)`, is its own. A
generic caller bounded by the crate's trait that extends `Iterator` can't
give them of `T::Item` (#934), as it couldn't the unstable sort's: refused,
naming #934 (`extension_trait_sorted_refused`; `extension_trait_params`
keeps its dictionaries' cases with a `reverse()` in the sort's place).

`sort_by_cached_key` sorts its `(key, index)` pairs with std's unstable sort
where the keys' order can be seen, `$sortUnstable` of the pairs by the
tuple's `lt` (the keys' `partial_cmp`, then the indexes), as std does.
Of a key that holds a type parameter, what that sort picks by `(K, usize)`
is its caller's: a built slot's `Sort` fact, beside its `Size` and `Align`
(#737), `KUsizeSort`, so itertools' `sorted_by_cached_key` still compiles,
and is exact.

**What moves to `$sortStable`**: a comparator or key that may panic,
`partial_cmp(..).unwrap()` of floats (f32.js; #935 proposes proving the order
a panic leaves unseen), a `%` by what may be zero, a key that may panic; a
`dyn`'s or a generic impl's order; a type parameter's items
(generic_from_iter_in_place.js, map_set_methods.js).

**Tests.** `sort_stable_seen` runs lengths 0 to 300 of a counting
comparator, and six shapes (random, few keys, sorted, reversed, runs, all
equal) of a `Copy` item of 8 bytes, one of 24, one that isn't `Freeze`, a
counting `Ord`, `String` pairs and `sort_by_cached_key`, against
`wasm32-wasip1`'s native Rust (`//@ native: wasm32`), each sort's count,
trace of calls and order. `sort_stable_unwinds` panics at the 1st to the
1500th comparison of lengths 4 to 300, by `sort_by`, `sort_by_key` and
`sort`. Before landing, a harness ran the runtime against native std on
37,680 sorts (every length to 300, then up to 400,000, past the full
scratch; three kinds, eight shapes, five panic points) with the same count,
trace and order each. #907's 66 unwind-table case-builds pass. Mutations:
scripts/mutations/runtime/sort_stable.ts, lower/std_types/slice.ts,
lower/analysis/unwinding.ts, lower/ordering.ts.

The B-tree's bulk build (`collect`, `append`) sorts with driftsort too;
`$treeFrom` still sorts with JS's sort, and is #926's with the node layer.
Kind: wrong decision. Area: iterators.

## Amendment: JS's sort only of a total order (2026-11-08, #936)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept.

A comparator whose calls can't be seen isn't therefore a total order, and
of one that isn't, each sort leaves its own order: `v.sort_by(|a, b|
a.partial_cmp(b).unwrap_or(Equal))` of 40 floats with NaNs panicked
natively, "user-provided comparison function does not correctly implement a
total order", where JS's `sort` ended and printed another order.

**JS's `sort` stays only of a quiet comparison that is a total order**
(`stable_sort_is_std`, beside `call_seen` and `order_seen`):
- `call_total(F)` (src/lower/ordering.rs): a closure or function whose body
  `orders_totally` (src/lower/body_queries.rs) says gives one: `Ord::cmp` of
  a type `order_total` says orders totally, a float's `total_cmp`, JS's
  `localeCompare`, `Equal`, and their `reverse()`, `then(..)` and
  `then_with(f)` of a total `f`; `then` written out, `if o != Equal { o }
  else { p }` (or its `==` the other way round) and `match o { Equal => p,
  o => o }`, through a `let` of `o`; std's `Ord::cmp` and `total_cmp`
  given as functions. Not a constant `Less`, which contradicts itself.
- `order_total(T)`: std's orders of numbers, text, `char`s and `bool`s and
  of what holds them; a derived one's, as its fields'; a hand-written one's
  whose `cmp` and `partial_cmp` bodies order totally (`Some(self.cmp(o))`
  too); never a type parameter's.
- Anything else, `partial_cmp(..).unwrap_or(..)`, an `if` of `<`, a
  `match` of the items, is `$sortStable`, exact for any comparator, since it
  is std's own.

`sort_not_total` checks it against native Rust: a derived order, a
`then_with`, `total_cmp` stay JS's; a hand-written rock-paper-scissors
`Ord` of 20 leaves std's order; NaNs by `unwrap_or(Equal)` leave std's
order of 6, then panic as std's merge does of 40. `sort_total_written_out`
keeps the written-out `then`s on JS's sort, examples/std_traits.rs's `Word`
among them, and one that isn't `then` on std's. No other corpus case's JS
changes; one snapshot does, approved: the playground's file explorer,
whose comparator puts the first entry first by a test of each side, total
only because one entry is, which a compiler can't prove, is
`$sortStable(entries, (a, b) => compare(a, b) < 0, "general8", 4)`.
Mutations: `orders-totally-any-call`, `comparator-always-total`,
`hand-written-order-always-total`, `orders-totally-written-then-unasked`,
`orders-totally-written-then-kept-unasked`, `orders-totally-match-unasked`.
Kind: wrong decision. Area: iterators.

## Amendment: a `Some` of what's collected that could look like `None` (#984)

`$collectOptions` gives `into(values)`, the collection, as its `Some`. Of
an `Option<()>`, an `Option<Option<_>>` or an `Option<V>` of a generic `V`,
that `Some` is boxed (`boxed_payload`, ADR 0051): a unit's `undefined`
read as `None`, so `[Some(()), Some(())].into_iter().collect::<Option<()>>()`
gave `None`. Results change ([0262](0262-when-rust-and-js-disagree.md)),
so the `Some` is boxed where any such `Some` is:
`$collectOptions(it, true, (values) => $some(values.forEach(() => {})))`.
Held by the corpus case `collect_into_option_of_unit` and the mutations
`collected-some-unboxed` and `collected-some-always-boxed`, the second of
an `Option<Vec<_>>`, whose `Some` is the vector itself.
