# 0230. A bound of a trait with nothing in it passes no dictionary

Status: Accepted. Amends [0049](0049-traits-and-generics.md): a generic
function takes no dictionary of a trait with nothing in it. Amends
[0229](0229-union-parameters.md): a union's trait of the crate's own
passes none either.

## Context

A generic function takes a dictionary of each bound (ADR 0049), what its
body calls through:

```rust
pub trait Marker {}
pub fn named<T: Marker>(value: T) -> u32 { .. }
```

```js
export function named(value, TMarker) { .. }
```

But `Marker` has nothing in it, so nothing reads `TMarker`. It's an
argument every caller makes, `named(1, u32Marker())`, for nothing, and a
JS caller of `named` has to know to. Its `.d.ts`, `named<T>(value: T)`,
says it takes one argument. A sealed trait (ADR 0229's `IntoUploadBody`),
a `Sealed` one, and any trait that only says what a type may be are such
traits.

## Decision

**A bound of a trait of the crate's or a library's with nothing in it, no
method, constant or type, and whose supertraits have nothing either,
passes no dictionary:**

```js
export function named(value) { .. }
export function local(value) { .. }     // of `value: impl Marker`
```

- **A supertrait counts**: `trait Sub: Marker {}` is one, as `Marker` is,
  and so is one of `Send`, which has no dictionary. `trait Copied: Copy
  {}` isn't: `Copy`'s dictionary copies, what generic code of a
  `T: Copied` reads through it.
- **A library's, the same**: a consumer and the library work it out from
  the trait, which the library's metadata has (ADR 0100), so they agree.
- **An impl still has its dictionary**, `u32Marker()`, as a `dyn Marker` is
  a value and its impl's (ADR 0049), whose `$drop` drops it (ADR 0098).
- **A `dyn` of a type parameter makes its own**, as no bound gave one:
  `Box::new(value)` of a `T: Marker` is `{ value, impl: { $drop: dropT }
  }`, its drop the one the caller gives for `T` (ADR 0106), and a
  supertrait's the same, `{ Marker: () => .. }`; `{}` of a type with no
  destructor.

## Why

- **The JS takes what the Rust takes**: `named(value)`, which a person
  writes, and which the `.d.ts` says.
- **Nothing is lost**: what a dictionary of nothing holds is its
  supertraits' dictionaries, of nothing too, and of a `dyn`, its drop,
  which is made where the `dyn` is.
- **It's tested**: `local`, `named` and a `T: Sub` take only their value,
  a `T: Copied` its dictionary still; values of each compute as they do
  natively; a `dyn` boxed by generic code of a type with a destructor is
  dropped. Four mutations put each rule back.

## Costs

- **An impl's dictionary is still written**, `export function
  u32Marker()`, though only a `dyn` reads it.
- **A trait with methods still passes one**, as ADR 0049's are usable from
  JS, and a `.d.ts` doesn't declare it: that's ADR 0049's, not this.
