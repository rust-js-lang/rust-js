# 0578. A nullable result is `null` where it's written; an item's place known in bounds is unchecked

Status: Accepted. Amends [0574](0574-what-reacts-scheduler-heap-asks.md),
whose `value ?? null` was written at the `return`, and extends
[0576](0576-what-a-bodys-integers-can-be.md) to `index_mut`. Amended
by [0638](0638-a-nullable-functions-call-is-null-already.md): a nullable
function's call is the result as it is, `null` for `None` already.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for each. What a
caller gets doesn't change, and an index left unchecked is one Rust's
check would pass.

## Context

React's heap's `pop`, written in upstream's shape, takes the last item
off and puts it at the top:

```rust
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn pop<T: Node>(heap: &mut Heap<T>) -> Option<T> {
    let last = heap.pop()?;
    if heap.is_empty() {
        return Some(last);
    }
    let first = std::mem::replace(&mut heap[0], last);
    sift_down(heap, 0);
    Some(first)
}
```

rust-js wrote its JS with two things no one writes:

```js
let result;
if (last == null) { result = undefined; return result ?? null; }
if (heap.length === 0) { result = last; return result ?? null; }
$index(heap, 0);
const first = heap[0];
```

- **A variable set in each branch, and `?? null` at each `return`.** ADR
  0574 coalesced the result where the function returns, so a `return`
  couldn't take the value its branch made: `return result ?? null` isn't
  `return result`, which tidy folds.
- **`$index(heap, 0);`**: `&mut heap[0]` is `IndexMut::index_mut`, whose
  index ADR 0576's ranges didn't judge, though `heap.is_empty()` had just
  failed.

## Decision

**A `#[rust_js::nullable]` function's result is `null` where it's written**
(`nullable_written`): a `None` it makes is `null` and a `Some(x)` is `x`,
as before; another `Option` written to it, a call's or a variable's, is
`null` for `undefined` there, `heap[0] ?? null`. The `return` gives what
was written, so each is where its value is made, `return null;`, `return
last;`.

**An item's place through `index_mut` is an index the ranges judge**:
`index_mut` asks of a `Vec`'s length as `index` does, and where it's known
in bounds, its place, `heap[0]`, is written with no check
(`index_call`, `mir/bounds.rs`, with the place it gives where it isn't).

```js
export function pop(heap) {
  const last = heap.pop();
  if (last == null) {
    return null;
  }
  if (heap.length === 0) {
    return last;
  }
  const first = heap[0];
  heap[0] = last;
  siftDown(heap, 0);
  return first;
}
```

## Why

- **A value's JS is made where the value is**, as ADR 0432's `return`s
  are: a result's `null` is the value's, not the `return`'s.
- **`index` and `index_mut` ask one thing of a length**, that the index
  is below it; one known and the other not was a gap, not a rule.

## Alternatives

- **Fold `result = x; return result ?? null;` in tidy**: a second place
  that knows a nullable result. Rejected for writing it once, where it's
  made.
- **Keep `pop` as a swap and a `pop()`** (ADR 0576): JS that does what
  upstream's does another way. Rejected on the user's decision for React's
  port.

## Consequences

- React's heap's `pop` reads as upstream's, but for what Rust can't do:
  it takes the last item off before it reads the first, so it tests
  `last == null` where upstream tests the length.
- `peek`'s `heap[0] ?? null` is as it was: `?? null` is how every
  nullable result of another `Option` is written, and `heap.first()` is
  the Rust.
- One lowering test that held `args[$at(args, 0)] = "HTML"` after
  `args.len() == 1` holds `args[0]`.

## What holds it

- `test/lowering.test.ts`, "a nullable result is written where it's made,
  and an index_mut known in bounds is unchecked": `pop`, `first`, a
  variable returned, a `mem::replace` of an item known in bounds and one
  not, run in Node.
- `test/bindings.test.ts`, "a nullable function's None is null".
- Mutations: the result coalesced at the `return`, a `None` left
  `undefined`, `index_mut` unasked, its place checked where known.
