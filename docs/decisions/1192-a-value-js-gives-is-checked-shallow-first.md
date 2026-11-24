# 1192. A value JS gives is checked against its Rust type, shallow first

Status: Accepted (2026-11-24, #2403 step 2a; the user's decision to check
at the boundary, its design approved by the lead). Amends
[0196](0196-typescript-declarations.md) and
[1190](1190-a-types-boundary-contract-is-one-classification.md): a type's
`Contract` is now read by the checks as well as the declarations. Amended
by step 2b(d) (2026-11-24): an `async fn`'s parameters, and a pattern's
parts, are checked too.

Case ([0262](0262-when-rust-and-js-disagree.md)): **C**. Results change:
a call JS makes with a value outside its parameter's Rust type throws,
where it ran on an impossible value before. Rust's meaning is kept, paid
for in output.

## Context

JS can give a Rust function anything. A `u32` parameter given `-1` was
`4294967294` to the arithmetic, `3.5` a fraction a `u32` can't hold, a
fieldless enum given `"lost"` a variant no `match` arm has, and a
`String` given a lone surrogate no valid UTF-8. The user decided (#2391's
open case, #2403) that every value JS gives is checked against its Rust
type, as wasm-bindgen's glue checks it: a value outside it throws, naming
the function and the parameter, before the body runs.

Step 1 (ADR 1190) made what a type is to JS one classification,
`Contract`, which the declarations print. This is step 2a, the checks of
the values that are whole in themselves. What holds other values, a
struct, a `Vec`, a callback, is step 2b, and branding the types whose
fields can't hold their invariants is 2c.

## Decision

**Each parameter of a function JS may call is checked first in its body,
one statement each, when its type is a number, a `bool`, a `char`, a
string, `()` or a fieldless enum, or an `Option` of one
(`boundary/checks.rs`).**

```rust
pub fn double(n: u32) -> u32 { n * 2 }
pub fn pad(text: &str, width: Option<u32>) -> String { .. }
```

```js
export function double(n) {
  $checkU32(n, "double(n)");
  return (n * 2) >>> 0;
}

export function pad(text, width) {
  $checkStr(text, "pad(text)");
  $checkU32(width, "pad(width)", "nullish");
  ..
}
```

- **Where:** first in the function's own body, built where its parameters
  are final, after tidy (amended, 2b(d)(2), the lead's choice: one rule
  for every parameter, `boundary/patterns.rs`' `parameter_checks`), so
  before #1723's kept copies and anything else the body does. A
  parameter its JS takes whole is checked whole by that name
  (`checks.rs`' `whole_check`); one its JS takes apart, each part by its
  path (below). The parameters its JS takes are its `JsSignature`'s, one
  for one; where they aren't, a function with a check fails to compile, an
  internal error naming it, never a function left unchecked.
- **Which functions:** every JS-facing one (`js_facing`). Not:
  - a closure's body, a promoted constant's, or a copy only Rust calls;
  - a function no JS caller can call;
  - a binding, whose JS is JS's own;
  - a function that only passes its parameters on to one JS may call
    of no type arguments, which checks them itself, so its module still
    re-exports that one in its place (rd575, ADR 1064);
  - `@rust-js/std`'s functions, compiled once, which only Rust calls.
- **An `async fn`** (amended, step 2b(d)): its checks are first in its
  own body, the async function's, so a value JS gives that isn't of its
  type rejects the promise, as every throw of an async function does,
  `later(n): expected a u32, got -1`.
- **Which parameters:** each. One Rust names plainly, `n: u32`, by that
  name in the error, where its body starts.
- **A pattern** (amended, step 2b(d)(2), the lead's choice):
  `Props { title, children }: Props`, `(w, h): (u32, u32)` or `&n: &u32`
  (an `async fn`'s, which rustc names `__arg0`, too). It keeps its shape
  where it's given, `Card({ title, children })`, and each part it binds is
  checked by its path, the parameter named as its `.d.ts` names it,
  `$checkStr(title, "Card(props.title)")`
  (`boundary/patterns.rs`).
  - **That's the whole check:** Rust reads only what the pattern binds; a
    field it skips, `..`, is never read. A part JS binds that the body
    never reads, `lang: _`, which keeps `lang` out of a `...rest`, isn't
    checked, unless its type has drops, which would read it.
  - **A callback it binds** is checked as a parameter's is (ADR 1195):
    `rule = $checkCallback(rule, "ruled(props.rule)", $checkU32)`.
  - **A `null` given** takes JS's own `TypeError` as it's taken apart,
    before Rust runs. No check of ours is added for it: it would only
    change the message.
  - **Read where the parameters are final,** after tidy, so whichever
    shape it chose is checked as it is: a name it didn't take apart,
    `deref(n)`, whole.
  - **Checking it whole was tried and refused:** `Card(props)` and a
    `const` of each part changed every destructuring component's shape.
- **Its cost: a pattern's function isn't split** (ADR 1193), as its entry
  can't give the value whole to its body. A Rust caller of one pays its
  parts' checks. A component, the common case, is called by React, not by
  Rust, so it pays nothing it wouldn't. A split function's parameters are
  all named plainly, and tidy takes apart only a pattern's temporary,
  never a named one, so a split function's entry always holds each value
  whole.
- **What's checked, by its `Contract`**, the one the `.d.ts` prints, so the
  two can't disagree:

  | Rust | Checked | Error |
  |---|---|---|
  | `u8` `u16` `u32` `usize` `i8` `i16` `i32` `isize` | a number, whole, in range | `TypeError` if not a number, `RangeError` if not whole or out of range |
  | `u64` `i64` `u128` `i128` | a bigint that `BigInt.asUintN`/`asIntN` of its width leaves as it is | as above |
  | `f64` | a number, NaN too | `TypeError` |
  | `f32` | a number `Math.fround` leaves as it is, or NaN | `RangeError` |
  | `bool` | `true` or `false` | `TypeError` |
  | `char` | one code point, no lone surrogate | `TypeError` |
  | `&str`, `String` | a well-formed string, `isWellFormed()` | `TypeError` |
  | `()` | `undefined` | `TypeError` |
  | a fieldless enum | one of its variants' names | `TypeError` naming them |

  A shared borrow is the value it borrows, so `&str` is a string. A
  fieldless enum counts only if each of its variants is its name's string:
  not untagged, not of a `#[rust_js::tag]` of its own, no `#[repr]`
  numbers, no `bool` variants.
- **An `Option`'s `None` is what its parameter's form reads as one,** given
  as the helper's last argument:
  - a plain `Option`: `null` or `undefined`, `"nullish"`;
  - `#[rust_js::nullable]`: `null`, `"null"`;
  - `#[rust_js::none_undefined]`: `undefined`, `"undefined"`.
- **The error** names the function's JS name (a method's after its
  type's, `Cart.add`), the parameter, what was expected and what was
  given: `RangeError: rust-js: double(n): expected a u32, got -1`. It's
  thrown before the body runs, so it's the caller's error, not a panic.
- **The helpers** are `std/runtime.js`'s, one for each kind, `$checkU32`,
  `$checkStr`, `$checkOneOf`, each taking the value, where it is, and the
  `None` its form allows.
- **Not checked yet:**
  - a struct, a tuple, a `Vec`, a map, an enum with fields, a callback, a
    `dyn` pair, a `&mut` box, and a binding's results (2b);
  - a `NonZero`, a pattern type and a `#[repr]` enum, narrower than their
    kind (2b);
  - JS's own types, `Unknown` and type parameters, which are any value.
    These are never checked.
- **A Rust caller pays the shallow check too,** as the function is one
  function. Splitting a function into its JS entry and its Rust body comes
  with 2b's deep checks, whose cost is a value's size.

## Why

- **Rust never sees an impossible value.** A check where the value enters,
  once, instead of each read.
- **One answer to "what is it to JS".** The check asks `Contract::of`, as
  the declarations do, so a value the `.d.ts` allows is one the check
  allows.
- **Cheap where it's shallow.** Measured, `bun` on an M-series Mac, each
  case its own process, the median of 7:

  | Case | Unchecked | Checked |
  |---|---|---|
  | `double(n: u32)` | 1.60 ns | 2.24 ns |
  | five parameters: two strings, a `char`, a `bool`, an enum | 0.60 ns (folded) | 13.18 ns |
  | a 10-character string's `isWellFormed` | 0.63 ns | 8.21 ns |
  | 1,000 characters | 1.18 ns | 34.16 ns |
  | 100,000 characters | 38.54 ns | 2,805 ns |

  `isWellFormed` is about 0.03 ns a character, less than any Rust code that
  reads the string pays.

## Alternatives

- **Checks in the `.d.ts` alone:** TypeScript's callers only, and `number`
  is every width.
- **A string's type only, `typeof`:** O(1), but a lone surrogate is no Rust
  `String`, and rust-js's byte counts assume it's well-formed.
- **Separate helpers for each `None`, `$checkU32OrNull`:** more names for
  one rule; one helper of each kind takes the `None` its form allows.

## Consequences

- Every exported function with such a parameter starts with its checks,
  measured on main 01cee508c:

  | Output | Files that change | Bytes before | After |
  |---|---|---|---|
  | the corpus | 64 | 9,754,049 | 9,763,946 (+0.10%) |
  | the examples' snapshots | 37, and the playground's 6 | 416,860 | 427,483 (+2.55%) |
  | the React port | 159 | 19,901,522 | 19,962,018 (+0.30%) |

  The port's checks are mostly strings (a DOM id, an attribute's name),
  then `i32`s and `f64`s (lanes, times).
- `#[rust_js::trusted]`, to skip a check on a hot path, waits on these
  numbers and the user's decision.

## What holds it

- `test/boundary-checks.test.ts`: each kind accepted and refused, with its
  exact message; each form's `None`; a struct and a tuple left to 2b; a
  private function and a Rust caller unaffected.
- The mutations in `scripts/mutations/lower/boundary/checks.ts`.

## Amendment: what JS may call is checked, in one `$checks` (#2403 lever (3) and (A), Tu's decisions, 2026-11-24)

**What's checked.** A function's parameters are checked where JS may call it, not where it's `pub`
(`FnCx::js_callable`). That's what the crate's root gives JS, its own items and its `pub use`s, as its
package's entry does (ADR 1008); what each module a hand-written JS file of the program or its Cargo workspace
imports gives (`JsModules::hand_imported`, a scan of their specifiers, deep imports through `node_modules`
too); each forwarder's target there; and a class's members wherever rustc's effective visibility exports the
class. One owner decides it, `reexports::JsReached`, the same resolution the root's JS is written from, so
the root's exports and what's checked can't disagree. A `pub mod` path alone, which only Rust names, isn't
reason to check: the port's crates are `pub mod` throughout, and their checks took it from +8.52% to +4.61%
raw. **The limit:** JS rust-js never compiles against that deep-imports a module no hand-written JS imports
reaches its functions unchecked, as any package's internals (the guide's limitations). A library's manifest
says per item whether its own JS checks it (`Item::checked`, library ABI 49), so a consumer that gives JS one
it doesn't checks it itself (ADR 1193, amended).

**How.** A function's checks of a value and its check alone are one statement, `$checks("scaled(p, by, label)",
p, checkPoint, by, $checkU32, label, $checkStr)`, the place written once; a failure names its parameter by its
index, and its path inside it, as before: `scaled(p.y): expected a u32, got -2`. Where it's no shorter, a
lone leaf's, one that says its `None`, a fieldless enum's or a callback's, it stays its own statement. Each
place string is kept, so a message survives minification. The port is +2.92% gzip, +4.32% raw over its pin;
gzip is the size measure (Tu's decision).
