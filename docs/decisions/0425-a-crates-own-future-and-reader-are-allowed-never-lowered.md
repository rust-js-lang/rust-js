# 0425. A type's own `Future`, `io::Read`, `BufRead` and `Seek` are allowed, and never lowered

Status: Accepted. Extends [0168](0168-user-hash.md), whose `Hash` impls
are allowed and never lowered, and [0029](0029-async-await.md), whose
futures are JS promises.

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)): an impl nothing
calls is no JS a program can see; what would call one, or await one as a
promise, is a compile error.

## Context

either's `Either<L, R>` forwards std's traits to the side it holds: among
them `Future`, `io::Read`, `BufRead` and `Seek`.

```rust
impl<L: Future, R: Future<Output = L::Output>> Future for Either<L, R> {
    type Output = L::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> { .. }
}
```

Each was an error, "user implementations of `Future`", at the impl. That
stopped either whole, so itertools as well, though a program that uses
`Either` as a value, `is_left()` or `map_left`, never calls them.

## Decision

**A type's own `impl Future`, `impl io::Read`, `impl BufRead` and `impl
Seek` are allowed and, like its `Hash` (ADR 0168), never lowered:** their
methods are no JS. Which std traits are like this is
`recognition::unlowered_trait`'s one list. `implementable` allows an impl
of one, and `is_unlowered_impl` keeps its methods out of what's lowered.

- **What would call one is refused where it's called.** A call of `read`,
  `fill_buf`, `seek` or what std provides of them, `read_to_string` or
  `bytes`, is "calling `std::io::Read::read`", as it was. So is a `dyn
  Read`, and a `BufReader` of one. JS has no stream these would be.
- **A future of the crate's own is no JS promise, so nothing may treat
  it as one.** In JS, `.await` is `await` (ADR 0029). Of a value that
  isn't a promise, `await` gives the value itself: `S(1).await` would be
  `await [1]`, which is `[1]` where Rust gives `1`. So
  `recognition::unlowered_future` says which types are such a future:
  ones whose `Future` resolves to an impl rust-js compiled, the crate's or
  a library's, seen through a reference, a `Box` or a `Pin`. Each of these
  is an error at its span:
  - a call whose bounds ask a `Future` or an `IntoFuture` of one
    (`awaited_unlowered_future`): `.await` of it, `into_future`'s, and a
    generic function's `F: Future`, a binding's (`js::promise`) too, as
    that function awaits `F` as a promise;
  - unsizing one to a `dyn Future`, `Box::pin(s)` as a `Pin<Box<dyn
    Future>>` (`unsized_unlowered_future`).

  ```text
  error: rust-js does not support `S` as a promise yet: its `Future` is the crate's own, which rust-js never compiles (ADR 0425)
  ```

  A binding's future, `js::Promise<T>`, is a promise: its crate is a
  binding crate, not one rust-js compiled. So are an `async` block and
  what an `async fn` returns.

## Why

- **It's exact.** The `unlowered_impls` corpus case is either's `Either`,
  with all four impls, run against native Rust: its own methods are
  called, and the impls are no JS. `diagnostics.test.ts` refuses its
  `.await`, a `Pin` of it awaited, it given to an `F: Future`, it made a
  `dyn Future`, and a `read` of a crate's reader.
- **It's no JS at all,** as a `Hash` impl is.
- **The silent case is closed.** Allowing the impl would have made
  `.await` of one compile to the wrong value. The refusal is where a
  future is used as a promise, so a type that only has the impl, as
  `Either` mostly is, costs nothing.

## Alternatives

- **Lower `poll` and drive it from JS**, an executor of the crate's
  futures. Not now: it's ADR 0029's rejected alternative, a state machine
  in the output, for a future that `async fn`s already write as a promise.
- **Refuse the impl, as before.** It stops either, and itertools with it,
  for impls that a program using `Either` as a value never calls.
- **Make `.await` of a crate's future call its `poll` once.** A future
  that's `Pending` once would never wake: it needs the executor above.

## Consequences

- A future of the crate's own can't be awaited, pinned and awaited, made a
  `dyn Future`, or given to what takes an `F: Future`. A type that needs
  that writes an `async fn` instead, as ADR 0029 has it.
- A type parameter `F: Future` is a promise in its generic code, as
  before: the call that gives it a crate's future is what's refused.
- either compiles past its impls. Its next refusal is generic code calling
  `Extend::extend` on an `L: Extend<A>` (`docs/crate-corpus.md`).
