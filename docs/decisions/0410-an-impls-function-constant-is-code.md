# 0410. An impl's constant of a function is lowered as code

Status: Accepted. Extends [0031](0031-consts.md),
[0047](0047-methods.md) and [0176](0176-generic-impl-consts.md). Amended
(#498): a `pub` type's value constants are its object's too, below.

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

Its `.d.ts` declares the object where it's constants alone; one with `pub`
methods too stays undeclared, as an object of methods is (#507), not
declared in part.

### Why

- **It's upstream's**: the object, and its uses, as TypeScript compiles a
  `const enum` with `preserveConstEnums`.
- **Uses are values, as before**: what this changes is what's exported,
  never a module that reads one, nor its load order.

### What holds it

- `test/impl-constants.test.ts`: the object, a `#[rust_js::name]`d
  constant, a private one left out, the uses' values, its declaration; a
  private type's and a generic impl's, no object; one of methods too,
  undeclared; a class's, its prototype's.
- Mutations `type-constants-unexported`, `type-constants-of-a-private-type`,
  `type-constants-of-a-class`, `type-constants-of-a-generic-impl`
  (`lower/analysis.ts`), `type-constant-read-by-name` (`lower/items.ts`),
  `type-constants-declared-in-part` and `type-constants-writable`
  (`lower/declarations/namespaces.ts`).
