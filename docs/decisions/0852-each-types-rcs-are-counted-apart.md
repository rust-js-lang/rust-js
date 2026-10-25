# 0852. Each type's `Rc`s are counted apart

Status: Accepted. Amends [0844](0844-a-library-that-reads-no-count-counts-no-rc.md),
whose library counted every `Rc` or none, and whose contract said which,
`counts_rcs`; [0523](0523-libraries-share-one-counted-rc.md), whose crate
a counting library handed an `Rc` counted every one; and
[0494](0494-a-generic-rc-is-counted-where-it-may-drop.md), whose generic
`Rc` of a type parameter's value that may drop counted every `Rc`, and
was found through what a body read behind a reference too.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for an `Rc` of a type
nothing counts, which can't tell it's the value itself; C for one whose
count is read or whose last owner runs a destructor, counted as Rust
counts it; D, refused, where a crate would count an `Rc` a library hands
it as the value itself.

## Context

The maintainer decided this one: make whether an `Rc` is counted a
decision per type, so that one generic function doesn't change how every
other `Rc` is represented.

React's port added `use_context`, as upstream writes it, of any `T`:

```rust
pub fn use_context<T>(context: &ReactContext<T>) -> T { /* .. */ }
```

The function only reads its context. But ADR 0844 counted every `Rc` in a
library once any one was counted. A body "held" an `Rc` of a type
parameter's value that may have a destructor, which made every `Rc`
counted. That `Rc` was found through `&ReactContext<T>`'s fields, behind
the reference. `ReactConsumerType<T>`'s `Rc` counted too, because its
`T` was taken to be owned, although only a `Weak` holds it. So
`react-isomorphic`'s `createContext` became the counted box again:
`{ value, strong, weak }`, `$downgrade`, and a `try` for every
statement. React's tests broke. The port had written `T: Copy`, which
upstream's `useContext` doesn't have, to dodge it.

Two faults combined. A crate counted every `Rc` or none, so one counted
type made every type counted. And what a body was taken to own included
what it only reads through a reference, and what a `Weak` points at.

## Decision

**Whether an `Rc` is counted is decided by what it points at. An `Rc`
is counted where it's of a type whose count the crate reads, or whose
last owner runs a destructor, or a type that may be one of those. Generic
code's `Rc<Slot<T>>` stands for every `Rc<Slot<_>>`, and an `Rc<T>` for
every `Rc`. A library's contract lists the types it counts, and a crate
it hands an `Rc` counts it exactly where the library does.**

```rust
pub fn use_context<T>(context: &ReactContext<T>) -> bool {
    let consumer = context.consumer.take();
    let found = consumer.is_some();
    context.consumer.set(consumer);
    found
}

pub fn share<T>(value: T) -> (Rc<Slot<T>>, Rc<Slot<T>>) {
    let slot = Rc::new(Slot { value });
    (Rc::clone(&slot), slot)
}
```

```js
export function createContext(defaultValue, TCopy) {
  const context = {
    value: TCopy.copy(defaultValue),
    // These are circular
    provider: null,
    consumer: null,
  };
  context.provider = context;
  context.consumer = {
    context,
  };
  return context;
}

export function share(value, dropT) {
  const slot = { value: { value }, strong: 1, weak: 0 };
  // ..
}
```

- **One owner**, `counted_rcs` (`analysis/rc_counts.rs`). It answers
  `Counted::counts(pointee)` for lowering, drops and declarations, as
  before. It starts from what reads a count, what a body owns, and what
  a library hands the crate counted. Then it runs to a fixed point: a
  pointee whose drop runs a destructor is counted, and so is every type
  the crate has that `may_be` one already counted.
- **What may be one type** (`may_be`, of each type's `head`): either
  type is a type parameter or a projection, or both have one constructor
  and each of their arguments may be the other's. This says "may" more
  often than rustc's unification does, never less. So a generic body's
  `Rc<Slot<T>>`, which is one JS function for every `T`, gets the same
  answer as each `Rc<Slot<X>>` it's given. A counted `Rc<Cell<f64>>`
  leaves an `Rc<Cell<u32>>` the value itself. `all` remains for a
  counted `Rc` of a bare type parameter's value, which may be any type.
- **What a body owns** (`owned_dropping`): a generic `Rc` is counted
  where a body owns one of a type parameter's value that may have a
  destructor, not a `T: Copy`. A body doesn't own what's behind a
  reference or a raw pointer, nor a place it reads through a deref,
  `*context` and its fields, nor what a function's type names,
  `Vec::<Rc<Item<T>>>::len`'s arguments' types, of which it holds no value. It doesn't own what a `Weak` points at,
  since a `Weak`'s drop drops nothing of it. Fields of the crates'
  types a body owns are owned, as `Holder<T>`'s `Rc<T>` is.
- **The contract lists the types counted**, `counted_rcs`. Each is a
  shape: its constructor's key, rustc's `DefPathHash` for an item and a
  name for a primitive, and the shapes of its arguments. A shape with no
  `of` is any type. A library that counts none lists nothing. A consumer
  asks of each `Rc` or `Weak` in a library item's declared signature or
  fields, instantiated as it uses the item, whether one of the library's
  shapes may be it. If so, it counts that type as the library does. If
  not, and it would count the type itself, it's refused: "this crate
  counts an `Rc<dep::Context<i32>>`, .., but `dep`, a rust-js library
  that counts none of them, hands it one, which both share as the value
  itself". An `Rc` reached only through a type parameter the consumer
  gives isn't the library's. The library reads it only through the
  consumer's dictionaries.
- **No library counts every `Rc` because it counts one.** ADR 0844's
  all-or-none rule was there so that consumers agree with the library.
  The listed types give the same agreement, one type at a time.

## Why

- **It's what the maintainer decided**, and it's upstream's JS: React's
  `createContext` stays the plain context beside any generic `Rc` its
  crate counts, and `use_context<T>` needs no `T: Copy`.
- **Correct within a crate**: a generic body's `Rc` and every type it may
  be given get one answer, because `may_be` covers every instantiation.
  A counted type stays counted wherever its count or its last drop can
  be seen.
- **Correct across crates**: a library and its consumer read each `Rc`
  they share the same way. Either both count it, or the consumer is
  refused.

## Alternatives

- **Only the precision, keeping ADR 0844's all or none**: `use_context`
  alone would count nothing again. But the first generic `Rc` a library
  owns of what may drop, such as `share<T>`'s, would turn every context
  back into the box. Rejected, as the maintainer asked.
- **Classes by outermost constructor, every `Rc<RefCell<_>>` as one**:
  simpler and sound, but a program counting one `Rc<RefCell<Logger>>`
  would count all its cells. Rejected: `may_be` compares arguments too.
- **A library counts every type it hands out once it counts any**: this
  would keep a consumer's `strong_count` of a library's `Rc<String>`
  working. But `share<T>` would count React's context again. Rejected. A
  consumer reads the count in the library instead, which then counts it.

## Consequences

- `three crates: rc_shared` read `Rc::strong_count` of `dep::tag`'s
  `Rc<String>`, though `dep` reads no count of one. Under ADR 0844 `dep`
  counted every `Rc` because it counted `Noisy`'s. Now `dep` counts only
  `Noisy`, and such a consumer is refused. The case now prints the tag
  without reading its count, and `test/crates.test.ts` checks the
  refusal. This is a program that compiled before and is now refused.
- A program's per-type counting is unchanged where it has no generic
  `Rc`. Where it has one, only the types that generic code may be given
  are counted, not every type.
- The library contract's `counts_rcs` is now `counted_rcs`. Its version
  is unchanged: a dependency's manifest must come from this compiler
  build (its compiler identity), and `@rust-js/std` counts none, as its
  field's absence says.

## What holds it

- `test/crates/pairs/rc_per_type`: React's context, `use_context<T>` of
  any `T`, a counted `share<T>` of a consumer's `Loud`, and a shelf of
  `Rc<Item<T>>`s read through a reference, run against native Rust.
  `test/crates.test.ts`, "a library counts each type's Rcs apart, and its
  consumer as it does", checks their JS, the contract's one shape, and
  the two refusals, of the context's count and of `rc_shared`'s
  `Rc<String>`'s.
- `test/corpus/rc_counted_per_type.rs`: a counted `Rc<Cell<f64>>` beside
  an uncounted `Rc<Cell<u32>>`, a generic `Rc<Slot<T>>` counted with
  the ones whose counts are read, and a generic `Rc<Pair<T, u8>>` that
  may be a counted `Rc<Pair<bool, u8>>`, counted, and with it the
  `Rc<Pair<f64, u8>>` it's given, run against native Rust.
- `test/manifest.test.ts`: the host validates `counted_rcs`' shapes.
- Mutations in `scripts/mutations/lower/analysis/rc_counts.ts`
  (`library-counts-every-rc`, `borrowed-place-owned`, `fn-item-owned`,
  `weak-owns-pointee`, `may-be-ignores-arguments`, and the earlier ones,
  rewritten), `scripts/mutations/lower/library.ts` (`library-shape-any`,
  `library-rc-unreached`, `uncounted-library-rcs-unseen`),
  `scripts/mutations/lower/analysis.ts` (`consumer-rcs-uncounted`) and
  `scripts/mutations/library.ts` (`contract-counts-unread`).
