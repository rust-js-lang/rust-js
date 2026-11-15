# 0030. `Option`: `Some(x)` is `x`, `None` is `undefined`

Status: Accepted. Changes [0024](0024-web-crate.md): results that may be
`null` are `Option`s now.

Case: A, C, B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

The web platform answers "nothing here" with `null`:
`document.getElementById("app")` when there's no `#app`, `textContent` of a
document. The webapi crate typed those results as if they were never null (ADR
0024), which isn't honest. Rust says "maybe nothing" with `Option`, and rust-js
couldn't compile it. How the others do it (checked in local clones):

- **ReScript**: `Some(x)` is `x` itself, and `None` is `undefined`. Only
  when that would be ambiguous (`Some(None)`) is the value boxed, as
  `{BS_PRIVATE_NESTED_SOME_NONE: 0}` (`Primitive_option.res`). JS's
  `null` is a separate type, `Nullable.t`, converted with `fromNullable`.
- **Scala.js**: Scala's `Option` stays a class, `Some` and `None`. At the JS
  boundary there's `js.UndefOr[A]`, which is `A | Unit`: a value or
  `undefined`.

## Decision

**`Some(x)` is `x`, and `None` is `undefined`**, as in ReScript, but with
`null` counting as `None` too, so the web platform's `null` needs no
conversion:

| Rust | JS |
|---|---|
| `Some(x)`, `None` | `x`, `undefined` |
| `match o { Some(0) => .., Some(n) => .., None => .. }` | `if (o === 0) .. else if (o != null) .. else ..` |
| `if let Some(el) = find() { .. }` | `const el = find(); if (el != null) { .. }` |
| `while let Some(h) = half(n) { .. }` | `while (true) { const h = half(n); if (h != null) { .. } else { break; } }` |
| `o.is_some()`, `o.is_none()` | `o != null`, `o == null` |
| `o.unwrap()`, `o.expect("why")` | `$unwrap(o)`, `$unwrap(o, "why")`, which throw Rust's message on `None` |
| `o.unwrap_or(d)` | `o ?? d` |
| `o.map(\|x\| x + 1)`, `o.map(f)` | `o != null ? o + 1 : undefined`, `o != null ? f(o) : undefined` |
| `a == b` on options of numbers, strings, `bool` and fieldless enums | `a == b` |
| `a == b` on options of structs | `$eq(a, b)`, where `null` and `undefined` are equal |

- **`!= null`, not `!== undefined`**: JS's loose comparison with `null` is
  true for both `null` and `undefined`, so `None` from Rust and "nothing"
  from the DOM are the same. It's also how JS programmers write "is there
  one".
- **`Option<T>` needs a `T` that's never `undefined` or `null` itself**, or
  `Some(x)` and `None` would be the same value. `Option<()>`, `Option` of a
  unit struct, and `Option<Option<T>>`, looking through references
  (`Option<&()>` too), box such a `Some`, as ReScript does, and as generic
  code does where `T` might be one ([0051](0051-generic-options.md)).
  (Amended: they were errors.)
- **`unwrap_or`'s argument runs even when it isn't needed**, as in Rust. `??`
  skips it, so an argument with effects is computed first, in order:
  `const option = half(n); const fallback = bump(); option ?? fallback`.
- **`map` puts a closure's body in place**, with the option for its
  parameter, as in `o != null ? o + 1 : undefined`. An option that's computed
  goes in a `const` first, named like the parameter, so it's computed once. A
  function, or a closure with statements, is called: `f(o)`. Mapping to a
  type that can itself be `undefined` or `null`, like `()`, boxes it
  (ADR 0051). (Amended: it was an error.)
- **`if let`** is new, and works with any pattern (`if let (0, y) = p`).
  When its value isn't already in a variable, it goes into a `const` named
  like the pattern's variable, which that variable then just is.

**The webapi crate's results that may be `null` are `Option`s**, 78 of them:
`document::get_element_by_id(..) -> Option<&Element>`,
`node::text_content(..) -> Option<String>`. Programs say what happens when
there's nothing:

```rust
let app = document::get_element_by_id(document, "app").expect("the page has an #app");
```

```js
const app = $unwrap(document.getElementById("app"), "the page has an #app");
```

## Why

- **No wrapper objects**: an `Option<&Element>` is the element or nothing, as
  the DOM itself gives it. The JS reads like the checks people write.
- **One representation for Rust's `None` and JS's `null`**, so bindings pass
  results straight through, with no `fromNullable` at every call.
- **Honest types**: a result that can be missing says so, and rustc makes
  the program deal with it, which the old non-null types skipped.

## Alternatives

- **`None` is `null`**: the DOM's own value. But JS's optional things
  (missing properties, `Array.prototype.find`, default parameters) are
  `undefined`, and so is ReScript's `None`. With `!= null` it doesn't
  matter which one arrives.
- **`{ tag: "Some", value }` objects, like any enum with fields**: faithful to
  Rust in every case, but an allocation per `Some`, and nothing a JS caller
  would expect.
- **A separate `Nullable<T>` type for the webapi crate**, as ReScript has:
  another type and a conversion at every call, to tell apart two things
  rust-js doesn't need to tell apart.

## Consequences

- The examples say `.expect("the page has an #app")` where they used to
  assume the element, and their tests `.unwrap()` what they query.
- `{:?}` of an option prints the value for `Some` and `()` for `None`, since
  `$debug` can't tell `None` from `()` at run time. `assert_eq!` messages on
  options read that way too.
- `?` on an `Option` came with ADR 0035. Not yet: `and_then`, `ok_or`, `take`, `as_ref`,
  `as_mut`, and nested options. Other enums with fields are tagged objects
  (ADR 0033); `Option` is the special case that needs no tag.

## Since

- **`if x.is_none() { x = Some(e) }` is `x ??= e`**: the same test, and `e`
  made only where `x` is none, as react.dev's errors page caches the codes
  it fetched, `cachedErrorCodes ||= ..`.
- **A property of what's in it, `o.map(|e| e.id)`, is `o?.id`**, reading
  `o` once as a `const` of a getter's would: `ref.current?.offsetHeight`,
  as react.dev's SocialBanner has it.
- **A chain with a `?.` in it is one chain**, `o?.inner.v`: each `?.` ends
  all of it where its object is `undefined`, as `map(|x| x.inner.v)` is
  `None`. (Amended: each `?.` was a chain of its own, `(o?.inner).v`, which
  read `.v` of `undefined` and threw.)
- **A method of what's in it, of a JS object, is `o?.m(x)`**, which calls
  it, its arguments too, only where `o` is, as a person writes
  `document.body?.appendChild(a)`. A Rust type's method takes the value as
  an argument, `Node.add(o, x)`, which `?.` can't skip: a test still.

## Amended by

- [0193](0193-some-of-a-unit-variant.md): `Some` of a unit variant is tested as the variant
- [0298](0298-truthy-options.md): An Option of a value never falsy is tested by its truth

## Amendment: `===` to a `Some`

**An `Option` compared with a `Some(..)` is `===`**, `p.as_deref() ==
Some(s)` `p === s`, as react.dev's getRouteMeta writes `currentRoute.path
=== searchPath`: the `Some` is never `null` or `undefined`, so `===` and
`==` answer the same, and `===` is what a person writes. Two `Option`s
keep `==`, as `null` and `undefined` are both `None`. The MIR lowering
knows the locals only ever made `Some(..)`, and a shared reference to
one (`OptionWrites::made_some`); `eq_value` writes `==` only of a
payload that's its `Some`, so that's the one question asked. Decided
with the user. Case N: the same answer.

## Amendment: `$unwrap` stays the one `expect` (rd661, the user's decision)

Decided by the user (2026-11-04): `$unwrap(o, "why")` stays the one form
of `expect`. ReactSharedInternalsServer's thread-local of its binding's
`expect` (ADR 0633, amended, rd661) is `$unwrap(..)` where upstream
writes `if (!ReactSharedInternalsServer) { throw new Error(..); }`; that
file's difference is accepted (react-in-rust's PORTING.md lists it), not
a second form of `expect` for a module's load check. Case N
([0262](0262-when-rust-and-js-disagree.md)): both throw `new Error` of
the same message as the module loads, and the internals, an object, are
falsy only where they're absent.

## Amendment: a promise's payload that may be `null` is an `Option` (2026-11-14, #1768)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change; Rust's
behavior is preserved.

The webapi generator made a function's result an `Option` where WebIDL's
type may be `null`, but not a promise's payload: `Promise<GPUAdapter?>`
was `Promise<&'static GPUAdapter>`, so a program couldn't ask whether the
browser gave an adapter, and used one that's `null` as one that's there.
A payload's nullability is the result's own rule, `nullableType`, of the
type or its typedef, asked by both: `Promise<Option<&'static GPUAdapter>>`,
and of the seven functions webref's IDL gives one, `pop_error_scope`'s
`GPUError?`, the credentials', the cookie store's, a push subscription's
and `FileSystemHandle::resolve`'s `sequence<USVString>?`. Awaited, its
`None` is the `null` JS gives, read as any `Option` is. The generator has
no mutation list; `test/webapi-promises.test.ts` holds the generated
signature and both answers.

## Amended (2026-11-15): Rust's one `None` where it leaves for JS as `undefined` (#1840, #1843)

A wrong answer (P0, #1840, #1843). An `Option` JS gives, a public
function's parameter, is declared `T | null | undefined`, and `is_none` is
true of either, so inside Rust's code the two read alike. But given on to
a native optional argument as it was, JS took `null` for a value:
`(1.25).toPrecision(null)` threw a RangeError where Rust's
`to_precision(1.25, None)` is "1.25", and `Intl.DateTimeFormat#format(null)`
gave 1970 where `None` means now. A `#[rust_js::nullable(..)]` parameter,
whose `None` is `null` (ADR 0861), given on to a binding's optional
argument was the same (#1843). Rust has one `None`.

**Where a `None` leaves for JS as `undefined` alone, what may hold one JS
gave is given `value ?? undefined`.** One owner answers whether it may,
`analysis/nulled_params/js_nones.rs`, asked through `mir::nulled`'s
`none_trusted`: a flow-insensitive taint of every body's MIR, unknown taken
as may, so the answer is sound by construction. Its sources are what JS or
code not seen gives: a parameter of what JS or a consumer calls
(`js_facing`, `library::reachable`, a trait's impl among them), whose
fields an object JS made holds are read through it; a binding's result
that declares neither form, or `null`, and what it writes through a
`&mut`; a nullable parameter, field, result or thread-local; a closure or
function given where its callers aren't seen, to a binding, a `dyn`, a fn
pointer or the crate's own generic function; a call through a `dyn` or a
type parameter's; another crate's result; and std's
result of what it's given of a type parameter, a `dyn` or an opaque type. They flow through moves, copies and aggregates,
both ways through what shares what it points at (a `&mut`, a shared
reference to what isn't `Freeze`, an `Rc`), through the crate's calls,
arguments to parameters and returns to results, and through std's calls by
the `Option` types each value carries, so `position()`'s fresh `None`
holds Rust's. The exits:

- a binding's argument it declares an `Option`, not a nullable one's, nor
  a type parameter's, `js::set`'s `T`, which JS takes as it's given; what
  `function::bind` binds of a binding, its `.bind(..)` kept; and a binding's
  function as a value, `values.map(text)`, whose arrow gives each so;
- each write of what a function JS calls returns, not a nullable one's
  (ADR 0841), at the write, so its returns are structured as before;
- a write of a `pub` or another crate's struct's plain `Option` field,
  which JS reads of the object;
- an argument a callback that may be JS's is given: a `dyn Fn`, a generic
  `Fn` or a pointer that `FnPointers` says may be a JS function, of a value
  the taint says JS may have given; and a trait object's method where JS
  may have made its pair.

Unchanged: a nullable place, which stores `null` (ADR 0275); an `Option`
of a value of any shape, `Option<&Unknown>` or `Option<&dyn IntoUnknown>`,
whose `undefined` and `null` are each a value of JS's own (rd275, #443);
`js::null()`, JS's `null` on purpose (ADR 0935); what's never `null`, a
literal, an object, `Some(..)` or `None` made; and comparisons, which read
the two alike. A Rust callee behind a `dyn` or a generic is no exit: it
holds what it's given as it may be and gives Rust's at its own exits.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's. The react-in-rust port gains 198 `?? undefined` in 72 files, most
of a nullable place's `None` or a callback JS calls, which are needed; the
corpus 10 snapshots. Approach (a), making a `None` JS gives Rust's where
it's first held, was measured at 900 in 147 files and rejected. Trusting a
binding's result its declaration types `T | undefined`, and a plain
`Option` parameter's or field's as #1845 declares it, are follow-ups.

Held by `test/none-given.test.ts` (Bun and Chromium, and a library a
consumer gives JS's `null`), and the corpus case
`none_given_to_an_optional_argument`, against native Rust: a nullable
parameter, field, result and thread-local given to `String(value)` through
a function, a trait object, `map`, a fn pointer and a `dyn Fn`. Mutations in
`analysis/nulled_params/js_nones.ts`, `mir/nulled.ts`, `mir/dispatch.ts`,
`mir/calls.ts`, `mir/assign.ts`, `mir/structure.ts`,
`mir/rvalues/aggregate.ts`, `items.ts` and `binding_calls.ts`.

## Amendment: a `Some` of what JS threw may be a `None` JS gave (2026-11-15, #1939)

A caught `null` or `undefined` is a `JsError`'s `None`. So an `Option`
made of a `JsError`, `.err()` of a `Result<_, &JsError>`, `.ok()` of a
`Result<&JsError, _>`, `Option::from(e)`, `Some(e)` in a match's arm, a
field written `held.error = Some(e)` or a struct made `Held { error:
Some(e) }`, may hold JS's `null`. An export gave it as it was, `null`,
where #1840's exits give Rust's `None` as `undefined` alone: a typed
caller's `=== undefined` was wrong (P0).

- `nulled_params::JsNones`, the one owner of which `Option`s may hold a
  `None` JS gave, counts a `JsError` as holding one of its own: a local
  holding one is tracked, a binding's `Result` carries it, and each
  `Option` made of it is tainted as what it's made of is.
- A `Some` whose payload may be that `null`, a `JsError` the taint
  reaches (`JsNones::payload_may_be_null`, its one question: a payload of
  another type is no `None`, whatever `None`s it holds), isn't trusted as
  Rust's own (`made_some_or_none`), and one written to a function's
  result is given at its exit as any `Option` is, its value no longer
  `Known::Present`: `return e ?? undefined;`, `{ error: e ?? undefined }`.
- A binding given one leaves it out for `None`, `e != null ? fallback(e)
  : fallback()`, so its default parameter is its. React's port: the five
  Flight server Node entries' busboy handlers, `error != null ?
  busboyStream.destroy(error) : busboyStream.destroy()` where they gave
  `busboyStream.destroy(error)` a caught `null`, its pin moved with this.

Case of [0262](0262-when-rust-and-js-disagree.md): results change, so
Rust's behavior. Held by js-errors (each of the six ways, `throw null`,
`throw undefined` and an `Error`, of the crate as a program and as a
library; a binding's default parameter; `thrown()`'s `=== undefined`);
mutations js-error-holds-no-none (`js_nones.rs`),
some-of-a-caught-null-trusted (`one_none.rs`),
some-of-a-caught-null-returned-as-is (`assign.rs`).
