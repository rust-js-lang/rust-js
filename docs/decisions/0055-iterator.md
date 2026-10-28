# 0055. The crate's own `Iterator` is a JS iterator

Status: Accepted. Extends [0036](0036-iterators-and-sorting.md) and [0052](0052-std-trait-impls.md). Amended by
[0473](0473-fuse-is-rusts-fuse.md): `fuse()` is std's `Fuse`, not the
iterator itself. Amended below: collected, an iterator is `Array.from(it)`
(the user's decision, 2026-10-28).

Case: C, A, D, B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0036 made an iterator a JS array: `v.iter().map(f)` is `v.map(f)`. That
works because a slice's or a `Vec`'s iterator has all its items already.

A hand-written `impl Iterator` doesn't. Its `next` makes one item at a
time, and it may never stop:

```rust
impl Iterator for Fibonacci {
    type Item = u32;
    fn next(&mut self) -> Option<u32> { .. }
}
fibonacci().find(|&x| x > 50)
```

Turned into an array first, that `find` would never return. Rust's adapters
are lazy, and JS now has lazy ones too: its iterator helpers,
`Iterator.from(..).map(f).take(n)`.

## Decision

**The crate's own iterator is a JS iterator wherever it's used as one:**

```js
$iterator(fibonacci(), fibonacciIterator_next).find((x) => x > 50)
```

- **`next` is a function like any method of a trait impl:**
  `countdownIterator_next(c)`, returning an `Option`. A direct `c.next()`
  calls it.
- **`$iterator(it, next)` makes a JS iterator from it**, with
  `Iterator.from`. `None` ends it.
- **Lazy adapters stay lazy:**
  - `map`, `filter`, `enumerate`, `find`, `any`, `all`, `for_each`, `fold`
    and `sum` are the JS iterator's own methods;
  - `take(n)` is `take(n)`, and `skip(n)` is `drop(n)`;
  - `for x in it` is `for (const x of $iterator(..))`.
- **Anything that needs every item takes them as an array first:** `collect`
  is `.toArray()`, `count()` is `.toArray().length`, and `max`, `min`,
  `last` and `position` do what ADR 0036 does with an array. Rust would
  also run through the whole iterator for these.
- **A generic `next` may box its `Some`** (ADR 0051). `$iterator(it, next,
  true)` unboxes each item, so an iterator of `()` or of `Option`s still
  yields every item.
- **An impl's `type Item` is allowed.** rustc works out what it is, so it
  needs no JS.
- **`fuse()` is the iterator itself, and `impl FusedIterator` is allowed.**
  A JS iterator chain ends at its first `done` and stays ended, as `for`
  does, and so does an array: what `Fuse` makes of an iterator that starts
  again after its `None`. `FusedIterator` has no items, and only promises
  that. One that breaks the promise is fused all the same, which Rust's
  `fuse()` wouldn't do for it, but only a direct `next()` on the `Fuse`
  could tell, and that's an error below.
- **Still errors:**
  - `DoubleEndedIterator` and other iterator traits, so no `rev()`;
  - `next()` on a generic `T: Iterator` (ADR 0061 has the rest of generic iterators);
  - std adapters' own `next()` on one of these.

## Why

- **It's lazy where Rust is lazy.** An endless iterator works with `take`,
  `find` and `for` with a `break`, as in Rust.
- **Arrays keep their ADR 0036 output.** Only a chain that starts from the
  crate's own iterator changes, and it uses the same method names wherever
  JS has them.
- **Nothing to add to JS.** `$iterator` is a few lines around
  `Iterator.from`.

## Alternatives

- **Collect into an array at the start.** That's simpler and keeps every
  adapter as it is, but an endless iterator would never finish, and a
  `find` would run `next` past what it needs.
- **A generator per impl** (`function* ()`), which would be as lazy. But
  `next` is what Rust code calls directly, so it has to exist anyway, and
  a generator would be a second copy of it.

## Consequences

- The generated JS needs iterator helpers (`Iterator.from`, `.take`,
  `.drop`, `.toArray`). Current browsers, Node 22 and Bun all have them.
- `for` over one is a `for .. of` on a JS iterator, which calls `next` once
  per item, as Rust does.
- **B: a `collect` std knows is empty runs nothing; rust-js's runs what's
  before it.** `vec![x].into_iter().map(|x| note(x)).skip(1).collect::<Vec<_>>()`:
  std's `Vec` from an iterator whose `size_hint` is `0` makes the empty
  `Vec` without iterating, so `note` isn't called; the JS's iterator steps,
  as Rust's iterators are lazy, through `map` to `drop(1)`, and calls it.
  A `sum` of the same chain, or a `collect` of one with an item left,
  calls it natively too. Only a closure that does something before a
  `skip` past every item, collected, can tell (a generated program's, seed
  80072, 2026-10-15).

## Amendment: collected, an iterator is `Array.from(it)`

The user decided (React port gaps bw8, cm7) that an iterator collected into
a `Vec` prints `Array.from(it)`, as upstream React writes
`Array.from(instance._transitions)` and as every host React supports runs,
and that no `.toArray()`, ES2025's iterator helper, is printed anywhere.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `Array.from` steps an
iterable's iterator to its first `done` as `toArray` does, each item once
in order; what differs, closing, is made the same below.

- One form, one owner: `js::Expr::collected(it)` makes `Array.from(it)`,
  and `collected_of` reads it, for every consumer that collects: `collect`
  into a `Vec`, a `String`'s join, a heap's items (`mir/iter/collect.rs`),
  `last` (`closing.rs`), `nth`'s `Array.from(it.drop(n).take(1))[0]`
  (`combinators/adapters.rs`, `iterators/in_place.rs`'s `nth_of`), the
  eager chains that make it an array's own (`iterators/eager.rs`: `[...a,
  ...b]` of a chain, `xs.slice()` of an array's, its argument stepped till
  its first `done`), and `expr_walk.rs`' list of JS's own calls.
- Of a runtime helper's items, an array or an iterable, `Iterator.from` is
  dropped inside it, `Array.from($rsplit(s, "="))`; not of what may be a
  binding's iterator with only a `next` (ADR 0140), which stays
  `Array.from(Iterator.from(it))`.
- `$Steps`, an iterator rust-js makes, closed itself at the end of its own
  `toArray()`, as `fold` does, which drops a crate's iterator stepped
  through an adapter and what a held one moved there has left (#206).
  `Array.from` closes nothing at the end, so `$Steps` now closes itself
  where JS's own protocol, `[Symbol.iterator]()`, steps it to its end:
  `Array.from`, `for..of`, a spread. The runtime's own adapters, `$lazyChain`,
  `$lazyZip`, `$scan` and the rest, step one as it is (`$stepsOf`), closed
  where Rust's adapter drops it, so a held chain's `b` stays its owner's
  to drop (#243). Its `toArray` is gone.
- Not yet: a `js::Set`'s `values()` or `keys()` collected is
  `Array.from(set.values())`, where the user's decision is
  `Array.from(set)`, and `js::set::new` takes a slice, `new
  Set(Array.from(..))`; each needs a binding of what iterates as the
  receiver itself, written up for the user (S/decisions-pending.md, bw8b).

Held by `test/lowering-collected.test.ts`, the corpus (every case run
against native Rust, its drops' order included), and mutations
`steps-collected-unclosed` (which takes `steps-held-fold-unclosed`'s place), `steps-of-js-protocol`,
`collected-helper-iterator-from-kept` and `collected-stepped-as-made`,
with `iter-chain-collected-lazily` and `position-by-next` retargeted.
The corpus' and snapshots' 180-odd `.toArray()`s re-blessed, std's compiled
bodies (`bun run std`) and the guide with them; `docs/readability.txt` is
blessed: no shape's count grew (`Iterator.from` fell 170 to 119), but 233
fewer lines make the same counts larger shares.
