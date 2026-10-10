# 0071. An iterator stepped through is a `$iter`, which knows where it is

Status: Accepted. (Amended: `for x in &mut it` and `for x in it.by_ref()` of an
iterator step through `it` itself, so a `break` leaves the rest in it,
where a loop that stopped early had left `it` as it was, a wrong answer;
`count()` of one is what it has left, where it was `undefined`; and
`by_ref()` anywhere else is an error, as a chain of an array wouldn't
know where it is. num-traits' float parser found them. `by_ref()` of an
iterator of the crate's is the iterator too, ADR 0184.) Extends [0036](0036-iterators-and-sorting.md) and [0055](0055-iterator.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An iterator over an array is the array itself (ADR 0036): `v.iter().map(f)`
is `v.map(f)`. That works while the iterator is only handed on. But a
tokenizer steps through its source:

```rust
let mut chars = src.chars().peekable();
while let Some(&c) = chars.peek() { if c.is_ascii_digit() { chars.next(); } else { break } }
```

An array doesn't know how far along it is, so `next()`, `peekable()` and
`peek()` were errors.

## Decision

**An iterator that's stepped through is a `$iter`: its items and where it
is,** `{ items, at }`, which is a JS iterator too. That's any `Peekable`,
wherever it's kept (a lexer's field), and a local that `next()` is called
on:

| Rust | JS |
|---|---|
| `src.chars().peekable()` | `$iter(Array.from(src))` |
| `let mut it = v.iter();` (then `it.next()`) | `let it = $iter(v);` |
| `it.next()` | `$next(it)`: the item, or `undefined` at the end |
| `it.peek()`, `it.next_if(f)`, `it.next_if_eq(&x)` | `$peek(it)`, `$nextIf(it, f)`, `$nextIf(it, (item) => item === x)` |
| `chars.as_str()` | `$restStr(chars)`: what's left, still there |
| `it.copied().collect()`, `for x in it` | `$rest(it)`: what's left, which it then hasn't |

- **Anything else done with one takes what it has left,** as Rust's
  adapters do, which take the iterator.
- **`next()` of an iterator just made** is its first item:
  `v.iter().skip(2).next()` is `v.slice(2)[0]`. Of a crate's own, it's its
  impl's `next` (ADR 0055), and of a lazy one, `$next` of the JS iterator.
- **`next()` of one kept elsewhere,** a `Chars` in a field or a parameter,
  or used in a closure, is an error: it would have to be a `$iter` wherever
  it came from. A `Peekable` is one everywhere, so the error says to use it.
- **A generic iterator stepped through, `mut it: I` or `let mut it = it;`,
  is a JS iterator from where it's bound:** `it = Iterator.from(it)`, an
  array's, a lazy one's or the crate's own alike, `$next(it)` steps it, and
  what's left goes to `collect` or a `for` loop as it is. `it.by_ref()` is
  still an error: a JS iterator helper, as `take`, closes what it takes
  from. (Amended: `next()` of one was an error, a `&mut` to a `I`.)
- **A generic iterator lent as a `&mut`, `fn skip<I: Iterator>(it: &mut I)`,
  is the lender's JS iterator:** the lender's local is bound as one that
  knows where it is, `$iter(v)`, as one it steps itself is, a lazy one and
  the crate's own already being one, and the borrower binds `it =
  $lent(it)`, which steps `it` and can't close it: what the borrower stops
  early, a `take(2)` or a loop that breaks, the lender goes on stepping.
  Lending one kept anywhere but a local, a field's, is an error, and so is
  a trait's `&mut self` method of one, which a dictionary gives a handle.
  (Amended: a `&mut` to a generic iterator was an error.)
- **`peekable()` of a lazy iterator** is an error: its items would have to
  be worked out first, which could change what runs when, or never end.
- **Items that could look like `None`** are an error, since `$next` is
  `undefined` at the end.

### Also here

- `collect::<String>()` of `Array.from(s)` is `s`: `c.to_uppercase()`
  collected is `c.toUpperCase()`.

## Why

- **It's Rust's answer.** The example's tokenizer, with numbers, words,
  strings, comments and an unterminated string, matches native Rust's, and
  so does `pairs`, which takes two items at a time and then the rest.
- **The arrays stay arrays** where nothing steps through them: only what
  needs to know where it is pays for it.

## Alternatives

- **JS's own iterators** (`array.values()`), with a buffer for `peek`.
  They step as well, but `as_str()` and `peek()` need the items and an
  index anyway, and a `$iter` can be read in a debugger.
- **Every iterator a `$iter`.** Uniform, but `v.iter().map(f)` would lose
  its plain `v.map(f)`.

## Since

- **`next_back()` of a std iterator, and `as_slice()` of a slice's or a
  `Vec`'s** (2026-10-10). Each was refused. A `$iter` keeps where its back
  is, `end`, which `next_back()` steps down as `next()` steps `at` up, and
  every reader of what's left, `$rest`, `$peek`, `$nextIf`, `len()`,
  `size_hint()`, `as_str()`, a clone and `as_slice()`, reads from `at` to
  `end`. Its items stay the array they are, which the iterator's source
  may be.

  ```rust
  let mut it = v.iter();
  println!("{:?} {:?}", it.next(), it.next_back());
  ```

  ```js
  const it = $iter(v);
  const arg = $next(it);
  const arg$1 = $nextBack(it);
  ```

  | Rust | JS |
  |---|---|
  | `it.next_back()` of one stepped through | `$nextBack(it)`, `$nextBackSome(it)` of a generic `T`'s |
  | `v.iter().next_back()`, a new one | its items' `.at(-1)`, `$someAt(items, items.length - 1)` of a generic `T`'s |
  | `it.as_slice()` | `it.items.slice(it.at, it.end)`, or a new one's items |
  | a `Peekable`'s `peek()` of a generic `T`'s | `$peekSome(it)`, as `$nextSome` |

  A chain whose closures do what can be seen, or a generic one, kept or
  not, is a JS iterator, which gives its items from the front only: its
  `next_back()` is refused, as Rust would run the closures on its last
  item alone, and a JS iterator has no back to take from. The
  `iter_next_back` corpus case runs, against native Rust, `next()` and
  `next_back()` meeting, `len`, `size_hint`, a generic one's, `as_slice`,
  a clone, `collect`, a `for` after them, a B-tree's range and `iter`,
  `map`, `chars`, `split`, `Option`s, a `Peekable` peeked after, `next_if`,
  `as_str` and a `RangeInclusive`; `next_back_lazy` and `next_back_generic`
  the refusals.
  Mutations take from the front, read past `end` in each reader, unbox,
  run the chain eagerly, and refuse each.
- **A slice's or a `VecDeque`'s `iter_mut()` kept, and a chain on
  `by_ref()`** (2026-10-10). A kept `iter_mut()` was refused; it's a `$iter`
  of what `for` takes, `$iter($mutItems(v))` of numbers or text, whose
  `next()` gives a handle a place may be, `*it.next().unwrap() += 1`. A
  chain on `it.by_ref()` was refused but in a `for`; it's a JS iterator of
  `it`, `Iterator.from(it).take(2)`, every stage of it lazy, so each takes
  from `it` only what it's asked for and leaves the rest, as Rust's does.

  ```rust
  let first: Vec<_> = it.by_ref().take(2).collect();
  let found = it.by_ref().find(|x| **x % 7 == 0);
  ```

  ```js
  const first = Iterator.from(it).take(2).toArray();
  const found = Iterator.from(it).find((x) => x % 7 === 0);
  ```

  The `iter_mut_kept` corpus case runs, against native Rust, numbers,
  a slice's part, strings, structs and a deque's, stepped by `next`,
  `next_back`, `by_ref().take` and `for`, and collected; `by_ref_chains`
  `take`, `take_while`, `find`, `map` and `sum`, `any`, `skip` and
  `next`, `position`, `filter` and `count`, each followed by what's left.
  Mutations make the chain eager or an array, miss a reborrow or a second
  stage, unhandle `next()`, and refuse each.
