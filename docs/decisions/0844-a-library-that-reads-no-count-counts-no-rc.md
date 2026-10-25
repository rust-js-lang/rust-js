# 0844. A library that reads no count counts no `Rc`

Status: Accepted; amended by [0847](0847-each-types-rcs-are-counted-apart.md):
each type's `Rc`s are counted apart, not a library's every one or none,
its contract's `counted_rcs` listing the types; and what a body holds
is what it owns, not what's behind a reference or a `Weak`. Amends [0523](0523-libraries-share-one-counted-rc.md),
whose every library counted every `Rc`; and
[0494](0494-a-generic-rc-is-counted-where-it-may-drop.md), whose generic
`Rc` of any type parameter's value counted every one, and whose crate
with no destructor of its own and no library took its type parameters to
drop nothing, a library's too. Extended by
[0850](0850-an-rc-is-defined-as-what-it-points-at.md): an `Rc` is
`js::Defined`, a counted one given to JS refused.

Case ([0262](0262-when-rust-and-js-disagree.md)): A where neither the
library nor a crate sharing its `Rc`s asks a count; D, refused, where a
crate would count an `Rc` a library that counts none hands it.

## Context

ADRs 0843 and 0846 let React's port write `Rc`s and `Weak`s where it had
leaked `Box`es: a context whose `Weak`s are itself, set after it's made,
and a lazy's payload shared by its callbacks. A program printed them as
upstream does. But Cargo builds every crate as a library, React's
`react` too, and a library counted every `Rc` (ADR 0523), so
`createContext` was:

```js
const context = {
  value: { $$typeof: REACT_CONTEXT_TYPE, /* .. */ Provider: {value: $weakNone}.value, Consumer: {}.value ?? null },
  strong: 1,
  weak: 0,
};
let tmp;
try {
  tmp = $downgrade(context);
} catch (error) {
  $rcDrop(context, value => { /* .. */ });
  throw error;
}
// .. 80 lines more
```

Not upstream's, and not what its JS consumers read: React's reconciler
reads `context._currentValue`, a field of the context, not of a box.
ADR 0523 counted every `Rc` because a library and its consumers must
agree on what one is, whatever generic code, callbacks and fields hand
across; it couldn't list which types count. It needn't: a library either
needs counts, and counts every `Rc`, or needs none.

And a library whose generic code held an `Rc` of a type parameter's
value, `Rc<ReactContext<T>>`, counted every `Rc` where a type might have
a destructor (ADR 0494), though `createContext`'s `T: Copy` can't have
one; while a library with no destructor of its own and no library took
every type parameter to drop nothing, which only its `Rc`s all counted
had hidden: a consumer's `Loud` of an associated type was never dropped
(`two crates: assoc_drop`).

## Decision

**A library counts every `Rc`, or none: every one where it reads a count,
has an `Rc` of a value with a destructor, holds a generic one of what may
have a destructor, or a counting library hands it one; else none, each
the value itself. Its contract says which, `counts_rcs`. A crate an
uncounting library hands an `Rc` of its own items counts none too, and
is refused where it would.**

```rust
// react: no count read, no destructor, `T: Copy`
pub fn create_context<T: Copy>(default_value: T) -> Rc<ReactContext<T>> {
    let context = Rc::new(ReactContext { /* .. */ provider: Cell::new(Weak::new()), consumer: Cell::new(None) });
    context.provider.set(Rc::downgrade(&context));
    context.consumer.set(Some(Rc::new(ReactConsumerType { r#typeof: REACT_CONSUMER_TYPE, context: Rc::downgrade(&context) })));
    context
}
```

```js
const context = { $$typeof: REACT_CONTEXT_TYPE, /* .. */ Provider: null, Consumer: null };
context.Provider = context;
context.Consumer = { $$typeof: REACT_CONSUMER_TYPE, _context: context };
```

- **One owner** (`counted_rcs`, told `Shared`): a library's analysis is
  a program's, and where it counts any `Rc` it counts every one; a crate
  a counting library hands an `Rc` (`Reached::rcs`) counts every one, as
  before.
- **What an uncounting library hands** (`Reached::uncounted_rcs`): an
  `Rc` or a `Weak` of its own items' declared types, their signatures and
  fields, not of a type parameter a consumer gives, which its code reads
  only by the dictionaries it's given, clone and drop, the consumer's. A
  crate that would count any `Rc` while one hands it such is refused:
  "this crate counts an `Rc<dep::Context>`, .., but `dep`, a rust-js
  library that counts none, hands it `Rc`s, which both share as the value
  itself". Two crates can't read one value apart.
- **A generic `Rc` counts where a body holds it of a parameter that may
  have a destructor** (`owns_dropping_param`): not a `T: Copy`, by the
  body's bounds; and through the fields of the crates' types a body's
  values and parameters hold, `Holder<T>`'s `Rc<T>` dropped by
  `release(holder)`, which no expression names. A field nothing holds no
  longer counts: a consumer that makes one counts it itself, or is
  refused.
- **A library's type parameters may drop** (`may_have_destructors`, the
  one answer, given whether the crate's a library): its consumers' types
  may have destructors (ADR 0163), whatever it counts. Drops had read
  "a library counts its `Rc`s" for it.
- **`ptr_eq` of a library's struct with data fields is its identity** as
  the crate's is (ADR 0582): the object its consumers share. Not of a
  binding's, of markers alone, whose value may be any JS value. Writing
  this found `ptr_eq` of a transparent struct, its number, was `===` of
  the numbers (ADR 0582, amended).
- **A contract a compiler wrote before counted every `Rc`**: one with no
  `counts_rcs` is read as counting, never as counting none.

## Why

- **Correct across crates**: every crate that shares an `Rc` reads it as
  one representation, the counted box or the value, and none counts what
  another doesn't; what can't agree is refused.
- **It's upstream's JS**, and what React's JS consumers read.
- **Per library, not per type**: a library's generic code, callbacks and
  fields hand `Rc`s no list of types can name (ADR 0523's reason), and
  needn't: all or none is one bit its contract carries.

## Alternatives

- **Count each type a library or its consumers read a count of**: a
  generic function can't say which `T`s its consumers count. ADR 0523's,
  rejected again.
- **Count where a library's `Rc` crosses to another rust-js crate**: a
  library can't see its consumers; JS ones need the value itself.
  Rejected.
- **React writes `T: Copy` on its struct**: the struct's own bound, which
  upstream's type hasn't, bent for the compiler; and a library's field
  nothing holds counts nothing anyway. Rejected.

## Consequences

- A library that reads no count, holds no destructor's `Rc` and no
  generic `Rc` of what may drop is plain JS where it was boxes: React's
  `createContext` and `lazy`.
- `the app's crate is a program where no crate uses it`
  (`test/cargo-workspace.test.ts`) reads a count in `app`, the library it
  shows counting; `user`, a library reading none, counts none.

## What holds it

- `test/crates.test.ts`, "a library that reads no count counts no Rc, and
  a crate counting one it hands is refused": React's context in a
  library, its contract's `counts_rcs`, its JS and its consumer's, a
  consumer reading a count refused, and `rc_generic_holder`'s library,
  which counts; `two crates: rc_uncounted` and `two crates:
  rc_generic_holder` run with native Rust; `three crates` share counted
  ones as before; `two crates: assoc_drop` drops a consumer's value of an
  associated type.
- Every corpus case compiled as a library, counting none where it reads
  none, against native Rust.
- `test/corpus/rc_of_param_unnamed.rs`: a generic function's `Rc` of its
  parameter's value, which its caller's expressions don't name, counted,
  dropping a `Loud` with the last clone, with native Rust; ADR 0494's
  `generic-rc-uncounted` holds it, as `generic_rc_drops`' `Tee`, its
  `Rc` in a field its caller's values reach, is now counted by its
  `Loud`.
- Mutations in `scripts/mutations/lower/analysis/rc_counts.ts`
  (`library-counts-every-rc`, `library-counts-some-rcs`,
  `copy-param-dropping`, `held-fields-unseen`),
  `scripts/mutations/lower/library.ts` (`uncounted-library-rcs-unseen`,
  and `library-rc-unreached`, ADR 0523's, of a counting one),
  `scripts/mutations/lower/analysis.ts` (`consumer-rcs-uncounted`,
  `library-rcs-uncounted`, ADR 0523's, moved),
  `scripts/mutations/lower/traits/drop_params.ts`
  (`library-params-never-drop`), `scripts/mutations/library.ts`
  (`contract-counts-unread`) and
  `scripts/mutations/lower/recognition/wrappers.ts`
  (`ptr-eq-of-library-struct-counts`).
