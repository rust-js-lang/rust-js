# 1189. A shared borrow of an array is declared `readonly`

Status: Accepted (2026-11-23, #2392; Tu approved, through the lead,
m6-react, with React's port's declarations moved). Amends
[0196](0196-typescript-declarations.md)'s declarations.

## Context

Rust says in a type whether code may change what it's lent: `&[T]` is
read, `&mut [T]` may be changed. A crate's declarations wrote both as
`T[]`, since `declarations.rs` declared a shared reference as what it
refers to. So the promise Rust keeps didn't reach TypeScript, and the
`.d.ts` refused what Rust takes. A TypeScript caller holding a `readonly
string[]` couldn't call a function that only reads it:

```ts
const frozen: readonly string[] = ["apple"];
withItem(frozen, "pear");
// error TS4104: The type 'readonly string[]' is 'readonly' and cannot be
// assigned to the mutable type 'string[]'.
```

Found writing coming-from-typescript's chapter 2, `let` and `let mut`.

## Decision

**A shared borrow of an array, a slice or a `Vec`, `&[T]`, `&[T; N]`,
`&Vec<T>`, is declared `readonly T[]`, wherever a type is declared: a
parameter, a result, a field. A `&mut` one is `T[]`, as it was.**

One rule, in `declarations.rs`' borrow case: a shared reference is what
it refers to, made `readonly` where that's an array. It is shallow, as
TypeScript's `readonly` is: `&[Vec<u8>]` is `readonly number[][]`. `&str`
is a `string`, and `&` of anything that isn't an array is what it
refers to, as before. The JS doesn't change: what's lent is the array
itself.

| Rust | Before | Now |
|---|---|---|
| `cart: &[String]` | `string[]` | `readonly string[]` |
| `-> &'a [String]` | `string[]` | `readonly string[]` |
| `items: &'a [String]`, a field | `string[]` | `readonly string[]` |
| `cart: &mut Vec<String>` | `string[]` | `string[]` |
| `Cow<'_, [f64]>`'s `Borrowed` | `number[]` | `readonly number[]` |

## Alternatives

- **Parameters only.** A result or a field lent to TypeScript is as much
  Rust's, and a caller that changes it changes what Rust holds. One rule
  for each position is the simpler promise.
- **Deep `readonly`.** TypeScript has no deep `readonly` array of one
  word; `ReadonlyArray<ReadonlyArray<T>>` of every level would be noise,
  and TypeScript's own convention is shallow.

## Consequences

- Declarations change wherever a crate has such a type: the tests'
  expectations, the guide's generated examples (`bun run docs`), and
  React's port, whose `.d.ts` is regenerated and its pin moved.
- Held by test/declarations-readonly.test.ts, type-checked: a `readonly`
  array accepted where Rust reads, refused where it takes `&mut`, and a
  borrowed result's `push` refused; and the mutation
  `shared-borrow-array-mutable`.
