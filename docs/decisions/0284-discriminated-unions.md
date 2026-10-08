# 0284. An enum tagged by a property of its own is a discriminated union

Status: Accepted. Extends [0033](0033-enums-with-fields.md) and [0196](0196-typescript-declarations.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An enum with fields is an object tagged `TAG` (ADR 0033):
`{ TAG: "Rect", w, h }`. JS's own APIs tag their objects by a property of
their choosing: `Promise.allSettled` gives `{ status: "fulfilled", value }`
or `{ status: "rejected", reason }`, an Intl part `{ type, value }`, a
Redux action `{ type: "add", .. }`. A Rust program couldn't type them: an
enum was `TAG`'s, and a struct of `Option`s says nothing of which fields
go together.

- **TypeScript** has discriminated unions: a union of object types that
  share a literal property, `{ status: "fulfilled"; value: T } |
  { status: "rejected"; reason: any }` (`lib.es2020.promise.d.ts`), which a
  `switch` on `status` narrows.
- **ReScript** has `@tag("status")` on a variant, each constructor `@as`
  its value, `Fulfilled({value: 'a})` an object of `status` and its fields
  (`Stdlib_Promise.resi`'s `settledResult`).

rust-js's `.d.ts` declared an enum with fields `any` (research
[type-foundations.md](../research/type-foundations.md), its third gap),
where ReScript's genType declares the union.

## Decision

**`#[rust_js::tag = "status"]` on an enum makes it a discriminated union:
each variant an object of the tag, its name, and its fields.**

```rust
#[rust_js::tag = "status"]
pub enum Settled<T> {
    #[rust_js::name = "fulfilled"]
    Fulfilled { value: T },
    #[rust_js::name = "rejected"]
    Rejected { reason: &'static Unknown },
}
```

```js
{ status: "fulfilled", value: 1 }      // Settled::Fulfilled { value: 1 }
if (s.status === "fulfilled") { .. }   // match s { Settled::Fulfilled { value } => .. }
```

- **Its tag is the property it names, in place of `TAG`**, wherever a
  variant is made, matched, compared, copied or dropped, and of a `const`.
- **A variant without fields is an object of its tag too**,
  `{ status: "pending" }`, as TypeScript's union members are objects:
  where an enum without a tag is its name, `"pending"`.
- **Its variants' fields are named.** A tuple variant's would be `_0`
  beside the tag, which no JS API has: an error, and so is a field named as
  the tag is.
- **Its declaration is TypeScript's discriminated union**, and so is any
  enum with fields': `{ TAG: "Circle"; _0: number } | "Empty"`, each
  variant as rust-js makes it, as ReScript's genType declares one.

## Why

- **It's how JS's APIs and both languages model it**, so a binding to
  `allSettled` or to an Intl part is typed as TypeScript and ReScript type
  it, and a `match` reads as a TypeScript `switch` does.
- **One attribute, a property's name**, as `rust_js::untagged` (ADR 0214)
  chooses another layout of the same enum: rustc checks the variants, and
  the names are `rust_js::name`'s (ADR 0013).

## Alternatives

- **A struct of `Option`s**, `{ status: String, value: Option<T>, reason:
  Option<..> }`: what there was, with no type saying which go together.
- **ReScript's per-variant tag values without objects for fieldless
  variants**: JS's APIs give objects; a bare string there would be a value
  no API makes.

## Consequences

- A constant of an enum's variant now writes its `rust_js::name` under
  `TAG`, as one made at run time does: it wrote the Rust name.
- `TAG` stays the default; nothing without the attribute changes, but its
  `.d.ts`, which is now the union, not `any`.
- A tagged enum whose variants have no fields is declared as JS has it,
  `{ kind: "on" } | { kind: "Off" }`. (Amended: it was declared as an
  untagged one's names, `"on" | "Off"`.)
