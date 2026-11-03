# 1073. A part of a slice reversed is reversed in place

Status: Accepted (2026-11-03, #524). Amends
[0335](0335-slice-views.md).

Case: None ([0262](0262-when-rust-and-js-disagree.md)): no observable
difference; the same items move to the same places, and the same range
panics with the same message before any moves.

## Context

ADR 0335 makes every `&mut` of part of a slice a view, `$view(v, a, b)`:
a `Proxy` of an array, with three closures, whose every read and write is
`v`'s. So `v[a..b].reverse()` made a proxy on each call and reversed it
through its traps:

```js
$view(queued, firstInsertedIndex, insertIndex).reverse();
```

alien-signals' `notify` does this on every update, reversing the effects
it just queued, as upstream does with a loop of swaps. Profiling a signal,
computed and effect updated 2,000,000 times: `$view` 6.5%, the proxy's
`get` and `at` 8.6%, `$proxy` 2.2%, and the garbage collector 20%. The
update took 3.5 times upstream's, and this was the largest share.

## Decision

**A `&mut` of part of a slice is held unmade until it's used,
`Value::View([v, a, b])`. Given alone to what has a form of its own over
a range of the slice, it's that form, of the slice and the range, no view
made; anywhere else it's the view it was.** `slice::ranged` names each
such form; one so far:

| Rust | JS |
|---|---|
| `v[a..b].reverse()`, `v[a..].reverse()` | `$reverseRange(v, a, b)`, `$reverseRange(v, a)` |
| `v[..].reverse()` | `v.reverse()`, as all of a slice is the slice (ADR 0335) |
| `let part = &mut v[a..]; part.reverse();` | `$view(v, a)`, reversed as an array: the view is kept |
| `v[1..5][1..3].reverse()` | `$reverseRange($view(v, 1, 5), 1, 3)` |

```rust
queued[first_inserted_index..insert_index].reverse();
```

```js
$reverseRange(queued, firstInsertedIndex, insertIndex);
```

`$reverseRange(items, start, end = items.length)` checks the range as
`$view` does, `$sliceIndexFail` with Rust's message, then swaps from both
ends inward, as upstream's loop does and as `<[T]>::reverse` does.

- **Where the view is made doesn't move**: it's the call's argument, made
  where the call is, as the view was. A view read anywhere else, held
  across a statement, kept in a variable or given to any other call, is
  `$view(..)` as before; `flush` and `value_expr` make it.
- **Its source span is the view's**, set where it's stored, as a list's
  is.
- **Not composed**: a view of a view stays one, the inner part made
  first, as each is checked against what it's of.

## Why

- **It's the same program**: the same items reach the same places; the
  range is checked first, with the same panic, before any is moved.
- **It reads closer to what a person writes**: a call that says what it
  does, of the array and the range, where a proxy was made to be reversed.
  JS has no `reverse` of a range; upstream's loop is the same swaps.
- **It's fast**: no allocation. On the alien-signals port, an update of
  a signal, computed and effect went from 3.5 times upstream's to about
  1.7 times; the propagation benchmarks from 1.3–3 times to 1.1–1.8
  times, on a loaded machine.
- **One rule, extended**: a form over a range for another method is a
  row of `slice::ranged` and a helper beside `$view` in `slice_view.js`.
  `fill`, `sort` and `copy_from_slice` of a part could follow; JS's own
  `fill(x, a, b)` needs the check besides.

## Consequences

- `test/corpus/slice_range_reverse.rs` runs, against native Rust, parts
  of a `Vec`, a slice and an array reversed, to its end, empty, of one
  item and inclusive; a view of a view; a part of what a call gives; a
  range whose bounds are calls, in order; `String`s and a struct's
  `Option`s; a view kept and `get_mut`'s, which stay views; and a range
  past the end and one ending before it starts, which panic; and a view
  past the end given with what's made after it, a print, calls, a value
  dropped, which none of runs or drops, as Rust's view panics first.
  `slice_views` reads `$reverseRange(v, 0, 3)`.
- `test/lowering-comments-items.test.ts`'s "a comment after a view
  argument is after it" holds its place: the comment after it, and its
  source map's end at the Rust's `]`.
- Mutations leave the range unchecked, swap one item short, make a view
  again where the form is taken, make the view's parts an array, and drop
  where it's made. A view acts, as the call it's made of does (`movable`
  says so); every statement that could run between it and its use holds
  it first anyway (`flush`), so no program tells that answer apart today,
  and no mutation is kept of it.
