# 0851. A closure of rest arguments as a value is JS's rest parameter

Status: Accepted. Amends [0667](0667-console-is-a-value-and-rest-arguments-are-one.md),
whose closure taking rest arguments as a value, and `dyn Fn` of them,
were refused; and [0664](0664-a-functions-rest-slice-is-jss-rest-parameter.md),
whose `#[rust_js::rest]` on a closure's parameter is refused still, its
error naming this form. Extends [0196](0196-typescript-declarations.md):
an `impl Fn` a function gives is declared as its function.

Case ([0262](0262-when-rust-and-js-disagree.md)): N where a call's type
says it gives rest arguments, which it spreads; D, refused, where a call
couldn't say so, a type parameter's value.

## Context

React's `cache(fn)` (`packages/react/src/ReactCacheImpl.js`) gives JS a
function its callers call with their arguments one by one:

```js
export function cache(fn) {
  return function () {
    // .. each of `arguments`, a key of the cache's
    const result = fn.apply(null, arguments);
    // ..
  };
}
```

A port's closure couldn't take them. `#[rust_js::rest]` is a function's
(ADR 0664) and refused on a closure's parameter, as a closure is given
what its type says: a `Box<dyn Fn(&[T])>` or an `impl Fn(&[T])` is called
from Rust with one slice, which a rest parameter would gather as one
array. Without it, `|args: &[Option<&Unknown>]|` is one array argument,
wrong for each JS caller of `cached(a, b)`. And ADR 0667 refused a closure
taking rest arguments as a value, `&dyn js::Rest<T>`, as nothing could
read them, and a `dyn Fn` of them, whose call gave them as one array.

## Decision

**A closure's last parameter of rest arguments as a value, `args: &dyn
js::Rest<T>`, is JS's rest parameter, `(...args) =>`; it reads them as
the slice they deref to, of what each `T` reads as (`js::RestItem`), and
every call through a type whose last parameter is one spreads them: the
closure's own, a `dyn Fn`, a `Box`, a `fn` pointer, a generic `F:
Fn(&dyn Rest<T>)`, and the `impl Fn` a function gives.**

```rust
pub fn cache(r#fn: &'static AnyFunction) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown> {
    move |args: &dyn Rest<dyn IntoUnknown>| {
        // ..
        for &arg in args.iter() {
            // ..
        }
        // ..
        match panic::catch_unwind(AssertUnwindSafe(|| function::apply(r#fn, Some(&Kind::Null), Some(&args)))) {
            // ..
        }
    }
}

let count = |args: &dyn Rest<dyn IntoUnknown>| args.len();
count(&(1, "a", 2)) + count(&v)
```

```js
export function cache(fn) {
  return (...args) => {
    // ..
    for (const arg of args) {
      // ..
    }
    // ..
    try {
      const result = fn.apply(null, args);
      // ..
    }
  };
}

count(1, "a", 2) + count(...v)
```

- **The type says the call.** A closure's parameters are its type's, which
  every caller sees however it's held, a `Box<dyn Fn(..)>` too, so the
  rest is the parameter's type, rest arguments as a value (ADR 0667), not
  an attribute no `dyn` keeps. A call of it gives them as a variadic
  binding's call does, one answer, `calls::spread_rest`: a tuple written
  out its items, `count(&(1, "a"))` `count(1, "a")`, `&()` none, a slice
  or a `Vec` held spread, `count(&v)` `count(...v)`.
- **Read, they're the slice they deref to**, the array itself: the js
  crate's `Deref` of a `dyn Rest<T>`, whose `Target` is `[T::Item]`,
  `js::RestItem`'s, `Option<&'static Unknown>` of any value's, `&str` of
  `str`'s, a number's own type: what JS gives, of any lifetime, as what
  `js::get` gives is. `args.iter()`, `args.len()`, `args[0]` are the
  array's (`bindings::reads_rest_value`). Passed on, `function::call(f,
  None, args)` is `f.call(undefined, ...args)`.
- **Its `.d.ts` is a rest parameter of what each reads as**, `(...args:
  (unknown | null | undefined)[]) => ..`, of a closure's type, a `dyn
  Fn`'s and a `fn`'s (`bindings::rest_items`); and an `impl Fn(..)` a
  function gives is the function it says, as one it takes is, where it
  was `any`.
- **Refused, each with what to write**:
  - before a closure's last parameter: "JS's rest parameter is a
    function's last";
  - a function's, which a `fn` pointer of it would be given as JS gives
    them, but whose rest is its last slice marked `#[rust_js::rest]`
    (ADR 0664), one way for a function;
  - as a type parameter's value, `call::<&dyn Rest<T>, _>(count, &(1,
    2))`: what it's given to calls an `F: Fn(A)`, whose type says one
    value, so its call gives one array (`calls::check_rest_unspread`). A
    binding's `impl Rest<T>`, which spreads them, takes them.
  - `#[rust_js::rest]` on a closure's slice: "a closure takes rest
    arguments as a value, `args: &dyn js::Rest<T>`, which its type says".

## Why

- **It's what React writes**, a function JS calls with its arguments, its
  callers' `cached(a, b)`; and the rest parameter reads as JS written
  today does, `(...args) =>`, where upstream's `function () { ..
  arguments .. }` avoids one only for its old transpiler (its own
  comment). An arrow has no `this` of its own, and upstream's never reads
  it; `fn.apply(null, args)` gives its `null`.
- **It's sound however the closure is held**: the type every caller
  calls through says it spreads, where an attribute on the closure's
  parameter would be lost the moment it's a `dyn Fn`, and its Rust
  callers would give JS's rest parameter one array.
- **It's rest arguments as a value, ADR 0667's one form**, which a
  variadic binding takes and a `fn` of a variadic binding is: the same
  type for the closure JS calls and the binding Rust calls.

## Alternatives

- **`#[rust_js::rest]` on a closure's slice**, as a function's: right
  where the closure is called by its own type, wrong through every `dyn
  Fn` and `impl Fn`, which ReactCacheClient's `Box` of either `cache`
  is. Each erased call would need refusing, which a `Box<dyn Fn(&[T])>`
  can't tell.
- **`function () { .. }` reading `arguments`**: JS's `arguments` is no
  array, so a body reading it as a slice, `.entries()`, `$slice(..)`,
  would break; and a `function` has a `this` of its own, which the
  closure doesn't read.
- **A function's rest as `&dyn js::Rest<T>` too, one form for both**: a
  function's slice runs as native Rust does, each of its callers'
  `total(&[1.0, 2.0])` a slice's, which a corpus case compares; rest
  arguments as a value are the js crate's, which runs nowhere but JS.
  ADR 0664's attribute stays the function's; a closure is a value, whose
  type is what's called.
- **A wrapper made at each coercion to a `dyn Fn(&[T])`**, `(x) =>
  f(...x)`, as a rest function's value is (ADR 0664): Rust's callers
  would be right, but JS's, which a `Box` is given to, would be given the
  wrapper, of one array.

## Consequences

- Rust's callers through a `dyn Fn` give a tuple or a slice by
  reference, `f(&(a, "b"))`, `f(&&held[..])`: `&[T]` itself isn't one,
  as a variadic binding's isn't (ADR 0640).
- A JS rest parameter can't be named `arguments`, which strict mode
  reserves: a port names it `args`, where upstream reads `arguments`.
- The js crate's `Deref` of `dyn Rest<T>` is a binding crate's impl for
  a `dyn` type: its call isn't the `dyn`'s dictionary's, which ADR 0667's
  `dyn Rest` has none of.

## What holds it

- `test/rest-parameters.test.ts`, "a closure of rest arguments as a
  value is JS's rest parameter: React's cache": the port of
  ReactCacheImpl, its JS and `.d.ts`, run against upstream's of each kind
  of argument, `this`, and each kind of throw; "a call of a closure of
  rest arguments spreads them, through any type that says so", each call
  run; and each refusal, by its message. `test/js-object.test.ts`, a
  `dyn Fn` of rest arguments called spread.
- Mutations in `scripts/mutations/lower/mir.ts`, `mir/calls.ts`,
  `support.ts`, `traits/calls.ts`, `bindings.ts`, `calls.ts` and
  `declarations.ts`.
