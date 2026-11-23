# 1187. A tuple struct no caller can make is declared opaque, branded by its module's own symbol

Status: Accepted (2026-11-23, #2356). Extends [0196](0196-typescript-declarations.md)
and [1149](1149-std-types-declared-opaque.md), whose branded form it gives
a crate's own type.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Only declarations
change; the JS is what it was.

## Context

A newtype with a private field is how Rust keeps a refinement: an
`Email(String)` that only `TryFrom` makes, `#[serde(try_from = "String")]`
for JSON, so every `Email` in the program passed the check, and a function
that takes one needs no check of its own
([coming-from-zod](https://github.com/rust-js-lang/coming-from-zod)'s
chapter 2, which found this):

```rust
pub struct Email(String);

pub fn parse(text: &str) -> Result<Email, String> { .. }
pub fn welcome(email: &Email) -> String { .. }
```

A tuple struct had no declaration (`declares_type`, ADR 0210), so what
named one named `any`:

```ts
export function parse(text: string): { TAG: "Ok"; _0: any } | { TAG: "Err"; _0: string };
export function welcome(email: any): string;
```

TypeScript took `welcome("not an email")`, and the JS read
`"not an email"[0]`, `"n"`, as the email, with no error: the promise Rust
keeps, that nothing makes an `Email` but its crate's code, was dropped at
the boundary. AGENTS.md counts a declaration looser than the Rust, `any`
included, as a gap to fix.

## Decision

**A tuple struct a caller outside its crate can't make, one with a field
that isn't `pub`, of no type parameter, is declared an interface branded
by a `unique symbol` its module declares and doesn't export:**

```ts
declare const emailBrand: unique symbol;

export interface Email {
    readonly [emailBrand]: never;
}

export function welcome(email: Email): string;
```

That is ADR 1149's form, which `@rust-js/std` gives `LazyLock<T>`: no code
outside the module can name the key, so TypeScript takes an `Email` only of
what rust-js's JS gave and gives it back. It refuses a string, an array, an
object, and a cast, `"x" as Email`, of types that don't overlap; only `as
unknown as Email`, TypeScript's own escape, gets one past it. The brand is
the type's name, its first letter lower, `Brand` after it, declared in the
type's module; another module that names the type imports it, as any
declared type (ADR 0210), and the brand stays its module's.

`declarations/structures.rs`' `sealed_tuple` is the one answer of which
tuple structs are so; `structure` declares one so, and `declares_type`
reads it, so a reference to one is its name where it was `any`.

Left as they were:

- **A tuple struct whose fields are all `pub`**, which any caller makes,
  `Open(pub String)`: still undeclared, `any`. Its declaration, the array
  its JS is, is another decision.
- **A generic one**, `Id<T>(u64, PhantomData<T>)`: a brand without `T`
  would make every `Id<T>` one type. Branding it by `T`, `readonly
  [idBrand]: T`, is another decision.
- **A binding's type**, one whose `#[rust_js::types]` names JS's
  (`Color(PhantomData<JsObject>)`): its declaration is JS's, imported; a
  second, branded here, would conflict with it.
- **A handle of `PhantomData` alone**, which holds no value Rust made: what
  JS gives it is JS's to declare.
- **A `#[repr(transparent)]` struct**, which ADR 0584 makes its field,
  `type Meters = number`, and **a struct of named fields**, declared an
  interface of them, a private field included: each is a type TypeScript
  can make, but changing either changes a declaration a caller may rely
  on, and is decided apart, if at all.

Alternatives:

- **The array its JS is, `[string]`.** TypeScript would make one,
  `["not an email"]`, and an array's shape is rust-js's representation,
  made an API (ADR 1149's reason against `{ init }`).
- **`string & { __brand: "Email" }`, zod's form.** A string casts to it in
  one `as`, and it says an `Email` is a string, which its JS, `[text]`,
  isn't.

## Consequences

- A crate's newtype of a private field is a type in its `.d.ts`, and what
  takes or gives one names it.
- Only declarations change, so no JS, and no caller that type-checked
  against a declaration of `any`, which takes everything, stops working.
  A caller that passed something else where an `Email` is asked for now
  fails to type-check, which is the fix.
- Held by `test/declarations-sealed.test.ts`: two modules' newtypes,
  imported across them, strict TypeScript callers given and refused, the
  JS run, and the tuple structs left as they were, a binding's type of a
  value-holding field, `#[rust_js::types = "Map<string, number>"]`,
  among them; `test/declarations-relative-imports.test.ts`, a binding's
  `PhantomData` handle; and mutations `sealed-tuple-undeclared`, `sealed-tuple-brand-exported`,
  `sealed-tuple-of-pub-fields`, `sealed-tuple-generic`,
  `sealed-tuple-handle`, `sealed-tuple-binding-type` and
  `sealed-tuple-referenced-any`.
