# 0214. An untagged enum is its payload: TS's `string | Blob`

Status: Accepted. Extends [0033](0033-enums-with-fields.md): an enum marked
`#[rust_js::untagged]` has no tag. Amended by [0229](0229-union-parameters.md): a
binding's parameter of one is `impl` a sealed trait of its members, of an
ordinary function (ADR 0039), as an `extern` one can't be generic.

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

TypeScript says "a string or a `Blob`" with a union, `string | Blob`, and
needs no wrapper: a JS value knows what it is, `typeof src === "string"`,
`src instanceof Blob`. The web platform is full of them: `fetch`'s
`RequestInfo`, `new Response(body)`'s `BodyInit` of six, `before(Node |
string)`. Rust erases types, so its sum type, an enum, carries a tag, and
rust-js's enums are tagged objects, `{ TAG: "Blob", _0: b }` (ADR 0033),
which no JS API takes. The webapi crate gave a function per member,
`before` and `before_with_str` (ADR 0024).

How the others do it (checked in local clones):

- **ReScript** (`@unboxed`, since 11): a variant whose value is its payload;
  `switch` tests `typeof`, `Array.isArray` or `instanceof` of std's classes,
  `Blob` and `File` among them. At most one case of each runtime kind, or
  the tests couldn't tell them apart (`compiler/ml/ast_untagged_variants.ml`).
- **Scala.js** (`js.|`): a type only the compiler sees, entered by implicit
  evidence, and taken apart by a cast, `isInstanceOf`, which nothing checks
  is exhaustive (`library/.../js/Union.scala`).

A Rust trait can't stand in: an `extern` function, a binding, can't be
generic, and a struct's field, a prop, can't be an `impl Trait`.

## Decision

**An enum marked `#[cfg_attr(rust_js, rust_js::untagged)]` is its payload,
as ReScript's `@unboxed` is:**

```rust
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Src<'a> { Text(&'a str), Bytes(&'a ArrayBuffer) }

impl<'a> From<&'a str> for Src<'a> {
    fn from(s: &'a str) -> Self { Src::Text(s) }
}
```

| Rust | JS |
|---|---|
| `Src::Text("a.png")`, `"a.png".into()` | `"a.png"` |
| `.map(Src::Text)` | `.map((value) => value)` |
| `match s { Src::Text(t) => .., Src::Bytes(b) => .. }` | `if (typeof s === "string") .. else if (s instanceof ArrayBuffer) ..` |
| `load(src)` of `fn load(src: Src)` bound to JS | `load(src)` |

- **Each variant holds one value**, `Text(&'a str)`. A variant of no
  fields, a string literal of TS's `"lazy" | "eager"`, is to come.
- **A variant's test is its payload's runtime kind:**

  | payload | test |
  |---|---|
  | a string, `&str`, `String`, `char` | `typeof v === "string"` |
  | a number of 32 bits or fewer, a float | `typeof v === "number"` |
  | a 64- or 128-bit integer, a BigInt (ADR 0086) | `typeof v === "bigint"` |
  | `bool` | `typeof v === "boolean"` |
  | a `Vec`, an array, a slice, a tuple | `Array.isArray(v)` |
  | a JS object type, `RegExp` (ADR 0111) | `v instanceof RegExp` |
  | one whose `#[rust_js::test]` names what tells it, react's `ReactElement` (ADR 0226) | `isValidElement(v)` |
  | a closure or a function | `typeof v === "function"` |
  | a struct of the crate's | `typeof v === "object"` |

  A JS object type's class is its `rust_js::name`, else its Rust name, as a
  binding's JS name is. Each test is exact, whatever the arms' order: a
  class's leaves out its subclasses in the enum, `File` of `Blob`, which
  `Deref` says, and a struct's every array and class in it.
- **One variant of each kind**, and of each class, as ReScript requires:
  two strings couldn't be told apart. Nor can a payload be what has no
  kind, `Option` (`undefined`), `()`, a generic `T`, another enum: each is
  an error where the enum is declared.
- **The last variant may be what the others aren't**,
  `#[cfg_attr(rust_js, rust_js::otherwise)]`, of a value of any kind, as
  TS's `string | ReactNode` is told by `typeof children === "string"`:
  its test is none of theirs, `typeof value !== "string"`. React's
  `NodeKind`, `Text(&str)` or `Other`, is one, which `react::kind_of(&children)`
  gives a node as, the node itself, as react.dev's Heading labels its link.
  One before another is an error.
- **A `From` into one is the value itself**, at a call, `"a.png".into()`,
  as in generic code (ADR 0108): its `from` must be the variant of its
  argument, `Src::Text(s)`, which is checked where it's declared, so a
  crate that uses another's needs no body to know.
- **`Clone`, `==`, `{:?}` and drops** test the variant as `match` does, and
  read the payload itself where a tagged one reads `_0`.

## Why

- **The value a JS API takes is the one Rust has**: no wrapper made before
  a call, none taken apart after.
- **Exhaustive, unlike TS's narrowing and Scala.js's cast:** `match` must
  name each variant, and rustc checks it.
- **Exact tests** make the arms' order free, as Rust's `match` is for
  disjoint variants.

## Consequences

- A program's own bindings take one where TS takes a union, and so do
  the webapi crate's (ADR 0215).
- A program's own JS class can say what it extends by a `Deref`, as the
  webapi crate's do: its body, a cast of a pointer, is never lowered, as
  a call of it is the object itself.
- Not yet: a variant of no fields, a string literal; a serde
  representation, which serde's own `#[serde(untagged)]` says.
