# 0571. `pin!` and a ready future

Status: Accepted. Extends [0029](0029-async-await.md) and
[0329](0329-pin.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): nothing the program
can observe differs. A future is a promise, which JS never moves, so what
`pin!` promises is kept already, and `ready(v)` resolves to `v` as Rust's
gives it.

## Context

`std::pin::pin!(fut)` and `std::future::ready(v)` were refused, both listed
in docs/guide/limitations.md. Each is common in async Rust: `pin!` to await or
poll a future held on the stack, `ready` for a value where a future is
asked.

`pin!` expands to std's own code: the value in a `PinMacroHelper { value }`
held in a variable the macro makes, and
`pin_new_unchecked_in_helper(&mut pinned)`, a `Pin::new_unchecked` of a
`&mut` to its field. rust-js refused the call, then the struct.

Looking at why showed a gap under it: a type that's the value it holds in
JS (a `ManuallyDrop`, ADR 0406) still had std's struct's shape, so a `&mut`
to one wasn't a `&mut` to what it holds. `&mut ManuallyDrop<u32>`, written
through its `DerefMut`, was refused.

## Decision

- **`future::ready(v)` is `Promise.resolve(v)`**, what a person writes for a
  promise that's done. `std_types/future.rs` owns it.
- **What `pin!` pins is the value**: `PinMacroHelper` is a type that's the
  value it holds (recognition's `transparent`), as `ManuallyDrop` is, and
  `pin_new_unchecked_in_helper` is `Pin::new_unchecked`, the pointer.
- **A type that always holds its value has its value's shape**
  (representation's `always_holds`, asked by `shape`): `ManuallyDrop`,
  `MaybeDangling`, `PinMacroHelper`. Not `MaybeUninit`, which may hold none
  yet and stays a cell to write (ADR 0332). Whether such a type is
  supported is whether what it holds is.
- **`.await` of a pinned `&mut`** to a future awaits the promise it points
  at, through its cell where it has one.
- **A `&mut` to a pinned reference is the pin** (ADR 0329), which only what
  it points at is written through, so it isn't counted as a write of the
  pin's variable, and a pin's `as_mut()` and `DerefMut` keep the place it's
  to, as `Pin::new` does.
- **The variable `pin!` makes is named** after the binding its `Pin` is:
  `let block = pin!(async { .. })` is `const block = (async () => ..)()`.

## Why

- **It reads as written by hand**: `pin!(fut).await` is `await fut`, with
  no box, no handle and no helper; `pin!` leaves no trace in the JS, as JS
  has nothing to pin.
- **One answer for a wrapper**: `shape` is where a type's form in JS is
  answered; asking it there makes every path, a `&mut`'s cell, a field's
  read, what's supported, see the held value, where the struct's shape had
  each path need its own exception.

## Alternatives

- **Classify `PinMacroHelper` field by field**: a `{ value }` object per
  `pin!`, and a handle on its `value`. Output no person writes, for a type
  std keeps private.
- **Keep `ManuallyDrop` a struct in `shape`** and see through it in each
  question asked of a `&mut`. Several places to keep alike, the gap this
  ADR found.

## Consequences

- Polling a future by hand, `fut.as_mut().poll(&mut cx)`, stays refused: a
  promise can't be polled (ADR 0029).
- A `&mut` to a pin written whole, `*r = Pin::new(&mut other)`, isn't
  supported, as ADR 0329's model already said; rustc's checks rarely let a
  program reach it.

## What holds it

- `test/async.rs`'s `pinned`: `pin!` of `ready(x)` awaited through
  `as_mut()`, and of an `async` block awaited as it is, run by
  `test/compiler.test.ts` and its snapshot read by `test/snapshots.test.ts`.
- `test/corpus/manually_drop_through_mut.rs`: a `&mut ManuallyDrop` of a
  number written through, and of a struct, against native Rust.
- `test/corpus/pin_box.rs`: a pinned `&mut` to a number, written as the
  number.
- `test/diagnostics.test.ts`'s "a ManuallyDrop of what's refused".
- Mutations: `future-ready-rejected`, `pin-macro-refused`,
  `pin-macro-helper-a-struct`, `pin-macro-helper-built`,
  `transparent-held-unchecked`, `manually-drop-shaped-a-struct`,
  `uninit-always-holds`, `await-pinned-cell`, `pin-mut-gives-a-handle`,
  `pin-borrow-a-write`, `pin-macro-unnamed`.

## Amendment: an `IntoFuture` of the crate's

**`.await` of a value whose `IntoFuture` is the crate's own awaits what its
`into_future` makes**: `Doubled(x).await` is `await
doubledIntoFuture_into_future([x])`, the impl's method named as every
std trait's impl method is. Its impl was refused; now it's compiled as any
impl's method is, and `.await` calls it first where `own_into_future`, the
one place that asks whether a type's `IntoFuture` is the crate's, says so.
`unlowered_future` asks the same of `Future::poll` through one function,
`own_impl_of`.

Generic code bound by `IntoFuture`, `async fn g<T: IntoFuture>(t: T)`,
awaits what it's given as a promise, which every other `IntoFuture` is;
given one of the crate's own, it's refused where it's called, naming the
type, rather than awaiting the value. A dictionary of `IntoFuture` would
need std's impl for every future, a promise, to be one too.

Held by `test/async.rs`'s `doubled`, the diagnostics test "an IntoFuture
of the crate's given to generic code", the limitation it lists, and
mutations `own-into-future-uncalled`, `own-into-future-given-unrefused`,
`into-future-unimplementable` and `own-into-future-awaited-refused`.
