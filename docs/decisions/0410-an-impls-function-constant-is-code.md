# 0410. An impl's constant of a function is lowered as code

Status: Accepted. Extends [0031](0031-consts.md),
[0047](0047-methods.md) and [0176](0176-generic-impl-consts.md). Amended
(#498): a `pub` type's value constants are its object's too, below; (#513):
one of a value no value tree says, its initializer's.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): one JS function is
every use of the constant, as Rust's one function pointer is; nothing can
tell them apart.

## Context

A free constant whose value rustc's value tree can't say, a function's,
a closure's or a `dyn`'s, is lowered as code, a constant of its module
(ADR 0096). An impl's associated constant of one,
`const ID: fn(&S<T>) -> &S<T> = |s| s;`, `const TABLE: [(&str, fn(i32) ->
i32); 2]`, or a trait impl's `const ASSOC: fn(u32) -> u32`, was refused:
its initializer was never lowered, and a trait's constant was looked up by
the trait's item, not its impl's.

## Decision

**An impl's associated constant whose type has a function, a closure or a
`dyn` in it is lowered as code, once: an inherent impl's is a property of
its type's object, `S.ID`, before its methods, as a hand-written object of
functions has them; a trait impl's is a constant of its module, which its
dictionary reads. A trait's constant of a type is its impl's.**

```js
export const S = {
  ID: (s) => s,
  id(s) {
    return S.ID(s);
  },
};
const ASSOC = (x) => (x + 1) >>> 0;
```

- **Only what the value tree can't say**: any other impl constant is
  still its value where it's used, `S.ZERO` of `0` is `0`, as before.
- **A generic trait impl's** stays its dictionary's, of its parameters'
  evidence (ADR 0176); a trait's default, its `Self`'s.
- **A library's dictionary** reads the module's constant, `get ASSOC() {
  return ASSOC; }`, where it read a generic impl's initializer.

## Why

- **It's the program Rust runs**: the `impl_fn_consts` corpus case calls
  an inherent impl's closure constant directly and through a method, a
  trait's through a type and through generic code, a generic impl's of its
  parameter's `Default`, and a table of named functions, each with native
  Rust.
- Three of rustc's tests pass: `mismatched_impl_ty_2.rs`,
  `mismatched_impl_ty_3.rs`, `issue-58435-ice-with-assoc-const.rs`.

## Consequences

- A trait's default constant of a function, `<()>::vftable`, is still
  refused where it's read concretely, and a generic impl's constant made of
  std's `UnsafeCell`.

## Amendment: a type's value constants are its object's (#498)

Case ([0262](0262-when-rust-and-js-disagree.md)): A, JS's own form, nothing
different to observe: each use is its value, as before.

alien-signals exports its flags, `export const enum ReactiveFlags { None
= 0, Mutable = 1, .. }`, compiled to `export const ReactiveFlags = { None:
0, Mutable: 1, .. }` and each use inlined, `flags & 2`; its test reads them
from JS, `getActiveSub()!.flags &= ~ReactiveFlags.RecursedCheck`. Rust's
form is a type's constants of the field's `i32`, `impl ReactiveFlags { pub
const MUTABLE: i32 = 1; }`, as a fieldless enum's variants aren't
combinations. Each use was its value, and the type exported nothing.

**An inherent impl's constant that another crate or JS can name, of a
type whose path is exported, not a class's (ADR 0761) nor a generic
impl's, is a property of its type's object, as its constants of code are,
and each use is still its value where it's written**
(`exported_value_const`, `analysis.rs`):

```js
export const ReactiveFlags = { None: 0, Mutable: 1, RecursedCheck: 4 };

export function watched(flags) {
  return (flags & (1 | 4)) !== 0;
}
```

```ts
export const ReactiveFlags: {
    readonly None: number;
    readonly Mutable: number;
    readonly RecursedCheck: number;
};
```

Its `.d.ts` declares the object, its constants and, since #507, its `pub`
methods with them (ADR 0047, amended).

### Why

- **It's upstream's**: the object, and its uses, as TypeScript compiles a
  `const enum` with `preserveConstEnums`.
- **Uses are values, as before**: what this changes is what's exported,
  never a module that reads one, nor its load order.

### What holds it

- `test/impl-constants.test.ts`: the object, a `#[rust_js::name]`d
  constant, a private one left out, the uses' values, its declaration; a
  private type's and a generic impl's, no object; one of methods too,
  declared whole (#507); a class's, its prototype's.
- Mutations `type-constants-unexported`, `type-constants-of-a-private-type`,
  `type-constants-of-a-class`, `type-constants-of-a-generic-impl`
  (`lower/analysis.ts`), `type-constant-read-by-name` (`lower/items.ts`),
  and `type-constants-writable`
  (`lower/declarations/namespaces.ts`).

### Amended (#513): one no value tree says is its initializer's

A regression (P1, a crate that compiled was refused). semver writes
`impl VersionReq { pub const STAR: Self = VersionReq { comparators:
Vec::new() }; }`, and since #498 its object holds `STAR`. But rustc's
value tree can't say a `Vec`, and only a constant of code (`impl_const`)
kept its initializer, so semver was refused: "constants of type
`VersionReq`". **An exported type's constant keeps its initializer too**
(`collect_initializers`), so a value no tree says is lowered as code, as a
module's `const EMPTY: Vec<i32> = Vec::new()` is (ADR 0031). Its object
holds the value, `export const Req = { STAR: { items: [] } }`, and each
use copies it where Rust copies.

Held by `test/impl-constants.test.ts`'s "a type's pub constant of a value
no value tree says is its initializer's", run, and `bun
scripts/crate-corpus.ts --check semver`; mutation
`type-constant-initializer-dropped` (`lower/analysis.ts`).

The first form of that fix broke chrono and rust_decimal, which compiled but
threw as they loaded: `export const MIN = { ...MIN }`, "Cannot access 'MIN'
before initialization" (a regression of 882cfa286). chrono writes
`pub(crate) const MIN: TimeDelta = ..` beside `impl TimeDelta { pub const
MIN: Self = MIN; }`. The type's constant, now lowered as code, was its
module's `const` of its name, `MIN`, and its object took its constants
from the module's by that name, so it took the module's own `MIN` and left
the type's reading itself. **A type's constant goes to its object by its
item, never by its name** (`owned_consts`, `pipeline.rs`): `export const
TimeDelta = { MIN, MAX, .. }` of the module's `const MIN = { .. }`. Held
by the corpus's `type_constant_of_a_module_constant`, chrono's shape,
against native Rust, and `bun scripts/crate-corpus.ts --check`; mutations
`type-constant-of-a-value-by-name` and `type-constant-of-code-module-level`
(`lower/pipeline.ts`).
