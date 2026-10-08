# 0283. The js crate covers TypeScript's ES library, and a test holds it

Status: Accepted. Extends [0102](0102-js-and-webapi.md) and [0281](0281-webapi-covers-typescripts-dom.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

[ADR 0281](0281-webapi-covers-typescripts-dom.md) holds the `webapi` crate
to TypeScript's DOM. The js crate, JS's own library, had what ports had
needed: JSON, a regular expression's `test`, a `Uint8Array`'s length. It
had no `Date`, no `Intl`, no typed array but bytes, no `WeakMap`, no
`Symbol`: what ReScript's Stdlib and TypeScript's `lib.es*.d.ts` have.

Most of JS's library is Rust's already. `Vec` is an `Array`, `str` a
string, `f64` a number with `Math`'s functions, `HashMap` a `Map`
(ADRs 0034, 0036, 0059). What's left is what Rust has no type for.

## Decision

**The js crate's goal is every member of the built-ins Rust has no type of
its own for, as TypeScript's ES2024 libs declare them; a test measures it.**

- **The built-ins measured** (`builtins/coverage.ts`'s `SCOPE`): `Date`,
  `RegExp`, `Error`, `Promise`, `Symbol`, `ArrayBuffer`,
  `SharedArrayBuffer`, `DataView`, the eleven typed arrays, `WeakMap`,
  `WeakSet`, `WeakRef`, `FinalizationRegistry`, `Proxy`, `Reflect`,
  `Atomics`, `JSON`, `Intl` and its classes, and the global functions
  (`parseInt`, `encodeURIComponent`, ..). Not `Array`, `String`, `Number`,
  `Math`, `Map`, `Set` nor `Object`: Rust's own types are those.
- **The reference** is TypeScript 5.9's `lib.es5` to `lib.es2024` files,
  pinned as the `typescript-es` package, read with TypeScript's parser: a
  built-in's interface's members, its constructor interface's (`Date.now`
  a static), and `new` where it has one.
- **A ratchet**: `builtins/coverage.txt` lists each member bound;
  `test/builtins-coverage.test.ts` fails when one isn't any more, or a new
  one isn't blessed, as webapi's does.
- **The bindings are written, not generated** (ADR 0116), as ReScript's
  Stdlib is: a built-in is a type, its members its methods (`date.get_time()`),
  its constructors and statics a module of its name (`date::now()`), as
  webapi's are (ADR 0282). Rust's own types' JS functions, `string::trim`,
  stay functions: Rust's `str` can't have methods of another crate's.

## Why

- **The same promise as the DOM's**, measured the same way.
- **Only what's missing is counted**: a program reaches `Math.sin` as
  `x.sin()`; measuring `Math` would count what Rust has as missing.

## Alternatives

- **Generate them from the `.d.ts`**: rejected for bindings (ADRs 0116,
  0119). TypeScript's overloads and generics have no one Rust a generator
  can choose; `Date` and `Intl` are few enough to write.
- **Measure all of JS's library**: a number that says `Array.prototype.at`
  is missing where `v.get(i)` is it.

## Consequences

- `typescript-es`, TypeScript 5.9.3, is a dev dependency beside TypeScript 7,
  which ships no lib files.
- At first 14 of 691 members, 2.0%.

## Since

- **`Date`**, all 47 of its members: `date::new_with_time(ms)`,
  `d.set_utc_hours(12.0)`, `date::now()`, `date::utc(..)`, numbers `f64`s as
  an invalid date's are `NaN`, and `to_iso_string()` a `Result` of the
  `RangeError` an invalid date throws. The crate's files beyond `lib.rs` are
  listed for its packages (`tooling/resources.js`). 8.8%.
- **The typed arrays, `ArrayBuffer`, `SharedArrayBuffer` and `DataView`**,
  every member: each element a Rust number of its kind, a `BigInt64Array`'s
  an `i64` (ADR 0086); `a.get(i)` and `a.set_index(i, x)` are `a[i]` and
  `a[i] = x`; a callback takes the element, `a.map(|x| x * 2.0)`, its index
  an `enumerate()` of `values()` away; `keys`, `values` and `entries` are
  JS iterators (ADR 0140). The eleven are one shape, written once in
  `builtins/typed_arrays.ts`, which writes `src/typed_arrays.rs`; a test
  holds the file to it. `uint8_array::new(buffer)` is `new_with_buffer`, as
  `new(length)` is the length's. 78.6%.
- **Checked against ReScript's Stdlib and TypeScript's libs, and adapted:**
  - `Date`: ReScript gives a part as an `int` both ways, which an invalid
    date's `NaN` makes unsound; here what a date gives is an `f64` and
    what it's given an `i32`. Its constructor of each arity,
    `makeWithYMD`, is `date::new_with_ymd`; its setters of several parts,
    `setHoursMS`, are `set_hours_m_s`; `Date.UTC` likewise. Its
    `toISOString` is a `string` that throws; here a `Result`.
  - Typed arrays: ReScript has one `TypedArray.t<'a>`, so an `Int8Array`
    is an `Int32Array`; here each is its own type, as TypeScript's are.
    From ReScript, each callback's `_with_index` form, a comparator that
    gives an `Ordering` (-1, 0 or 1 in JS, ADR 0057), `copy`,
    `slice_to_end`, `subarray_to_end`, `fill_range` and `index_of_from`.
  - `RegExp`'s members are methods, its match a `RegExpMatch` with
    `full_match()` and `get(i)`, and `reg_exp::escape`, as ReScript's.
  - `Error`'s `name`, `cause` and `stack`, `js_error::new`; `Symbol`, its
    registry and its well-known symbols as statics; the global functions.
  - `WeakMap`, `WeakSet`, `WeakRef` and `FinalizationRegistry`, whose keys
    are `WeakKey`s: JS throws on a string as one, so Rust doesn't take
    one, and a struct is one as its `unsafe impl` says.
- **What TypeScript marks `@deprecated` isn't counted**: `RegExp.$1` and
  the rest of Annex B's legacy, by the interface it's on (`String.sub` is,
  `Atomics.sub` isn't). 88.1% of 675.
- **`Promise`**, every member, after ReScript's `Stdlib_Promise`:
  `promise::new(|resolve, reject| ..)`, `then` of a closure that gives a
  promise (`then_resolve` of one that gives a value, as ReScript's
  `thenResolve`), `catch`, `finally`, and `resolve`, `reject`, `race`,
  `any`, `all` and `with_resolvers` in its module. `all_settled` gives
  `PromiseSettledResult`s, a union tagged by `status` (ADR 0284), as
  TypeScript's `PromiseSettledResult` is and ReScript's `settledResult`.
  ReScript's `all2` to `all6` are what `Promise.all` makes of a tuple; here
  `all2` to `all4`, a tuple of promises to a promise of a tuple. A
  rejection's reason is a `JsError`, as `settle`'s is, where TypeScript has
  `any` and ReScript `exn`. 89.6%.
- **The measure counts what it missed**: a constructor interface in
  `namespace Intl` ends at its own brace, a `new` after its doc comment is
  one, and an interface's members are those of what it extends too
  (`Intl.Locale`'s `region` is `LocaleOptions`'). 87.8% of 689.
- **`Reflect` and `Proxy`**, every member, after TypeScript's; ReScript has
  neither. Each is of a JS value of any shape, an `Unknown`, as `get` and
  `set` are, a key a `PropertyKey`. A proxy is an `Unknown`, not its
  target's type: its traps may give anything, which a Rust type's fields
  wouldn't be. A `PropertyDescriptor` and a `ProxyHandler` are made empty,
  `{}`, and given what's set, as JS tells what's left out from what's
  `undefined` there: a descriptor's `writable` left out is the property's
  as it was. `set_prototype_of`'s `None` is `null` (ADR 0275); a
  `getPrototypeOf` trap gives an object, since a closure's `None` is
  `undefined`, which JS throws of. 90.0%.
