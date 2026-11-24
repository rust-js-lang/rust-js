# 1195. What a JS function gives back is checked where it returns

Status: Accepted in part (2026-11-24, #2403 step 2c(a), the lead's split of
step (c)). Callbacks' results and the crate's own bindings' results that
aren't text are accepted. A binding's text result is the user's decision,
open below. Amends [1192](1192-a-value-js-gives-is-checked-shallow-first.md):
what JS gives Rust back, not only what it gives it, is checked.

Case ([0262](0262-when-rust-and-js-disagree.md)): **C**, as 1192's.

## Context

ADRs 1192 and 1194 check what a JS caller gives a function's parameters.
JS also gives Rust values another way: what a function returns.
- **A callback JS gives**, an `impl Fn(u32) -> u32` rule, returns into
  Rust's code: a "discount" of `-100` adds 100.
- **A binding**, `#[rust_js::link_name]`, is Rust's word for a JS
  function's type, which nothing held it to (chapter 24 of
  coming-from-javascript pins it): `formatMoney` giving `42` where Rust
  declared a `String`.

## Decision

**A callback JS gives is a function, and where Rust reads what it gives
back, it's wrapped at entry, each call's result checked where it returns
(`boundary/callbacks.rs`):**

```js
export function discounted(price, rule) {
  $checkU32(price, "discounted(price)");
  rule = $checkCallback(rule, "discounted(rule)", $checkU32);
  return discounted$(price, rule);
}
```

- **What a callback is:** an `impl Fn(..) -> R`, a type parameter the
  signature takes a JS function of (`callables`), a `dyn Fn`, or a `fn`
  pointer.
- **Its result is checked by its type's checker,** 1192's or 1194's:
  `discounted(rule) returned -100, expected a u32`, and of a struct,
  `pick(make) returned "3" (qty), expected a u32`.
- **One whose result is `()` or `!`,** which Rust ignores, or that may be
  any value, is checked to be a function and given as it is,
  `$checkCallback(visit, "each(visit)")`. A wrapper is another function,
  which JS can tell apart where Rust gives it back, to a
  `removeEventListener` say, so a function is wrapped only where its result
  is read.
- **A Rust caller's closure is given as it is, to the body** (ADR 1193):
  `discounted$(n, (p) => ..)`.

**The crate's own binding's result is checked where its call returns
(`boundary/results.rs`):**

```js
$checkResult(width(good), "./box.js#width", $checkU32)
```

- The error names the binding's JS:
  `./box.js#width returned -1, expected a u32`. An `Option` result's
  `None` is its form's: `#[rust_js::nullable]`'s `null`, `none_undefined`'s
  `undefined`, else either.
- **Of a result of a number, a `bool` or a fieldless enum,** this step's
  part.
- **Not a binding crate's,** `js`'s, `webapi`'s, `react`'s: see below.

**Why the binding crates are trusted, and a crate's own bindings aren't.**
The two make different kinds of claim:
- **`js`, `webapi` and `react` are generated from the platform's own
  declarations:** WebIDL for the DOM, TypeScript's lib for JS's built-ins,
  React's published types. That `Math.floor` gives a number is the
  platform's guarantee, which every browser and runtime is held to, not
  something rust-js claims. Checking it would cost every call of every
  program, and catch nothing the platform doesn't already promise.
- **A crate's own `#[rust_js::link_name]` binding is a user's claim:** the
  author's word that `./box.js#width` gives a `u32`, about a module
  nothing else describes and nothing held to it. That claim is what's
  checked, where its call returns.

A binding crate's declaration can be wrong, as any generator's can; that's
a bug in the crate, fixed there, not a check every program pays for.

## Open: a binding's text result

A DOM string can hold a lone surrogate: an element's `textContent` of
odd input, a file name. A Rust `String` can't (ADR 1192 checks a
parameter's `isWellFormed()`). Two answers, for the user:

1. **Lossy, as wasm-bindgen does:** a binding's text result gets
   `toWellFormed()`, each lone surrogate U+FFFD. This is a conversion at
   the binding's boundary, close to an ADR 0262 exception. Nothing throws,
   and text with a lone surrogate is changed.
2. **A refusal:** the result is checked as a parameter is, and throws.
   A binding that can't promise well-formed text declares its result
   `&js::JsString`, JS's own string, which Rust reads by its methods and
   converts where it chooses.

Until it's decided, a binding's `String`, `&str` or `char` result, or one
that holds them, is unchecked, as before.

## Why

- **Rust's code never reads an impossible value a JS function gave back.**
- **Wrapped only where the result is read,** so a function's identity is
  kept wherever it can be.
- **Bindings are checked where they're the program's own word,** and the
  binding crates are trusted, as their declarations are the platform's.

## What holds it

- `test/boundary-callbacks.test.ts`:
  - a callback wrapped and its result checked;
  - a non-function refused;
  - a `()` callback given as it is;
  - a `fn` pointer's struct result checked with its path;
  - a Rust caller's closure given to the body;
  - a binding's result checked, and its text left alone.
- The mutations in `scripts/mutations/lower/boundary/callbacks.ts` and
  `results.ts`.
